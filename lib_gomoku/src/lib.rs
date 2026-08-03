use pyo3::prelude::*;
use std::collections::HashMap;
use colored::*;

pub mod heuristic;
pub mod minimax;

#[pyclass]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MoveResult {
    Valid = 0,
    OutOfBoard = 1,
    NotEmpty = 2,
    DoubleThree = 3,
}

#[pymethods]
impl MoveResult {
    #[classattr]
    const VALID: MoveResult = MoveResult::Valid;
    #[classattr]
    const OUT_OF_BOARD: MoveResult = MoveResult::OutOfBoard;
    #[classattr]
    const NOT_EMPTY: MoveResult = MoveResult::NotEmpty;
    #[classattr]
    const DOUBLE_THREE: MoveResult = MoveResult::DoubleThree;

    // Add these methods:
    #[getter]
    fn name(&self) -> &'static str {
        match self {
            MoveResult::Valid => "VALID",
            MoveResult::OutOfBoard => "OUT_OF_BOARD",
            MoveResult::NotEmpty => "NOT_EMPTY",
            MoveResult::DoubleThree => "DOUBLE_THREE",
        }
    }

    fn __repr__(&self) -> String {
        format!("MoveResult.{}", self.name())
    }

    fn __str__(&self) -> String {
        self.name().to_string()
    }

    fn __eq__(&self, other: &Self) -> bool {
        self == other
    }

    fn __ne__(&self, other: &Self) -> bool {
        self != other
    }

    fn __hash__(&self) -> u64 {
        *self as u64
    }

    // Add this to get the integer value if needed
    #[getter]
    fn value(&self) -> i32 {
        *self as i32
    }
}

type Position = (i32, i32);
type Pattern = Vec<Position>;

struct LineScan {
    contig_my: i32,
    contig_open: bool,
    total_my: i32,
    empty_count: i32,
    hole: bool,
}

