use pyo3::prelude::*;
use std::collections::{HashMap, HashSet};

#[pyclass]
#[derive(Clone, Copy, Debug, PartialEq)]
enum MoveResult {
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

#[pyclass]
pub struct Gomoku {
    size: usize,
    board: Vec<Vec<String>>,
    current_player: String,
    opponent_player: String,
    capture_count: HashMap<String, i32>,
    free_three_list: HashMap<String, Vec<Pattern>>,
    five_row: HashMap<String, Vec<Pattern>>,
    open_two: HashMap<String, Vec<Pattern>>,
    open_three: HashMap<String, Vec<Pattern>>,
    open_four: HashMap<String, Vec<Pattern>>,
    win_capture_count: i32,
    current_move: Option<Position>,
}

#[pymethods]
impl Gomoku {
    #[new]
    #[pyo3(signature = (size = 19))]
    fn new(size: usize) -> Self {
        let board = vec![vec![".".to_string(); size]; size];
        let mut capture_count = HashMap::new();
        capture_count.insert("X".to_string(), 0);
        capture_count.insert("O".to_string(), 0);

        let mut free_three_list = HashMap::new();
        free_three_list.insert("X".to_string(), Vec::new());
        free_three_list.insert("O".to_string(), Vec::new());

        let mut five_row = HashMap::new();
        five_row.insert("X".to_string(), Vec::new());
        five_row.insert("O".to_string(), Vec::new());

        let mut open_two = HashMap::new();
        open_two.insert("X".to_string(), Vec::new());
        open_two.insert("O".to_string(), Vec::new());

        let mut open_three = HashMap::new();
        open_three.insert("X".to_string(), Vec::new());
        open_three.insert("O".to_string(), Vec::new());

        let mut open_four = HashMap::new();
        open_four.insert("X".to_string(), Vec::new());
        open_four.insert("O".to_string(), Vec::new());

        Gomoku {
            size,
            board,
            current_player: "X".to_string(),
            opponent_player: "O".to_string(),
            capture_count,
            free_three_list,
            five_row,
            open_two,
            open_three,
            open_four,
            win_capture_count: 5,
            current_move: None,
        }
    }

    fn print_board(&self) {
        print!("  ");
        for i in 0..self.size {
            print!("{:2} ", i);
        }
        println!();

        for (i, row) in self.board.iter().enumerate() {
            print!("{:2} ", i);
            for cell in row {
                print!("{:2} ", cell);
            }
            println!();
        }
    }

    fn is_on_board(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && (x as usize) < self.size && (y as usize) < self.size
    }

    fn is_valid_move(&mut self, x: i32, y: i32) -> MoveResult {
        if !self.is_on_board(x, y) {
            return MoveResult::OutOfBoard;
        }
        if self.board[x as usize][y as usize] != "." {
            return MoveResult::NotEmpty;
        }
        if self.is_double_three_move(x, y) {
            return MoveResult::DoubleThree;
        }
        MoveResult::Valid
    }

