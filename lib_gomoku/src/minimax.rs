use crate::heuristic::heuristic_evaluation;
use crate::MoveResult;
use crate::{Gomoku, Stone};

use linked_hash_set::LinkedHashSet;
use pyo3::prelude::*;

use std::cmp::{max, min};
use std::time::Instant;

// --- Transposition Table ---

#[derive(Clone, Copy, PartialEq)]
enum TTFlag {
    Exact,
    LowerBound,
    UpperBound,
}

#[derive(Clone, Copy)]
struct TTEntry {
    hash: u64,
    depth_remaining: usize,
    value: i32,
    flag: TTFlag,
    best_move: Option<(usize, usize)>,
}

struct TranspositionTable {
    entries: Vec<Option<TTEntry>>,
    mask: usize,
}

impl TranspositionTable {
    fn new(size_bits: usize) -> Self {
        let size = 1 << size_bits;
        TranspositionTable {
            entries: vec![None; size],
            mask: size - 1,
        }
    }

    fn lookup(&self, hash: u64) -> Option<&TTEntry> {
        let idx = hash as usize & self.mask;
        self.entries[idx].as_ref().filter(|e| e.hash == hash)
    }

    fn store(&mut self, hash: u64, entry: TTEntry) {
        let idx = hash as usize & self.mask;
        self.entries[idx] = Some(entry);
    }
}

pub struct SearchStats {
    pub nodes_visited: u64,
    pub cutoffs: u64,
    pub total_children: u64,
    pub children_explored: u64,
    pub internal_nodes: u64,
    pub branch_times: Vec<(usize, usize, Option<i32>, f64)>,
    pub max_depth: usize,
    /// Per-depth: (depth, elapsed_secs, nodes_visited)
    pub depth_times: Vec<(usize, f64, u64)>,
    pub tt_hits: u64,
}

impl SearchStats {
    pub fn new() -> Self {
        SearchStats {
            nodes_visited: 0,
            cutoffs: 0,
            total_children: 0,
            children_explored: 0,
            internal_nodes: 0,
            branch_times: Vec::new(),
            max_depth: 0,
            depth_times: Vec::new(),
            tt_hits: 0,
        }
    }

    pub fn avg_branching_factor(&self) -> f64 {
        if self.internal_nodes == 0 {
            return 0.0;
        }
        self.total_children as f64 / self.internal_nodes as f64
    }

    pub fn estimated_full_tree(&self) -> f64 {
        if self.internal_nodes == 0 {
            return 1.0;
        }
        let b = self.avg_branching_factor();
        if b <= 1.0 {
            return (self.max_depth + 1) as f64;
        }
        (b.powf(self.max_depth as f64 + 1.0) - 1.0) / (b - 1.0)
    }

    pub fn pruning_percent(&self) -> f64 {
        let full = self.estimated_full_tree();
        if full <= 1.0 {
            return 0.0;
        }
        (1.0 - self.nodes_visited as f64 / full).max(0.0) * 100.0
    }
}

const MAX_VALUE: i32 = 100_000;
const MIN_VALUE: i32 = -100_000;
const MAX_DEPTH: usize = 5;

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

    let _radius = radius as usize;

    let radius = 1 as usize;
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
pub fn get_ai_move(
    state: &Gomoku,
) -> (
    Option<(usize, usize, i32)>,
    Vec<(usize, usize, Option<i32>)>,
) {
    println!("state: {:#?}", state);
    let (best, moves, _) = get_ai_move_with_stats(state);
    (best, moves)
}

