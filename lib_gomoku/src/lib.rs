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
    end_open: bool,
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
    let contig_total = plus.contig_my + minus.contig_my + center_stone;
    let empty = plus.empty_count + minus.empty_count + (1 - center_stone);    
    if contig_total == 5 && center_stone != 0 {
        Some(PatternKind::FiveRow)
    } else if contig_total == 4 && center_stone != 0 && plus.empty_count > 0 && minus.empty_count > 0 {
        // 기준 돌이 내 돌이고, 이어진 돌이 4개이며, 양쪽에 최소 한개의 빈칸이 있을 때 
        Some(PatternKind::OpenFour)
    } else if contig_total == 4 && (plus.end_open || minus.end_open) {
        // 한쪽 끝만 넓게 열려 있으면 total==4 && empty==1 조건을 못 맞추므로 별도로 잡는다.
        Some(PatternKind::BlockFour)
    } else if total == 4 && empty == 1 {
        Some(PatternKind::BlockFour)
    } else if total == 3 && empty == 3 {
        Some(PatternKind::FreeThree)
    } else if total == 3 && empty == 2 {
        Some(PatternKind::OpenThree)
    } else if total == 2 && plus.empty_count > 0 && minus.empty_count > 0 {
        Some(PatternKind::OpenTwo)
    } else {
        None
    }
}

// 패턴의 실제 좌표 범위: FiveRow는 이미 꽉 찼으니 contig만, 나머지는 열린 끝의 reach 칸까지 포함한다.
fn build_pattern_range(
    kind: PatternKind,
    dx: i32,
    dy: i32,
    x0: i32,
    y0: i32,
    plus: &LineScan,
    minus: &LineScan,
) -> Pattern {
    let (lower, upper) = if kind == PatternKind::FiveRow {
        (-minus.contig_my, plus.contig_my)
    } else {
        (
            -(minus.contig_my + minus.empty_count),
            plus.contig_my + plus.empty_count,
        )
    };
    (lower..=upper).map(|i| (x0 + dx * i, y0 + dy * i)).collect()
}

