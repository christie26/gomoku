//! Zero-heap board representation and pattern recognizer used by the search.
//!
//! This module owns the crate's only pattern recognizer. It replaces the
//! coordinate-based system kept for reference in `pattern_legacy.rs`.

use crate::constants::{
    CAPTURE_BONUS_1, CAPTURE_BONUS_2, CAPTURE_BONUS_3, CAPTURE_BONUS_4_PLUS, COMBO_BLOCK_FOUR_AND_THREE, COMBO_CAPTURE_AND_FOUR, COMBO_DOUBLE_BLOCK_FOUR, COMBO_DOUBLE_OPEN_FOUR, COMBO_DOUBLE_THREE, COMBO_OPEN_AND_BLOCK_FOUR, COMBO_OPEN_FOUR_AND_THREE, DEEP_RADIUS, DEEP_RADIUS_DEPTH, RADIUS, SB_EVAL_CLAMP, SHALLOW_ORDER_DEPTH, WEIGHT_BLOCK_FOUR, WEIGHT_BLOCK_FOUR_TEMPO, WEIGHT_FIVE, WEIGHT_FREE_THREE, WEIGHT_FREE_THREE_TEMPO, WEIGHT_OPEN_FOUR, WEIGHT_OPEN_THREE, WEIGHT_OPEN_TWO,
};
use crate::{zobrist, Gomoku, Pattern, Stone};

// =====================================================================
// SearchBoard: zero-heap make/unmake board for the search
// =====================================================================

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub(crate) enum Cell {
    Empty = 0,
    Black = 1,
    White = 2,
}

impl Cell {
    pub(crate) fn of(stone: Stone) -> Cell {
        match stone {
            Stone::Black => Cell::Black,
            Stone::White => Cell::White,
            Stone::Empty => Cell::Empty,
        }
    }

    #[inline]
    pub(crate) fn opponent(self) -> Cell {
        match self {
            Cell::Black => Cell::White,
            Cell::White => Cell::Black,
            Cell::Empty => Cell::Empty,
        }
    }

    #[inline]
    pub(crate) fn zobrist_idx(self) -> usize {
        // Black=0, White=1
        (self as usize) - 1
    }
}

#[derive(Clone)]
pub(crate) struct SearchBoard {
    pub(crate) cells: [[Cell; 19]; 19],
    pub(crate) current: Cell,
    pub(crate) opponent: Cell,
    pub(crate) captures: [i32; 3], // Index by Cell as u8: [unused, Black, White]
    pub(crate) hash: u64,
    pub(crate) move_count: usize,
    pub(crate) last_move: Option<(usize, usize)>,
    pub(crate) total_stones: usize,
    // Incrementally maintained by make_move/undo_move — see sb_local_patterns.
    pub(crate) black_patterns: PatternCounts,
    pub(crate) white_patterns: PatternCounts,
}


#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub(crate) struct PatternCounts {
    pub(crate) five_rows: i32,
    pub(crate) open_fours: i32,
    pub(crate) block_fours: i32,
    pub(crate) open_threes: i32,
    pub(crate) open_twos: i32,
    pub(crate) free_threes: i32,
}


impl std::ops::Sub for PatternCounts {
    type Output = PatternCounts;
    fn sub(self, other: PatternCounts) -> PatternCounts {
        PatternCounts {
            five_rows: self.five_rows - other.five_rows,
            open_fours: self.open_fours - other.open_fours,
            block_fours: self.block_fours - other.block_fours,
            open_threes: self.open_threes - other.open_threes,
            open_twos: self.open_twos - other.open_twos,
            free_threes: self.free_threes - other.free_threes,
        }
    }
}

impl PatternCounts {
    /// Add `delta` scaled by `sign` (1 to apply a move, -1 to undo it).
    pub(crate) fn apply_delta(&mut self, delta: &PatternCounts, sign: i32) {
        self.five_rows += delta.five_rows * sign;
        self.open_fours += delta.open_fours * sign;
        self.block_fours += delta.block_fours * sign;
        self.open_threes += delta.open_threes * sign;
        self.open_twos += delta.open_twos * sign;
        self.free_threes += delta.free_threes * sign;
    }

    pub(crate) fn score(&self, captures: i32, is_active: bool) -> i32 {
        let mut score = 0i32;
        score += self.five_rows * WEIGHT_FIVE;
        score += self.open_fours * WEIGHT_OPEN_FOUR;
        score += self.open_threes * WEIGHT_OPEN_THREE;
        score += self.open_twos * WEIGHT_OPEN_TWO;

        score += match captures {
            0 => 0,
            1 => CAPTURE_BONUS_1,
            2 => CAPTURE_BONUS_2,
            3 => CAPTURE_BONUS_3,
            4 => CAPTURE_BONUS_4_PLUS,
            _ => CAPTURE_BONUS_4_PLUS,
        };

        if is_active {
          score += self.block_fours * WEIGHT_BLOCK_FOUR_TEMPO;
          score += self.free_threes * WEIGHT_FREE_THREE_TEMPO;
        } else {
          score += self.block_fours * WEIGHT_BLOCK_FOUR;
          score += self.free_threes * WEIGHT_FREE_THREE;
        }

        let total_threes = self.open_threes + self.free_threes;
        if self.open_fours >= 2 { score += COMBO_DOUBLE_OPEN_FOUR; }
        if self.open_fours >= 1 && self.block_fours >= 1 { score += COMBO_OPEN_AND_BLOCK_FOUR; }
        if self.block_fours >= 2 { score += COMBO_DOUBLE_BLOCK_FOUR; }
        if self.open_fours >= 1 && total_threes >= 1 { score += COMBO_OPEN_FOUR_AND_THREE; }
        if self.block_fours >= 1 && total_threes >= 1 { score += COMBO_BLOCK_FOUR_AND_THREE; }
        if total_threes >= 2 { score += COMBO_DOUBLE_THREE; }
        if captures >= 4 && (self.block_fours >= 1 || self.open_fours >= 1) { score += COMBO_CAPTURE_AND_FOUR; }



        // if is_active {
        //     if self.open_fours >= 1 { score += TEMPO_OPEN_FOUR; }
        //     if self.block_fours >= 1 { score += TEMPO_BLOCK_FOUR; }
        //     if captures >= 4 { score += TEMPO_CAPTURE; }
        // }

        score / 10
    }
}

