use crate::constants::{
    LMR_DEEP_MOVE, LMR_MIN_DEPTH, LMR_MIN_MOVE, MAX_DEPTH, MAX_VALUE, MIN_VALUE,
    CANDIDATE_CAP, CANDIDATE_CAP_DEPTH, NULL_MOVE_MIN_DEPTH, NULL_MOVE_REDUCTION,
    RANDOMIZE_TIED_MOVES, TIME_LIMIT_MS, TT_SHARDS, TT_SHARD_MASK, TT_SIZE_BITS,
};
use crate::search_board::{Cell, SearchBoard};
use crate::Gomoku;

use pyo3::prelude::*;
use rayon::prelude::*;

use std::cmp::{max, min};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};


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

// --- Sharded Transposition Table for parallel access ---

pub struct ShardedTT {
    shards: Vec<Mutex<TranspositionTable>>,
}

impl ShardedTT {
    pub fn new(size_bits_per_shard: usize) -> Self {
        let mut shards = Vec::with_capacity(TT_SHARDS);
        for _ in 0..TT_SHARDS {
            shards.push(Mutex::new(TranspositionTable::new(size_bits_per_shard)));
        }
        ShardedTT { shards }
    }

    fn lookup(&self, hash: u64) -> Option<TTEntry> {
        let shard = (hash & TT_SHARD_MASK) as usize;
        self.shards[shard].lock().unwrap().lookup(hash).copied()
    }

    fn store(&self, hash: u64, entry: TTEntry) {
        let shard = (hash & TT_SHARD_MASK) as usize;
        self.shards[shard].lock().unwrap().store(hash, entry);
    }

    /// Drop every entry. Entries are keyed by Zobrist hash so stale ones are
    /// not wrong, but they do make one search's cost depend on what ran before
    /// it — clear between unrelated positions to get comparable timings.
    pub fn clear(&self) {
        for shard in &self.shards {
            let mut t = shard.lock().unwrap();
            for slot in t.entries.iter_mut() {
                *slot = None;
            }
        }
    }
}

// Kept alive for the process lifetime so entries survive across turns —
// most sub-positions from one turn's search recur in the next turn's tree.
static GLOBAL_TT: OnceLock<ShardedTT> = OnceLock::new();

/// Empty the shared transposition table. Useful between games, and required
/// for any benchmark that wants one position's cost not to depend on the last.
#[pyfunction]
pub fn clear_transposition_table() {
    shared_tt().clear();
}

fn shared_tt() -> &'static ShardedTT {
    GLOBAL_TT.get_or_init(|| ShardedTT::new(TT_SIZE_BITS))
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
    pub pv: Vec<(usize, usize)>,
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
            pv: Vec::new(),
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

    /// Merge counters from a child worker into this stats. Per-thread fields like
    /// max_depth, branch_times, depth_times and pv stay with the master.
    pub fn merge(&mut self, other: &SearchStats) {
        self.nodes_visited += other.nodes_visited;
        self.cutoffs += other.cutoffs;
        self.total_children += other.total_children;
        self.children_explored += other.children_explored;
        self.internal_nodes += other.internal_nodes;
        self.tt_hits += other.tt_hits;
    }
}

fn search_deadline() -> Instant {
    match TIME_LIMIT_MS {
        Some(ms) => Instant::now() + Duration::from_millis(ms),
        None => Instant::now() + Duration::from_secs(365 * 24 * 60 * 60),
    }
}

/// Small dependency-free xorshift64 PRNG, seeded from OS randomness via
/// `RandomState` (the stdlib already draws that entropy for HashMap keys, so
/// this needs no `rand` crate). Only used to break exact-score ties at the
/// root — never touches search internals.
struct Rng(u64);
impl Rng {
    fn seeded() -> Self {
        use std::collections::hash_map::RandomState;
        use std::hash::{BuildHasher, Hasher};
        Rng(RandomState::new().build_hasher().finish() | 1)
    }
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn next_usize(&mut self, bound: usize) -> usize {
        (self.next_u64() % bound as u64) as usize
    }
}

