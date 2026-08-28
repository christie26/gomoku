use crate::constants::{
    CAPTURE_BONUS_1, CAPTURE_BONUS_2, CAPTURE_BONUS_3, CAPTURE_BONUS_4_PLUS,
    COMBO_BLOCK_FOUR_AND_THREE, COMBO_CAPTURE_AND_FOUR, COMBO_DOUBLE_BLOCK_FOUR,
    COMBO_DOUBLE_OPEN_FOUR, COMBO_DOUBLE_THREE, COMBO_OPEN_AND_BLOCK_FOUR,
    COMBO_OPEN_FOUR_AND_THREE, HEURISTIC_EVAL_CLAMP, TEMPO_BLOCK_FOUR, TEMPO_CAPTURE,
    TEMPO_OPEN_FOUR, WEIGHT_BLOCK_FOUR, WEIGHT_FIVE, WEIGHT_FREE_THREE, WEIGHT_OPEN_FOUR,
    WEIGHT_OPEN_THREE, WEIGHT_OPEN_TWO,
};
use crate::{Gomoku, Stone};
use pyo3::prelude::*;

// <<<<<<< HEAD
fn evaluate_player(state: &Gomoku, player: &Stone, is_active: bool) -> i32 {
    let p = state.patterns.get(player);
    let captures = *state.capture_count.get(player).unwrap_or(&0);
    let open_twos = p.map_or(0, |p| p.open_two.len() as i32);
    let free_threes = p.map_or(0, |p| p.free_three.len() as i32);
    let open_threes = p.map_or(0, |p| p.open_three.len() as i32);
    let block_fours = p.map_or(0, |p| p.block_four.len() as i32);
    let open_fours = p.map_or(0, |p| p.open_four.len() as i32);
    let five_rows = p.map_or(0, |p| p.five_row.len() as i32);

    let mut score = 0i32;

    // --- Base pattern scores ---
    score += five_rows * WEIGHT_FIVE;
    score += open_fours * WEIGHT_OPEN_FOUR;
    score += block_fours * WEIGHT_BLOCK_FOUR;
    score += free_threes * WEIGHT_FREE_THREE;
    score += open_threes * WEIGHT_OPEN_THREE;
    score += open_twos * WEIGHT_OPEN_TWO;

    // --- Non-linear capture scaling ---
    // Each capture pair gets progressively more dangerous as you approach 5 (win)
    score += match captures {
        0 => 0,
        1 => CAPTURE_BONUS_1,
        2 => CAPTURE_BONUS_2,
        3 => CAPTURE_BONUS_3,
        4 => CAPTURE_BONUS_4_PLUS,
        _ => CAPTURE_BONUS_4_PLUS,
    };

    // --- Double-threat detection ---
    // Combinations that can't all be blocked in one move
    let total_threes = open_threes + free_threes;

    if open_fours >= 2 {
        score += COMBO_DOUBLE_OPEN_FOUR; // Two open fours = unstoppable
    }
    if open_fours >= 1 && block_fours >= 1 {
        score += COMBO_OPEN_AND_BLOCK_FOUR; // Open four + block four = can't address both
    }
    if block_fours >= 2 {
        score += COMBO_DOUBLE_BLOCK_FOUR; // Two block fours = opponent can only block one
    }
    if open_fours >= 1 && total_threes >= 1 {
        score += COMBO_OPEN_FOUR_AND_THREE; // Open four + three = overwhelming
    }
    if block_fours >= 1 && total_threes >= 1 {
        score += COMBO_BLOCK_FOUR_AND_THREE; // Block four + three = force block, then threaten with three
    }
    if total_threes >= 2 {
        score += COMBO_DOUBLE_THREE; // Double three = likely winning
    }
    // Capture threats compound with pattern threats
    if captures >= 4 && (block_fours >= 1 || open_fours >= 1) {
        score += COMBO_CAPTURE_AND_FOUR; // One capture from win + four threat = two winning paths
    }

    // --- Tempo / turn awareness ---
    // Active player's threats are immediate; opponent must respond
    if is_active {
        if open_fours >= 1 {
            score += TEMPO_OPEN_FOUR;
        }
        if block_fours >= 1 {
            score += TEMPO_BLOCK_FOUR;
        }
        if captures >= 4 {
            score += TEMPO_CAPTURE;
        }
    }

    score
// =======
// const CAPTURE_WEIGHT: f32 = 50_000.;
//
// const OPEN_TWO_WEIGHT: f32 = 100.;
// const OPEN_THREE_WEIGHT: f32 = 1_000.;
// const FREE_THREE_WEIGHT: f32 = 70_000.;
// const BLOCK_FOUR_WEIGHT: f32 = 50_000.;
// const OPEN_FOUR_WEIGHT: f32 = 900_000.;
//
// fn multiplier(rank: usize) -> f32 {
//   match rank {
//     0 => 0.0,
//     1 => 0.1,
//     2 => 1.0,
//     3 => 2.,
//     _ => 2.,
//   }
// }
//
// fn capture_multiplier(rank: &i32) -> f32 {
//   match rank {
//     0 => 0.0,
//     1 => 0.1,
//     2 => 0.3,
//     3 => 0.5,
//     4 => 0.9,
//     _ => 1.,
//   }
// }
//
// fn evaluate_player(state: &Gomoku, player: &Stone) -> f32 {
//   
//     let capture_count = state.capture_count.get(player).unwrap_or(&0);
//     let capture_score = capture_multiplier(capture_count) * CAPTURE_WEIGHT;
//
//     let p = state.patterns.get(player);
//
//     let open_two_count = p.map_or(0, |p| p.open_two.len());
//     let open_two_score = multiplier(open_two_count) * OPEN_TWO_WEIGHT;
//
//     let free_three_count = p.map_or(0, |p| p.free_three.len());
//     let free_three_score = multiplier(free_three_count) * FREE_THREE_WEIGHT;
//     
//     let open_three_count = p.map_or(0, |p| p.open_three.len());
//     let open_three_score = multiplier(open_three_count) * OPEN_THREE_WEIGHT;
//
//     let block_four_count = p.map_or(0, |p| p.block_four.len());
//     let block_four_score = multiplier(block_four_count) * BLOCK_FOUR_WEIGHT;
//
//     let open_four_count = p.map_or(0, |p| p.open_four.len());
//     let open_four_score = multiplier(open_four_count) * OPEN_FOUR_WEIGHT;
//
//     capture_score
//         + open_two_score
//         + open_three_score
//         + block_four_score
//         + open_four_score
//         + free_three_score
// >>>>>>> origin/main
}

#[pyfunction]
pub fn heuristic_evaluation(state: &Gomoku) -> i32 {
    let black_score = evaluate_player(
        state,
        &Stone::Black,
        state.current_player == Stone::Black,
    );
    let white_score = evaluate_player(
        state,
        &Stone::White,
        state.current_player == Stone::White,
    );
    // Clamp to stay within alpha-beta bounds (leaves room for depth adjustment)
    (black_score - white_score).clamp(-HEURISTIC_EVAL_CLAMP, HEURISTIC_EVAL_CLAMP)
}