pub(crate) struct UndoInfo {
    pub(crate) placed: (usize, usize),
    pub(crate) _placed_player: Cell,
    pub(crate) captured_stones: [(usize, usize); 8], // max 4 captures * 2 stones each = 8
    pub(crate) num_captured: usize,
    pub(crate) old_captures_current: i32,
    pub(crate) old_hash: u64,
    pub(crate) old_move_count: usize,
    pub(crate) old_last_move: Option<(usize, usize)>,
    pub(crate) old_total_stones: usize,
    pub(crate) delta_black: PatternCounts,
    pub(crate) delta_white: PatternCounts,
}


/// The six formations the evaluator scores. Same taxonomy as the legacy
/// coordinate scanner kept in `pattern_legacy.rs`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum PatternKind {
    OpenTwo,
    OpenThree,
    FreeThree,
    BlockFour,
    OpenFour,
    FiveRow,
}

/// The four scan directions. Their opposites are covered by anchoring on every
/// stone rather than by walking both ways.
const SCAN_DIRS: [(i32, i32); 4] = [(1, 0), (0, 1), (1, 1), (1, -1)];

/// How far along a line a single changed cell can alter another anchor's
/// classification. A scored formation spans at most 6 cells and a ray reads at
/// most 2 empties past its last stone, so 8 is comfortably beyond the reach.
const LOCAL_RADIUS: i32 = 8;

/// A move changes the cell it lands on plus at most four captured pairs.
const MAX_CHANGED_CELLS: usize = 9;

/// One ray walked outward from an anchor. Port of `pattern_legacy::LineScan`.
struct LineScan {
    contig_my: i32,
    end_open: bool,
    total_my: i32,
    empty_count: i32,
    /// Bit `i-1` is set when the cell at offset `i` along the ray held my stone.
    stones: u32,
}

/// A classified formation, anchored on its first stone. `mask` records which
/// cells from that anchor hold stones.
#[derive(PartialEq, Eq)]
struct Formation {
    color: Cell,
    kind: PatternKind,
    mask: u32,
}

/// Verbatim port of `pattern_legacy::classify`.
fn classify(plus: &LineScan, minus: &LineScan, center_stone: i32) -> Option<PatternKind> {
    let total = plus.total_my + minus.total_my + center_stone;
    let contig_total = if center_stone != 0 {
        plus.contig_my + minus.contig_my + center_stone
    } else {
        plus.contig_my.max(minus.contig_my)
    };
    let empty = plus.empty_count + minus.empty_count + (1 - center_stone);
    // `>= 5`, not `== 5`: an overline still wins, and the legacy classifier's
    // exact `== 5` left runs of six or more scoring nothing at all.
    if contig_total >= 5 && center_stone != 0 {
        Some(PatternKind::FiveRow)
    } else if contig_total == 4 && plus.empty_count > 0 && minus.empty_count > 0 {
        Some(PatternKind::OpenFour)
    } else if total == 4 && (plus.end_open || minus.end_open) {
        Some(PatternKind::BlockFour)
    } else if total == 4 && empty == 1 {
        Some(PatternKind::BlockFour)
    } else if (contig_total == 3 && empty >= 3)
        || (total == 3 && empty >= 3 && plus.end_open && minus.end_open)
    {
        Some(PatternKind::FreeThree)
    } else if total == 3 && empty >= 2 {
        Some(PatternKind::OpenThree)
    } else if total == 2 && plus.empty_count > 0 && minus.empty_count > 0 {
        Some(PatternKind::OpenTwo)
    } else {
        None
    }
}

/// Where a scan reports the formations it finds. One scan drives both the
/// counters the search runs on and the coordinate lists the debug getters want,
/// so the two can never disagree about what the board contains.
pub(crate) trait PatternSink {
    fn hit(&mut self, color: Cell, kind: PatternKind, start: (i32, i32), dir: (i32, i32), len: i32);
}

#[derive(Default)]
pub(crate) struct CountSink {
    pub(crate) black: PatternCounts,
    pub(crate) white: PatternCounts,
}

impl PatternSink for CountSink {
    fn hit(&mut self, color: Cell, kind: PatternKind, _s: (i32, i32), _d: (i32, i32), _l: i32) {
        let counts = if color == Cell::Black { &mut self.black } else { &mut self.white };
        match kind {
            PatternKind::OpenTwo => counts.open_twos += 1,
            PatternKind::OpenThree => counts.open_threes += 1,
            PatternKind::FreeThree => counts.free_threes += 1,
            PatternKind::BlockFour => counts.block_fours += 1,
            PatternKind::OpenFour => counts.open_fours += 1,
            PatternKind::FiveRow => counts.five_rows += 1,
        }
    }
}

/// Coordinate lists per kind, for one colour. Only the `Gomoku` debug getters
/// and `print_state` use these; the search never allocates them.
#[derive(Clone, Debug, Default)]
pub struct PlayerRanges {
    pub open_two: Vec<Pattern>,
    pub open_three: Vec<Pattern>,
    pub free_three: Vec<Pattern>,
    pub open_four: Vec<Pattern>,
    pub block_four: Vec<Pattern>,
    pub five_row: Vec<Pattern>,
}

#[derive(Default)]
pub(crate) struct RangeSink {
    pub(crate) black: PlayerRanges,
    pub(crate) white: PlayerRanges,
}

impl PatternSink for RangeSink {
    fn hit(&mut self, color: Cell, kind: PatternKind, start: (i32, i32), dir: (i32, i32), len: i32) {
        let ranges = if color == Cell::Black { &mut self.black } else { &mut self.white };
        let cells = (0..len).map(|i| (start.0 + dir.0 * i, start.1 + dir.1 * i)).collect();
        match kind {
            PatternKind::OpenTwo => ranges.open_two.push(cells),
            PatternKind::OpenThree => ranges.open_three.push(cells),
            PatternKind::FreeThree => ranges.free_three.push(cells),
            PatternKind::BlockFour => ranges.block_four.push(cells),
            PatternKind::OpenFour => ranges.open_four.push(cells),
            PatternKind::FiveRow => ranges.five_row.push(cells),
        }
    }
}

pub fn position_name(pos: &(i32, i32)) -> String {
    let (y, x) = pos;
    let x = "abcdefghijklmnopqrstuvwxyz".chars().nth(*x as usize).unwrap_or('-');
    let y = y + 1;
    format!("{x}{y}")
}

