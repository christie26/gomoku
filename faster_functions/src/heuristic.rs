use crate::{Gomoku, Stone};
use pyo3::prelude::*;

fn evaluate_player(state: &Gomoku, player: Stone) -> i32 {
    let capture_score = state.capture_count.get(&player).unwrap_or(&0) * 2000;
    let open_two_score = state.open_two.get(&player).map_or(0, |v| v.len() as i32) * 10;
    let open_three_score = state.open_three.get(&player).map_or(0, |v| v.len() as i32) * 100;
    let block_four_score = state.block_four.get(&player).map_or(0, |v| v.len() as i32) * 1000;
    let open_four_score = state.open_four.get(&player).map_or(0, |v| v.len() as i32) * 5000;
    let free_three_score = state.free_three.get(&player).map_or(0, |v| v.len() as i32) * 10000;
    let five_row_score = state.five_row.get(&player).map_or(0, |v| v.len() as i32) * 100000;

    // println!("[Player {}] Capture: {}, Open 2: {}, Open 3: {}, \
    // Block 4: {}, Open 4: {}, Free 3: {}, 5: {}", 
    // player, capture_score, open_two_score, open_three_score,
    // block_four_score, open_four_score,free_three_score, five_row_score);
    

    capture_score
        + open_two_score
        + open_three_score
        + block_four_score
        + open_four_score
        + free_three_score
        + five_row_score
}


#[pyfunction]
pub fn heuristic_evaluation(state: &Gomoku) -> i32 {
    let current = evaluate_player(state, state.current_player);
    let opponent = evaluate_player(state, state.opponent_player);
    let heuristic = current - opponent;

    // println!("Move: ({}), heuristic: {}\n",state.current_move.map(|x| 
    //   format!("{}, {}", x.0, x.1))
    //   .unwrap_or("null".to_string()) ,heuristic);
    heuristic
}