#[derive(Clone, Debug, Default)]
struct PlayerPatterns {
    open_two: Vec<Pattern>,
    open_three: Vec<Pattern>,
    free_three: Vec<Pattern>,
    open_four: Vec<Pattern>,
    block_four: Vec<Pattern>,
    five_row: Vec<Pattern>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PatternKind {
    OpenTwo,
    OpenThree,
    FreeThree,
    BlockFour,
    OpenFour,
    FiveRow,
}

fn classify(
    plus: &LineScan, 
    minus: &LineScan,
    center_stone: i32,
) -> Option<PatternKind> {
    let total = plus.total_my + minus.total_my + center_stone;
    match total {
        2 if plus.contig_open && minus.contig_open => Some(PatternKind::OpenTwo),
        3 if plus.contig_open && minus.contig_open && plus.empty_count + minus.empty_count < 3 => Some(PatternKind::OpenThree),
        4 if plus.contig_open && minus.contig_open => Some(PatternKind::OpenFour),
        4 if plus.contig_open ^ minus.contig_open => Some(PatternKind::BlockFour),
        5 => Some(PatternKind::FiveRow),
        _ => None,
    }
}

// pos가 pattern의 진짜 바깥쪽 끝(첫/마지막 좌표)일 때만 trim하고, kind별로 남은 모양을 재분류한다.
// OpenFour는 한쪽 끝이 막히면 BlockFour로 강등되고, FreeThree는 같은 kind로 유지된 채 줄어든다.
// 그 외 kind는 trim 규칙이 없어 pos를 포함하면 항상 통째로 제거된다.
fn endpoint_trim_rule(
    kind: PatternKind,
    pattern: &Pattern,
    pos: Position,
) -> Option<(PatternKind, Pattern)> {
    let is_first = pattern.first() == Some(&pos);
    let is_last = pattern.last() == Some(&pos);
    if !is_first && !is_last {
        return None;
    }
    let trimmed = || -> Pattern {
        if is_first {
            pattern[1..].to_vec()
        } else {
            pattern[..pattern.len() - 1].to_vec()
        }
    };
    match kind {
        PatternKind::OpenFour => Some((PatternKind::BlockFour, trimmed())),
        PatternKind::FreeThree => Some((PatternKind::FreeThree, trimmed())),
        _ => None,
    }
}

fn free_three_from_scan(dx: i32, dy: i32, x0: i32, y0: i32, plus: &LineScan, minus: &LineScan) -> Option<Pattern> {
    if plus.total_my + minus.total_my != 2 || plus.empty_count + minus.empty_count < 3 {
        return None;
    }

    let mut adjusted_plus_empty = plus.empty_count;
    let mut adjusted_minus_empty = minus.empty_count;

    if plus.hole && minus.empty_count == 2 {
        adjusted_minus_empty = 1;
    }
    if minus.hole && plus.empty_count == 2 {
        adjusted_plus_empty = 1;
    }

    let plus_end = adjusted_plus_empty + plus.total_my;
    let minus_end = adjusted_minus_empty + minus.total_my;

    Some((-minus_end..=plus_end).map(|i| (x0 + dx * i, y0 + dy * i)).collect())
}

fn free_three_from_capture_scan(dx: i32, dy: i32, x0: i32, y0: i32, plus: &LineScan, minus: &LineScan) -> Vec<Pattern> {
    let mut out = Vec::new();
    let plus_three = plus.total_my == 3 && plus.empty_count == 2;
    let minus_three = minus.total_my == 3 && minus.empty_count == 2;

    if plus_three || minus_three {
        if plus_three {
            out.push((0..=(plus.total_my + plus.empty_count)).map(|i| (x0 + dx * i, y0 + dy * i)).collect());
        }
        if minus_three {
            out.push((-(minus.total_my + minus.empty_count)..=0).map(|i| (x0 + dx * i, y0 + dy * i)).collect());
        }
    } else if !plus.hole && !minus.hole && plus.total_my + minus.total_my == 3 && plus.empty_count > 0 && minus.empty_count > 0 {
        let plus_end = plus.total_my + 1;
        let minus_end = minus.total_my + 1;
        out.push((-minus_end..=plus_end).map(|i| (x0 + dx * i, y0 + dy * i)).collect());
    }
    out
}

// fn capture_open_pattern(dx: i32, dy: i32, x0: i32, y0: i32, scan: &LineScan) -> Option<(PatternKind, Pattern)> {
//     let kind = classify(scan.contig_my, scan.contig_open, 0, true, 0)?;
//     let (lower, upper) = if kind == PatternKind::FiveRow {
//         (1, scan.contig_my)
//     } else {
//         (0, scan.contig_my + scan.contig_open as i32)
//     };
//     Some((kind, (lower..=upper).map(|i| (x0 + dx * i, y0 + dy * i)).collect()))
// }

impl Gomoku {
    fn scan_line(&self, sign: i32, dx: i32, dy: i32, x0: i32, y0: i32) -> LineScan {
        let mut contig_my = 0;
        let mut contig_open = false;
        let mut contig_done = false;
        let mut total_my = 0;
        let mut empty_count = 0;
        let mut hole = false;
        let mut i = 1;

        loop {
            let x = x0 + dx * i * sign;
            let y = y0 + dy * i * sign;

            if !self.is_on_board(x, y)
                || self.board[x as usize][y as usize] == self.opponent_player
                || empty_count == 2
            {
                break;
            }

            if self.board[x as usize][y as usize] == self.current_player {
              // 내 돌
                if !contig_done {
                    contig_my += 1;
                }
                if empty_count > 0 {
                    hole = true;
                }
                total_my += 1;
            } else {
              // 빈칸
                if !contig_done {
                    contig_done = true;
                    contig_open = true;
                }
                empty_count += 1;
            }
            i += 1;
        }

        LineScan {
            contig_my,
            contig_open,
            total_my,
            empty_count,
            hole,
        }
    }