pub(crate) fn print_pattern_kind(name: &str, patterns: &[Pattern]) {
    let rendered: Vec<String> = patterns
        .iter()
        .map(|pattern| {
            let positions: Vec<String> = pattern.iter().map(position_name).collect();
            format!("[{}]", positions.join(","))
        })
        .collect();
    println!("  {name}: {} {}", patterns.len(), rendered.join(" "));
}

const ALL_DIRS: [(i32, i32); 8] = [
    (1, 0), (0, 1), (1, 1), (1, -1),
    (-1, 0), (0, -1), (-1, -1), (-1, 1),
];

impl SearchBoard {
    /// Copy a `Gomoku`'s cells and scalars. No pattern scan.
    pub(crate) fn cells_from_gomoku(g: &Gomoku) -> Self {
        let mut cells = [[Cell::Empty; 19]; 19];
        let mut total_stones = 0usize;
        for r in 0..19 {
            for c in 0..19 {
                cells[r][c] = match g.board[r][c] {
                    Stone::Black => { total_stones += 1; Cell::Black }
                    Stone::White => { total_stones += 1; Cell::White }
                    Stone::Empty => Cell::Empty,
                };
            }
        }
        let current = match g.current_player {
            Stone::Black => Cell::Black,
            Stone::White => Cell::White,
            _ => Cell::Empty,
        };
        let cap_b = *g.capture_count.get(&Stone::Black).unwrap_or(&0);
        let cap_w = *g.capture_count.get(&Stone::White).unwrap_or(&0);
        let last_move = g.current_move.map(|(x, y)| (x as usize, y as usize));
        SearchBoard {
            cells,
            current,
            opponent: current.opponent(),
            captures: [0, cap_b, cap_w],
            hash: g.hash,
            move_count: g.move_count,
            last_move,
            total_stones,
            black_patterns: PatternCounts::default(),
            white_patterns: PatternCounts::default(),
        }
    }

    /// `cells_from_gomoku` plus the pattern counts the search maintains from
    /// there on. Callers that only need board geometry can skip the scan.
    pub(crate) fn from_gomoku(g: &Gomoku) -> Self {
        let mut board = Self::cells_from_gomoku(g);
        let (black_patterns, white_patterns) = board.sb_scan_patterns();
        board.black_patterns = black_patterns;
        board.white_patterns = white_patterns;
        board
    }

    #[inline]
    pub(crate) fn in_bounds(r: i32, c: i32) -> bool {
        r >= 0 && r < 19 && c >= 0 && c < 19
    }

    #[inline]
    pub(crate) fn get(&self, r: i32, c: i32) -> Cell {
        self.cells[r as usize][c as usize]
    }

    pub(crate) fn make_move(&mut self, r: usize, c: usize) -> UndoInfo {
        let z = zobrist();
        let old_hash = self.hash;
        let old_captures = self.captures[self.current as usize];
        let old_move_count = self.move_count;
        let old_last_move = self.last_move;
        let old_total_stones = self.total_stones;

        let ri = r as i32;
        let ci = c as i32;

        // Find captures before touching anything. The before/after snapshots
        // have to cover every cell this move changes, and which stones come off
        // the board is only knowable by looking. Detection reads offsets 1..3
        // along each direction and never (r,c) itself, so it gives the same
        // answer before the stone is placed as after.
        let mut captured_stones = [(0usize, 0usize); 8];
        let mut num_captured = 0usize;

        for &(dr, dc) in ALL_DIRS.iter() {
            let (r1, c1) = (ri + dr, ci + dc);
            let (r2, c2) = (ri + 2 * dr, ci + 2 * dc);
            let (r3, c3) = (ri + 3 * dr, ci + 3 * dc);

            if !Self::in_bounds(r3, c3) { continue; }
            if self.get(r1, c1) == self.opponent
                && self.get(r2, c2) == self.opponent
                && self.get(r3, c3) == self.current
            {
                captured_stones[num_captured] = (r1 as usize, c1 as usize);
                captured_stones[num_captured + 1] = (r2 as usize, c2 as usize);
                num_captured += 2;
            }
        }

        // Every cell this move touches, so one local sweep covers the lot.
        let mut changed = [(0i32, 0i32); MAX_CHANGED_CELLS];
        changed[0] = (ri, ci);
        for i in 0..num_captured {
            changed[i + 1] = (captured_stones[i].0 as i32, captured_stones[i].1 as i32);
        }
        let changed = &changed[..num_captured + 1];

        let (before_b, before_w) = self.sb_local_patterns(changed);

        // Place stone
        self.cells[r][c] = self.current;
        self.hash ^= z.board[r * 19 + c][self.current.zobrist_idx()];
        self.total_stones += 1;
        self.move_count += 1;
        self.last_move = Some((r, c));

        // Lift the captured stones
        let opp_zi = self.opponent.zobrist_idx();
        for i in 0..num_captured {
            let (ur, uc) = captured_stones[i];
            self.cells[ur][uc] = Cell::Empty;
            self.hash ^= z.board[ur * 19 + uc][opp_zi];
        }
        self.captures[self.current as usize] += (num_captured / 2) as i32;
        self.total_stones -= num_captured;

        // Switch player
        self.hash ^= z.player;
        let tmp = self.current;
        self.current = self.opponent;
        self.opponent = tmp;

        // Update the incrementally maintained pattern counts. A move can only
        // change classifications near the cells it touched, so diffing a local
        // sweep over those cells before and after is enough — captures included.
        let (after_b, after_w) = self.sb_local_patterns(changed);
        let (delta_black, delta_white) = (after_b - before_b, after_w - before_w);
        self.black_patterns.apply_delta(&delta_black, 1);
        self.white_patterns.apply_delta(&delta_white, 1);

        UndoInfo {
            placed: (r, c),
            _placed_player: tmp, // the player who made the move
            captured_stones,
            num_captured,
            old_captures_current: old_captures,
            old_hash: old_hash,
            old_move_count,
            old_last_move,
            old_total_stones,
            delta_black,
            delta_white,
        }
    }

    pub(crate) fn undo_move(&mut self, info: &UndoInfo) {
        // Swap players back
        let tmp = self.current;
        self.current = self.opponent;
        self.opponent = tmp;

        self.black_patterns.apply_delta(&info.delta_black, -1);
        self.white_patterns.apply_delta(&info.delta_white, -1);

        // Remove placed stone
        let (r, c) = info.placed;
        self.cells[r][c] = Cell::Empty;

        // Restore captured stones
        for i in 0..info.num_captured {
            let (cr, cc) = info.captured_stones[i];
            self.cells[cr][cc] = self.opponent; // opponent of the player who moved
        }

        // Restore scalars
        self.captures[self.current as usize] = info.old_captures_current;
        self.hash = info.old_hash;
        self.move_count = info.old_move_count;
        self.last_move = info.old_last_move;
        self.total_stones = info.old_total_stones;
    }

