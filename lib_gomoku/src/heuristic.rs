use crate::{Gomoku, Stone};
use pyo3::prelude::*;

fn evaluate_player(state: &Gomoku, player: &Stone, is_active: bool) -> i32 {
    let captures = *state.capture_count.get(player).unwrap_or(&0);
    let open_twos = state.open_two.get(player).map_or(0, |v| v.len() as i32);
    let free_threes = state.free_three.get(player).map_or(0, |v| v.len() as i32);
    let open_threes = state.open_three.get(player).map_or(0, |v| v.len() as i32);
    let block_fours = state.block_four.get(player).map_or(0, |v| v.len() as i32);
    let open_fours = state.open_four.get(player).map_or(0, |v| v.len() as i32);
    let five_rows = state.five_row.get(player).map_or(0, |v| v.len() as i32);

    let mut score = 0i32;

    // --- Base pattern scores ---
    score += five_rows * 80_001;
    score += open_fours * 35_000;
    score += block_fours * 7_000;
    score += free_threes * 5_000;
    score += open_threes * 100;
    score += open_twos * 50;

    // --- Non-linear capture scaling ---
    // Each capture pair gets progressively more dangerous as you approach 5 (win)
    score += match captures {
        0 => 0,
        1 => 5_000,
        2 => 12_000,
        3 => 25_000,
        4 => 50_000,
        _ => 50_000,
    };

    // --- Double-threat detection ---
    // Combinations that can't all be blocked in one move
    let total_threes = open_threes + free_threes;

    if open_fours >= 2 {
        score += 40_000; // Two open fours = unstoppable
    }
    if open_fours >= 1 && block_fours >= 1 {
        score += 35_000; // Open four + block four = can't address both
    }
    if block_fours >= 2 {
        score += 30_000; // Two block fours = opponent can only block one
    }
    if open_fours >= 1 && total_threes >= 1 {
        score += 30_000; // Open four + three = overwhelming
    }
    if block_fours >= 1 && total_threes >= 1 {
        score += 20_000; // Block four + three = force block, then threaten with three
    }
    if total_threes >= 2 {
        score += 15_000; // Double three = likely winning
    }
    // Capture threats compound with pattern threats
    if captures >= 4 && (block_fours >= 1 || open_fours >= 1) {
        score += 25_000; // One capture from win + four threat = two winning paths
    }

    // --- Tempo / turn awareness ---
    // Active player's threats are immediate; opponent must respond
    if is_active {
        if open_fours >= 1 {
            score += 5_000;
        }
        if block_fours >= 1 {
            score += 3_000;
        }
        if captures >= 4 {
            score += 8_000;
        }
    }

    score
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
    // Clamp to stay within alpha-beta bounds (±99_999 leaves room for depth adjustment)
    (black_score - white_score).clamp(-99_999, 99_999)
}
