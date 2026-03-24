use crate::heuristic::heuristic_evaluation;
use crate::MoveResult;
use crate::{Gomoku, Stone};

use linked_hash_set::LinkedHashSet;
use pyo3::prelude::*;

use std::cmp::{max, min};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

const MAX_VALUE: i32 = 100_000;
const MIN_VALUE: i32 = -100_000;
const MAX_DEPTH: usize = 10;

pub const BOARD_SIZE: usize = 19;
const DIRECTIONS: &[(i32, i32)] = &[(1, 0), (0, 1), (1, 1), (1, -1)];

// Count stones in one direction from (x,y) for player
fn count_stones(
    board: &Vec<Vec<Stone>>,
    x: usize,
    y: usize,
    dx: i32,
    dy: i32,
    player: &Stone,
) -> i32 {
    let mut count = 0;
    for i in 1..5 {
        let nx = x as i32 + dx * i;
        let ny = y as i32 + dy * i;
        if nx < 0 || nx >= BOARD_SIZE as i32 || ny < 0 || ny >= BOARD_SIZE as i32 {
            break;
        }
        if board[nx as usize][ny as usize] == *player {
            count += 1;
        } else {
            break;
        }
    }
    count
}

fn opponent(player: Stone) -> Stone {
    match player {
        Stone::Black => Stone::White,
        Stone::White => Stone::Black,
        Stone::Empty => Stone::Empty, // no opponent if empty
    }
}

fn can_capture(
    board: &Vec<Vec<Stone>>,
    x: usize,
    y: usize,
    dx: i32,
    dy: i32,
    player: Stone,
) -> bool {
    let opp = opponent(player);

    // Positions to check:
    // (x + dx, y + dy), (x + 2*dx, y + 2*dy) - opponent stones
    // (x + 3*dx, y + 3*dy) - player stone

    let nx1 = x as i32 + dx;
    let ny1 = y as i32 + dy;
    let nx2 = x as i32 + 2 * dx;
    let ny2 = y as i32 + 2 * dy;
    let nx3 = x as i32 + 3 * dx;
    let ny3 = y as i32 + 3 * dy;

    if nx3 < 0 || nx3 >= BOARD_SIZE as i32 || ny3 < 0 || ny3 >= BOARD_SIZE as i32 {
        return false;
    }
    if nx2 < 0 || nx2 >= BOARD_SIZE as i32 || ny2 < 0 || ny2 >= BOARD_SIZE as i32 {
        return false;
    }
    if nx1 < 0 || nx1 >= BOARD_SIZE as i32 || ny1 < 0 || ny1 >= BOARD_SIZE as i32 {
        return false;
    }

    board[nx1 as usize][ny1 as usize] == opp
        && board[nx2 as usize][ny2 as usize] == opp
        && board[nx3 as usize][ny3 as usize] == player
}

// Evaluate score for one position for one player
fn evaluate_position(board: &Vec<Vec<Stone>>, x: usize, y: usize, player: &Stone) -> i32 {
    let mut score = 0;

    for &(dx, dy) in DIRECTIONS {
        let mut count = 1; // Count the current spot as occupied (hypothetically)

        count += count_stones(board, x, y, dx, dy, player);
        count += count_stones(board, x, y, -dx, -dy, player);

        score += match count {
            n if n >= 5 => 100_000,
            4 => 10_000,
            3 => 1_000,
            2 => 100,
            _ => 0,
        };

        if can_capture(board, x, y, dx, dy, *player) {
            score += 50000;
        }
        if can_capture(board, x, y, -dx, -dy, *player) {
            score += 50000;
        }
    }

    score
}

fn get_critical_moves(
    mut critical_moves: LinkedHashSet<(usize, usize)>,
    state: &Gomoku,
) -> LinkedHashSet<(usize, usize)> {
    for player in [&state.opponent_player, &state.current_player] {
        for (pattern_type, patterns) in [
            ("block_four", &state.block_four),
            ("open_four", &state.open_four),
            ("open_three", &state.open_three),
            ("open_two", &state.open_two),
            ("free_three", &state.free_three),
        ] {
            for pattern in patterns.get(player).unwrap() {
                let points: Vec<(i32, i32)> = if pattern_type == "free_three" {
                    pattern.clone()
                } else {
                    vec![pattern[0], pattern[pattern.len() - 1]]
                };

                for (x, y) in points {
                    critical_moves.insert((x as usize, y as usize));
                }
            }
        }
    }

    return critical_moves;
}