    // ---- Board scanning helpers ----

    /// Count consecutive stones of `player` starting from (r,c) going (dr,dc), max 4 steps
    pub(crate) fn count_stones(&self, r: usize, c: usize, dr: i32, dc: i32, player: Cell) -> i32 {
        let mut count = 0;
        for i in 1..5i32 {
            let nr = r as i32 + dr * i;
            let nc = c as i32 + dc * i;
            if !Self::in_bounds(nr, nc) { break; }
            if self.cells[nr as usize][nc as usize] == player {
                count += 1;
            } else {
                break;
            }
        }
        count
    }

    /// Check if placing `player` at (r,c) can capture in direction (dr,dc)
    pub(crate) fn can_capture_at(&self, r: usize, c: usize, dr: i32, dc: i32, player: Cell) -> bool {
        let opp = player.opponent();
        let ri = r as i32;
        let ci = c as i32;
        let r1 = ri + dr; let c1 = ci + dc;
        let r2 = ri + 2*dr; let c2 = ci + 2*dc;
        let r3 = ri + 3*dr; let c3 = ci + 3*dc;
        if !Self::in_bounds(r3, c3) { return false; }
        self.get(r1, c1) == opp && self.get(r2, c2) == opp && self.get(r3, c3) == player
    }

    /// Evaluate move ordering score for an empty cell (r,c) for `player`
    pub(crate) fn evaluate_position(&self, r: usize, c: usize, player: Cell) -> i32 {
        let dirs: [(i32,i32); 4] = [(1,0),(0,1),(1,1),(1,-1)];
        let mut score = 0i32;
        for &(dr, dc) in &dirs {
            let mut count = 1i32;
            count += self.count_stones(r, c, dr, dc, player);
            count += self.count_stones(r, c, -dr, -dc, player);
            score += match count {
                n if n >= 5 => 100_000,
                4 => 10_000,
                3 => 1_000,
                2 => 100,
                _ => 0,
            };
            if self.can_capture_at(r, c, dr, dc, player) { score += 50_000; }
            if self.can_capture_at(r, c, -dr, -dc, player) { score += 50_000; }
        }
        score
    }

    // ---- Free-three / double-three detection ----

    /// Port of `Gomoku::is_double_three_move`: classify all four directions as
    /// if `self.current` already had a stone on (x0,y0), which is still empty.
    /// Could a stone at (x0,y0) possibly be part of a free three along
    /// (dx,dy)? A free three puts three stones inside a six-cell window, and
    /// the two furthest apart sit three cells apart, so the other two stones
    /// must both lie within three steps. Six array reads, and it rejects most
    /// candidates before the ray scans below have to run.
    #[inline]
    fn could_be_free_three(&self, x0: i32, y0: i32, dx: i32, dy: i32, player: Cell) -> bool {
        let mut nearby = 0;
        for i in 1..=3i32 {
            for sign in [1i32, -1] {
                let (x, y) = (x0 + dx * i * sign, y0 + dy * i * sign);
                if Self::in_bounds(x, y) && self.get(x, y) == player {
                    nearby += 1;
                }
            }
        }
        nearby >= 2
    }

    /// Port of `Gomoku::is_double_three_move`: classify all four directions as
    /// if `self.current` already had a stone on (x0,y0), which is still empty.
    pub(crate) fn sb_is_double_three(&self, x0: i32, y0: i32) -> bool {
        let player = self.current;
        let dirs = [(1, -1), (1, 0), (1, 1), (0, 1)];
        let mut free_three_count = 0;
        for (i, (dx, dy)) in dirs.into_iter().enumerate() {
            // Can't reach two even if every remaining direction is a free three.
            if free_three_count + (4 - i) < 2 {
                return false;
            }
            if !self.could_be_free_three(x0, y0, dx, dy, player) {
                continue;
            }
            let plus = self.scan_line_as(player, 1, dx, dy, x0, y0);
            let minus = self.scan_line_as(player, -1, dx, dy, x0, y0);
            if classify(&plus, &minus, 1) == Some(PatternKind::FreeThree) {
                free_three_count += 1;
                if free_three_count > 1 {
                    return true;
                }
            }
        }
        false
    }

    // ---- Terminal detection ----

    /// Check if `player` has 5+ in a row anywhere on the board
    pub(crate) fn has_five_in_row(&self, player: Cell) -> bool {
        let dirs: [(i32,i32); 4] = [(1,0),(0,1),(1,1),(1,-1)];
        for r in 0..19i32 {
            for c in 0..19i32 {
                if self.get(r, c) != player { continue; }
                for &(dr, dc) in &dirs {
                    // Only count from the "start" of a run
                    let pr = r - dr;
                    let pc = c - dc;
                    if Self::in_bounds(pr, pc) && self.get(pr, pc) == player { continue; }
                    let mut count = 1;
                    let mut nr = r + dr;
                    let mut nc = c + dc;
                    while Self::in_bounds(nr, nc) && self.get(nr, nc) == player {
                        count += 1;
                        nr += dr;
                        nc += dc;
                    }
                    if count >= 5 { return true; }
                }
            }
        }
        false
    }