    fn is_double_three_move(&mut self, x: i32, y: i32) -> bool {
        self.board[x as usize][y as usize] = self.current_player.clone();
        let new_free_threes = self.get_free_threes_from_move(x, y);
        self.board[x as usize][y as usize] = ".".to_string();

        if new_free_threes.len() > 1 {
            true
        } else {
            self.add_free_threes(new_free_threes, &self.current_player.clone());
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

    fn capture_center(&mut self, x0: i32, y0: i32) -> i32 {
        let directions = [
            (1, -1),
            (1, 0),
            (1, 1),
            (0, -1),
            (0, 1),
            (-1, -1),
            (-1, 0),
            (-1, 1),
        ];
        let mut capture_count = 0;

        for (dx, dy) in directions {
            if self.is_capture(x0, y0, dx, dy) {
                self.apply_capture(x0, y0, dx, dy);
                capture_count += 1;
            }
        }

        if capture_count > 0 {
            let current = self.current_player.clone();
            *self.capture_count.get_mut(&current).unwrap() += capture_count;
        }
        capture_count
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

    fn apply_capture(&mut self, x0: i32, y0: i32, dx: i32, dy: i32) {
        let mut new_free_threes = Vec::new();

        for i in 1..3 {
            let x = x0 + dx * i;
            let y = y0 + dy * i;
            self.board[x as usize][y as usize] = ".".to_string();
            self.remove_free_three(x, y, &self.opponent_player.clone());
            self.remove_opens(x, y);
        }

        for i in 1..3 {
            let x = x0 + dx * i;
            let y = y0 + dy * i;
            let threes = self.get_free_threes_from_capture(x, y);
            new_free_threes.extend(threes);
        }

        self.add_free_threes(new_free_threes, &self.current_player.clone());
    }

    fn check_opens(&mut self, x0: i32, y0: i32) {
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
        let open = self.is_on_board(x, y) && self.board[x as usize][y as usize] == ".";

        (my_count, open)
    }

    fn add_free_threes(&mut self, new_free_threes: Vec<Pattern>, player: &str) {
        let player_list = self.free_three_list.get_mut(player).unwrap();
        for pattern in new_free_threes {
            if !player_list.contains(&pattern) {
                player_list.push(pattern);
            }
        }
    }

    fn remove_free_three(&mut self, x: i32, y: i32, player: &str) {
        let player_list = self.free_three_list.get_mut(player).unwrap();
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
        let players = [&self.current_player.clone(), &self.opponent_player.clone()];

        for player in &players {
            self.open_two
                .get_mut(*player)
                .unwrap()
                .retain(|pattern| !pattern.contains(&pos));
            self.open_three
                .get_mut(*player)
                .unwrap()
                .retain(|pattern| !pattern.contains(&pos));
            self.open_four
                .get_mut(*player)
                .unwrap()
                .retain(|pattern| !pattern.contains(&pos));
            self.five_row
                .get_mut(*player)
                .unwrap()
                .retain(|pattern| !pattern.contains(&pos));
        }
    }

    fn handle_move(&mut self, x: i32, y: i32) -> (MoveResult, i32) {
        let result = self.is_valid_move(x, y);
        let mut capture_count = 0;

        if result == MoveResult::Valid {
            self.current_move = Some((x, y));
            self.board[x as usize][y as usize] = self.current_player.clone();
            self.remove_free_three(x, y, &self.opponent_player.clone());
            self.remove_opens(x, y);
            self.check_opens(x, y);
            capture_count = self.capture_center(x, y);
        }

        (result, capture_count)
    }

    fn count_empty_spots(&self) -> i32 {
        self.board
            .iter()
            .map(|row| row.iter().filter(|&cell| cell == ".").count() as i32)
            .sum()
    }

    fn get_winner(&self) -> Option<String> {
        if let Some((x, y)) = self.current_move {
            // Check five captures
            if *self.capture_count.get(&self.current_player).unwrap() >= self.win_capture_count {
                return Some(self.current_player.clone());
            }

            // Check if opponent has five in a row
            if !self.five_row.get(&self.opponent_player).unwrap().is_empty() {
                for five_row in self.five_row.get(&self.opponent_player).unwrap() {
                    let mut all_opponent = true;
                    for &(fx, fy) in five_row {
                        if self.board[fx as usize][fy as usize] != self.opponent_player {
                            all_opponent = false;
                            break;
                        }
                    }
                    if all_opponent {
                        return Some(self.opponent_player.clone());
                    }
                }
            }
        }
        None
    }

    fn check_draw(&self) -> bool {
        self.count_empty_spots() == 0
    }

    fn switch_player(&mut self) {
        if self.current_player == "X" {
            self.current_player = "O".to_string();
            self.opponent_player = "X".to_string();
        } else {
            self.current_player = "X".to_string();
            self.opponent_player = "O".to_string();
        }
    }

    // Getter methods for Python access
    #[getter]
    fn size(&self) -> usize {
        self.size
    }

    #[getter]
    fn current_player(&self) -> String {
        self.current_player.clone()
    }

    #[getter]
    fn opponent_player(&self) -> String {
        self.opponent_player.clone()
    }

    #[getter]
    fn capture_count(&self) -> HashMap<String, i32> {
        self.capture_count.clone()
    }

    #[getter]
    fn current_move(&self) -> Option<(i32, i32)> {
        self.current_move
    }

    #[getter]
    fn board(&self) -> Vec<Vec<String>> {
        self.board.clone()
    }

    #[getter]
    fn free_three_list(&self) -> HashMap<String, Vec<Vec<(i32, i32)>>> {
        self.free_three_list.clone()
    }

    #[getter]
    fn five_row(&self) -> HashMap<String, Vec<Vec<(i32, i32)>>> {
        self.five_row.clone()
    }

    #[getter]
    fn open_two(&self) -> HashMap<String, Vec<Vec<(i32, i32)>>> {
        self.open_two.clone()
    }

    #[getter]
    fn open_three(&self) -> HashMap<String, Vec<Vec<(i32, i32)>>> {
        self.open_three.clone()
    }

    #[getter]
    fn open_four(&self) -> HashMap<String, Vec<Vec<(i32, i32)>>> {
        self.open_four.clone()
    }

    #[getter]
    fn win_capture_count(&self) -> i32 {
        self.win_capture_count
    }

    fn __getstate__(
        &self,
    ) -> PyResult<(
        usize,
        Vec<Vec<String>>,
        String,
        String,
        HashMap<String, i32>,
        HashMap<String, Vec<Vec<(i32, i32)>>>,
        HashMap<String, Vec<Vec<(i32, i32)>>>,
        HashMap<String, Vec<Vec<(i32, i32)>>>,
        HashMap<String, Vec<Vec<(i32, i32)>>>,
        HashMap<String, Vec<Vec<(i32, i32)>>>,
        i32,
        Option<(i32, i32)>,
    )> {
        Ok((
            self.size,
            self.board.clone(),
            self.current_player.clone(),
            self.opponent_player.clone(),
            self.capture_count.clone(),
            self.free_three_list.clone(),
            self.five_row.clone(),
            self.open_two.clone(),
            self.open_three.clone(),
            self.open_four.clone(),
            self.win_capture_count,
            self.current_move,
        ))
    }

    fn __setstate__(
        &mut self,
        state: (
            usize,
            Vec<Vec<String>>,
            String,
            String,
            HashMap<String, i32>,
            HashMap<String, Vec<Vec<(i32, i32)>>>,
            HashMap<String, Vec<Vec<(i32, i32)>>>,
            HashMap<String, Vec<Vec<(i32, i32)>>>,
            HashMap<String, Vec<Vec<(i32, i32)>>>,
            HashMap<String, Vec<Vec<(i32, i32)>>>,
            i32,
            Option<(i32, i32)>,
        ),
    ) -> PyResult<()> {
        self.size = state.0;
        self.board = state.1;
        self.current_player = state.2;
        self.opponent_player = state.3;
        self.capture_count = state.4;
        self.free_three_list = state.5;
        self.five_row = state.6;
        self.open_two = state.7;
        self.open_three = state.8;
        self.open_four = state.9;
        self.win_capture_count = state.10;
        self.current_move = state.11;
        Ok(())
    }
}

#[pyfunction]
fn get_candidate_moves(state: &mut Gomoku, radius: i32) -> Vec<(usize, usize)> {
    if state.count_empty_spots() as usize == state.size * state.size {
        return vec![(9, 9)];
    }

    let mut candidates = HashSet::with_capacity(100);
    let (rows, cols) = (state.board.len(), state.board[0].len());
    let radius = radius as usize;

    for row in 0..rows {
        for col in 0..cols {
            if state.board[row][col] != "." {
                let start_row = row.saturating_sub(radius);
                let end_row = (row + radius + 1).min(rows);
                let start_col = col.saturating_sub(radius);
                let end_col = (col + radius + 1).min(cols);

                for r in start_row..end_row {
                    for c in start_col..end_col {
                        if state.is_valid_move(r as i32, c as i32) == MoveResult::Valid {
                            candidates.insert((r, c));
                        }
                    }
                }
            }
        }
    }

    candidates.into_iter().collect()
}

#[pymodule]
fn faster_functions(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<MoveResult>()?;
    m.add_class::<Gomoku>()?;
    m.add_function(wrap_pyfunction!(get_candidate_moves, m)?)?;
    let gomoku_class = m.getattr("Gomoku")?;
    gomoku_class.setattr("__module__", "faster_functions")?;

    let move_result_class = m.getattr("MoveResult")?;
    move_result_class.setattr("__module__", "faster_functions")?;
    Ok(())
}
