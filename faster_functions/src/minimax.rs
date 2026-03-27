use crate::search_state::{zobrist, SearchState, BOARD_SIZE};
use crate::Gomoku;

use pyo3::prelude::*;

use std::cmp::{max, min};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;

const MAX_VALUE: i32 = 100_000;
const MIN_VALUE: i32 = -100_000;
const MAX_DEPTH: usize = 8;

const TT_SIZE: usize = 1 << 22; // 4M entries, ~64 MB
const MAX_PLY: usize = 64;

// ---- Search Context (killer moves + history heuristic) ----

struct SearchContext {
    killer_moves: [[Option<(u8, u8)>; 2]; MAX_PLY],
    history: [[[i32; BOARD_SIZE]; BOARD_SIZE]; 2], // [color_idx][row][col]
}

impl SearchContext {
    fn new() -> Self {
        SearchContext {
            killer_moves: [[None; 2]; MAX_PLY],
            history: [[[0i32; BOARD_SIZE]; BOARD_SIZE]; 2],
        }
    }

    fn decay_history(&mut self) {
        for color in 0..2 {
            for r in 0..BOARD_SIZE {
                for c in 0..BOARD_SIZE {
                    self.history[color][r][c] /= 2;
                }
            }
        }
    }

    fn update_killer(&mut self, ply: usize, mov: (u8, u8)) {
        if ply < MAX_PLY && self.killer_moves[ply][0] != Some(mov) {
            self.killer_moves[ply][1] = self.killer_moves[ply][0];
            self.killer_moves[ply][0] = Some(mov);
        }
    }

    fn update_history(&mut self, is_black: bool, mov: (u8, u8), depth: usize) {
        let cidx = if is_black { 0 } else { 1 };
        self.history[cidx][mov.0 as usize][mov.1 as usize] += (depth * depth) as i32;
    }
}

// ---- Transposition Table ----

#[derive(Clone, Copy, PartialEq)]
#[repr(u8)]
enum TTFlag {
    Exact = 0,
    LowerBound = 1,
    UpperBound = 2,
}

#[derive(Clone, Copy)]
struct TTEntry {
    hash: u64,
    score: i32,
    depth: u8,
    flag: TTFlag,
    best_move: Option<(u8, u8)>,
}

impl TTEntry {
    fn pack_data(&self) -> u64 {
        let mut data: u64 = self.score as u32 as u64;
        data |= (self.depth as u64) << 32;
        data |= (self.flag as u64) << 40;
        if let Some((x, y)) = self.best_move {
            data |= (x as u64) << 42;
            data |= (y as u64) << 47;
            data |= 1u64 << 52;
        }
        data
    }

    fn unpack(hash: u64, data: u64) -> Self {
        let score = data as u32 as i32;
        let depth = ((data >> 32) & 0xFF) as u8;
        let flag = match (data >> 40) & 0x03 {
            1 => TTFlag::LowerBound,
            2 => TTFlag::UpperBound,
            _ => TTFlag::Exact,
        };
        let has_best = ((data >> 52) & 1) != 0;
        let best_move = if has_best {
            Some((((data >> 42) & 0x1F) as u8, ((data >> 47) & 0x1F) as u8))
        } else {
            None
        };
        TTEntry { hash, score, depth, flag, best_move }
    }
}

struct TTSlot {
    key: AtomicU64,
    data: AtomicU64,
}

struct TranspositionTable {
    slots: Vec<TTSlot>,
    mask: usize,
}

unsafe impl Sync for TranspositionTable {}
unsafe impl Send for TranspositionTable {}

impl TranspositionTable {
    fn new(size: usize) -> Self {
        let mut slots = Vec::with_capacity(size);
        for _ in 0..size {
            slots.push(TTSlot {
                key: AtomicU64::new(0),
                data: AtomicU64::new(0),
            });
        }
        TranspositionTable { slots, mask: size - 1 }
    }

    fn probe(&self, hash: u64) -> Option<TTEntry> {
        let index = (hash as usize) & self.mask;
        let slot = &self.slots[index];
        let stored_key = slot.key.load(Ordering::Relaxed);
        let stored_data = slot.data.load(Ordering::Relaxed);
        let recovered_hash = stored_key ^ stored_data;
        if recovered_hash == hash {
            Some(TTEntry::unpack(hash, stored_data))
        } else {
            None
        }
    }