    /// Check if stone at (x,y) belongs to `player` and is in a capturable pair
    pub(crate) fn sb_stone_in_capturable_pair(&self, x: i32, y: i32, player: Cell) -> bool {
        if !Self::in_bounds(x, y) || self.get(x, y) != player { return false; }
        let opp = player.opponent();
        let dirs: [(i32,i32); 4] = [(1,0),(0,1),(1,1),(1,-1)];
        for (dx, dy) in dirs {
            // (x,y)-(x+dx,y+dy) pair
            let nx = x + dx; let ny = y + dy;
            if Self::in_bounds(nx, ny) && self.get(nx, ny) == player {
                // Check OPP-PP-EMPTY
                let bx = x - dx; let by = y - dy;
                let ax = nx + dx; let ay = ny + dy;
                if Self::in_bounds(bx, by) && Self::in_bounds(ax, ay)
                    && self.get(bx, by) == opp && self.get(ax, ay) == Cell::Empty
                {
                    return true;
                }
                // Check EMPTY-PP-OPP
                if Self::in_bounds(bx, by) && Self::in_bounds(ax, ay)
                    && self.get(bx, by) == Cell::Empty && self.get(ax, ay) == opp
                {
                    return true;
                }
            }
            // (x-dx,y-dy)-(x,y) pair
            let px = x - dx; let py = y - dy;
            if Self::in_bounds(px, py) && self.get(px, py) == player {
                let bx = px - dx; let by = py - dy;
                let ax = x + dx; let ay = y + dy;
                if Self::in_bounds(bx, by) && Self::in_bounds(ax, ay)
                    && self.get(bx, by) == opp && self.get(ax, ay) == Cell::Empty
                {
                    return true;
                }
                if Self::in_bounds(bx, by) && Self::in_bounds(ax, ay)
                    && self.get(bx, by) == Cell::Empty && self.get(ax, ay) == opp
                {
                    return true;
                }
            }
        }
        false
    }

    /// Check if `player` has an uncapturable five-in-a-row
    pub(crate) fn has_uncapturable_five(&self, player: Cell) -> bool {
        let dirs: [(i32,i32); 4] = [(1,0),(0,1),(1,1),(1,-1)];
        for r in 0..19i32 {
            for c in 0..19i32 {
                if self.get(r, c) != player { continue; }
                for &(dr, dc) in &dirs {
                    let pr = r - dr; let pc = c - dc;
                    if Self::in_bounds(pr, pc) && self.get(pr, pc) == player { continue; }
                    // Count run length
                    let mut count = 1;
                    let mut nr = r + dr; let mut nc = c + dc;
                    while Self::in_bounds(nr, nc) && self.get(nr, nc) == player {
                        count += 1;
                        nr += dr; nc += dc;
                    }
                    if count < 5 { continue; }
                    // Check if any stone in this run is capturable
                    let mut any_capturable = false;
                    for i in 0..count {
                        let sr = r + dr * i;
                        let sc = c + dc * i;
                        if self.sb_stone_in_capturable_pair(sr, sc, player) {
                            any_capturable = true;
                            break;
                        }
                    }
                    if !any_capturable { return true; }
                }
            }
        }
        false
    }

    /// Get winner after a make_move (which already switched players).
    /// `board.opponent` is who just moved, `board.current` is who moves next.
    /// Counts for `player`, whichever colour that is.
    #[inline]
    pub(crate) fn patterns_of(&self, player: Cell) -> &PatternCounts {
        if player == Cell::Black { &self.black_patterns } else { &self.white_patterns }
    }

    /// Winner after a `make_move`, which has already switched players — so
    /// `self.opponent` is whoever just moved and `self.current` moves next.
    ///
    /// Reads the incrementally maintained five counts instead of sweeping the
    /// board. `five_rows > 0` is exactly `has_five_in_row`: `classify` matches
    /// `FiveRow` on any contiguous run of five or more, ahead of every other
    /// rule, and `scan_all`/`scan_around` report each run once. The board sweep
    /// only survives for the capture-breakable case, which needs the run's
    /// cells and is reached only once a five already exists.
    pub(crate) fn get_winner(&self) -> Option<Cell> {
        // 1. Five captured pairs by whoever just moved.
        if self.captures[self.opponent as usize] >= 5 {
            return Some(self.opponent);
        }
        // 2. A five for the side about to move (a capture can open one up).
        if self.patterns_of(self.current).five_rows > 0 {
            return Some(self.current);
        }
        // 3. A five for the mover that no capture can break apart.
        if self.patterns_of(self.opponent).five_rows > 0
            && self.has_uncapturable_five(self.opponent)
        {
            return Some(self.opponent);
        }
        None
    }

    pub(crate) fn is_terminal(&self) -> bool {
        self.get_winner().is_some() || self.total_stones >= 361
    }

    // ---- Pattern scanning ----

    /// Walk one ray outward from (x0,y0). Stops at the edge, at an opponent
    /// stone, or once two empties have piled up. Port of
    /// `pattern_legacy::Gomoku::scan_line_as`, with a bitmask of which offsets
    /// held my stones added so callers can identify the formation.
    fn scan_line_as(&self, me: Cell, sign: i32, dx: i32, dy: i32, x0: i32, y0: i32) -> LineScan {
        let opponent = me.opponent();
        let mut contig_my = 0;
        let mut end_open = false;
        let mut contig_done = false;
        let mut total_my = 0;
        let mut empty_count = 0;
        let mut stones = 0u32;
        let mut i = 1i32;

        loop {
            let x = x0 + dx * i * sign;
            let y = y0 + dy * i * sign;

            if !Self::in_bounds(x, y) || self.get(x, y) == opponent {
                if empty_count == 2 {
                    end_open = true;
                }
                break;
            } else if empty_count == 2 {
                end_open = true;
                break;
            }

            if self.get(x, y) == me {
                if !contig_done {
                    contig_my += 1;
                }
                total_my += 1;
                if i < 32 {
                    stones |= 1 << (i - 1);
                }
            } else {
                contig_done = true;
                empty_count += 1;
            }
            i += 1;
        }

        LineScan { contig_my, end_open, total_my, empty_count, stones }
    }

    /// Classify the formation anchored on the stone at (r,c) along (dr,dc).
    ///
    /// Only the formation's *first* stone reports it: if the backward ray finds
    /// another stone of the same colour, some earlier anchor owns this
    /// formation and this one returns `None`. That keeps each formation counted
    /// once without a dedup set, and it matches the legacy recognizer, which
    /// classified from a single anchor rather than accepting any stone whose own
    /// rays happened to look open. `OOX.X.O` is the case that separates the two:
    /// from the right-hand X the gaps look open both ways, but from the leading
    /// X the line is walled in, and it is the leading X that decides.
    fn classify_at(&self, r: i32, c: i32, dr: i32, dc: i32) -> Option<Formation> {
        if !Self::in_bounds(r, c) {
            return None;
        }
        let me = self.get(r, c);
        if me == Cell::Empty {
            return None;
        }
        let minus = self.scan_line_as(me, -1, dr, dc, r, c);
        let plus = self.scan_line_as(me, 1, dr, dc, r, c);
        let kind = classify(&plus, &minus, 1)?;

        // Which stone owns the formation depends on which rule matched, because
        // `classify` does not use every stone its rays saw. The five/open-four
        // rules are decided by the contiguous run alone, so a loose stone
        // further back is irrelevant to them; the rest are decided by the whole
        // stone count, so the first stone the rays reached owns those. Getting
        // this wrong either double-counts a formation or loses it: keying five
        // on the ray's first stone made `X.XXXXX` score zero, because the run's
        // real start was rejected in favour of the detached stone.
        let owner_offset = match kind {
            PatternKind::FiveRow | PatternKind::OpenFour => -minus.contig_my,
            _ => -((32 - minus.stones.leading_zeros()) as i32),
        };
        if owner_offset != 0 {
            return None;
        }

        // Bit k of the mask marks a stone k cells along `dir` from (r,c).
        Some(Formation { color: me, kind, mask: 1 | (plus.stones << 1) })
    }