    fn patterns_mut(&mut self, kind: PatternKind, player: &Stone) -> &mut Vec<Pattern> {
        let p = self.patterns.get_mut(player).unwrap();
        match kind {
            PatternKind::OpenTwo => &mut p.open_two,
            PatternKind::OpenThree => &mut p.open_three,
            PatternKind::FreeThree => &mut p.free_three,
            PatternKind::BlockFour => &mut p.block_four,
            PatternKind::OpenFour => &mut p.open_four,
            PatternKind::FiveRow => &mut p.five_row,
        }
    }

    fn register(&mut self, kind: PatternKind, player: &Stone, pattern: Pattern) {
        let list = self.patterns_mut(kind, player);
        if !list.contains(&pattern) {
            list.push(pattern);
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
#[pyclass]
pub enum Stone {
    Empty,
    Black,
    White,
}

pub fn position_name(pos: &(i32, i32)) -> String {
        let (y, x) = pos;
        let x = "abcdefghijklmnopqrstuvwxyz".chars().nth(*x as usize).unwrap_or('-');
        let y = y + 1;
        format!("{x}{y}")
}

fn print_pattern_kind(name: &str, patterns: &[Pattern]) {
    let rendered: Vec<String> = patterns
        .iter()
        .map(|pattern| {
            let positions: Vec<String> = pattern.iter().map(position_name).collect();
            format!("[{}]", positions.join(","))
        })
        .collect();
    println!("  {name}: {} {}", patterns.len(), rendered.join(" "));
}

impl std::fmt::Display for Stone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let c = match self {
            Stone::Empty => '.',
            Stone::Black => 'X',
            Stone::White => 'O',
        };
        write!(f, "{c}")
    }
}

impl Into<String> for Stone {
    fn into(self) -> String {
        self.to_string()
    }
}

#[derive(Clone, Debug)]
#[pyclass]
pub struct Gomoku {
    size: usize,
    board: Vec<Vec<Stone>>,
    pub current_player: Stone,
    opponent_player: Stone,
    capture_count: HashMap<Stone, i32>,
    patterns: HashMap<Stone, PlayerPatterns>,
    win_capture_count: i32,
    current_move: Option<Position>,
    move_count: usize,
}

#[pymethods]
impl Gomoku {
    #[new]
    #[pyo3(signature = (size = 19))]
    pub fn new(size: usize) -> Self {
        let board = vec![vec![Stone::Empty; size]; size];
        let mut capture_count = HashMap::new();
        capture_count.insert(Stone::Black, 0);
        capture_count.insert(Stone::White, 0);

        let mut patterns = HashMap::new();
        patterns.insert(Stone::Black, PlayerPatterns::default());
        patterns.insert(Stone::White, PlayerPatterns::default());

        Gomoku {
            size,
            board,
            current_player: Stone::Black,
            opponent_player: Stone::White,
            capture_count,
            patterns,
            win_capture_count: 5,
            current_move: None,
            move_count: 0,
        }
    }

    pub fn print_board(&self, highlight: Vec<(usize, usize)>) {
        print!("  ");
        for i in 0..self.size {
            let c = "abcdefghijklmnopqrstuvwxyz".chars().nth(i).unwrap_or('-');
            print!(" {:2}", c);
        }
        println!();

        for (i, row) in self.board.iter().enumerate() {
            print!("{:2} ", i + 1);
            for (j, cell) in row.iter().enumerate() {
                if highlight.iter().any(|u| *u == (i, j)) {
                    print!("{}", format!("{:2}  ", cell).bold().yellow());
                } else {
                    print!("{:2}  ", cell);
                }
            }
            print!("{:<2} ", i + 1);
            println!();
        }
        print!("  ");
        for i in 0..self.size {
            let c = "abcdefghijklmnopqrstuvwxyz".chars().nth(i).unwrap_or('-');
            print!(" {:2}", c);
        }
        println!();
    }