// =====================================================================
// SearchBoard-based search functions
// =====================================================================

/// Terminal value with mate distance encoded in the score itself.
/// Black win = MAX_VALUE - depth (shorter win = higher score).
/// White win = MIN_VALUE + depth (shorter loss = lower/more negative score).
fn sb_state_value(board: &SearchBoard, depth: usize) -> i32 {
    match board.get_winner() {
        Some(Cell::Black) => MAX_VALUE - depth as i32,
        Some(Cell::White) => MIN_VALUE + depth as i32,
        _ => 0, // draw
    }
}

// Strict sequential PVS inside the search. Parallelism is applied only at the
// root in `get_ai_move_with_stats`. Recursive YBWC was tried but caused a
// 100×+ blowup in nodes_visited because losing α-tightening between siblings
// compounds at every level of recursion.
/// Killer-move and history tables: memory of which moves have been causing
/// beta cutoffs, used only to decide what order to try moves in.
///
/// Killers are per-ply — a refutation that works against one move usually
/// works against its siblings. History is board-wide and depth-weighted, so a
/// cutoff found deep in the tree counts for more than a shallow one.
///
/// Purely an ordering hint: it never changes which moves are legal, nor what
/// any node evaluates to — only how fast alpha-beta finds the cutoff.
/// One instance per search task, so no locking and no cross-thread sharing.
pub struct MoveHeuristics {
    killers: [[Option<(usize, usize)>; 2]; MAX_PLY],
    history: [u32; 361],
}

/// Deepest ply the killer table can index. `depth` never exceeds `max_depth`,
/// which is capped at `MAX_DEPTH`; the slack keeps indexing safe regardless.
const MAX_PLY: usize = MAX_DEPTH + 4;

impl MoveHeuristics {
    pub fn new() -> Self {
        MoveHeuristics { killers: [[None; 2]; MAX_PLY], history: [0; 361] }
    }

    #[inline]
    fn killers_at(&self, ply: usize) -> [Option<(usize, usize)>; 2] {
        if ply < MAX_PLY { self.killers[ply] } else { [None, None] }
    }

    /// Remember that `mv` caused a cutoff at `ply` with `depth_remaining` plies
    /// left to search.
    #[inline]
    fn record_cutoff(&mut self, ply: usize, mv: (usize, usize), depth_remaining: usize) {
        if ply < MAX_PLY && self.killers[ply][0] != Some(mv) {
            self.killers[ply][1] = self.killers[ply][0];
            self.killers[ply][0] = Some(mv);
        }
        // Depth-squared: a cutoff that pruned a deep subtree is worth more.
        let bonus = (depth_remaining * depth_remaining) as u32;
        let slot = &mut self.history[mv.0 * 19 + mv.1];
        *slot = slot.saturating_add(bonus);
        // Keep the table from saturating and going flat over a long search.
        if *slot > HISTORY_CEILING {
            for h in self.history.iter_mut() {
                *h /= 2;
            }
        }
    }
}

const HISTORY_CEILING: u32 = 1 << 20;