    /// Report the formation anchored at (r,c) along (dr,dc) to `sink`, once.
    ///
    /// The reported range is the formation's own cells plus one empty cell on
    /// each side where the board has one — enough for the debug/display callers
    /// that want coordinates.
    fn emit_at<S: PatternSink>(&self, r: i32, c: i32, dr: i32, dc: i32, sink: &mut S) {
        let Some(f) = self.classify_at(r, c, dr, dc) else {
            return;
        };

        let span = (32 - f.mask.leading_zeros()) as i32;
        let mut start = (r, c);
        let mut len = span;
        let (br, bc) = (r - dr, c - dc);
        if Self::in_bounds(br, bc) && self.get(br, bc) == Cell::Empty {
            start = (br, bc);
            len += 1;
        }
        let (ar, ac) = (r + dr * span, c + dc * span);
        if Self::in_bounds(ar, ac) && self.get(ar, ac) == Cell::Empty {
            len += 1;
        }

        sink.hit(f.color, f.kind, start, (dr, dc), len);
    }

    /// Every formation on the board, both colors, in one pass.
    fn scan_all<S: PatternSink>(&self, sink: &mut S) {
        for r in 0..19i32 {
            for c in 0..19i32 {
                for &(dr, dc) in &SCAN_DIRS {
                    self.emit_at(r, c, dr, dc, sink);
                }
            }
        }
    }

    /// Every formation whose classification the given changed cells could alter.
    ///
    /// Only anchors on the same line as a changed cell can see it, and only
    /// within `LOCAL_RADIUS`. Diffing this before and after a move gives the
    /// same answer as re-running `scan_all` — the invariant
    /// `incremental_patterns_match_full_rescan_over_random_games` checks.
    ///
    /// The changed cells all sit within two steps of the placed stone, so along
    /// any one scan direction they fall on only a few lines. Cells sharing a
    /// line get one merged sweep instead of one sweep each, which is what stops
    /// the sweeps from overlapping and double-counting a formation.
    fn scan_around<S: PatternSink>(&self, changed: &[(i32, i32)], sink: &mut S) {
        for &(dr, dc) in &SCAN_DIRS {
            // Per line: (line key, a cell on it, that cell's position along the
            // line, and the lowest/highest changed position on the line).
            let mut lines = [(0i32, (0i32, 0i32), 0i32, 0i32, 0i32); MAX_CHANGED_CELLS];
            let mut n = 0usize;

            for &(x, y) in changed {
                // Constant along `dir`, so it names the line (x,y) sits on.
                let key = dc * x - dr * y;
                // Steps by one per step along `dir`, so it orders cells on it.
                let pos = if dr != 0 { x } else { y };
                match lines[..n].iter_mut().find(|l| l.0 == key) {
                    Some(l) => {
                        l.3 = l.3.min(pos);
                        l.4 = l.4.max(pos);
                    }
                    None => {
                        lines[n] = (key, (x, y), pos, pos, pos);
                        n += 1;
                    }
                }
            }

            for &(_, (x0, y0), origin, lo, hi) in &lines[..n] {
                for pos in (lo - LOCAL_RADIUS)..=(hi + LOCAL_RADIUS) {
                    let k = pos - origin;
                    self.emit_at(x0 + dr * k, y0 + dc * k, dr, dc, sink);
                }
            }
        }
    }

    pub(crate) fn sb_local_patterns(&self, changed: &[(i32, i32)]) -> (PatternCounts, PatternCounts) {
        let mut sink = CountSink::default();
        self.scan_around(changed, &mut sink);
        (sink.black, sink.white)
    }

    pub(crate) fn sb_scan_patterns(&self) -> (PatternCounts, PatternCounts) {
        let mut sink = CountSink::default();
        self.scan_all(&mut sink);
        (sink.black, sink.white)
    }

    /// Same scan as `sb_scan_patterns`, but keeping each formation's cells.
    /// Only for the `Gomoku` debug getters — never on a search path.
    pub(crate) fn sb_collect_patterns(&self) -> (PlayerRanges, PlayerRanges) {
        let mut sink = RangeSink::default();
        self.scan_all(&mut sink);
        (sink.black, sink.white)
    }
    pub(crate) fn sb_heuristic_evaluation(&self) -> i32 {
        let black_score = self.black_patterns.score(self.captures[Cell::Black as usize], self.current == Cell::Black);
        let white_score = self.white_patterns.score(self.captures[Cell::White as usize], self.current == Cell::White);
        (black_score - white_score).clamp(-SB_EVAL_CLAMP, SB_EVAL_CLAMP)
    }

    // ---- Candidate move generation ----

