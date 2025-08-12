use crate::{Gomoku, Stone};
use pyo3::prelude::*;

fn evaluate_player(state: &Gomoku, player: Stone) -> i32 {
    let capture_score = state.capture_count.get(&player).unwrap_or(&0) * 2000;

    capture_score
        + state.open_two.get(&player).map_or(0, |v| v.len() as i32) * 50
        + state.open_three.get(&player).map_or(0, |v| v.len() as i32) * 500
        + state.open_four.get(&player).map_or(0, |v| v.len() as i32) * 1000
        + state.five_row.get(&player).map_or(0, |v| v.len() as i32) * 10000
}

#[pyfunction]
pub fn heuristic_evaluation(state: &Gomoku) -> i32 {
    let current = evaluate_player(state, state.current_player);
    let opponent = evaluate_player(state, state.opponent_player);
    current - opponent
}