fn sb_alphabeta(
    board: &mut SearchBoard,
    mut alpha: i32,
    mut beta: i32,
    is_max_player: bool,
    depth: usize,
    max_depth: usize,
    stats: &mut SearchStats,
    tt: &ShardedTT,
    deadline: Instant,
    heur: &mut MoveHeuristics,
    allow_null: bool,
) -> (i32, Vec<(usize, usize)>, bool) {
    if Instant::now() >= deadline {
        return (0, vec![], true);
    }

    stats.nodes_visited += 1;

    if board.is_terminal() {
        return (sb_state_value(board, depth), vec![], false);
    }

    if depth == max_depth {
        return (board.sb_heuristic_evaluation(), vec![], false);
    }

    let depth_remaining = max_depth - depth;
    let orig_alpha = alpha;

    // TT lookup
    let mut tt_best_move: Option<(usize, usize)> = None;
    if let Some(entry) = tt.lookup(board.hash) {
        tt_best_move = entry.best_move;
        if entry.depth_remaining >= depth_remaining {
            stats.tt_hits += 1;
            match entry.flag {
                TTFlag::Exact => return (entry.value, vec![], false),
                TTFlag::LowerBound => alpha = max(alpha, entry.value),
                TTFlag::UpperBound => beta = min(beta, entry.value),
            }
            if alpha >= beta {
                return (entry.value, vec![], false);
            }
        }
    }

    // Facing a four, every move is a forced response, so nothing here is
    // "late" enough to reduce.
    let threats = board.patterns_of(board.opponent);
    let under_threat =
        threats.five_rows > 0 || threats.open_fours > 0 || threats.block_fours > 0;

    // The null-move probe needs a stricter condition than that. `get_winner`
    // reads the position as "whoever is `opponent` just moved", and a pass
    // breaks that assumption, so terminal scores inside the probe are only
    // trustworthy where no terminal state is in reach. Requiring both sides to
    // be four-free and five-free buys that. Skipping this is what let the
    // engine walk past a split four: the probe mis-scored the position where
    // the opponent completed it.
    let own = board.patterns_of(board.current);
    let tactical_position = under_threat
        || own.five_rows > 0
        || own.open_fours > 0
        || own.block_fours > 0;

    // Null-move probe: hand the turn over and see whether the position still
    // beats beta. Passing is never an advantage in gomoku, so a position that
    // survives a free enemy move would survive a real reply too.
    if allow_null && !tactical_position && depth_remaining >= NULL_MOVE_MIN_DEPTH {
        let probe_depth = max_depth - NULL_MOVE_REDUCTION;
        board.make_null_move();
        let (null_val, _, null_timed_out) = if is_max_player {
            sb_alphabeta(board, beta - 1, beta, false, depth + 1, probe_depth,
                         stats, tt, deadline, heur, false)
        } else {
            sb_alphabeta(board, alpha, alpha + 1, true, depth + 1, probe_depth,
                         stats, tt, deadline, heur, false)
        };
        board.make_null_move();

        if null_timed_out {
            return (0, vec![], true);
        }
        if is_max_player && null_val >= beta {
            return (null_val, vec![], false);
        }
        if !is_max_player && null_val <= alpha {
            return (null_val, vec![], false);
        }
    }

    let mut candidates = board.get_candidate_moves(depth, Some(&heur.history), false);
    stats.internal_nodes += 1;
    stats.total_children += candidates.len() as u64;

    // Try the moves most likely to cause a cutoff first: the transposition
    // table's best move, then this ply's killers. `get_candidate_moves` has
    // already ordered the rest by history and static score.
    let mut front = 0;
    if let Some(tt_move) = tt_best_move {
        if let Some(pos) = candidates.iter().position(|&m| m == tt_move) {
            candidates.swap(front, pos);
            front += 1;
        }
    }
    for killer in heur.killers_at(depth).into_iter().flatten() {
        if let Some(pos) = candidates.iter().skip(front).position(|&m| m == killer) {
            candidates.swap(front, front + pos);
            front += 1;
        }
    }

    // Drop the tail of the ordering. Safe to do here and not during generation:
    // the transposition-table move and this ply's killers are already at the
    // front, so the cap can only fall on moves the ordering rates lowest.
    if depth >= CANDIDATE_CAP_DEPTH {
        candidates.truncate(front.max(CANDIDATE_CAP));
    }

    let mut best_value = if is_max_player { MIN_VALUE - 1 } else { MAX_VALUE + 1 };
    let mut best_move_here: Option<(usize, usize)> = None;
    let mut best_pv: Vec<(usize, usize)> = vec![];
    let mut first = true;

    let mut searched_any = false;
    for (move_index, &(move_r, move_c)) in candidates.iter().enumerate() {
        // Legality is checked here rather than during generation, so it is
        // only paid for the moves actually searched.
        if board.sb_is_double_three(move_r as i32, move_c as i32) {
            continue;
        }
        stats.children_explored += 1;
        searched_any = true;

        let mover = board.current;
        let undo = board.make_move(move_r, move_c);

        // A move is tactical if it made a four or five, or captured. Those
        // decide games, so they always get searched at full depth.
        let gain = if mover == Cell::Black { &undo.delta_black } else { &undo.delta_white };
        let is_tactical = undo.num_captured > 0
            || gain.five_rows > 0
            || gain.open_fours > 0
            || gain.block_fours > 0;

        // Late move reduction: this far down the ordering a move rarely raises
        // alpha, so probe it shallower. Anything that beats alpha anyway is
        // re-searched at full depth below, so a wrong guess costs time, not
        // correctness.
        let reduction = if depth_remaining >= LMR_MIN_DEPTH
            && move_index >= LMR_MIN_MOVE
            && !is_tactical
            && !under_threat
        {
            if move_index >= LMR_DEEP_MOVE { 2 } else { 1 }
        } else {
            0
        };
        let reduced_depth = max_depth - reduction.min(depth_remaining - 1);

        let (mut child_val, mut child_pv, mut child_timed_out);
        if first {
            (child_val, child_pv, child_timed_out) = sb_alphabeta(
                board, alpha, beta, !is_max_player, depth + 1, max_depth, stats, tt, deadline, heur, true,
            );
            first = false;
        } else if is_max_player {
            (child_val, child_pv, child_timed_out) = sb_alphabeta(
                board, alpha, alpha + 1, false, depth + 1, reduced_depth, stats, tt, deadline, heur, true,
            );
            // Re-search at full depth if the shallow probe looked promising.
            if !child_timed_out && reduction > 0 && child_val > alpha {
                (child_val, child_pv, child_timed_out) = sb_alphabeta(
                    board, alpha, alpha + 1, false, depth + 1, max_depth, stats, tt, deadline, heur, true,
                );
            }
            if !child_timed_out && child_val > alpha && child_val < beta {
                (child_val, child_pv, child_timed_out) = sb_alphabeta(
                    board, alpha, beta, false, depth + 1, max_depth, stats, tt, deadline, heur, true,
                );
            }
        } else {
            (child_val, child_pv, child_timed_out) = sb_alphabeta(
                board, beta - 1, beta, true, depth + 1, reduced_depth, stats, tt, deadline, heur, true,
            );
            if !child_timed_out && reduction > 0 && child_val < beta {
                (child_val, child_pv, child_timed_out) = sb_alphabeta(
                    board, beta - 1, beta, true, depth + 1, max_depth, stats, tt, deadline, heur, true,
                );
            }
            if !child_timed_out && child_val < beta && child_val > alpha {
                (child_val, child_pv, child_timed_out) = sb_alphabeta(
                    board, alpha, beta, true, depth + 1, max_depth, stats, tt, deadline, heur, true,
                );
            }
        }

        board.undo_move(&undo);

        if child_timed_out {
            return (best_value, best_pv, true);
        }

        if is_max_player {
            if child_val > best_value {
                best_value = child_val;
                best_move_here = Some((move_r, move_c));
                best_pv = std::iter::once((move_r, move_c))
                    .chain(child_pv.into_iter())
                    .collect();
            }
            alpha = max(alpha, best_value);
        } else {
            if child_val < best_value {
                best_value = child_val;
                best_move_here = Some((move_r, move_c));
                best_pv = std::iter::once((move_r, move_c))
                    .chain(child_pv.into_iter())
                    .collect();
            }
            beta = min(beta, best_value);
        }

        if alpha >= beta {
            stats.cutoffs += 1;
            heur.record_cutoff(depth, (move_r, move_c), depth_remaining);
            break;
        }
    }

    // Every candidate turned out to be an illegal double three: there is no
    // move to make, so the position stands as it is.
    if !searched_any {
        return (board.sb_heuristic_evaluation(), vec![], false);
    }

    // TT store
    let flag = if best_value <= orig_alpha {
        TTFlag::UpperBound
    } else if best_value >= beta {
        TTFlag::LowerBound
    } else {
        TTFlag::Exact
    };
    tt.store(board.hash, TTEntry {
        hash: board.hash,
        depth_remaining,
        value: best_value,
        flag,
        best_move: best_move_here,
    });

    (best_value, best_pv, false)
}

