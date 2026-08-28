use pyo3::prelude::*;
use std::collections::HashMap;
use std::sync::OnceLock;
use colored::*;

pub mod heuristic;
pub mod minimax;

// --- Zobrist hashing ---

struct XorShift64 {
    state: u64,
}

impl XorShift64 {
    fn next(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }
}

pub struct ZobristSeeds {
    pub board: [[u64; 2]; 19 * 19], // [cell_index][0=Black, 1=White]
    pub player: u64,                  // XOR when switching player
}

static ZOBRIST: OnceLock<ZobristSeeds> = OnceLock::new();

pub fn zobrist() -> &'static ZobristSeeds {
    ZOBRIST.get_or_init(|| {
        let mut rng = XorShift64 { state: 0x12345678DEADBEEF };
        let mut seeds = ZobristSeeds {
            board: [[0u64; 2]; 19 * 19],
            player: 0,
        };
        for cell in seeds.board.iter_mut() {
            cell[0] = rng.next();
            cell[1] = rng.next();
        }
        seeds.player = rng.next();
        seeds
    })
}

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
            -(minus.total_my + minus.empty_count),
            plus.total_my + plus.empty_count,
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
    pub hash: u64,
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
            hash: 0, // empty board, Black to play
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
        let z = zobrist();
        let opp_idx = if self.opponent_player == Stone::Black { 0 } else { 1 };
        let mut removed = Vec::new();
        for i in 1..3 {
            let x = x0 + dx * i;
            let y = y0 + dy * i;
            // XOR out removed opponent stone
            removed.push((x, y));
            self.hash ^= z.board[x as usize * 19 + y as usize][opp_idx];
            self.board[x as usize][y as usize] = Stone::Empty;

            self.remove_patterns_at(x, y);
        }

        // 캡처 라인(dx,dy) 자체는 두 빈칸이 같은 라인 위에 있어서, 각 빈칸을 독립된 center로
        // 스캔하면 서로 다른 range가 두 번 등록된다(register()의 dedup은 내용이 완전히 같을
        // 때만 걸러줌). mover(x0,y0)를 새 돌처럼 취급해 이 라인만 한 번에 재구성한다.
        self.add_patterns_for_capture_axis(x0, y0, dx, dy);

        // 캡처 라인이 아닌 나머지 방향은 두 빈칸에서 서로 다른 독립적인 라인이라 각자 스캔한다.
        for i in 1..3 {
            let x = x0 + dx * i;
            let y = y0 + dy * i;

            self.add_patterns_for_capture_off_axis(x, y, dx, dy);
        }

        removed
    }

    // 캡처 라인 위의 패턴을 mover 기준 하나의 계산으로 합친다. mover 쪽(반대편)은 그냥
    // 평범하게 스캔하고, anchor 쪽은 "캡처로 빈 두 칸 + anchor부터 이어지는 스캔"을 합성해서
    // scan_line의 2칸-cap 때문에 anchor 너머를 못 보는 문제를 피한다.
    fn add_patterns_for_capture_axis(&mut self, mover_x: i32, mover_y: i32, dx: i32, dy: i32) {
        let player = self.current_player;

        let near_anchor_x = mover_x + dx * 2;
        let near_anchor_y = mover_y + dy * 2;
        // near_anchor에서 anchor 방향으로 스캔하면 i=1이 정확히 anchor 자신이라 hole 없이
        // anchor부터 그 너머까지 정확하게 본다.
        let beyond_anchor = self.scan_line(1, dx, dy, near_anchor_x, near_anchor_y);

        let plus_toward_anchor = LineScan {
            contig_my: 0,
            end_open: beyond_anchor.end_open,
            total_my: beyond_anchor.total_my,
            empty_count: 2 + beyond_anchor.empty_count,
            hole: true,
        };
        let minus_away = self.scan_line(-1, dx, dy, mover_x, mover_y);

        if let Some(pattern) = free_three_for_move(dx, dy, mover_x, mover_y, &plus_toward_anchor, &minus_away) {
            self.register(PatternKind::FreeThree, &player, pattern);
        }

        // open_two(total==2)만 따로 처리한다: classify()의 plus.empty_count>0 체크에
        // plus_toward_anchor를 그대로 넘기면, 캡처로 생긴 두 칸이 항상 empty_count에 +2로
        // 얹혀 있어서 anchor 저편이 완전히 막혀 있어도 무조건 "열림"으로 통과해버린다.
        // 그러면 벽을 anchor 쪽에 두느냐 mover 쪽에 두느냐에 따라(둘은 좌우 대칭인 같은 모양인데도)
        // 결과가 달라지는 비대칭이 생긴다. 그래서 열림 판정만은 캡처 두 칸을 빼고 순수하게
        // anchor 저편(beyond_anchor)/mover 저편(minus_away) 각각의 empty로 판단한다.
        // range의 실제 폭 계산(build_pattern_range)에는 캡처 두 칸이 그대로 들어가야 하므로
        // plus_toward_anchor는 그대로 쓴다.
        if beyond_anchor.total_my + minus_away.total_my + 1 == 2 {
            if beyond_anchor.empty_count > 0 && minus_away.empty_count > 0 {
                let pattern = build_pattern_range(PatternKind::OpenTwo, dx, dy, mover_x, mover_y, &plus_toward_anchor, &minus_away);
                self.register(PatternKind::OpenTwo, &player, pattern);
            }
            return;
        }

        let Some(kind) = classify(&plus_toward_anchor, &minus_away, 1) else {
            return;
        };

        let pattern = build_pattern_range(kind, dx, dy, mover_x, mover_y, &plus_toward_anchor, &minus_away);
        self.register(kind, &player, pattern);
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

    // 캡처로 빈 칸(x0,y0) 하나에서, 캡처가 일어난 축(capture_dx,capture_dy)을 제외한
    // 나머지 3방향을 스캔한다. 캡처 축은 add_patterns_for_capture_axis가 따로 한 번에 처리한다.
    fn add_patterns_for_capture_off_axis(&mut self, x0: i32, y0: i32, capture_dx: i32, capture_dy: i32) {
        let player = self.current_player;
        let directions = [(1, -1), (1, 0), (1, 1), (0, 1)];

        for (dx, dy) in directions {
            if (dx, dy) == (capture_dx, capture_dy) || (dx, dy) == (-capture_dx, -capture_dy) {
                continue;
            }

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

    // pub fn handle_move_simple_ruleset(&mut self, x: i32, y: i32) -> (MoveResult, i32) {
    //     let result = self.is_valid_move_simple_ruleset(x, y);
    //     let capture_count = 0;
    //
    //     if result == MoveResult::Valid {
    //         self.current_move = Some((x, y));
    //         self.move_count += 1;
    //         self.board[x as usize][y as usize] = self.current_player.clone();
    //
    //         // self.remove_free_three(x, y, &self.opponent_player.clone());
    //         self.remove_opens(x, y);
    //
    //         // self.add_free_threes(
    //         //     self.get_free_threes_from_move(x, y),
    //         //     &self.current_player.clone(),
    //         // );
    //         self.add_opens_from_move(x, y);
    //
    //         // capture_count = self.capture_center(x, y);
    //     }
    //
    //     (result, capture_count)
    // }

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

            // Update Zobrist hash for placed stone
            let z = zobrist();
            let color_idx = if self.current_player == Stone::Black { 0 } else { 1 };
            self.hash ^= z.board[x as usize * 19 + y as usize][color_idx];
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
        self.hash ^= zobrist().player;
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
    m.add_function(wrap_pyfunction!(minimax::get_ai_move_stats, m)?)?;
    m.add_function(wrap_pyfunction!(minimax::get_hint, m)?)?;
    m.add_function(wrap_pyfunction!(minimax::get_move_pv, m)?)?;

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

    // 테스트 보드 생성 헬퍼. 'X'=Black, 'O'=White(벽), '.'=빈칸.
    // add_patterns_for_move는 "방금 둔 돌" 위치에서만 스캔하므로, new_index가 가리키는 'X'을
    // 맨 마지막에 둬서 그게 classify()를 트리거하는 "새 수"가 되게 한다. 벽('O')은 항상 먼저 둔다.
    // new_index를 두기 직전의 패턴 스냅샷도 같이 반환해서, 테스트가 "그 수가 실제로 패턴을
    // 등록시켰는지"(이전엔 없다가 이후에 생겼는지)를 확인할 수 있게 한다.
    // 캡처처럼 "감싸는 쪽/감싸이는 쪽" 순서 자체가 다른 케이스는 이 헬퍼로 표현 안 되니 수동으로 놓는다.
    fn setup_window(row: i32, base: i32, pattern: &str, new_index: usize) -> (Gomoku, PlayerPatterns) {
        let mut game = Gomoku::new(19);
        for (i, ch) in pattern.chars().enumerate() {
            if ch == 'O' {
                place(&mut game, Stone::White, row, base + i as i32);
            } else if ch == 'X' && i != new_index {
                place(&mut game, Stone::Black, row, base + i as i32);
            }
        }
        let before = black_patterns(&game).clone();
        place(&mut game, Stone::Black, row, base + new_index as i32);
        (game, before)
    }

    #[test]
    fn register_from_move_open_two_variants() {
        let cases: Vec<(&str, usize, Option<Vec<Position>>)> = vec![
            // ..XX..
            ("..XX..", 2, Some(vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9), (5, 10)])),
            // ..XX.O
            ("..XX.O", 2, Some(vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9)])),
            ("..XX.O", 3, Some(vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9)])),
            // O.X.X.O
            ("O.X.X.O", 2, Some(vec![(5, 6), (5, 7), (5, 8), (5, 9), (5, 10)])),
        ];

        let mut failures = Vec::new();
        for (pattern, new_index, expected) in cases {
            let (game, before) = setup_window(5, 5, pattern, new_index);
            if !before.open_two.is_empty() {
                failures.push(format!("{pattern:?} idx={new_index}: 마지막 수 이전에 이미 등록됨"));
                continue;
            }

            let actual = &black_patterns(&game).open_two;
            match expected {
                Some(expected) => {
                    if *actual != vec![expected] {
                        failures.push(format!("{pattern:?} idx={new_index}: 등록된 결과가 기대값과 다름: {actual:?}"));
                    }
                }
                None => {
                    if !actual.is_empty() {
                        failures.push(format!("{pattern:?} idx={new_index}: 등록되지 않아야 하는데 {actual:?}"));
                    }
                }
            }
        }
        assert!(failures.is_empty(), "\n{}", failures.join("\n"));
    }

    #[test]
    fn register_from_move_open_three_variants() {
        let cases: Vec<(i32, i32, &str, usize, Option<Vec<Position>>)> = vec![
            // XXX..
            (5, 4, "OXXX..", 1, Some(vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9)])),
            (5, 4, "OXXX..", 2, Some(vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9)])),
            (5, 4, "OXXX..", 3, Some(vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9)])),
            // XX.X.
            (1, 5, "OXX.X.O", 1, Some(vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)])),
            (1, 5, "OXX.X.O", 2, Some(vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)])),
            (1, 5, "OXX.X.O", 4, Some(vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)])),
            // X.XX.
            (1, 5, "OX.XX.O", 1, Some(vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)])),
            (1, 5, "OX.XX.O", 3, Some(vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)])),
            (1, 5, "OX.XX.O", 4, Some(vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)])),
            // .XXX.
            (1, 5, "O.XXX.O", 2, Some(vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)])),
            (1, 5, "O.XXX.O", 3, Some(vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)])),
            (1, 5, "O.XXX.O", 4, Some(vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)])),
            // X..XX
            (1, 5, "OX..XXO", 1, Some(vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)])),
            (1, 5, "OX..XXO", 4, Some(vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)])),
            (1, 5, "OX..XXO", 5, Some(vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)])),
            // X.X.X
            (1, 5, "OX.X.XO", 1, Some(vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)])),
            (1, 5, "OX.X.XO", 3, Some(vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)])),
            (1, 5, "OX.X.XO", 5, Some(vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)])),
        ];

        let mut failures = Vec::new();
        for (row, base, pattern, new_index, expected) in cases {
            let (game, before) = setup_window(row, base, pattern, new_index);
            if !before.open_three.is_empty() {
                failures.push(format!("{pattern:?} idx={new_index}: 마지막 수 이전에 이미 등록됨"));
                continue;
            }

            let actual = &black_patterns(&game).open_three;
            match expected {
                Some(expected) => {
                    if *actual != vec![expected] {
                        failures.push(format!("{pattern:?} idx={new_index}: 등록된 결과가 기대값과 다름: {actual:?}"));
                    }
                }
                None => {
                    if !actual.is_empty() {
                        failures.push(format!("{pattern:?} idx={new_index}: 등록되지 않아야 하는데 {actual:?}"));
                    }
                }
            }
        }
        assert!(failures.is_empty(), "\n{}", failures.join("\n"));
    }

    #[test]
    fn register_from_move_free_three_variants() {
        let cases: Vec<(i32, i32, &str, usize, Vec<Position>)> = vec![
            // ..XXX..
            (5, 5, "..XXX..", 2, vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9), (5, 10), (5, 11)]),
            (5, 5, "..XXX..", 3, vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9), (5, 10), (5, 11)]),
            // .XXX..
            (5, 5, "O.XXX..O", 2, vec![(5, 6), (5, 7), (5, 8), (5, 9), (5, 10), (5, 11)]),
            (5, 5, "O.XXX..O", 3, vec![(5, 6), (5, 7), (5, 8), (5, 9), (5, 10), (5, 11)]),
            (5, 5, "O.XXX..O", 4, vec![(5, 6), (5, 7), (5, 8), (5, 9), (5, 10), (5, 11)]),
            // .XX.X.
            (5, 5, "O.XX.X.O", 2, vec![(5, 6), (5, 7), (5, 8), (5, 9), (5, 10), (5, 11)]),
            (5, 5, "O.XX.X.O", 3, vec![(5, 6), (5, 7), (5, 8), (5, 9), (5, 10), (5, 11)]),
            (5, 5, "O.XX.X.O", 5, vec![(5, 6), (5, 7), (5, 8), (5, 9), (5, 10), (5, 11)]),
        ];

        let mut failures = Vec::new();
        for (row, base, pattern, new_index, expected) in cases {
            let (game, before) = setup_window(row, base, pattern, new_index);
            if !before.free_three.is_empty() {
                failures.push(format!("{pattern:?} idx={new_index}: 마지막 수 이전에 이미 등록됨"));
                continue;
            }

            let actual = &black_patterns(&game).free_three;
            if *actual != vec![expected] {
                failures.push(format!("{pattern:?} idx={new_index}: 등록된 결과가 기대값과 다름: {actual:?}"));
            }
            if !black_patterns(&game).open_three.is_empty() {
                failures.push(format!("{pattern:?} idx={new_index}: free_three와 open_three가 동시에 등록됨"));
            }
        }
        assert!(failures.is_empty(), "\n{}", failures.join("\n"));
    }

    #[test]
    fn register_from_move_block_four_variants() {
        let cases: Vec<(i32, i32, &str, usize, Vec<Position>)> = vec![
            // O.XXXXO
            (5, 4, "O.XXXXO", 2, vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9)]),
            (5, 4, "O.XXXXO", 3, vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9)]),
            (5, 4, "O.XXXXO", 4, vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9)]),
            (5, 4, "O.XXXXO", 5, vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9)]),
            // OX.XXXO
            (5, 4, "OX.XXXO", 1, vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9)]),
            (5, 4, "OX.XXXO", 3, vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9)]),
            (5, 4, "OX.XXXO", 4, vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9)]),
            (5, 4, "OX.XXXO", 5, vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9)]),
            // OXX.XXO
            (5, 4, "OXX.XXO", 1, vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9)]),
            (5, 4, "OXX.XXO", 2, vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9)]),
        ];

        let mut failures = Vec::new();
        for (row, base, pattern, new_index, expected) in cases {
            let (game, before) = setup_window(row, base, pattern, new_index);
            if !before.block_four.is_empty() {
                failures.push(format!("{pattern:?} idx={new_index}: 마지막 수 이전에 이미 등록됨"));
                continue;
            }

            let actual = &black_patterns(&game).block_four;
            if *actual != vec![expected] {
                failures.push(format!("{pattern:?} idx={new_index}: 등록된 결과가 기대값과 다름: {actual:?}"));
            }
            if !black_patterns(&game).open_four.is_empty() {
                failures.push(format!("{pattern:?} idx={new_index}: block_four와 open_four가 동시에 등록됨"));
            }
        }
        assert!(failures.is_empty(), "\n{}", failures.join("\n"));
    }
    
    #[test]
    fn register_from_move_open_four_variants() {
        let cases: Vec<(i32, i32, &str, usize, Vec<Position>)> = vec![
            // O.XXXX.O
            (5, 4, "O.XXXX.O", 2, vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9), (5, 10)]),
            (5, 4, "O.XXXX.O", 3, vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9), (5, 10)]),
        ];

        let mut failures = Vec::new();
        for (row, base, pattern, new_index, expected) in cases {
            let (game, before) = setup_window(row, base, pattern, new_index);
            if !before.open_four.is_empty() {
                failures.push(format!("{pattern:?} idx={new_index}: 마지막 수 이전에 이미 등록됨"));
                continue;
            }

            let actual = &black_patterns(&game).open_four;
            if *actual != vec![expected] {
                failures.push(format!("{pattern:?} idx={new_index}: 등록된 결과가 기대값과 다름: {actual:?}"));
            }
        }
        assert!(failures.is_empty(), "\n{}", failures.join("\n"));
    }
    
    #[test]
    fn register_from_move_five_row() {
        // XXXXX 완성 -> five_row 등록, 직전 open_four는 사라져야 한다 (이중 집계 방지).
        let (game, before) = setup_window(5, 5, "XXXXX", 4);
        assert!(before.five_row.is_empty());

        let expected = vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9)];
        assert_eq!(black_patterns(&game).five_row, vec![expected]);
        assert!(black_patterns(&game).open_four.is_empty());
        assert!(black_patterns(&game).block_four.is_empty());
    }

    // capture helper function
    fn setup_capture_axis(row: i32, mover_col: i32, beyond_anchor_pattern: &str, away_pattern: &str) -> (Gomoku, PlayerPatterns) {
        let mut game = Gomoku::new(19);
        let anchor_col = mover_col - 3;

        for (i, ch) in beyond_anchor_pattern.chars().enumerate() {
            let col = anchor_col - 1 - i as i32;
            match ch {
                'X' => place(&mut game, Stone::Black, row, col),
                'O' => place(&mut game, Stone::White, row, col),
                _ => {}
            }
        }
        place(&mut game, Stone::Black, row, anchor_col);
        place(&mut game, Stone::White, row, anchor_col + 1);
        place(&mut game, Stone::White, row, anchor_col + 2);

        for (i, ch) in away_pattern.chars().enumerate() {
            let col = mover_col + 1 + i as i32;
            match ch {
                'X' => place(&mut game, Stone::Black, row, col),
                'O' => place(&mut game, Stone::White, row, col),
                _ => {}
            }
        }

        let before = black_patterns(&game).clone();
        place(&mut game, Stone::Black, row, mover_col); // capture 발동
        (game, before)
    }

    #[test]
    fn register_from_capture_axis_open_two() {
        // (beyond_anchor_pattern, away_pattern, expected)
        let cases: Vec<(&str, &str, Option<Vec<Position>>)> = vec![
            // X..X
            ("", "", Some(vec![(5, 10), (5, 9), (5, 8), (5, 7), (5, 6), (5, 5), (5, 4), (5, 3)])),
            // X..X + .O
            ("", ".O", Some(vec![(5, 9), (5, 8), (5, 7), (5, 6), (5, 5), (5, 4), (5, 3)])),
            // O + X..X
            ("O", "", None),
            // X..X + O
            ("", "O", None),
        ];

        let mut failures = Vec::new();
        for (beyond_anchor_pattern, away_pattern, expected) in cases {
            let (game, before) = setup_capture_axis(5, 8, beyond_anchor_pattern, away_pattern);
            if !before.open_two.is_empty() {
                failures.push(format!("beyond={beyond_anchor_pattern:?} away={away_pattern:?}: 캡처 전에 이미 등록됨"));
                continue;
            }

            let actual = &black_patterns(&game).open_two;
            match expected {
                Some(expected) => {
                    if *actual != vec![expected] {
                        failures.push(format!("beyond={beyond_anchor_pattern:?} away={away_pattern:?}: 등록된 결과가 기대값과 다름: {actual:?}"));
                    }
                }
                None => {
                    if !actual.is_empty() {
                        failures.push(format!("beyond={beyond_anchor_pattern:?} away={away_pattern:?}: 등록되지 않아야 하는데 {actual:?}"));
                    }
                }
            }
        }
        assert!(failures.is_empty(), "\n{}", failures.join("\n"));
    }

    #[test]
    fn register_from_capture_axis_open_three() {
        let cases: Vec<(&str, &str, Vec<Position>)> = vec![
            // O + X..X + XO -> OX..XXO
            ("O", "XO", vec![(5, 9), (5, 8), (5, 7), (5, 6), (5, 5)]),
        ];

        let mut failures = Vec::new();
        for (beyond_anchor_pattern, away_pattern, expected) in cases {
            let (game, before) = setup_capture_axis(5, 8, beyond_anchor_pattern, away_pattern);
            if !before.open_three.is_empty() {
                failures.push(format!("beyond={beyond_anchor_pattern:?} away={away_pattern:?}: 캡처 전에 이미 등록됨"));
                continue;
            }

            let actual = &black_patterns(&game).open_three;
            if *actual != vec![expected] {
                failures.push(format!("beyond={beyond_anchor_pattern:?} away={away_pattern:?}: 등록된 결과가 기대값과 다름: {actual:?}"));
            }
        }
        assert!(failures.is_empty(), "\n{}", failures.join("\n"));
    }

    #[test]
    fn register_from_capture_axis_free_three() {
        let cases: Vec<(&str, &str, Vec<Position>)> = vec![
            // X..X + .X -> X..X.X
            ("", ".X", vec![(5, 10), (5, 9), (5, 8), (5, 7), (5, 6), (5, 5), (5, 4), (5, 3)]),
            // O + X..X + .XO -> OX..X.XO
            ("O", ".XO", vec![(5, 10), (5, 9), (5, 8), (5, 7), (5, 6), (5, 5)]),
        ];

        let mut failures = Vec::new();
        for (beyond_anchor_pattern, away_pattern, expected) in cases {
            let (game, before) = setup_capture_axis(5, 8, beyond_anchor_pattern, away_pattern);
            if !before.free_three.is_empty() {
                failures.push(format!("beyond={beyond_anchor_pattern:?} away={away_pattern:?}: 캡처 전에 이미 등록됨"));
                continue;
            }

            let actual = &black_patterns(&game).free_three;
            if *actual != vec![expected.clone()] {
                failures.push(format!("beyond={beyond_anchor_pattern:?} away={away_pattern:?}: 등록된 결과가 기대값과 다름: {actual:?}"));
            }
            if !black_patterns(&game).open_three.is_empty() {
                failures.push(format!("beyond={beyond_anchor_pattern:?} away={away_pattern:?}: free_three와 open_three가 동시에 등록됨: {:?}", black_patterns(&game).open_three));
            }
        }
        assert!(failures.is_empty(), "\n{}", failures.join("\n"));
    }

    #[test]
    fn register_from_capture_axis_block_four() {
        // X..X + XXXO => X..XXXXO
        let (game, before) = setup_capture_axis(5, 8, "", "XXXO");
        assert!(before.block_four.is_empty(), "캡처 전에 이미 등록됨");

        let expected = vec![
            (5, 11), (5, 10), (5, 9), (5, 8), (5, 7), (5, 6), (5, 5), (5, 4), (5, 3),
        ];
        assert_eq!(black_patterns(&game).block_four, vec![expected]);
        assert!(black_patterns(&game).open_four.is_empty());
    }

    #[test]
    fn capture_exposes_off_axis_pattern_through_near_mover_cell() {
        // 가로 캡처: Black(5,5)-White(5,6)-White(5,7)-Black(5,8). (5,7)은 mover(5,8)에 붙은 칸.
        // 세로로 (4,7)/(6,7)에 미리 Black을 둬서, 캡처로 (5,7)이 비면 그 자리를 지나는 세로
        // open_two가 새로 생겨야 한다.
        let mut game = Gomoku::new(19);
        place(&mut game, Stone::Black, 4, 7);
        place(&mut game, Stone::Black, 6, 7);
        place(&mut game, Stone::Black, 5, 5);
        place(&mut game, Stone::White, 5, 6);
        place(&mut game, Stone::White, 5, 7);

        let before = black_patterns(&game).clone();
        place(&mut game, Stone::Black, 5, 8); // capture 발동, (5,6)/(5,7) 빈칸으로

        assert!(before.open_two.is_empty(), "캡처 전에 이미 등록됨");
        assert_eq!(game.board[5][7], Stone::Empty);

        let vertical_expected = vec![(2, 7), (3, 7), (4, 7), (5, 7), (6, 7), (7, 7), (8, 7)];
        assert!(
            black_patterns(&game).open_two.contains(&vertical_expected),
            "(5,7)을 지나는 세로 open_two가 있어야 함: {:?}",
            black_patterns(&game).open_two
        );
    }

    #[test]
    fn capture_exposes_off_axis_pattern_through_near_anchor_cell() {
        // 위와 대칭: (5,6)은 anchor(5,5)에 붙은 칸. 세로로 (3,6)/(7,6)에 Black을 둬서
        // 캡처 후 (5,6)을 지나는 세로 open_two가 생겨야 한다. (4,6)/(6,6)처럼 anchor 바로
        // 옆(대각선)에 두면 anchor와 대각선 open_three를 이뤄 setup 도중 already-registered
        // 체크에 걸리므로 한 칸 더 띄운다.
        let mut game = Gomoku::new(19);
        place(&mut game, Stone::Black, 3, 6);
        place(&mut game, Stone::Black, 7, 6);
        place(&mut game, Stone::Black, 5, 5);
        place(&mut game, Stone::White, 5, 6);
        place(&mut game, Stone::White, 5, 7);

        let before = black_patterns(&game).clone();
        place(&mut game, Stone::Black, 5, 8); // capture 발동, (5,6)/(5,7) 빈칸으로

        assert!(before.open_two.is_empty(), "캡처 전에 이미 등록됨");
        assert_eq!(game.board[5][6], Stone::Empty);

        let vertical_expected = vec![(2, 6), (3, 6), (4, 6), (5, 6), (6, 6), (7, 6), (8, 6)];
        assert!(
            black_patterns(&game).open_two.contains(&vertical_expected),
            "(5,6)을 지나는 세로 open_two가 있어야 함: {:?}",
            black_patterns(&game).open_two
        );
    }
}