    fn store(&self, entry: &TTEntry) {
        let index = (entry.hash as usize) & self.mask;
        let slot = &self.slots[index];
        let data = entry.pack_data();
        slot.key.store(entry.hash ^ data, Ordering::Relaxed);
        slot.data.store(data, Ordering::Relaxed);
    }
}

// ---- Public API (still wraps Gomoku for Python) ----

#[pyfunction]
pub fn get_candidate_moves(state: &Gomoku, radius: i32) -> Vec<(usize, usize)> {
    let ss = SearchState::from_gomoku(state);
    ss.get_candidate_moves(radius as usize)
        .into_iter()
        .map(|(r, c)| (r as usize, c as usize))
        .collect()
}

#[pyfunction]
pub fn get_ai_move_iterative_deepening(
    state: &Gomoku,
) -> (Option<(usize, usize, i32)>, Vec<(usize, usize, Option<i32>)>) {
    let _ = zobrist();

    let ss = SearchState::from_gomoku(state);
    let max_depth = if ss.move_count < 4 { 3 } else { MAX_DEPTH };
    let num_threads = thread::available_parallelism()
        .map(|n| n.get().min(8))
        .unwrap_or(4);

    let tt = Arc::new(TranspositionTable::new(TT_SIZE));

    let mut handles: Vec<thread::JoinHandle<()>> = Vec::new();
    for _ in 1..num_threads {
        let tt_clone = Arc::clone(&tt);
        let ss_clone = ss.clone();
        handles.push(thread::spawn(move || {
            worker_iterative_deepening(ss_clone, max_depth, &tt_clone);
        }));
    }

    let result = main_thread_iterative_deepening(ss, max_depth, &tt);

    for h in handles {
        let _ = h.join();
    }

    result
}

// ---- Iterative Deepening ----

fn worker_iterative_deepening(mut ss: SearchState, max_depth: usize, tt: &TranspositionTable) {
    let is_max_player = ss.is_black_turn;
    let mut previous_ordering: Option<Vec<(u8, u8)>> = None;
    let mut ctx = SearchContext::new();

    for iterative_depth in 1..=max_depth {
        ctx.decay_history();
        let (_, all_moves) =
            get_ai_move_internal(&mut ss, iterative_depth, &previous_ordering, tt, &mut ctx);
        let mut scored = all_moves;
        scored.sort_by(|a, b| {
            let sa = a.2.unwrap_or(if is_max_player { MIN_VALUE - 1 } else { MAX_VALUE + 1 });
            let sb = b.2.unwrap_or(if is_max_player { MIN_VALUE - 1 } else { MAX_VALUE + 1 });
            if is_max_player { sb.cmp(&sa) } else { sa.cmp(&sb) }
        });
        previous_ordering =
            Some(scored.into_iter().map(|(r, c, _)| (r as u8, c as u8)).collect());
    }
}

fn main_thread_iterative_deepening(
    mut ss: SearchState,
    max_depth: usize,
    tt: &TranspositionTable,
) -> (Option<(usize, usize, i32)>, Vec<(usize, usize, Option<i32>)>) {
    let is_max_player = ss.is_black_turn;
    let mut previous_ordering: Option<Vec<(u8, u8)>> = None;
    let mut last_result: (Option<(usize, usize, i32)>, Vec<(usize, usize, Option<i32>)>) =
        (None, vec![]);
    let mut ctx = SearchContext::new();

    for iterative_depth in 1..=max_depth {
        ctx.decay_history();
        let (best_move, all_moves) =
            get_ai_move_internal(&mut ss, iterative_depth, &previous_ordering, tt, &mut ctx);

        let mut scored = all_moves.clone();
        scored.sort_by(|a, b| {
            let sa = a.2.unwrap_or(if is_max_player { MIN_VALUE - 1 } else { MAX_VALUE + 1 });
            let sb = b.2.unwrap_or(if is_max_player { MIN_VALUE - 1 } else { MAX_VALUE + 1 });
            if is_max_player { sb.cmp(&sa) } else { sa.cmp(&sb) }
        });
        previous_ordering =
            Some(scored.into_iter().map(|(r, c, _)| (r as u8, c as u8)).collect());

        last_result = (best_move, all_moves);
    }

    last_result
}

// ---- Root search ----