    /// Legal moves worth searching, best-first.
    ///
    /// `history` is the caller's move-ordering table, when it has one: moves
    /// that have caused cutoffs elsewhere in this search are tried first, and
    /// the static score only breaks ties between moves with equal history.
    /// `check_legality` filters out double-three moves up front. Interior
    /// search nodes pass `false` and check the few moves they actually search
    /// instead: alpha-beta explores a handful of the candidates offered, so
    /// legality-testing all of them is mostly wasted work. Callers that hand
    /// the list to someone else — the root, the UI — must pass `true`.
    pub(crate) fn get_candidate_moves(
        &mut self,
        depth: usize,
        history: Option<&[u32; 361]>,
        check_legality: bool,
    ) -> Vec<(usize, usize)> {
        // Empty board → center
        if self.total_stones == 0 {
            return vec![(9, 9)];
        }

        // Radius-1 moves around existing stones, dedup with flat array
        let mut seen = [false; 361];
        for r in 0..19usize {
            for c in 0..19usize {
                if self.cells[r][c] != Cell::Empty {
                    let radius = if depth >= DEEP_RADIUS_DEPTH { DEEP_RADIUS } else { RADIUS };
                    let r_start = r.saturating_sub(radius);
                    let r_end = (r + radius + 1).min(19);
                    let c_start = c.saturating_sub(radius);
                    let c_end = (c + radius + 1).min(19);
                    for rr in r_start..r_end {
                        for cc in c_start..c_end {
                            seen[rr * 19 + cc] = true;
                        }
                    }
                }
            }
        }

        // Collect valid moves (empty, not double-three)
        let mut moves: Vec<(usize, usize)> = Vec::with_capacity(64);
        for idx in 0..361 {
            if !seen[idx] { continue; }
            let r = idx / 19;
            let c = idx % 19;
            if self.cells[r][c] != Cell::Empty { continue; }
            if check_legality && self.sb_is_double_three(r as i32, c as i32) { continue; }
            moves.push((r, c));
        }

        // Move ordering: near the root (depth <= SHALLOW_ORDER_DEPTH), play each
        // candidate and score the resulting board with the full-board heuristic
        // used at leaf nodes for the best ordering quality. Deeper nodes fall
        // back to the cheap local evaluate_position, since the full-board scan
        // per candidate is too expensive to afford at every node.
        if depth <= SHALLOW_ORDER_DEPTH {
            let mover = self.current;
            moves.sort_by_cached_key(|&(r, c)| {
                let undo = self.make_move(r, c);
                let score = self.sb_heuristic_evaluation();
                self.undo_move(&undo);
                if mover == Cell::Black { -score } else { score }
            });
        } else {
            let mover = self.current;
            moves.sort_by_cached_key(|&(r, c)| {
                let hist = history.map_or(0, |h| h[r * 19 + c]);
                std::cmp::Reverse((hist, self.evaluate_position(r, c, mover)))
            });
        }

        moves
    }
}

#[cfg(test)]
impl SearchBoard {
    /// Empty 19x19 board, Black to move.
    pub(crate) fn empty() -> SearchBoard {
        SearchBoard {
            cells: [[Cell::Empty; 19]; 19],
            current: Cell::Black,
            opponent: Cell::White,
            captures: [0, 0, 0],
            hash: 0,
            move_count: 0,
            last_move: None,
            total_stones: 0,
            black_patterns: PatternCounts::default(),
            white_patterns: PatternCounts::default(),
        }
    }