    pub fn print_state(&self) {
        self.print_board(vec![]);
        // println!("size: {:?}", self.size);
        println!("current_player: {:?}", self.current_player);
        println!("opponent_player: {:?}", self.opponent_player);
        println!("current_move: {:?}", self.current_move);
        for player in [Stone::Black, Stone::White] {
            println!("patterns[{player}]:");
            let p = self.patterns.get(&player).unwrap();
            print_pattern_kind("open_two", &p.open_two);
            print_pattern_kind("open_three", &p.open_three);
            print_pattern_kind("free_three", &p.free_three);
            print_pattern_kind("block_four", &p.block_four);
            print_pattern_kind("open_four", &p.open_four);
            print_pattern_kind("five_row", &p.five_row);
        }
        println!("capture_count: {:?}", self.capture_count);
        // println!("win_capture_count: {:?}", self.win_capture_count);
    }

    fn clone_gomoku(&self) -> Gomoku {
        self.clone()
    }

    fn is_on_board(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && (x as usize) < self.size && (y as usize) < self.size
    }

    fn is_valid_move(&self, x: i32, y: i32) -> MoveResult {
        // return self.is_valid_move_simple_ruleset(x, y);
        if !self.is_on_board(x, y) {
            return MoveResult::OutOfBoard;
        }
        if self.board[x as usize][y as usize] != Stone::Empty {
            return MoveResult::NotEmpty;
        }
        if self.is_double_three_move(x, y) {
            return MoveResult::DoubleThree;
        }
        MoveResult::Valid
    }

    fn is_double_three_move(&self, x0: i32, y0: i32) -> bool {
        let directions = [(1, -1), (1, 0), (1, 1), (0, 1)];

        let free_three_count = directions
            .into_iter()
            .filter(|&(dx, dy)| {
                let plus = self.scan_line(1, dx, dy, x0, y0);
                let minus = self.scan_line(-1, dx, dy, x0, y0);
                free_three_from_scan(dx, dy, x0, y0, &plus, &minus).is_some()
            })
            .count();

        free_three_count > 1
    }

    fn capture_center(&mut self, x0: i32, y0: i32) -> (i32, Vec<(i32, i32)>) {
        let directions = [
            (1, -1), (1, 0), (1, 1),
            (0, -1), (0, 1),
            (-1, -1), (-1, 0), (-1, 1),
        ];

        let mut capture_count = 0;
        let mut captured_positions = Vec::new();

        for (dx, dy) in directions {
            if self.is_capture(x0, y0, dx, dy) {
                let removed = self.apply_capture(x0, y0, dx, dy);
                captured_positions.extend(removed); // 👈 핵심
                capture_count += 1;
            }
        }

        if capture_count > 0 {
            let current = self.current_player.clone();
            *self.capture_count.get_mut(&current).unwrap() += capture_count;
        }

        (capture_count, captured_positions)
    }

    fn is_capture(&self, x0: i32, y0: i32, dx: i32, dy: i32) -> bool {
        for i in 1..3 {
            let x = x0 + dx * i;
            let y = y0 + dy * i;
            if !self.is_on_board(x, y) || self.board[x as usize][y as usize] != self.opponent_player
            {
                return false;
            }
        }
        let x = x0 + dx * 3;
        let y = y0 + dy * 3;
        self.is_on_board(x, y) && self.board[x as usize][y as usize] == self.current_player
    }

    fn apply_capture(&mut self, x0: i32, y0: i32, dx: i32, dy: i32) -> Vec<(i32, i32)> {
        let mut removed = Vec::new();

        for i in 1..3 {
            let x = x0 + dx * i;
            let y = y0 + dy * i;

            removed.push((x, y));
            self.board[x as usize][y as usize] = Stone::Empty;

            self.remove_patterns_at(x, y);
        }

        for i in 1..3 {
            let x = x0 + dx * i;
            let y = y0 + dy * i;

            self.add_patterns_for_capture(x, y);
        }

        removed
    }