fn free_three_for_move(dx: i32, dy: i32, x0: i32, y0: i32, plus: &LineScan, minus: &LineScan) -> Option<Pattern> {
    if plus.total_my + minus.total_my != 2 
    || plus.empty_count + minus.empty_count < 3 
    || ! plus.end_open 
    || ! minus.end_open {
        return None;
    }

    // adjust empty space number because [..0.@0..] in this case, empty space on the rightest side is useless
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

fn free_three_for_capture(dx: i32, dy: i32, x0: i32, y0: i32, plus: &LineScan, minus: &LineScan) -> Vec<Pattern> {
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


impl Gomoku {
    fn scan_line(&self, sign: i32, dx: i32, dy: i32, x0: i32, y0: i32) -> LineScan {
        self.scan_line_as(self.current_player, sign, dx, dy, x0, y0)
    }

    // scan_line은 원래 self.current_player를 "나"로 취급했는데, 지금 수를 두는 사람과
    // 재계산하려는 패턴 주인이 다를 수 있는 rescan_pattern에서는 그게 틀린 기준이 된다.
    // 그래서 "나"를 명시적으로 받는 버전을 따로 두고, scan_line은 이걸 감싸기만 한다.
    fn scan_line_as(&self, me: Stone, sign: i32, dx: i32, dy: i32, x0: i32, y0: i32) -> LineScan {
        let opponent = match me {
            Stone::Black => Stone::White,
            Stone::White => Stone::Black,
            Stone::Empty => Stone::Empty,
        };
        let mut contig_my = 0;
        let mut end_open = false;
        let mut contig_done = false;
        let mut total_my = 0;
        let mut empty_count = 0;
        let mut hole = false;
        let mut i = 1;

        loop {
            let x = x0 + dx * i * sign;
            let y = y0 + dy * i * sign;

            if !self.is_on_board(x, y)
                || self.board[x as usize][y as usize] == opponent
            {
                if empty_count == 2 {
                  end_open = true;
                }
                break;
            }
            else if empty_count == 2 {
              end_open = true;
              break;
            }

            if self.board[x as usize][y as usize] == me {
                // my stone
                if !contig_done {
                  contig_my += 1;
                } else {
                  hole = true;
                }
                total_my += 1;
            } else {
                // empty 
                contig_done = true;
                empty_count += 1;
            }
            i += 1;
        }

        LineScan {
            contig_my,
            end_open,
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

    fn patterns_ref(&self, kind: PatternKind, player: &Stone) -> &Vec<Pattern> {
        let p = self.patterns.get(player).unwrap();
        match kind {
            PatternKind::OpenTwo => &p.open_two,
            PatternKind::OpenThree => &p.open_three,
            PatternKind::FreeThree => &p.free_three,
            PatternKind::BlockFour => &p.block_four,
            PatternKind::OpenFour => &p.open_four,
            PatternKind::FiveRow => &p.five_row,
        }
    }

    // pos를 잃은 뒤에도 살아남은 돌 하나(anchor)를 기준으로 scan_line+classify를 다시 돌려
    // 그 라인의 진짜 현재 모양을 재계산한다. 좌표 슬라이싱 추측이 아니라 board 실체를 읽는다.
    fn rescan_pattern(&self, pattern: &Pattern, pos: Position, player: &Stone) -> Option<(PatternKind, Pattern)> {
        if pattern.len() < 2 {
            return None;
        }
        let (dx, dy) = (pattern[1].0 - pattern[0].0, pattern[1].1 - pattern[0].1);

        let &(ax, ay) = pattern.iter().find(|&&(px, py)| {
            (px, py) != pos && self.board[px as usize][py as usize] == *player
        })?;

        let plus = self.scan_line_as(*player, 1, dx, dy, ax, ay);
        let minus = self.scan_line_as(*player, -1, dx, dy, ax, ay);
        let kind = classify(&plus, &minus, 1)?;
        Some((kind, build_pattern_range(kind, dx, dy, ax, ay, &plus, &minus)))
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
                free_three_for_move(dx, dy, x0, y0, &plus, &minus).is_some()
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

            if let Some(pattern) = free_three_for_move(dx, dy, x0, y0, &plus, &minus) {
                self.register(PatternKind::FreeThree, &player, pattern);
            }

            let Some(kind) = classify(&plus, &minus, 1)
            else {
                continue;
            };

            let pattern = build_pattern_range(kind, dx, dy, x0, y0, &plus, &minus);
            self.register(kind, &player, pattern);
        }
    }

    fn add_patterns_for_capture(&mut self, x0: i32, y0: i32) {
        let player = self.current_player;
        let directions = [(1, -1), (1, 0), (1, 1), (0, 1)];

        for (dx, dy) in directions {
            let plus = self.scan_line(1, dx, dy, x0, y0);
            let minus = self.scan_line(-1, dx, dy, x0, y0);

            for pattern in free_three_for_capture(dx, dy, x0, y0, &plus, &minus) {
                self.register(PatternKind::FreeThree, &player, pattern);
            }

            let Some(kind) = classify(&plus, &minus, 0)
            else {
                continue;
            };

            let pattern = build_pattern_range(kind, dx, dy, x0, y0, &plus, &minus);
            self.register(kind, &player, pattern);
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

        // 1) READ: pos를 포함한 패턴마다, 살아남은 돌 기준으로 현재 모양을 다시 계산해둔다.
        //    (self.patterns를 불변으로만 빌리므로 아래 retain의 가변 대여와 안 겹친다)
        for player in [Stone::Black, Stone::White] {
            for kind in KINDS {
                for pattern in self.patterns_ref(kind, &player) {
                    if !pattern.contains(&pos) {
                        continue;
                    }
                    if let Some((new_kind, new_pattern)) = self.rescan_pattern(pattern, pos, &player) {
                        pending.push((player, new_kind, new_pattern));
                    }
                }
            }
        }

        // 2) MUTATE: pos를 포함했던 패턴은 전부 제거한다 (대체본은 pending에 이미 있음).
        for player in [Stone::Black, Stone::White] {
            for kind in KINDS {
                self.patterns_mut(kind, &player).retain(|pattern| !pattern.contains(&pos));
            }
        }

        // 3) APPLY: 재계산된 패턴을 등록한다.
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

#[cfg(test)]
mod tests {
    use super::*;

    // 테스트 전용 헬퍼: player를 강제로 세팅하고 수를 둔다. Valid가 아니면 바로 실패시켜서
    // (더블쓰리 등으로) 세팅 자체가 잘못됐을 때 조용히 넘어가지 않게 한다.
    fn place(game: &mut Gomoku, player: Stone, x: i32, y: i32) {
        game.current_player = player;
        game.opponent_player = match player {
            Stone::Black => Stone::White,
            Stone::White => Stone::Black,
            Stone::Empty => panic!("Empty는 플레이어가 될 수 없다"),
        };
        let (result, _, _) = game.handle_move(x, y);
        assert_eq!(result, MoveResult::Valid, "({x},{y}) {:?} 착수가 실패함", player);
    }

    fn black_patterns(game: &Gomoku) -> &PlayerPatterns {
        game.patterns.get(&Stone::Black).unwrap()
    }

    // 테스트 보드 생성 헬퍼. '0'=Black, 'O'=White(벽), '.'=빈칸.
    // add_patterns_for_move는 "방금 둔 돌" 위치에서만 스캔하므로, new_index가 가리키는 '0'을
    // 맨 마지막에 둬서 그게 classify()를 트리거하는 "새 수"가 되게 한다. 벽('O')은 항상 먼저 둔다.
    // new_index를 두기 직전의 패턴 스냅샷도 같이 반환해서, 테스트가 "그 수가 실제로 패턴을
    // 등록시켰는지"(이전엔 없다가 이후에 생겼는지)를 확인할 수 있게 한다.
    // 캡처처럼 "감싸는 쪽/감싸이는 쪽" 순서 자체가 다른 케이스는 이 헬퍼로 표현 안 되니 수동으로 놓는다.
    fn setup_window(row: i32, base: i32, pattern: &str, new_index: usize) -> (Gomoku, PlayerPatterns) {
        let mut game = Gomoku::new(19);
        for (i, ch) in pattern.chars().enumerate() {
            if ch == 'O' {
                place(&mut game, Stone::White, row, base + i as i32);
            } else if ch == '0' && i != new_index {
                place(&mut game, Stone::Black, row, base + i as i32);
            }
        }
        let before = black_patterns(&game).clone();
        place(&mut game, Stone::Black, row, base + new_index as i32);
        (game, before)
    }

    // 아래 테스트는 전부 19x19 새 board, row=5 가로줄만 사용한다.
    // 다른 방향(세로/대각선)엔 아무 돌도 없어서 그쪽에서 우연히 패턴이 잡힐 일이 없다.

    #[test]
    fn register_open_two_variants() {
        // (pattern, new_index, expected) - expected가 None이면 open_two가 등록되지 않아야 한다.
        let cases: Vec<(&str, usize, Option<Vec<Position>>)> = vec![
            // ..00..
            ("..00..", 2, Some(vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9), (5, 10)])),
            // ..00.O
            ("..00.O", 2, Some(vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9)])),
            ("..00.O", 3, Some(vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9)])),
            // O.0.0.O
            ("O.0.0.O", 2, Some(vec![(5, 6), (5, 7), (5, 8), (5, 9)])),
        ];

        for (pattern, new_index, expected) in cases {
            let (game, before) = setup_window(5, 5, pattern, new_index);
            assert!(before.open_two.is_empty(), "{pattern:?} idx={new_index}: 마지막 수 이전에 이미 등록됨");

            let actual = &black_patterns(&game).open_two;
            match expected {
                Some(expected) => assert_eq!(*actual, vec![expected], "{pattern:?} idx={new_index}"),
                None => assert!(actual.is_empty(), "{pattern:?} idx={new_index}: 등록되지 않아야 하는데 {actual:?}"),
            }
        }
    }

    #[test]
    fn open_two_disappears_when_near_side_blocked() {
        // ..00.. 상태에서 White가 바로 옆(5,4)을 막으면, 한쪽만 막힌 open_two는 사라져야 한다.
        let (mut game, _before) = setup_window(5, 5, "00", 1);
        place(&mut game, Stone::White, 5, 4);

        assert!(black_patterns(&game).open_two.is_empty());
    }

    #[test]
    fn register_open_three_variants() {
        // (row, base, pattern, new_index, expected) - expected가 None이면 open_three가 등록되지 않아야 한다.
        let cases: Vec<(i32, i32, &str, usize, Option<Vec<Position>>)> = vec![
            // O.000..: 왼쪽은 벽으로 막히고 오른쪽은 넓게 열림
            (5, 4, "O000", 3, Some(vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9)])),
            // O.0.00O: 왼쪽 끝 1칸 막고 벽, 안쪽에 gap 1칸
            (1, 5, "O.0.00O", 5, Some(vec![(1, 7), (1, 8), (1, 9), (1, 10)])),
            // O.00.0O: 위와 좌우 대칭 (gap이 반대쪽)
            (2, 5, "O.00.0O", 5, Some(vec![(2, 8), (2, 9), (2, 10)])),
            // O.000.O: 양쪽 다 1칸씩만 열리고 바로 벽 (총 empty=2)
            (3, 5, "O.000.O", 4, Some(vec![(3, 6), (3, 7), (3, 8), (3, 9), (3, 10)])),
            // O0..00O: 표대로면 open_three(empty=2)여야 하지만, scan_line이 빈칸 2개 연속에서
            // "열렸다"고 확정하고 멈춰서 그 너머의 외톨이 돌을 못 본다 -> 등록 안 됨.
            (4, 5, "O0..00O", 5, None),
            // O0.0.0O: 가운데 돌(index3)이 새로 놓이면 양쪽 외톨이 돌이 각각 1칸 거리서 바로 보여 등록됨.
            (5, 5, "O0.0.0O", 3, Some(vec![(5, 7), (5, 8), (5, 9)])),
            // O0.0.0O: 최종 board는 위와 동일해도, 바깥쪽 돌(index5)이 새로 놓이면 왼쪽 스캔의
            // 누적 empty_count가 2가 되는 순간 멈춰서 그 너머의 돌을 못 본다 -> 등록 안 됨
            // (add_patterns_for_move는 항상 "방금 둔 돌" 위치에서만 스캔하기 때문).
            (5, 5, "O0.0.0O", 5, None),
        ];

        for (row, base, pattern, new_index, expected) in cases {
            let (game, before) = setup_window(row, base, pattern, new_index);
            assert!(before.open_three.is_empty(), "{pattern:?} idx={new_index}: 마지막 수 이전에 이미 등록됨");

            let actual = &black_patterns(&game).open_three;
            match expected {
                Some(expected) => assert_eq!(*actual, vec![expected], "{pattern:?} idx={new_index}"),
                None => assert!(actual.is_empty(), "{pattern:?} idx={new_index}: 등록되지 않아야 하는데 {actual:?}"),
            }
        }
    }

    #[test]
    fn open_three_disappears_when_open_side_also_blocked() {
        // 위 상태에서 열린 쪽 바로 옆(5,8)까지 White로 막으면 O-000-O, open_three는 사라져야 한다.
        let (mut game, _before) = setup_window(5, 4, "O000", 3);
        place(&mut game, Stone::White, 5, 8);

        assert!(black_patterns(&game).open_three.is_empty());
    }

    #[test]
    fn register_free_three_variants() {
        // (row, base, pattern, new_index, expected)
        let cases: Vec<(i32, i32, &str, usize, Vec<Position>)> = vec![
            // ...000...: 양쪽 다 넓게 열림 -> free_three_for_move 경로
            (5, 5, "000", 2, vec![(5, 3), (5, 4), (5, 5), (5, 6), (5, 7), (5, 8), (5, 9)]),
            // ..000.O: 왼쪽은 넓게 열리고 오른쪽은 1칸만 열린 뒤 벽 -> classify()의
            // total==3&&empty==3 분기 (free_three_for_move는 양쪽 다 end_open이어야 해서 안 탐)
            (6, 8, "000.O", 2, vec![(6, 6), (6, 7), (6, 8), (6, 9), (6, 10), (6, 11)]),
            // O.00.0.O: 양쪽 다 1칸씩 + 안쪽 gap 1칸 (총 empty=3) -> classify() free_three 분기
            (7, 5, "O.00.0.O", 5, vec![(7, 8), (7, 9), (7, 10), (7, 11)]),
        ];

        for (row, base, pattern, new_index, expected) in cases {
            let (game, before) = setup_window(row, base, pattern, new_index);
            assert!(before.free_three.is_empty(), "{pattern:?} idx={new_index}: 마지막 수 이전에 이미 등록됨");

            assert_eq!(black_patterns(&game).free_three, vec![expected], "{pattern:?} idx={new_index}");
            assert!(
                black_patterns(&game).open_three.is_empty(),
                "{pattern:?} idx={new_index}: free_three와 open_three가 동시에 등록됨"
            );
        }
    }

    #[test]
    fn register_block_four_variants() {
        // (row, base, pattern, new_index, expected)
        let cases: Vec<(i32, i32, &str, usize, Vec<Position>)> = vec![
            // O.0000..: 한쪽만 열려서 total==4&&empty==1 조건을 못 맞추는 케이스의 회귀 테스트.
            (5, 4, "O0000", 4, vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9), (5, 10)]),
            // O.0000O: 왼쪽 끝 1칸만 열리고 바로 벽, 오른쪽도 바로 벽
            (8, 5, "O.0000O", 5, vec![(8, 6), (8, 7), (8, 8), (8, 9), (8, 10)]),
            // O0.000O: 돌 하나 - gap 1칸 - 돌 셋 (hole, 채우면 5줄)
            (9, 5, "O0.000O", 5, vec![(9, 7), (9, 8), (9, 9), (9, 10)]),
            // O00.00O: 돌 둘 - gap 1칸 - 돌 둘 (중간 hole)
            (10, 5, "O00.00O", 5, vec![(10, 8), (10, 9), (10, 10)]),
        ];

        for (row, base, pattern, new_index, expected) in cases {
            let (game, before) = setup_window(row, base, pattern, new_index);
            assert!(before.block_four.is_empty(), "{pattern:?} idx={new_index}: 마지막 수 이전에 이미 등록됨");

            assert_eq!(black_patterns(&game).block_four, vec![expected], "{pattern:?} idx={new_index}");
            assert!(
                black_patterns(&game).open_four.is_empty(),
                "{pattern:?} idx={new_index}: block_four와 open_four가 동시에 등록됨"
            );
        }
    }

    #[test]
    fn open_four_downgrades_to_block_four_when_blocked() {
        // ...0000...: 양쪽 다 열림 -> open_four. 이후 한쪽 옆(5,4)을 막으면 block_four로 강등돼야
        // 한다 (예전엔 interior trim이 통째로 삭제해서 위협이 소리소문없이 사라지는 버그가 있었다).
        let (mut game, before) = setup_window(5, 5, "0000", 3);
        assert!(before.open_four.is_empty());

        let open_four_expected = vec![
            (5, 3), (5, 4), (5, 5), (5, 6), (5, 7), (5, 8), (5, 9), (5, 10),
        ];
        assert_eq!(black_patterns(&game).open_four, vec![open_four_expected]);

        place(&mut game, Stone::White, 5, 4);

        assert!(black_patterns(&game).open_four.is_empty());
        let block_four_expected = vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9), (5, 10)];
        assert_eq!(black_patterns(&game).block_four, vec![block_four_expected]);
    }

    #[test]
    fn five_row_registers_on_five_in_a_row() {
        // 00000 완성 -> five_row 등록, 직전 open_four는 사라져야 한다 (이중 집계 방지).
        let (game, before) = setup_window(5, 5, "00000", 4);
        assert!(before.five_row.is_empty());

        let expected = vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9)];
        assert_eq!(black_patterns(&game).five_row, vec![expected]);
        assert!(black_patterns(&game).open_four.is_empty());
        assert!(black_patterns(&game).block_four.is_empty());
    }

    // 캡처 테스트 2개는 setup_window를 안 쓴다: "감싸는 쪽/감싸이는 쪽" 착수 순서가 서로
    // 반대라 공용 헬퍼로 표현할 수 없다.

    #[test]
    fn capture_removes_pattern_when_both_stones_captured() {
        // ..00.. 로 open_two를 만든 뒤 White가 O-0-0-O로 감싸서 캡처하면, 남은 anchor가
        // 없어서 open_two도 완전히 사라져야 한다.
        let mut game = Gomoku::new(19);
        place(&mut game, Stone::Black, 5, 5);
        place(&mut game, Stone::Black, 5, 6);
        place(&mut game, Stone::White, 5, 4);
        place(&mut game, Stone::White, 5, 7); // (5,5),(5,6) capture 됨

        assert_eq!(game.board[5][5], Stone::Empty);
        assert_eq!(game.board[5][6], Stone::Empty);
        assert!(black_patterns(&game).open_two.is_empty());
    }

    #[test]
    fn capture_registers_pattern_with_full_reach_range() {
        // 0-O-O-0: Black이 (5,8)을 두면서 White 두 개를 캡처. 캡처로 빈 칸이 된 자리를 기준으로
        // 다시 스캔해도, add_patterns_for_move와 같은 build_pattern_range를 써서 (5,5)~(5,8)이
        // 전부 포함된 온전한 range가 나와야 한다.
        let mut game = Gomoku::new(19);
        place(&mut game, Stone::Black, 5, 5);
        place(&mut game, Stone::White, 5, 6);
        place(&mut game, Stone::White, 5, 7);
        place(&mut game, Stone::Black, 5, 8); // capture 발생, (5,6)/(5,7) 빈칸으로

        assert_eq!(game.board[5][6], Stone::Empty);
        assert_eq!(game.board[5][7], Stone::Empty);

        let full_range = vec![(5, 3), (5, 4), (5, 5), (5, 6), (5, 7), (5, 8)];
        assert!(
            black_patterns(&game).open_two.contains(&full_range),
            "capture 이후 open_two 패턴에 (5,8)까지 포함된 전체 range가 있어야 함: {:?}",
            black_patterns(&game).open_two
        );
    }
}