fn get_radius_moves(
    mut radius_moves: LinkedHashSet<(usize, usize)>,
    state: &Gomoku,
    radius: usize,
) -> LinkedHashSet<(usize, usize)> {
    let (rows, cols) = (state.board.len(), state.board[0].len());

    for row in 0..rows {
        for col in 0..cols {
            if state.board[row][col] != Stone::Empty {
                let start_row = row.saturating_sub(radius);
                let end_row = (row + radius + 1).min(rows);
                let start_col = col.saturating_sub(radius);
                let end_col = (col + radius + 1).min(cols);

                for r in start_row..end_row {
                    for c in start_col..end_col {
                        radius_moves.insert((r, c));
                    }
                }
            }
        }
    }

    radius_moves
}

#[pyfunction]
pub fn get_candidate_moves(state: &Gomoku, radius: i32) -> Vec<(usize, usize)> {
    if state.count_empty_spots() as usize == state.size * state.size {
        return vec![(state.size / 2, state.size / 2)];
    }

    let radius = radius as usize;

    // let radius = 2 as usize;
    let move_set = LinkedHashSet::new();
    let move_set = get_critical_moves(move_set, state);
    let move_set = get_radius_moves(move_set, state, radius);

    let mut valid_moves: Vec<_> = move_set
        .into_iter()
        .filter(|&(r, c)| state.is_valid_move(r as i32, c as i32) == MoveResult::Valid)
        .collect();

    valid_moves
        .sort_by_key(|&(r, c)| -(evaluate_position(&state.board, r, c, &state.current_player)));

    // // Create a vector of (move, score)
    // let mut moves_with_scores: Vec<((usize, usize), i32)> = valid_moves
    //     .iter()
    //     .map(|&(r, c)| {
    //         let score = evaluate_position(&state.board, r, c, &state.current_player);
    //         // println!("Move: ({}, {}), Score: {}", r, c, score);
    //         ((r, c), score)
    //     })
    //     .collect();

    // // Sort descending by score
    // moves_with_scores.sort_by_key(|&(_, score)| -score);

    // // Extract just the sorted moves if you want
    // let _sorted_moves: Vec<(usize, usize)> = moves_with_scores
    //     .into_iter()
    //     .map(|(mv, _score)| mv)
    //     .collect();

    valid_moves
}

fn is_terminal_state(state: &Gomoku) -> bool {
    state.check_draw() || state.get_winner().is_some()
}

fn state_value(state: &Gomoku) -> i32 {
    match state.get_winner() {
        None => 0,
        Some(winner) if winner == "X" => MAX_VALUE,
        Some(_) => MIN_VALUE,
    }
}

fn make_next_state(state: &Gomoku, move_x: usize, move_y: usize) -> Gomoku {
    let mut new_state = state.clone_gomoku();
    new_state.handle_move(move_x.try_into().unwrap(), move_y.try_into().unwrap());
    new_state.switch_player();
    new_state
}

#[pyfunction]
pub fn get_ai_move_iterative_deepening(state: &Gomoku) -> (
    Option<(usize, usize, i32)>,
    Vec<(usize, usize, Option<i32>)>,
) {
    let max_depth = if state.move_count < 4 { 3 } else { MAX_DEPTH };
    let mut recommended_move = (0, 0);
    for iterative_depth in 1..(max_depth + 1) {
        let (new_recommended_move, other_moves) = get_ai_move(&state.clone(), iterative_depth, recommended_move);
        if iterative_depth == max_depth {
            return (new_recommended_move, other_moves);
        }
        if let Some(m) = new_recommended_move {
            recommended_move = (m.0, m.1);
        }
    }

    return (None, vec![]);
}