fn get_ai_move_internal(
    ss: &mut SearchState,
    max_depth: usize,
    previous_ordering: &Option<Vec<(u8, u8)>>,
    tt: &TranspositionTable,
    ctx: &mut SearchContext,
) -> (Option<(usize, usize, i32)>, Vec<(usize, usize, Option<i32>)>) {
    let is_max_player = ss.is_black_turn;

    let mut best_value = if is_max_player { MIN_VALUE - 1 } else { MAX_VALUE + 1 };
    let mut alpha = MIN_VALUE;
    let mut beta = MAX_VALUE;
    let mut best_move: Option<(usize, usize, i32)> = None;

    let candidate_moves = ss.get_candidate_moves(3);
    let ordered_moves: Vec<(u8, u8)> = match previous_ordering {
        Some(prev) => {
            let prev_set: std::collections::HashSet<(u8, u8)> = prev.iter().cloned().collect();
            let mut moves = prev.clone();
            for m in candidate_moves {
                if !prev_set.contains(&m) {
                    moves.push(m);
                }
            }
            moves
        }
        None => candidate_moves,
    };

    let mut ordered_moves = ordered_moves;
    if ordered_moves.len() > 30 {
        ordered_moves.truncate(30);
    }

    let mut all_moves: Vec<(usize, usize, Option<i32>)> = ordered_moves
        .iter()
        .map(|&(r, c)| (r as usize, c as usize, None))
        .collect();

    for entry in all_moves.iter_mut() {
        let (move_r, move_c) = (entry.0 as u8, entry.1 as u8);

        let undo = ss.make_move(move_r, move_c);
        let (value, depth) =
            alphabeta(ss, alpha, beta, !is_max_player, 1, max_depth, tt, ctx);
        ss.unmake_move(undo);

        let depth_i32: i32 = depth as i32;
        let value = if value > depth_i32 {
            value - depth_i32
        } else if value < -depth_i32 {
            value + depth_i32
        } else {
            value
        };

        entry.2 = Some(value);

        if is_max_player && value > best_value {
            best_value = value;
            alpha = max(alpha, best_value);
            best_move = Some((entry.0, entry.1, value));
        } else if !is_max_player && value < best_value {
            best_value = value;
            beta = min(beta, best_value);
            best_move = Some((entry.0, entry.1, value));
        }

        if alpha >= beta {
            break;
        }
    }

    (best_move, all_moves)
}

// ---- Alpha-Beta with make/unmake ----

