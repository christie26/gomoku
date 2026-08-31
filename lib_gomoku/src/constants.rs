// Board

pub const BOARD_SIZE: usize = 19;

// Search bounds

pub const MAX_VALUE: i32 = 100_000;
pub const MIN_VALUE: i32 = -100_000;
pub const MAX_DEPTH: usize = 9;
pub const SHALLOW_ORDER_DEPTH: usize = 2;
pub const RADIUS: usize = 2;
// From this ply (inclusive) onward, shrink candidate-move radius to DEEP_RADIUS.
pub const DEEP_RADIUS_DEPTH: usize = 3;
pub const DEEP_RADIUS: usize = 1;

/// Wall-clock budget for the iterative-deepening search in
/// `get_ai_move_with_stats`/`get_move_pv`. `Some(ms)` aborts an in-progress
/// depth once exceeded, falling back to the last fully-completed depth.
/// `None` disables the timer entirely (search always runs to MAX_DEPTH) —
/// flip this to compare timed vs. untimed behavior.
// pub const TIME_LIMIT_MS: Option<u64> = Some(500);
pub const TIME_LIMIT_MS: Option<u64> = None;

/// When multiple root moves end up tied for the best score, pick randomly
/// among them instead of always keeping the first (scan-order) one. Flip to
/// `false` to restore fully deterministic move selection.
pub const RANDOMIZE_TIED_MOVES: bool = false;

// Late move reductions. Moves far down the ordering rarely raise alpha, so
// they are searched shallower first and only re-searched at full depth if one
// beats alpha. Tactical moves (fours, fives, captures) are never reduced.

/// Plies left below which reducing is not worth it.
pub const LMR_MIN_DEPTH: usize = 3;
/// Ordering position from which reductions start (0-based).
pub const LMR_MIN_MOVE: usize = 3;
/// Ordering position from which the reduction doubles.
pub const LMR_DEEP_MOVE: usize = 6;

// Transposition table

pub const TT_SHARDS: usize = 64;
pub const TT_SHARD_MASK: u64 = (TT_SHARDS as u64) - 1;
pub const TT_SIZE_BITS: usize = 14;

// Evaluation clamps (kept distinct: the two evaluators historically used
// slightly different clamp margins, preserved as-is)

pub const SB_EVAL_CLAMP: i32 = 99_991;
pub const HEURISTIC_EVAL_CLAMP: i32 = 99_999;

// Pattern scoring weights, shared by minimax::PatternCounts::score and
// heuristic::evaluate_player.

pub const WEIGHT_FIVE: i32 = 80_001;
pub const WEIGHT_OPEN_FOUR: i32 = 70_000;
pub const WEIGHT_BLOCK_FOUR: i32 = 7_000;
pub const WEIGHT_BLOCK_FOUR_TEMPO: i32 = 70_000;
pub const WEIGHT_FREE_THREE: i32 = 5_000;
pub const WEIGHT_FREE_THREE_TEMPO: i32 = 60_000;
pub const WEIGHT_OPEN_THREE: i32 = 100;
pub const WEIGHT_OPEN_TWO: i32 = 50;

pub const CAPTURE_BONUS_1: i32 = 5_000;
pub const CAPTURE_BONUS_2: i32 = 12_000;
pub const CAPTURE_BONUS_3: i32 = 25_000;
pub const CAPTURE_BONUS_4_PLUS: i32 = 50_000;

pub const COMBO_DOUBLE_OPEN_FOUR: i32 = 40_000;
pub const COMBO_OPEN_AND_BLOCK_FOUR: i32 = 35_000;
pub const COMBO_DOUBLE_BLOCK_FOUR: i32 = 30_000;
pub const COMBO_OPEN_FOUR_AND_THREE: i32 = 30_000;
pub const COMBO_BLOCK_FOUR_AND_THREE: i32 = 20_000;
pub const COMBO_DOUBLE_THREE: i32 = 15_000;
pub const COMBO_CAPTURE_AND_FOUR: i32 = 25_000;