    /// Lift a stone off the board outside of a capture.
    ///
    /// No production path does this — `make_move` is the only thing that
    /// removes stones — so there is no incremental delta to apply and the
    /// counts are re-derived with a full rescan.
    pub(crate) fn remove_stone_raw(&mut self, r: usize, c: usize) {
        let stone = self.cells[r][c];
        assert_ne!(stone, Cell::Empty, "no stone to remove at ({r},{c})");
        self.hash ^= zobrist().board[r * 19 + c][stone.zobrist_idx()];
        self.cells[r][c] = Cell::Empty;
        self.total_stones -= 1;
        let (black, white) = self.sb_scan_patterns();
        self.black_patterns = black;
        self.white_patterns = white;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Small self-contained PRNG so the test doesn't need the `rand` crate.
    struct TestRng(u64);
    impl TestRng {
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


    pub(crate) fn assert_patterns_match(board: &SearchBoard, ctx: &str) {
        let (black, white) = board.sb_scan_patterns();
        assert_eq!(board.black_patterns, black, "black patterns mismatch: {ctx}");
        assert_eq!(board.white_patterns, white, "white patterns mismatch: {ctx}");
    }

    /// Incrementally maintained black_patterns/white_patterns must always equal
    /// a from-scratch sb_scan_patterns() — this is the invariant the fast path in
    /// make_move relies on. Random-play both colors, including whatever captures
    /// happen to occur, and check after every move and after undoing every move.
    #[test]
    pub(crate) fn incremental_patterns_match_full_rescan_over_random_games() {
        let mut rng = TestRng(0xC0FFEE_u64);
        let mut captures_seen = 0usize;
        for game in 0..30usize {
            let mut board = SearchBoard::empty();
            let mut undos = Vec::new();

            loop {
                let empties: Vec<(usize, usize)> = (0..19usize)
                    .flat_map(|r| (0..19usize).map(move |c| (r, c)))
                    .filter(|&(r, c)| board.cells[r][c] == Cell::Empty)
                    .collect();
                if empties.is_empty() {
                    break;
                }
                let (r, c) = empties[rng.next_usize(empties.len())];

                let undo = board.make_move(r, c);
                captures_seen += undo.num_captured;
                assert_patterns_match(&board, &format!("game {game} move {} at ({r},{c})", undos.len()));
                undos.push(undo);

                // Real search never calls make_move again once a side has won —
                // is_terminal() is always checked first. Mirror that here since
                // the incremental path assumes the mover never already owns a
                // pre-existing five-in-a-row of their own color.
                if board.get_winner().is_some() {
                    break;
                }
            }

            while let Some(undo) = undos.pop() {
                board.undo_move(&undo);
                assert_patterns_match(&board, &format!("game {game} after undoing move {}", undos.len()));
            }
            assert_eq!(board.black_patterns, PatternCounts::default());
            assert_eq!(board.white_patterns, PatternCounts::default());
        }

        // Captures take the multi-cell path through make_move. If random play
        // ever stops producing them, this test silently stops covering it.
        assert!(
            captures_seen > 50,
            "only {captures_seen} stones captured across all games — the capture path is barely covered"
        );
    }

    /// A run longer than five still counts as a five. The legacy classifier
    /// tested `contig_total == 5` exactly, which left overlines scoring
    /// nothing — no five, and no four either.
    #[test]
    fn overlines_count_as_five() {
        for len in 5..=8usize {
            let mut board = SearchBoard::empty();
            for c in 5..5 + len {
                board.cells[9][c] = Cell::Black;
                board.total_stones += 1;
            }
            let (black, _) = board.sb_scan_patterns();
            assert_eq!(black.five_rows, 1, "run of {len} should count as one five");
            assert!(board.has_five_in_row(Cell::Black), "run of {len} should be a win");
        }
    }

    /// A detached stone further along the line must not hide the run next to
    /// it. `X.XXXXX` used to score zero: the ray from the five's first stone
    /// reached the loose stone, so that anchor was rejected as "not first",
    /// while the loose stone itself classified as nothing.
    #[test]
    fn detached_stone_does_not_hide_a_five() {
        for (label, cols) in [
            ("XXXXX", vec![5, 6, 7, 8, 9]),
            ("X.XXXXX", vec![5, 7, 8, 9, 10, 11]),
            ("XX.XXXXX", vec![5, 6, 8, 9, 10, 11, 12]),
            ("X..XXXXX", vec![5, 8, 9, 10, 11, 12]),
        ] {
            let mut board = SearchBoard::empty();
            for c in cols {
                board.cells[9][c] = Cell::Black;
                board.total_stones += 1;
            }
            let (black, _) = board.sb_scan_patterns();
            assert_eq!(black.five_rows, 1, "{label} should hold exactly one five");
        }
    }

    /// The counterpart: a formation must not be counted once per stone in it.
    /// `OOX.X.O` is the case that pins the rule down — from the right-hand X
    /// the gaps look open both ways, but the line is walled in.
    #[test]
    fn formations_are_counted_once() {
        let mut board = SearchBoard::empty();
        for (c, cell) in [(9, Cell::White), (10, Cell::White), (11, Cell::Black),
                          (13, Cell::Black), (15, Cell::White)] {
            board.cells[9][c] = cell;
            board.total_stones += 1;
        }
        let (black, _) = board.sb_scan_patterns();
        assert_eq!(black.open_twos, 0, "OOX.X.O is walled in, not an open two");
    }

    /// `get_winner` reads `five_rows` instead of sweeping the board, which is
    /// only sound if the maintained count agrees with a real board scan on
    /// every position the search can reach.
    #[test]
    fn five_count_agrees_with_board_scan() {
        let mut rng = TestRng(0x5EED_5EED);
        for game in 0..200usize {
            let mut board = SearchBoard::empty();
            let mut undos = Vec::new();
            for _ in 0..60 {
                let empties: Vec<(usize, usize)> = (0..19usize)
                    .flat_map(|r| (0..19usize).map(move |c| (r, c)))
                    .filter(|&(r, c)| board.cells[r][c] == Cell::Empty)
                    .collect();
                if empties.is_empty() { break; }
                let (r, c) = empties[rng.next_usize(empties.len())];
                undos.push(board.make_move(r, c));
                for player in [Cell::Black, Cell::White] {
                    assert_eq!(
                        board.patterns_of(player).five_rows > 0,
                        board.has_five_in_row(player),
                        "game {game} move {} at ({r},{c}): five count disagrees with board scan for {player:?}",
                        undos.len(),
                    );
                }
                if board.get_winner().is_some() { break; }
            }
        }
    }

    /// `sb_is_double_three` skips its ray scans when a cheap neighbour count
    /// rules the direction out. That shortcut must never change a verdict.
    #[test]
    fn double_three_prefilter_matches_unfiltered() {
        // The same check with no pre-filter and no early exit.
        fn reference(board: &SearchBoard, x0: i32, y0: i32) -> bool {
            let player = board.current;
            let mut count = 0;
            for (dx, dy) in [(1, -1), (1, 0), (1, 1), (0, 1)] {
                let plus = board.scan_line_as(player, 1, dx, dy, x0, y0);
                let minus = board.scan_line_as(player, -1, dx, dy, x0, y0);
                if classify(&plus, &minus, 1) == Some(PatternKind::FreeThree) {
                    count += 1;
                }
            }
            count > 1
        }

        let mut rng = TestRng(0xD3ADB33F);
        for game in 0..60usize {
            let mut board = SearchBoard::empty();
            for _ in 0..50 {
                let empties: Vec<(usize, usize)> = (0..19usize)
                    .flat_map(|r| (0..19usize).map(move |c| (r, c)))
                    .filter(|&(r, c)| board.cells[r][c] == Cell::Empty)
                    .collect();
                if empties.is_empty() { break; }
                for &(r, c) in &empties {
                    assert_eq!(
                        board.sb_is_double_three(r as i32, c as i32),
                        reference(&board, r as i32, c as i32),
                        "game {game}: pre-filter changed the verdict at ({r},{c})",
                    );
                }
                let (r, c) = empties[rng.next_usize(empties.len())];
                board.make_move(r, c);
                if board.get_winner().is_some() { break; }
            }
        }
    }

    /// A stone that captures in several directions at once changes up to nine
    /// cells spread over a 5x5 box. That is the widest the local sweep in
    /// `make_move` ever has to cover, so check it against a full rescan.
    #[test]
    fn multi_direction_capture_matches_full_rescan() {
        for placements in [
            // Two pairs, opposite directions along one row.
            vec![((9, 6), Cell::Black), ((9, 7), Cell::White), ((9, 8), Cell::White),
                 ((9, 10), Cell::White), ((9, 11), Cell::White), ((9, 12), Cell::Black)],
            // Three pairs at once: along the row, the column and a diagonal.
            vec![((9, 6), Cell::Black), ((9, 7), Cell::White), ((9, 8), Cell::White),
                 ((6, 9), Cell::Black), ((7, 9), Cell::White), ((8, 9), Cell::White),
                 ((6, 6), Cell::Black), ((7, 7), Cell::White), ((8, 8), Cell::White)],
            // Four pairs radiating from the landing square — eight stones lifted,
            // the most a single move can change.
            vec![((9, 6), Cell::Black), ((9, 7), Cell::White), ((9, 8), Cell::White),
                 ((9, 12), Cell::Black), ((9, 11), Cell::White), ((9, 10), Cell::White),
                 ((6, 9), Cell::Black), ((7, 9), Cell::White), ((8, 9), Cell::White),
                 ((12, 9), Cell::Black), ((11, 9), Cell::White), ((10, 9), Cell::White)],
        ] {
            let mut board = SearchBoard::empty();
            for ((r, c), cell) in placements {
                board.cells[r][c] = cell;
                board.total_stones += 1;
                board.hash ^= zobrist().board[r * 19 + c][cell.zobrist_idx()];
            }
            let (b, w) = board.sb_scan_patterns();
            board.black_patterns = b;
            board.white_patterns = w;

            board.current = Cell::Black;
            board.opponent = Cell::White;
            let undo = board.make_move(9, 9);
            assert!(undo.num_captured >= 4, "expected a multi-direction capture, got {}", undo.num_captured);
            assert_patterns_match(&board, "after multi-direction capture");

            board.undo_move(&undo);
            assert_patterns_match(&board, "after undoing multi-direction capture");
        }
    }
}

