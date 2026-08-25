use crate::{Gomoku, Stone};
use pyo3::prelude::*;

const CAPTURE_WEIGHT: i32 = 20_000;

const OPEN_TWO_WEIGHT: i32 = 10;
const OPEN_THREE_WEIGHT: i32 = 100;
const FREE_THREE_WEIGHT: i32 = 1_000;
const BLOCK_FOUR_WEIGHT: i32 = 10_000;
const OPEN_FOUR_WEIGHT: i32 = 100_000;

fn evaluate_player(state: &Gomoku, player: &Stone) -> i32 {
  
    let capture_score = state.capture_count.get(player).unwrap_or(&0) * CAPTURE_WEIGHT;

    let p = state.patterns.get(player);
    let open_two_score = p.map_or(0, |p| p.open_two.len() as i32) * OPEN_TWO_WEIGHT;

    let free_three_count = p.map_or(0, |p| p.free_three.len() as i32);
    let free_three_score = free_three_count * FREE_THREE_WEIGHT;
    let open_three_score =
        (p.map_or(0, |p| p.open_three.len() as i32) - free_three_count) * OPEN_THREE_WEIGHT;

    let block_four_score = p.map_or(0, |p| p.block_four.len() as i32) * BLOCK_FOUR_WEIGHT;

    let open_four_score = p.map_or(0, |p| p.open_four.len() as i32) * OPEN_FOUR_WEIGHT;

    capture_score
        + open_two_score
        + open_three_score
        + block_four_score
        + open_four_score
        + free_three_score
}

#[pyfunction]
pub fn heuristic_evaluation(state: &Gomoku) -> i32 {
    let black_score = evaluate_player(state, &Stone::Black);
    let white_score = evaluate_player(state, &Stone::White);
    let heuristic = black_score - white_score;

    // println!("Current: {:?}, Move: ({}), heuristic: {}\n",state.current_player, state.current_move.map(|x|
    //   format!("{}, {}", x.0, x.1))
    //   .unwrap_or("null".to_string()) ,heuristic);
    heuristic
}
