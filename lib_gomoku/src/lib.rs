use pyo3::prelude::*;
use std::collections::HashMap;
use std::sync::OnceLock;
use colored::*;

pub mod constants;
pub mod heuristic;
pub mod minimax;
pub mod search_board;

use search_board::{print_pattern_kind, Cell, SearchBoard};
pub use search_board::position_name;

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

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
#[pyclass]
pub enum Stone {
    Empty,
    Black,
    White,
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
    win_capture_count: i32,
    current_move: Option<Position>,
    move_count: usize,
    pub hash: u64,
}

impl Gomoku {
    /// Debug-only view of the recognized formations, keyed by stone. Derived
    /// from the board on demand — `SearchBoard` owns pattern recognition and
    /// keeps counts, not coordinates.
    fn pattern_ranges(&self) -> HashMap<String, search_board::PlayerRanges> {
        let (black, white) = SearchBoard::from_gomoku(self).sb_collect_patterns();
        HashMap::from([
            (Stone::Black.to_string(), black),
            (Stone::White.to_string(), white),
        ])
    }

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

        Gomoku {
            size,
            board,
            current_player: Stone::Black,
            opponent_player: Stone::White,
            capture_count,
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
        let (black, white) = SearchBoard::from_gomoku(self).sb_collect_patterns();
        for (player, p) in [(Stone::Black, &black), (Stone::White, &white)] {
            println!("patterns[{player}]:");
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
        SearchBoard::cells_from_gomoku(self).sb_is_double_three(x0, y0)
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
                captured_positions.extend(removed);
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
            self.remove_stone(x, y);
        }

        removed
    }

    /// Clear a stone (capture). Pattern state is derived on demand from the
    /// board by `SearchBoard`, so there is nothing to invalidate here.
    fn remove_stone(&mut self, x: i32, y: i32) {
        let z = zobrist();
        let color_idx = match self.board[x as usize][y as usize] {
            Stone::Black => 0,
            Stone::White => 1,
            Stone::Empty => return,
        };
        self.hash ^= z.board[x as usize * 19 + y as usize][color_idx];
        self.board[x as usize][y as usize] = Stone::Empty;
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

            // Update Zobrist hash for placed stone
            let z = zobrist();
            let color_idx = if self.current_player == Stone::Black { 0 } else { 1 };
            self.hash ^= z.board[x as usize * 19 + y as usize][color_idx];

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

            let board = SearchBoard::cells_from_gomoku(self);
            // 2. check opponent's five_row
            if board.has_five_in_row(Cell::of(self.opponent_player)) {
                return Some(self.opponent_player.to_string());
            }
            // 3. check current's five_row that no capture can break
            if board.has_uncapturable_five(Cell::of(self.current_player)) {
                return Some(self.current_player.to_string());
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
        self.pattern_ranges().into_iter().map(|(k, p)| (k, p.free_three)).collect()
    }

    #[getter]
    fn five_row(&self) -> HashMap<String, Vec<Vec<(i32, i32)>>> {
        self.pattern_ranges().into_iter().map(|(k, p)| (k, p.five_row)).collect()
    }

    #[getter]
    fn open_two(&self) -> HashMap<String, Vec<Vec<(i32, i32)>>> {
        self.pattern_ranges().into_iter().map(|(k, p)| (k, p.open_two)).collect()
    }

    #[getter]
    fn open_three(&self) -> HashMap<String, Vec<Vec<(i32, i32)>>> {
        self.pattern_ranges().into_iter().map(|(k, p)| (k, p.open_three)).collect()
    }

    #[getter]
    fn open_four(&self) -> HashMap<String, Vec<Vec<(i32, i32)>>> {
        self.pattern_ranges().into_iter().map(|(k, p)| (k, p.open_four)).collect()
    }

    #[getter]
    fn block_four(&self) -> HashMap<String, Vec<Vec<(i32, i32)>>> {
        self.pattern_ranges().into_iter().map(|(k, p)| (k, p.block_four)).collect()
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
mod tests;

