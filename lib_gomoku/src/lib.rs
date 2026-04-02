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
    free_three: HashMap<Stone, Vec<Pattern>>,
    five_row: HashMap<Stone, Vec<Pattern>>,
    open_two: HashMap<Stone, Vec<Pattern>>,
    open_three: HashMap<Stone, Vec<Pattern>>,
    open_four: HashMap<Stone, Vec<Pattern>>,
    block_four: HashMap<Stone, Vec<Pattern>>,
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

        let mut free_three = HashMap::new();
        free_three.insert(Stone::Black, Vec::new());
        free_three.insert(Stone::White, Vec::new());

        let mut five_row = HashMap::new();
        five_row.insert(Stone::Black, Vec::new());
        five_row.insert(Stone::White, Vec::new());

        let mut open_two = HashMap::new();
        open_two.insert(Stone::Black, Vec::new());
        open_two.insert(Stone::White, Vec::new());

        let mut open_three = HashMap::new();
        open_three.insert(Stone::Black, Vec::new());
        open_three.insert(Stone::White, Vec::new());

        let mut open_four = HashMap::new();
        open_four.insert(Stone::Black, Vec::new());
        open_four.insert(Stone::White, Vec::new());

        let mut block_four = HashMap::new();
        block_four.insert(Stone::Black, Vec::new());
        block_four.insert(Stone::White, Vec::new());

        Gomoku {
            size,
            board,
            current_player: Stone::Black,
            opponent_player: Stone::White,
            capture_count,
            free_three,
            five_row,
            open_two,
            open_three,
            open_four,
            block_four,
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
        println!("size: {:?}", self.size);
        println!("current_player: {:?}", self.current_player);
        println!("opponent_player: {:?}", self.opponent_player);
        println!("capture_count: {:?}", self.capture_count);
        println!("free_three: {:?}", self.free_three);
        println!("five_row: {:?}", self.five_row);
        println!("open_two: {:?}", self.open_two);
        println!("open_three: {:?}", self.open_three);
        println!("open_four: {:?}", self.open_four);
        println!("block_four: {:?}", self.block_four);
        println!("win_capture_count: {:?}", self.win_capture_count);
        println!("current_move: {:?}", self.current_move);
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

    fn is_double_three_move(&self, x: i32, y: i32) -> bool {
        let new_free_threes = self.get_free_threes_from_move(x, y);

        if new_free_threes.len() > 1 {
            true
        } else {
            false
        }
    }

    fn get_free_threes_from_move(&self, x0: i32, y0: i32) -> Vec<Pattern> {
        let mut new_free_threes = Vec::new();
        let directions = [(1, -1), (1, 0), (1, 1), (0, 1)];

        for (dx, dy) in directions {
            let (plus_my, plus_empty, plus_hole) = self.count_free_three(1, dx, dy, x0, y0);
            let (minus_my, minus_empty, minus_hole) = self.count_free_three(-1, dx, dy, x0, y0);

            if plus_my + minus_my == 2 && plus_empty + minus_empty >= 3 {
                let mut adjusted_plus_empty = plus_empty;
                let mut adjusted_minus_empty = minus_empty;

                if plus_hole && minus_empty == 2 {
                    adjusted_minus_empty = 1;
                }
                if minus_hole && plus_empty == 2 {
                    adjusted_plus_empty = 1;
                }

                let plus_end = adjusted_plus_empty + plus_my;
                let minus_end = adjusted_minus_empty + minus_my;

                let mut points = Vec::new();
                for i in (-minus_end as i32)..=(plus_end as i32) {
                    points.push((x0 + dx * i, y0 + dy * i));
                }
                new_free_threes.push(points);
            }
        }
        new_free_threes
    }

    fn count_free_three(&self, sign: i32, dx: i32, dy: i32, x0: i32, y0: i32) -> (i32, i32, bool) {
        let mut my_count = 0;
        let mut empty_count = 0;
        let mut i = 1;
        let mut hole = false;

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
                if empty_count > 0 {
                    hole = true;
                }
                my_count += 1;
            } else {
                empty_count += 1;
            }
            i += 1;
        }
        (my_count, empty_count, hole)
    }

    fn get_free_threes_from_capture(&self, x0: i32, y0: i32) -> Vec<Pattern> {
        let mut new_free_threes = Vec::new();
        let directions = [(1, -1), (1, 0), (1, 1), (0, 1)];

        for (dx, dy) in directions {
            let (plus_my, plus_empty, plus_hole) = self.count_free_three(1, dx, dy, x0, y0);
            let (minus_my, minus_empty, minus_hole) = self.count_free_three(-1, dx, dy, x0, y0);

            if (plus_my == 3 && plus_empty == 2) || (minus_my == 3 && minus_empty == 2) {
                if plus_my == 3 && plus_empty == 2 {
                    let mut points = Vec::new();
                    for i in 0..=(plus_my + plus_empty) {
                        points.push((x0 + dx * i, y0 + dy * i));
                    }
                    new_free_threes.push(points);
                }
                if minus_my == 3 && minus_empty == 2 {
                    let mut points = Vec::new();
                    for i in (-(minus_my + minus_empty))..=0 {
                        points.push((x0 + dx * i, y0 + dy * i));
                    }
                    new_free_threes.push(points);
                }
            } else if !plus_hole && !minus_hole {
                if plus_my + minus_my == 3 && plus_empty > 0 && minus_empty > 0 {
                    let plus_end = plus_my + 1;
                    let minus_end = minus_my + 1;
                    let mut points = Vec::new();
                    for i in (-minus_end)..=(plus_end) {
                        points.push((x0 + dx * i, y0 + dy * i));
                    }
                    new_free_threes.push(points);
                }
            }
        }
        new_free_threes
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

            self.remove_free_three(x, y, &self.opponent_player.clone());
            self.remove_opens(x, y);
        }

        for i in 1..3 {
            let x = x0 + dx * i;
            let y = y0 + dy * i;

            self.add_free_threes(
                self.get_free_threes_from_capture(x, y),
                &self.current_player.clone(),
            );
            self.add_opens_from_capture(x, y);
        }

        removed
    }

    fn add_opens_from_move(&mut self, x0: i32, y0: i32) {
        let directions = [(1, 0), (0, 1), (1, 1), (1, -1)];

        for (dx, dy) in directions {
            let (plus_my, plus_open) = self.count_open(1, dx, dy, x0, y0);
            let (minus_my, minus_open) = self.count_open(-1, dx, dy, x0, y0);

            let total_my = plus_my + minus_my;

            if total_my == 1 && plus_open && minus_open {
                let mut points = Vec::new();
                for i in (-(minus_my + 1))..=(plus_my + 1) {
                    points.push((x0 + dx * i, y0 + dy * i));
                }
                self.open_two
                    .get_mut(&self.current_player)
                    .unwrap()
                    .push(points);
            } else if total_my == 2 && plus_open && minus_open {
                let mut points = Vec::new();
                for i in (-(minus_my + 1))..=(plus_my + 1) {
                    points.push((x0 + dx * i, y0 + dy * i));
                }
                self.open_three
                    .get_mut(&self.current_player)
                    .unwrap()
                    .push(points);
            } else if total_my == 3 && (plus_open || minus_open) {
                let mut points = Vec::new();
                for i in (-(minus_my + 1))..=(plus_my + 1) {
                    points.push((x0 + dx * i, y0 + dy * i));
                }
                self.open_four
                    .get_mut(&self.current_player)
                    .unwrap()
                    .push(points);
            } else if total_my == 3 && (plus_open || minus_open) {
                let plus_end = plus_my + 1 + (plus_open as i32);
                let minus_end = minus_my + (minus_open as i32);

                let mut points = Vec::new();
                for i in (-(minus_end))..(plus_end) {
                    points.push((x0 + dx * i, y0 + dy * i));
                }
                self.block_four
                    .get_mut(&self.current_player)
                    .unwrap()
                    .push(points);
            } else if total_my == 4 {
                let mut points = Vec::new();
                for i in (-minus_my)..=(plus_my) {
                    points.push((x0 + dx * i, y0 + dy * i));
                }
                self.five_row
                    .get_mut(&self.current_player)
                    .unwrap()
                    .push(points);
            }
        }
    }

    fn add_opens_from_capture(&mut self, x0: i32, y0: i32) {
        let directions = [
            (1, 0),
            (0, 1),
            (1, 1),
            (1, -1),
            (-1, 0),
            (0, -1),
            (-1, -1),
            (-1, 1),
        ];

        for (dx, dy) in directions {
            let (count_my, open) = self.count_open(1, dx, dy, x0, y0);

            if count_my == 2 && open {
                let mut points = Vec::new();
                for i in (0)..=(count_my + 1) {
                    points.push((x0 + dx * i, y0 + dy * i));
                }
                self.open_two
                    .get_mut(&self.current_player)
                    .unwrap()
                    .push(points);
            } else if count_my == 3 && open {
                let mut points = Vec::new();
                for i in (0)..=(count_my + 1) {
                    points.push((x0 + dx * i, y0 + dy * i));
                }
                self.open_three
                    .get_mut(&self.current_player)
                    .unwrap()
                    .push(points);
            } else if count_my == 4 && open {
                if open {
                    let mut points = Vec::new();
                    for i in (0)..=(count_my + 1) {
                        points.push((x0 + dx * i, y0 + dy * i));
                    }
                    self.open_four
                        .get_mut(&self.current_player)
                        .unwrap()
                        .push(points);
                } else {
                    let mut points = Vec::new();
                    for i in (0)..=(count_my) {
                        points.push((x0 + dx * i, y0 + dy * i));
                    }
                    self.block_four
                        .get_mut(&self.current_player)
                        .unwrap()
                        .push(points);
                }
            }
        }
    }

    fn count_open(&self, sign: i32, dx: i32, dy: i32, x0: i32, y0: i32) -> (i32, bool) {
        let mut my_count = 0;
        let mut i = 1;

        loop {
            let x = x0 + dx * i * sign;
            let y = y0 + dy * i * sign;

            if !self.is_on_board(x, y) || self.board[x as usize][y as usize] != self.current_player
            {
                break;
            }
            my_count += 1;
            i += 1;
        }

        let x = x0 + dx * i * sign;
        let y = y0 + dy * i * sign;
        let open = self.is_on_board(x, y) && self.board[x as usize][y as usize] == Stone::Empty;

        (my_count, open)
    }

    fn add_free_threes(&mut self, new_free_threes: Vec<Pattern>, player: &Stone) {
        let player_list = self.free_three.get_mut(player).unwrap();
        for pattern in new_free_threes {
            if !player_list.contains(&pattern) {
                player_list.push(pattern);
            }
        }
    }

    fn remove_free_three(&mut self, x: i32, y: i32, player: &Stone) {
        let player_list = self.free_three.get_mut(player).unwrap();
        let pos = (x, y);

        player_list.retain_mut(|free_three| {
            if !free_three.contains(&pos) {
                return true;
            }
            if free_three.len() == 7 && (free_three[0] == pos || free_three[6] == pos) {
                free_three.retain(|&p| p != pos);
                return true;
            }
            false
        });
    }

    fn remove_opens(&mut self, x: i32, y: i32) {
        let pos = (x, y);
        let players = [self.current_player.clone(), self.opponent_player.clone()];

        let mut new_block_four_x = Vec::new();
        let mut new_block_four_y = Vec::new();
        for player in &players {
            self.open_two
                .get_mut(player)
                .unwrap()
                .retain(|pattern| !pattern.contains(&pos));
            self.open_three
                .get_mut(player)
                .unwrap()
                .retain(|pattern| !pattern.contains(&pos));
            self.open_four.get_mut(player).unwrap().retain(|pattern| {
                if !pattern.contains(&pos) {
                    true
                } else {
                    if pos == pattern[0] {
                        if player == &Stone::Black {
                            new_block_four_x.push(pattern[1..].to_vec());
                        } else {
                            new_block_four_y.push(pattern[1..].to_vec());
                        }
                    } else if pos == pattern[pattern.len() - 1] {
                        if player == &Stone::Black {
                            new_block_four_x.push(pattern[..pattern.len() - 1].to_vec());
                        } else {
                            new_block_four_y.push(pattern[..pattern.len() - 1].to_vec());
                        }
                    }
                    false
                }
            });
            self.block_four
                .get_mut(player)
                .unwrap()
                .retain(|pattern| !pattern.contains(&pos));
            self.five_row
                .get_mut(player)
                .unwrap()
                .retain(|pattern| !pattern.contains(&pos));
        }
        self.block_four
            .get_mut(&Stone::Black)
            .unwrap()
            .extend(new_block_four_x);
        self.block_four
            .get_mut(&Stone::White)
            .unwrap()
            .extend(new_block_four_y);
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

    pub fn handle_move_simple_ruleset(&mut self, x: i32, y: i32) -> (MoveResult, i32) {
        let result = self.is_valid_move_simple_ruleset(x, y);
        let mut capture_count = 0;

        if result == MoveResult::Valid {
            self.current_move = Some((x, y));
            self.move_count += 1;
            self.board[x as usize][y as usize] = self.current_player.clone();

            // self.remove_free_three(x, y, &self.opponent_player.clone());
            self.remove_opens(x, y);

            // self.add_free_threes(
            //     self.get_free_threes_from_move(x, y),
            //     &self.current_player.clone(),
            // );
            self.add_opens_from_move(x, y);

            // capture_count = self.capture_center(x, y);
        }

        (result, capture_count)
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

            self.remove_free_three(x, y, &self.opponent_player.clone());
            self.remove_opens(x, y);

            self.add_free_threes(
                self.get_free_threes_from_move(x, y),
                &self.current_player.clone(),
            );
            self.add_opens_from_move(x, y);

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
            if let Some(opponent_fives) = self.five_row.get(&self.opponent_player) {
                if !opponent_fives.is_empty() {
                    return Some(self.opponent_player.to_string());
                }
                // for five_row in opponent_fives {
                //     println!("five_row: {:?}", five_row);
                //     let mut all_opponent = true;
                //     for &(fx, fy) in five_row {
                //         if self.board[fx as usize][fy as usize] != self.opponent_player {
                //             all_opponent = false;
                //             break;
                //         }
                //     }
                //     if all_opponent {
                //         return Some(self.opponent_player.to_string());
                //     }
                // }
            }
            // 3. check current's five_row
            if let Some(my_fives) = self.five_row.get(&self.current_player) {
                'each_five: for five_row in my_fives {
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
        self.free_three
            .iter()
            .map(|(stone, c)| (stone.to_string(), c.clone()))
            .collect()
    }

    #[getter]
    fn five_row(&self) -> HashMap<String, Vec<Vec<(i32, i32)>>> {
        self.five_row
            .iter()
            .map(|(stone, c)| (stone.to_string(), c.clone()))
            .collect()
    }

    #[getter]
    fn open_two(&self) -> HashMap<String, Vec<Vec<(i32, i32)>>> {
        self.open_two
            .iter()
            .map(|(stone, c)| (stone.to_string(), c.clone()))
            .collect()
    }

    #[getter]
    fn open_three(&self) -> HashMap<String, Vec<Vec<(i32, i32)>>> {
        self.open_three
            .iter()
            .map(|(stone, c)| (stone.to_string(), c.clone()))
            .collect()
    }

    #[getter]
    fn open_four(&self) -> HashMap<String, Vec<Vec<(i32, i32)>>> {
        self.open_four
            .iter()
            .map(|(stone, c)| (stone.to_string(), c.clone()))
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
