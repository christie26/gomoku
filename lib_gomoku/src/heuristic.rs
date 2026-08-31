use crate::constants::HEURISTIC_EVAL_CLAMP;
use crate::search_board::{Cell, SearchBoard};
use crate::Gomoku;
use pyo3::prelude::*;

/// Static evaluation of a `Gomoku` position, from Black's point of view.
///
/// Delegates to the same scanner and weights the search runs on
/// (`SearchBoard` / `PatternCounts::score`), so the GUI's readout and the AI's
/// leaf scores can't drift apart. Note the clamp differs from
/// `sb_heuristic_evaluation`'s: the two evaluators have always used slightly
/// different margins and constants.rs keeps them distinct.
#[pyfunction]
pub fn heuristic_evaluation(state: &Gomoku) -> i32 {
    let board = SearchBoard::from_gomoku(state);
    let active = Cell::of(state.current_player);
    let black_score = board
        .black_patterns
        .score(board.captures[Cell::Black as usize], active == Cell::Black);
    let white_score = board
        .white_patterns
        .score(board.captures[Cell::White as usize], active == Cell::White);
    (black_score - white_score).clamp(-HEURISTIC_EVAL_CLAMP, HEURISTIC_EVAL_CLAMP)
}