// =====================================================================
// Public API functions (Python bindings + Rust self-play)
// =====================================================================

#[pyfunction]
pub fn get_ai_move(
    _py: Python,
    state: &Gomoku,
) -> (
    Option<(usize, usize, i32)>,
    Vec<(usize, usize, Option<i32>)>,
) {
    let (best, moves, _) = get_ai_move_with_stats(state);
    (best, moves)
}

/// Same search as `get_ai_move`, plus the wall-clock time, node count and
/// pruning percentage of the last completed iterative-deepening depth —
/// for surfacing search performance in the UI.
#[pyfunction]
pub fn get_ai_move_stats(
    _py: Python,
    state: &Gomoku,
) -> (
    Option<(usize, usize, i32)>,
    Vec<(usize, usize, Option<i32>)>,
    f64,
    u64,
    f64,
) {
    let start = Instant::now();
    let (best, moves, stats) = get_ai_move_with_stats(state);
    let elapsed = start.elapsed().as_secs_f64();
    (best, moves, elapsed, stats.nodes_visited, stats.pruning_percent())
}

#[pyfunction]
pub fn get_hint(
    _py: Python,
    state: &Gomoku,
) -> Vec<(usize, usize, Option<i32>)> {
    let (_, moves, _) = get_ai_move_with_stats(state);
    moves
}