    fn update_patterns_for_move(&mut self, x: i32, y: i32) {
        self.remove_patterns_at(x, y);
        self.add_patterns_for_move(x, y);
    }

    fn add_patterns_for_move(&mut self, x0: i32, y0: i32) {
        let player = self.current_player;
        let directions = [(1, 0), (0, 1), (1, 1), (1, -1)];

        for (dx, dy) in directions {
            let plus = self.scan_line(1, dx, dy, x0, y0);
            let minus = self.scan_line(-1, dx, dy, x0, y0);

            if let Some(pattern) = free_three_from_scan(dx, dy, x0, y0, &plus, &minus) {
                self.register(PatternKind::FreeThree, &player, pattern);
            }

            let Some(kind) = classify(&plus, &minus, 1)
            else {
                continue;
            };

            let (lower, upper) = if kind == PatternKind::FiveRow {
                (-minus.contig_my, plus.contig_my)
            } else {
                (
                    -(minus.contig_my + minus.contig_open as i32),
                    plus.contig_my + plus.contig_open as i32,
                )
            };
            let pattern: Pattern = (lower..=upper).map(|i| (x0 + dx * i, y0 + dy * i)).collect();
            self.register(kind, &player, pattern);
        }
    }

    fn add_patterns_for_capture(&mut self, x0: i32, y0: i32) {
        let player = self.current_player;
        let directions = [(1, -1), (1, 0), (1, 1), (0, 1)];

        for (dx, dy) in directions {
            let plus = self.scan_line(1, dx, dy, x0, y0);
            let minus = self.scan_line(-1, dx, dy, x0, y0);

            for pattern in free_three_from_capture_scan(dx, dy, x0, y0, &plus, &minus) {
                self.register(PatternKind::FreeThree, &player, pattern);
            }

            // (x0,y0)은 캡처로 비워진 칸: 양쪽을 각각 독립된 한쪽짜리 런(run)으로 취급한다.
            // if let Some((kind, pattern)) = capture_open_pattern(dx, dy, x0, y0, &plus) {
            //     self.register(kind, &player, pattern);
            // }
            // if let Some((kind, pattern)) = capture_open_pattern(-dx, -dy, x0, y0, &minus) {
            //     self.register(kind, &player, pattern);
            // }
        }
    }

    fn remove_patterns_at(&mut self, x: i32, y: i32) {
        let pos = (x, y);
        const KINDS: [PatternKind; 6] = [
            PatternKind::OpenTwo,
            PatternKind::OpenThree,
            PatternKind::FreeThree,
            PatternKind::BlockFour,
            PatternKind::OpenFour,
            PatternKind::FiveRow,
        ];
        let mut pending: Vec<(Stone, PatternKind, Pattern)> = Vec::new();

        for player in [Stone::Black, Stone::White] {
            for kind in KINDS {
                self.patterns_mut(kind, &player).retain_mut(|pattern| {
                    if !pattern.contains(&pos) {
                        return true;
                    }
                    match endpoint_trim_rule(kind, pattern, pos) {
                        Some((new_kind, new_pattern)) => {
                            pending.push((player, new_kind, new_pattern));
                            false
                        }
                        None => false,
                    }
                });
            }
        }

        for (player, kind, pattern) in pending {
            self.register(kind, &player, pattern);
        }
    }

    fn is_valid_move_simple_ruleset(&self, x: i32, y: i32) -> MoveResult {
        if !self.is_on_board(x, y) {
            return MoveResult::OutOfBoard;
        }
        if self.board[x as usize][y as usize] != Stone::Empty {
            return MoveResult::NotEmpty;
        }
        // if self.is_double_three_move(x, y) {
        //     return MoveResult::DoubleThree;
        // }
        MoveResult::Valid
    }