fn alphabeta(
    state: &mut SearchState,
    mut alpha: i32,
    mut beta: i32,
    is_max_player: bool,
    depth: usize,
    max_depth: usize,
    tt: &TranspositionTable,
    ctx: &mut SearchContext,
) -> (i32, usize) {
    // Terminal check
    if let Some(val) = state.terminal_value() {
        return (val, depth);
    }

    if depth == max_depth {
        return (state.heuristic_eval(), depth);
    }

    // TT probe
    let hash = state.zobrist_hash;
    let remaining_depth = (max_depth - depth) as u8;
    let mut tt_best_move: Option<(u8, u8)> = None;

    if let Some(entry) = tt.probe(hash) {
        tt_best_move = entry.best_move;
        if entry.depth >= remaining_depth {
            match entry.flag {
                TTFlag::Exact => return (entry.score, depth),
                TTFlag::LowerBound => {
                    if entry.score >= beta {
                        return (entry.score, depth);
                    }
                    alpha = max(alpha, entry.score);
                }
                TTFlag::UpperBound => {
                    if entry.score <= alpha {
                        return (entry.score, depth);
                    }
                    beta = min(beta, entry.score);
                }
            }
            if alpha >= beta {
                return (entry.score, depth);
            }
        }
    }

    let remaining = max_depth - depth;

    let original_alpha = alpha;
    let original_beta = beta;

    // Move generation (minimum radius of 2 to avoid missing blocking moves)
    let radius = if depth < 3 { 3 } else { 2 };
    let scored_moves = state.get_candidate_moves_scored(radius);

    // Split into moves and scores (already sorted by score descending)
    let mut moves: Vec<(u8, u8)> = scored_moves.iter().map(|&(r, c, _)| (r, c)).collect();
    let mut move_scores: Vec<i32> = scored_moves.iter().map(|&(_, _, s)| s).collect();
    let max_score = move_scores.first().copied().unwrap_or(0);
    let is_tactical = max_score >= 800_000;

    // Null move pruning: skip if position is tactical (top move is a threat/block)
    if remaining >= 3 && !is_tactical && state.captures(!state.is_black_turn) < 3 {
        let z = zobrist();
        state.is_black_turn = !state.is_black_turn;
        state.zobrist_hash ^= z.side_to_move;

        let null_reduction = 2;
        let (null_val, _) = alphabeta(
            state,
            alpha,
            beta,
            !is_max_player,
            depth + 1 + null_reduction,
            max_depth,
            tt,
            ctx,
        );

        state.is_black_turn = !state.is_black_turn;
        state.zobrist_hash ^= z.side_to_move;

        if is_max_player && null_val >= beta {
            return (beta, depth);
        }
        if !is_max_player && null_val <= alpha {
            return (alpha, depth);
        }
    }

    // TT best-move ordering: use remove+insert to preserve sort order
    if let Some((br, bc)) = tt_best_move {
        if let Some(pos) = moves.iter().position(|&(r, c)| r == br && c == bc) {
            let m = moves.remove(pos);
            let s = move_scores.remove(pos);
            moves.insert(0, m);
            move_scores.insert(0, s);
        }
    }

    // Killer move ordering: insert killers at positions 1-2 (after TT move)
    if depth < MAX_PLY {
        let mut insert_pos = 1usize.min(moves.len());
        for k in 0..2 {
            if let Some(km) = ctx.killer_moves[depth][k] {
                if let Some(pos) = moves[insert_pos..].iter().position(|&(r, c)| r == km.0 && c == km.1) {
                    let m = moves.remove(insert_pos + pos);
                    let s = move_scores.remove(insert_pos + pos);
                    moves.insert(insert_pos, m);
                    move_scores.insert(insert_pos, s);
                    insert_pos += 1;
                }
            }
        }
    }

    // Move count caps (based on remaining depth)
    let max_moves = if is_tactical {
        match remaining {
            0..=1 => 7,
            2..=3 => 9,
            4..=5 => 11,
            _ => 15,
        }
    } else {
        match remaining {
            0..=1 => 5,
            2..=3 => 7,
            4..=5 => 9,
            _ => 12,
        }
    };
    moves.truncate(max_moves);
    move_scores.truncate(max_moves);

    let mut best_score = if is_max_player { MIN_VALUE - 1 } else { MAX_VALUE + 1 };
    let mut best_depth = max_depth;
    let mut best_move_found: Option<(u8, u8)> = None;

    for (move_idx, (move_r, move_c)) in moves.into_iter().enumerate() {
        let undo = state.make_move(move_r, move_c);

        // LMR: reduce depth for late moves at sufficient depth
        // Skip LMR for high-priority moves (threats/blocks scoring >= 800K)
        let move_score = move_scores[move_idx];
        let (val, d) = if move_idx >= 2 && remaining >= 3 && move_score < 800_000 {
            let r = ((remaining as f64).sqrt() * (move_idx as f64).sqrt() / 2.0) as usize;
            let reduction = r.max(1).min(remaining - 1);
            let reduced_max = max_depth - reduction;

            let (v, d) = alphabeta(
                state, alpha, beta, !is_max_player, depth + 1, reduced_max, tt, ctx,
            );

            // Re-search at full depth if reduced search improved alpha/beta
            if (is_max_player && v > alpha) || (!is_max_player && v < beta) {
                alphabeta(state, alpha, beta, !is_max_player, depth + 1, max_depth, tt, ctx)
            } else {
                (v, d)
            }
        } else {
            alphabeta(state, alpha, beta, !is_max_player, depth + 1, max_depth, tt, ctx)
        };

        state.unmake_move(undo);

        if is_max_player {
            if val > best_score || (val == best_score && d < best_depth) {
                best_score = val;
                best_depth = d;
                best_move_found = Some((move_r, move_c));
            }
            alpha = max(alpha, best_score);
        } else {
            if val < best_score || (val == best_score && d < best_depth) {
                best_score = val;
                best_depth = d;
                best_move_found = Some((move_r, move_c));
            }
            beta = min(beta, best_score);
        }

        if alpha >= beta {
            // Update killer moves and history on beta cutoff
            ctx.update_killer(depth, (move_r, move_c));
            ctx.update_history(state.is_black_turn, (move_r, move_c), remaining);
            break;
        }
    }

    // TT store
    let flag = if best_score <= original_alpha {
        TTFlag::UpperBound
    } else if best_score >= original_beta {
        TTFlag::LowerBound
    } else {
        TTFlag::Exact
    };

    tt.store(&TTEntry {
        hash,
        score: best_score,
        depth: remaining_depth,
        flag,
        best_move: best_move_found,
    });

    (best_score, best_depth)
}