pub fn get_ai_move(
    state: &Gomoku,
    max_depth: usize,
    recommended_move: (usize, usize),
) -> (
    Option<(usize, usize, i32)>,
    Vec<(usize, usize, Option<i32>)>,
) {
    let is_max_player = state.current_player == Stone::Black;

    let mut best_value = if is_max_player {
        MIN_VALUE - 1
    } else {
        MAX_VALUE + 1
    };
    let alpha: Arc<Mutex<i32>> = Arc::new(Mutex::new(MIN_VALUE));
    let beta: Arc<Mutex<i32>> = Arc::new(Mutex::new(MAX_VALUE));
    // let mut alpha = MIN_VALUE;
    // let mut beta = MAX_VALUE;

    let all_moves: Vec<(usize, usize, Option<i32>)> = std::iter::once(recommended_move)
        .chain(get_candidate_moves(state, 3).into_iter())
        .map(|(x, y)| (x, y, None))
        .collect();

    let handles: Vec<_> = all_moves
        .clone()
        .into_iter()
        .map(|(move_x, move_y, _)| {
            let next_state = make_next_state(state, move_x, move_y);
            let is_max_player2 = is_max_player.clone();
            let max_depth2 = max_depth.clone();
            let alpha = alpha.clone();
            let beta = beta.clone();
            thread::spawn(move || {
                let alpha_val = *alpha.lock().unwrap();
                let beta_val = *beta.lock().unwrap();
                let (value, depth) = alphabeta(
                    &next_state,
                    alpha_val,
                    beta_val,
                    // MIN_VALUE,
                    // MAX_VALUE,
                    !is_max_player2,
                    1,
                    max_depth2,
                );

                let depth: i32 = depth.try_into().unwrap();
                let value = if value > depth {
                    value - depth
                } else if value < -depth {
                    value + depth
                } else {
                    value
                };

                let score = Some(value);

                let mut alpha_lock = alpha.lock().unwrap();
                let mut beta_lock = beta.lock().unwrap();
                if is_max_player2 && value > best_value {
                    best_value = value;
                    if *alpha_lock < value {
                        *alpha_lock = value;
                    }
                } else if !is_max_player2 && value < best_value {
                    best_value = value;
                    if *beta_lock > value {
                        *beta_lock = value;
                    }
                }

                if *alpha_lock >= *beta_lock {
                    println!("SHOULD HAVE ENDED!");
                }
                (move_x, move_y, score)
            })
        })
        .collect();

    let scored_moves: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

    return (
        scored_moves
            .iter()
            .filter_map(|(x, y, s)| s.map(|s| (x.clone(), y.clone(), s.clone())))
            .max_by_key(|a| if is_max_player { a.2 } else { -a.2 }),
        scored_moves,
    );
    //
    // for (move_x, move_y, score) in all_moves.iter_mut() {
    //     let next_state = make_next_state(state, *move_x, *move_y);
    //     let (value, depth) = alphabeta(&next_state, alpha, beta, !is_max_player, 1, max_depth);
    //
    //     let depth: i32 = depth.try_into().unwrap();
    //     let value = if value > depth {
    //         value - depth
    //     } else if value < -depth {
    //         value + depth
    //     } else {
    //         value
    //     };
    //
    //     *score = Some(value);
    //
    //     if is_max_player && value > best_value {
    //         best_value = value;
    //         alpha = max(alpha, best_value);
    //         best_move = Some((*move_x, *move_y, value));
    //     } else if !is_max_player && value < best_value {
    //         best_value = value;
    //         beta = min(beta, best_value);
    //         best_move = Some((*move_x, *move_y, value));
    //     }
    //
    //     if alpha >= beta {
    //         break;
    //     }
    // }
    //
    // (best_move, all_moves)
}

fn alphabeta(
    state: &Gomoku,
    mut alpha: i32,
    mut beta: i32,
    is_max_player: bool,
    depth: usize,
    max_depth: usize,
) -> (i32, usize) {
    if is_terminal_state(state) {
        return (state_value(state), depth);
    }

    if depth == max_depth {
        return (heuristic_evaluation(state), depth);
    }

    let mut value = if is_max_player {
        (MIN_VALUE - 1, max_depth)
    } else {
        (MAX_VALUE + 1, max_depth)
    };

    let radius = if depth < 5 {
        3
    } else if depth < 7 {
        2
    } else {
        1
    };
    for (move_x, move_y) in get_candidate_moves(state, radius) {
        let next_state = make_next_state(state, move_x, move_y);

        if is_max_player {
            value = max(
                value,
                alphabeta(&next_state, alpha, beta, false, depth + 1, max_depth),
            );
            alpha = max(alpha, value.0);
        } else {
            value = min(
                value,
                alphabeta(&next_state, alpha, beta, true, depth + 1, max_depth),
            );
            beta = min(beta, value.0);
        }

        if alpha >= beta {
            break;
        }
    }

    value
}