pub fn get_ai_move_with_stats(
    state: &Gomoku,
) -> (
    Option<(usize, usize, i32)>,
    Vec<(usize, usize, Option<i32>)>,
    SearchStats,
) {
    let is_max_player = state.current_player == Stone::Black;

    let mut all_moves: Vec<(usize, usize, Option<i32>)> = get_candidate_moves(state, 3)
        .into_iter()
        .map(|(x, y)| (x, y, None))
        .collect();

    let max_depth = if state.move_count < 4 { 3 } else { MAX_DEPTH };

    let mut best_move: Option<(usize, usize, i32)> = None;
    let mut final_stats = SearchStats::new();
    let mut depth_times: Vec<(usize, f64, u64)> = Vec::new();
    let mut tt = TranspositionTable::new(20); // 1M entries

    // Iterative deepening: search at increasing depths
    for depth in 1..=max_depth {
        let depth_start = Instant::now();
        let mut stats = SearchStats::new();
        stats.max_depth = depth;
        // Count root as an internal node
        stats.internal_nodes += 1;
        stats.total_children += all_moves.len() as u64;

        let mut alpha = MIN_VALUE;
        let mut beta = MAX_VALUE;
        let mut best_value = if is_max_player {
            MIN_VALUE - 1
        } else {
            MAX_VALUE + 1
        };
        let mut iteration_best_move: Option<(usize, usize, i32)> = None;
        let mut branch_times = Vec::new();

        let mut first = true;
        for (move_x, move_y, score) in all_moves.iter_mut() {
            stats.children_explored += 1;
            let branch_start = Instant::now();
            let next_state = make_next_state(state, *move_x, *move_y);

            let (mut raw_value, mut d);
            if first {
                // Full window search on PV move
                (raw_value, d) = alphabeta(&next_state, alpha, beta, !is_max_player, 1, depth, &mut stats, &mut tt);
                first = false;
            } else {
                // Null window scout search
                if is_max_player {
                    (raw_value, d) = alphabeta(&next_state, alpha, alpha + 1, false, 1, depth, &mut stats, &mut tt);
                    if raw_value > alpha && raw_value < beta {
                        (raw_value, d) = alphabeta(&next_state, alpha, beta, false, 1, depth, &mut stats, &mut tt);
                    }
                } else {
                    (raw_value, d) = alphabeta(&next_state, beta - 1, beta, true, 1, depth, &mut stats, &mut tt);
                    if raw_value < beta && raw_value > alpha {
                        (raw_value, d) = alphabeta(&next_state, alpha, beta, true, 1, depth, &mut stats, &mut tt);
                    }
                }
            }

            let branch_elapsed = branch_start.elapsed().as_secs_f64();

            let d: i32 = d.try_into().unwrap();
            let value = if raw_value > d {
                raw_value - d
            } else if raw_value < -d {
                raw_value + d
            } else {
                raw_value
            };

            *score = Some(value);
            branch_times.push((*move_x, *move_y, Some(value), branch_elapsed));

            if is_max_player && value > best_value {
                best_value = value;
                alpha = max(alpha, best_value);
                iteration_best_move = Some((*move_x, *move_y, value));
            } else if !is_max_player && value < best_value {
                best_value = value;
                beta = min(beta, best_value);
                iteration_best_move = Some((*move_x, *move_y, value));
            }

            if alpha >= beta {
                stats.cutoffs += 1;
                break;
            }
        }

        stats.branch_times = branch_times;
        depth_times.push((depth, depth_start.elapsed().as_secs_f64(), stats.nodes_visited));
        best_move = iteration_best_move;
        final_stats = stats;

        // Reorder moves for next iteration: best-scored moves first
        // This improves pruning at the next deeper search
        if is_max_player {
            all_moves.sort_by(|a, b| {
                let sa = a.2.unwrap_or(MIN_VALUE);
                let sb = b.2.unwrap_or(MIN_VALUE);
                sb.cmp(&sa) // descending for maximizer
            });
        } else {
            all_moves.sort_by(|a, b| {
                let sa = a.2.unwrap_or(MAX_VALUE);
                let sb = b.2.unwrap_or(MAX_VALUE);
                sa.cmp(&sb) // ascending for minimizer
            });
        }
    }

    final_stats.depth_times = depth_times;

    (best_move, all_moves, final_stats)
}

fn alphabeta(
    state: &Gomoku,
    mut alpha: i32,
    mut beta: i32,
    is_max_player: bool,
    depth: usize,
    max_depth: usize,
    stats: &mut SearchStats,
    tt: &mut TranspositionTable,
) -> (i32, usize) {
    stats.nodes_visited += 1;

    if is_terminal_state(state) {
        return (state_value(state), depth);
    }

    if depth == max_depth {
        return (heuristic_evaluation(state), depth);
    }

    let depth_remaining = max_depth - depth;
    let orig_alpha = alpha;

    // TT lookup
    let mut tt_best_move: Option<(usize, usize)> = None;
    if let Some(entry) = tt.lookup(state.hash) {
        tt_best_move = entry.best_move;
        if entry.depth_remaining >= depth_remaining {
            stats.tt_hits += 1;
            match entry.flag {
                TTFlag::Exact => return (entry.value, depth),
                TTFlag::LowerBound => alpha = max(alpha, entry.value),
                TTFlag::UpperBound => beta = min(beta, entry.value),
            }
            if alpha >= beta {
                return (entry.value, depth);
            }
        }
    }

    let mut candidates = get_candidate_moves(state, 3);
    stats.internal_nodes += 1;
    stats.total_children += candidates.len() as u64;

    // Move ordering: put TT best move first
    if let Some(tt_move) = tt_best_move {
        if let Some(pos) = candidates.iter().position(|&m| m == tt_move) {
            candidates.swap(0, pos);
        }
    }

    let mut value = if is_max_player {
        (MIN_VALUE - 1, max_depth)
    } else {
        (MAX_VALUE + 1, max_depth)
    };

    let mut best_move_here: Option<(usize, usize)> = None;
    let mut first = true;
    for (move_x, move_y) in &candidates {
        stats.children_explored += 1;
        let next_state = make_next_state(state, *move_x, *move_y);

        let mut child;
        if first {
            // Full window search on PV move
            child = alphabeta(&next_state, alpha, beta, !is_max_player, depth + 1, max_depth, stats, tt);
            first = false;
        } else {
            // Null window scout search
            if is_max_player {
                child = alphabeta(&next_state, alpha, alpha + 1, false, depth + 1, max_depth, stats, tt);
                if child.0 > alpha && child.0 < beta {
                    child = alphabeta(&next_state, alpha, beta, false, depth + 1, max_depth, stats, tt);
                }
            } else {
                child = alphabeta(&next_state, beta - 1, beta, true, depth + 1, max_depth, stats, tt);
                if child.0 < beta && child.0 > alpha {
                    child = alphabeta(&next_state, alpha, beta, true, depth + 1, max_depth, stats, tt);
                }
            }
        }

        if is_max_player {
            if child >= value {
                value = child;
                best_move_here = Some((*move_x, *move_y));
            }
            alpha = max(alpha, value.0);
        } else {
            if child <= value {
                value = child;
                best_move_here = Some((*move_x, *move_y));
            }
            beta = min(beta, value.0);
        }

        if alpha >= beta {
            stats.cutoffs += 1;
            break;
        }
    }

    // TT store
    let flag = if value.0 <= orig_alpha {
        TTFlag::UpperBound
    } else if value.0 >= beta {
        TTFlag::LowerBound
    } else {
        TTFlag::Exact
    };
    tt.store(state.hash, TTEntry {
        hash: state.hash,
        depth_remaining,
        value: value.0,
        flag,
        best_move: best_move_here,
    });

    value
}
