use crate::{Gomoku, Stone};
use pyo3::prelude::*;

const CAPTURE_WEIGHT: f32 = 50_000.;

const OPEN_TWO_WEIGHT: f32 = 100.;
const OPEN_THREE_WEIGHT: f32 = 1_000.;
const FREE_THREE_WEIGHT: f32 = 70_000.;
const BLOCK_FOUR_WEIGHT: f32 = 50_000.;
const OPEN_FOUR_WEIGHT: f32 = 900_000.;

fn multiplier(rank: usize) -> f32 {
  match rank {
    0 => 0.0,
    1 => 0.1,
    2 => 1.0,
    3 => 2.,
    _ => 2.,
  }
}

fn capture_multiplier(rank: &i32) -> f32 {
  match rank {
    0 => 0.0,
    1 => 0.1,
    2 => 0.3,
    3 => 0.5,
    4 => 0.9,
    _ => 1.,
  }
}

fn evaluate_player(state: &Gomoku, player: &Stone) -> f32 {
  
    let capture_count = state.capture_count.get(player).unwrap_or(&0);
    let capture_score = capture_multiplier(capture_count) * CAPTURE_WEIGHT;

    let p = state.patterns.get(player);

    let open_two_count = p.map_or(0, |p| p.open_two.len());
    let open_two_score = multiplier(open_two_count) * OPEN_TWO_WEIGHT;

    let free_three_count = p.map_or(0, |p| p.free_three.len());
    let free_three_score = multiplier(free_three_count) * FREE_THREE_WEIGHT;
    
    let open_three_count = p.map_or(0, |p| p.open_three.len());
    let open_three_score = multiplier(open_three_count) * OPEN_THREE_WEIGHT;

    let block_four_count = p.map_or(0, |p| p.block_four.len());
    let block_four_score = multiplier(block_four_count) * BLOCK_FOUR_WEIGHT;

    let open_four_count = p.map_or(0, |p| p.open_four.len());
    let open_four_score = multiplier(open_four_count) * OPEN_FOUR_WEIGHT;

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
    (heuristic as i32).clamp(-99_999, 99_999)
}