    pub fn handle_move(&mut self, x: i32, y: i32)
        -> (MoveResult, i32, Vec<(i32, i32)>)
    {
        let result = self.is_valid_move(x, y);
        let mut capture_count = 0;
        let mut captured_positions = Vec::new();

        if result == MoveResult::Valid {
            self.current_move = Some((x, y));
            self.move_count += 1;
            self.board[x as usize][y as usize] = self.current_player.clone();

            self.update_patterns_for_move(x, y);

            let (count, positions) = self.capture_center(x, y);
            capture_count = count;
            captured_positions = positions;
        }

        (result, capture_count, captured_positions)
    }

    fn count_empty_spots(&self) -> i32 {
        self.board
            .iter()
            .map(|row| row.iter().filter(|&cell| cell == &Stone::Empty).count() as i32)
            .sum()
    }

    fn in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && (x as usize) < self.board.len() && (y as usize) < self.board[0].len()
    }

    fn cell_eq(&self, x: i32, y: i32, who: &Stone) -> bool {
        self.in_bounds(x, y) && self.board[x as usize][y as usize] == *who
    }

    fn is_empty(&self, x: i32, y: i32) -> bool {
        self.in_bounds(x, y) && self.board[x as usize][y as usize] == Stone::Empty
    }

    // (p0, p1)가 인접한 내 돌 두 개(PP)일 때, OPP-PP-EMPTY 또는 EMPTY-PP-OPP 패턴인지 검사
    fn is_pair_capturable(&self, p0: (i32, i32), p1: (i32, i32)) -> bool {
        let (x0, y0) = p0;
        let (x1, y1) = p1;

        // 방향 벡터(인접 전제)
        let dx = (x1 - x0).clamp(-1, 1);
        let dy = (y1 - y0).clamp(-1, 1);

        // 패턴 A: OPP - P(x0,y0) - P(x1,y1) - EMPTY
        let a_left_x = x0 - dx;
        let a_left_y = y0 - dy;
        let a_right_x = x1 + dx;
        let a_right_y = y1 + dy;

        let pattern_a = self.cell_eq(a_left_x, a_left_y, &self.opponent_player)
            && self.is_empty(a_right_x, a_right_y);

        // 패턴 B: EMPTY - P(x0,y0) - P(x1,y1) - OPP
        let pattern_b = self.is_empty(a_left_x, a_left_y)
            && self.cell_eq(a_right_x, a_right_y, &self.opponent_player);

        pattern_a || pattern_b
    }

    fn stone_in_capturable_pair(&self, x: i32, y: i32) -> bool {
        if !self.cell_eq(x, y, &self.current_player) {
            return false;
        }
        const DIRS: [(i32, i32); 4] = [(1, 0), (0, 1), (1, 1), (1, -1)];

        for (dx, dy) in DIRS {
            // (x,y) - (x+dx,y+dy) 가 PP
            let nx = x + dx;
            let ny = y + dy;
            if self.cell_eq(nx, ny, &self.current_player)
                && self.is_pair_capturable((x, y), (nx, ny))
            {
                return true;
            }
            // (x-dx,y-dy) - (x,y) 가 PP
            let px = x - dx;
            let py = y - dy;
            if self.cell_eq(px, py, &self.current_player)
                && self.is_pair_capturable((px, py), (x, y))
            {
                return true;
            }
        }
        false
    }
    pub fn get_winner(&self) -> Option<String> {
        if let Some((_x, _y)) = self.current_move {
            // 1. check current's five capture
            if *self.capture_count.get(&self.current_player).unwrap() >= self.win_capture_count {
                return Some(self.current_player.to_string());
            }

            // 2. check opponent's five_row
            if let Some(opponent_patterns) = self.patterns.get(&self.opponent_player) {
                if !opponent_patterns.five_row.is_empty() {
                    return Some(self.opponent_player.to_string());
                }
            }
            // 3. check current's five_row
            if let Some(my_patterns) = self.patterns.get(&self.current_player) {
                'each_five: for five_row in &my_patterns.five_row {
                    for &(fx, fy) in five_row {
                        if self.stone_in_capturable_pair(fx as i32, fy as i32) {
                            continue 'each_five;
                        }
                    }
                    return Some(self.current_player.to_string());
                }
            }
        }
        // 4. other than 3 cases, there's no winner
        None
    }