#[pyfunction]
pub fn get_move_pv(state: &Gomoku, x: usize, y: usize) -> (Vec<(usize, usize)>, i32) {
    let mut board = SearchBoard::from_gomoku(state);
    let is_max_player = board.current == Cell::Black;
    let max_depth = if board.move_count < 4 { 3 } else { MAX_DEPTH };
    let tt = shared_tt();
    let deadline = search_deadline();

    let undo = board.make_move(x, y);

    let mut final_pv = vec![];
    let mut final_value = 0i32;
    let mut heur = MoveHeuristics::new();
    for depth in 1..=max_depth {
        if Instant::now() >= deadline {
            break;
        }
        let mut stats = SearchStats::new();
        stats.max_depth = depth;
        let (value, child_pv, timed_out) = sb_alphabeta(
            &mut board, MIN_VALUE, MAX_VALUE, !is_max_player,
            1, depth, &mut stats, tt, deadline, &mut heur, true,
        );
        if timed_out {
            break;
        }
        final_value = value;
        final_pv = child_pv;
    }

    board.undo_move(&undo);

    let mut pv = vec![(x, y)];
    pv.extend(final_pv);
    (pv, final_value)
}

pub fn get_ai_move_with_stats(
    state: &Gomoku,
) -> (
    Option<(usize, usize, i32)>,
    Vec<(usize, usize, Option<i32>)>,
    SearchStats,
) {
    let mut board = SearchBoard::from_gomoku(state);
    let is_max_player = board.current == Cell::Black;

    let candidates = board.get_candidate_moves(0, None, true);
    let mut all_moves: Vec<(usize, usize, Option<i32>)> = candidates
        .into_iter()
        .map(|(r, c)| (r, c, None))
        .collect();

    let max_depth = if board.move_count < 4 { 3 } else { MAX_DEPTH };

    let mut best_move: Option<(usize, usize, i32)> = None;
    let mut final_stats = SearchStats::new();
    let mut depth_times: Vec<(usize, f64, u64)> = Vec::new();
    let tt = shared_tt();
    let deadline = search_deadline();
    // Kept across iterative-deepening iterations: what refuted a move at depth
    // d is usually still the refutation at depth d+1.
    let mut root_heur = MoveHeuristics::new();

    for depth in 1..=max_depth {
        if Instant::now() >= deadline {
            break;
        }
        let depth_start = Instant::now();
        let mut stats = SearchStats::new();
        stats.max_depth = depth;
        stats.internal_nodes += 1;
        stats.total_children += all_moves.len() as u64;

        let mut alpha = MIN_VALUE;
        let mut beta = MAX_VALUE;
        let mut best_value = if is_max_player { MIN_VALUE - 1 } else { MAX_VALUE + 1 };
        let mut iteration_best_move: Option<(usize, usize, i32)> = None;
        let mut iteration_pv: Vec<(usize, usize)> = Vec::new();
        let mut branch_times: Vec<(usize, usize, Option<i32>, f64)> = Vec::new();

        if all_moves.is_empty() {
            depth_times.push((depth, depth_start.elapsed().as_secs_f64(), stats.nodes_visited));
            final_stats = stats;
            break;
        }

        // ---- Phase 1: Sequential first child with full window ----
        let mut phase1_timed_out = false;
        {
            let m = &mut all_moves[0];
            let move_r = m.0;
            let move_c = m.1;
            stats.children_explored += 1;
            let branch_start = Instant::now();

            let undo = board.make_move(move_r, move_c);
            let (value, child_pv, timed_out) = sb_alphabeta(
                &mut board, alpha, beta, !is_max_player, 1, depth, &mut stats, tt, deadline,
                &mut root_heur,
                true,
            );
            board.undo_move(&undo);

            if timed_out {
                phase1_timed_out = true;
            } else {
                let branch_elapsed = branch_start.elapsed().as_secs_f64();
                m.2 = Some(value);
                branch_times.push((move_r, move_c, Some(value), branch_elapsed));

                if is_max_player && value > best_value {
                    best_value = value;
                    alpha = max(alpha, best_value);
                    iteration_best_move = Some((move_r, move_c, value));
                    iteration_pv = std::iter::once((move_r, move_c))
                        .chain(child_pv.into_iter())
                        .collect();
                } else if !is_max_player && value < best_value {
                    best_value = value;
                    beta = min(beta, best_value);
                    iteration_best_move = Some((move_r, move_c, value));
                    iteration_pv = std::iter::once((move_r, move_c))
                        .chain(child_pv.into_iter())
                        .collect();
                }
            }
        }
        if phase1_timed_out {
            break;
        }

        // ---- Phase 2: Parallel YBWC for the remaining root moves ----
        let mut phase2_timed_out = false;
        if alpha < beta && all_moves.len() > 1 {
            let parent_alpha = alpha;
            let parent_beta = beta;

            let tasks: Vec<((usize, usize), SearchBoard)> = all_moves[1..]
                .iter()
                .map(|m| {
                    let r = m.0;
                    let c = m.1;
                    let mut clone = board.clone();
                    let _ = clone.make_move(r, c);
                    ((r, c), clone)
                })
                .collect();

            let results: Vec<(
                (usize, usize),
                i32,
                Vec<(usize, usize)>,
                SearchStats,
                f64,
                bool,
            )> = tasks
                .into_par_iter()
                .map(|((r, c), mut child_board)| {
                    let mut child_stats = SearchStats::new();
                    // Each task orders its own subtree; sharing one table
                    // across rayon workers would need a lock per cutoff.
                    let mut child_heur = MoveHeuristics::new();
                    let branch_start = Instant::now();
                    let (v, pv, timed_out) = if is_max_player {
                        let (mut v, mut pv, mut timed_out) = sb_alphabeta(
                            &mut child_board,
                            parent_alpha,
                            parent_alpha + 1,
                            false,
                            1,
                            depth,
                            &mut child_stats,
                            tt,
                            deadline,
                            &mut child_heur,
                            true,
                        );
                        if !timed_out && v > parent_alpha && v < parent_beta {
                            let r2 = sb_alphabeta(
                                &mut child_board,
                                parent_alpha,
                                parent_beta,
                                false,
                                1,
                                depth,
                                &mut child_stats,
                                tt,
                                deadline,
                                &mut child_heur,
                                true,
                            );
                            v = r2.0;
                            pv = r2.1;
                            timed_out = r2.2;
                        }
                        (v, pv, timed_out)
                    } else {
                        let (mut v, mut pv, mut timed_out) = sb_alphabeta(
                            &mut child_board,
                            parent_beta - 1,
                            parent_beta,
                            true,
                            1,
                            depth,
                            &mut child_stats,
                            tt,
                            deadline,
                            &mut child_heur,
                            true,
                        );
                        if !timed_out && v < parent_beta && v > parent_alpha {
                            let r2 = sb_alphabeta(
                                &mut child_board,
                                parent_alpha,
                                parent_beta,
                                true,
                                1,
                                depth,
                                &mut child_stats,
                                tt,
                                deadline,
                                &mut child_heur,
                                true,
                            );
                            v = r2.0;
                            pv = r2.1;
                            timed_out = r2.2;
                        }
                        (v, pv, timed_out)
                    };
                    let branch_elapsed = branch_start.elapsed().as_secs_f64();
                    ((r, c), v, pv, child_stats, branch_elapsed, timed_out)
                })
                .collect();

            phase2_timed_out = results.iter().any(|r| r.5);

            if !phase2_timed_out {
                // Merge phase: serial best-update, stats merge, branch_times.
                for ((r, c), value, child_pv, child_stats, branch_elapsed, _) in results {
                    stats.children_explored += 1;
                    stats.merge(&child_stats);

                    // Write the score back into all_moves.
                    for m in all_moves.iter_mut() {
                        if m.0 == r && m.1 == c {
                            m.2 = Some(value);
                            break;
                        }
                    }
                    branch_times.push((r, c, Some(value), branch_elapsed));

                    if is_max_player && value > best_value {
                        best_value = value;
                        alpha = max(alpha, best_value);
                        iteration_best_move = Some((r, c, value));
                        iteration_pv = std::iter::once((r, c))
                            .chain(child_pv.into_iter())
                            .collect();
                    } else if !is_max_player && value < best_value {
                        best_value = value;
                        beta = min(beta, best_value);
                        iteration_best_move = Some((r, c, value));
                        iteration_pv = std::iter::once((r, c))
                            .chain(child_pv.into_iter())
                            .collect();
                    }
                }
            }
        }
        if phase2_timed_out {
            break;
        }

        stats.branch_times = branch_times;
        stats.pv = iteration_pv;
        depth_times.push((depth, depth_start.elapsed().as_secs_f64(), stats.nodes_visited));
        best_move = iteration_best_move;
        final_stats = stats;

        if is_max_player {
            all_moves.sort_by(|a, b| {
                let sa = a.2.unwrap_or(MIN_VALUE);
                let sb = b.2.unwrap_or(MIN_VALUE);
                sb.cmp(&sa)
            });
        } else {
            all_moves.sort_by(|a, b| {
                let sa = a.2.unwrap_or(MAX_VALUE);
                let sb = b.2.unwrap_or(MAX_VALUE);
                sa.cmp(&sb)
            });
        }
    }

    final_stats.depth_times = depth_times;

    if RANDOMIZE_TIED_MOVES {
        if let Some((_, _, best_value)) = best_move {
            let tied: Vec<(usize, usize)> = all_moves
                .iter()
                .filter(|m| m.2 == Some(best_value))
                .map(|m| (m.0, m.1))
                .collect();
            if tied.len() > 1 {
                let (r, c) = tied[Rng::seeded().next_usize(tied.len())];
                best_move = Some((r, c, best_value));
            }
        }
    }

    (best_move, all_moves, final_stats)
}