    fn check_draw(&self) -> bool {
        self.count_empty_spots() == 0
    }

    pub fn switch_player(&mut self) {
        match self.current_player {
            Stone::Black => {
                self.current_player = Stone::White;
                self.opponent_player = Stone::Black;
            }
            Stone::White => {
                self.current_player = Stone::Black;
                self.opponent_player = Stone::White;
            }
            Stone::Empty => panic!("Player cannot be the empty stone"),
        }
    }

    // Getter methods for Python access
    #[getter]
    fn size(&self) -> usize {
        self.size
    }

    #[getter]
    fn current_player(&self) -> String {
        self.current_player.to_string()
    }

    #[getter]
    fn opponent_player(&self) -> String {
        self.opponent_player.to_string()
    }

    #[getter]
    fn capture_count(&self) -> HashMap<String, i32> {
        self.capture_count
            .iter()
            .map(|(stone, c)| (stone.to_string(), *c))
            .collect()
    }

    #[getter]
    fn current_move(&self) -> Option<(i32, i32)> {
        self.current_move
    }

    #[getter]
    fn board(&self) -> Vec<Vec<String>> {
        self.board
            .iter()
            .map(|v| v.iter().map(|s| s.to_string()).collect())
            .collect()
    }

    #[getter]
    fn free_three_list(&self) -> HashMap<String, Vec<Vec<(i32, i32)>>> {
        self.patterns
            .iter()
            .map(|(stone, p)| (stone.to_string(), p.free_three.clone()))
            .collect()
    }

    #[getter]
    fn five_row(&self) -> HashMap<String, Vec<Vec<(i32, i32)>>> {
        self.patterns
            .iter()
            .map(|(stone, p)| (stone.to_string(), p.five_row.clone()))
            .collect()
    }

    #[getter]
    fn open_two(&self) -> HashMap<String, Vec<Vec<(i32, i32)>>> {
        self.patterns
            .iter()
            .map(|(stone, p)| (stone.to_string(), p.open_two.clone()))
            .collect()
    }

    #[getter]
    fn open_three(&self) -> HashMap<String, Vec<Vec<(i32, i32)>>> {
        self.patterns
            .iter()
            .map(|(stone, p)| (stone.to_string(), p.open_three.clone()))
            .collect()
    }

    #[getter]
    fn open_four(&self) -> HashMap<String, Vec<Vec<(i32, i32)>>> {
        self.patterns
            .iter()
            .map(|(stone, p)| (stone.to_string(), p.open_four.clone()))
            .collect()
    }

    #[getter]
    fn win_capture_count(&self) -> i32 {
        self.win_capture_count
    }

    pub fn parse_board(&mut self, board_str: &str) {
        self.board = board_str
            .lines()
            .map(|line| {
                line.chars()
                    .map(|ch| match ch {
                        'X' => Stone::Black,
                        'O' => Stone::White,
                        '.' => Stone::Empty,
                        _ => Stone::Empty, // Handle any unexpected characters as empty
                    })
                    .collect()
            })
            .collect();
    }
}

#[pymodule]
fn lib_gomoku(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<MoveResult>()?;
    m.add_class::<Gomoku>()?;
    m.add_function(wrap_pyfunction!(heuristic::heuristic_evaluation, m)?)?;
    m.add_function(wrap_pyfunction!(minimax::get_ai_move, m)?)?;

    let gomoku_class = m.getattr("Gomoku")?;
    gomoku_class.setattr("__module__", "lib_gomoku")?;

    let move_result_class = m.getattr("MoveResult")?;
    move_result_class.setattr("__module__", "lib_gomoku")?;
    Ok(())
}
