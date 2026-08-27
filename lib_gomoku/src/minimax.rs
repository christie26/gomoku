use crate::{zobrist, Gomoku, Stone};

use pyo3::prelude::*;
use rayon::prelude::*;

use std::cmp::{max, min};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

// =====================================================================
// SearchBoard: zero-heap make/unmake board for the search
// =====================================================================

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum Cell {
    Empty = 0,
    Black = 1,
    White = 2,
}

impl Cell {
    #[inline]
    fn opponent(self) -> Cell {
        match self {
            Cell::Black => Cell::White,
            Cell::White => Cell::Black,
            Cell::Empty => Cell::Empty,
        }
    }

    #[inline]
    fn zobrist_idx(self) -> usize {
        // Black=0, White=1
        (self as usize) - 1
    }
}

#[derive(Clone)]
struct SearchBoard {
    cells: [[Cell; 19]; 19],
    current: Cell,
    opponent: Cell,
    captures: [i32; 3], // Index by Cell as u8: [unused, Black, White]
    hash: u64,
    move_count: usize,
    last_move: Option<(usize, usize)>,
    total_stones: usize,
}

#[derive(Default)]
struct PatternCounts {
    five_rows: i32,
    open_fours: i32,
    block_fours: i32,
    open_threes: i32,
    open_twos: i32,
    free_threes: i32,
}

impl PatternCounts {
    fn score(&self, captures: i32, is_active: bool) -> i32 {
        let mut score = 0i32;
        score += self.five_rows * 80_001;
        score += self.open_fours * 35_000;
        score += self.block_fours * 7_000;
        score += self.free_threes * 5_000;
        score += self.open_threes * 100;
        score += self.open_twos * 50;

        score += match captures {
            0 => 0,
            1 => 5_000,
            2 => 12_000,
            3 => 25_000,
            4 => 50_000,
            _ => 50_000,
        };

        let total_threes = self.open_threes + self.free_threes;
        if self.open_fours >= 2 { score += 40_000; }
        if self.open_fours >= 1 && self.block_fours >= 1 { score += 35_000; }
        if self.block_fours >= 2 { score += 30_000; }
        if self.open_fours >= 1 && total_threes >= 1 { score += 30_000; }
        if self.block_fours >= 1 && total_threes >= 1 { score += 20_000; }
        if total_threes >= 2 { score += 15_000; }
        if captures >= 4 && (self.block_fours >= 1 || self.open_fours >= 1) { score += 25_000; }

        if is_active {
            if self.open_fours >= 1 { score += 5_000; }
            if self.block_fours >= 1 { score += 3_000; }
            if captures >= 4 { score += 8_000; }
        }

        score
    }
}

struct UndoInfo {
    placed: (usize, usize),
    _placed_player: Cell,
    captured_stones: [(usize, usize); 8], // max 4 captures * 2 stones each = 8
    num_captured: usize,
    old_captures_current: i32,
    old_hash: u64,
    old_move_count: usize,
    old_last_move: Option<(usize, usize)>,
    old_total_stones: usize,
}

const ALL_DIRS: [(i32, i32); 8] = [
    (1, 0), (0, 1), (1, 1), (1, -1),
    (-1, 0), (0, -1), (-1, -1), (-1, 1),
];

impl SearchBoard {
    fn from_gomoku(g: &Gomoku) -> Self {
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
        }
    }

    #[inline]
    fn in_bounds(r: i32, c: i32) -> bool {
        r >= 0 && r < 19 && c >= 0 && c < 19
    }

    #[inline]
    fn get(&self, r: i32, c: i32) -> Cell {
        self.cells[r as usize][c as usize]
    }

    fn make_move(&mut self, r: usize, c: usize) -> UndoInfo {
        let z = zobrist();
        let old_hash = self.hash;
        let old_captures = self.captures[self.current as usize];
        let old_move_count = self.move_count;
        let old_last_move = self.last_move;
        let old_total_stones = self.total_stones;

        // Place stone
        self.cells[r][c] = self.current;
        self.hash ^= z.board[r * 19 + c][self.current.zobrist_idx()];
        self.total_stones += 1;
        self.move_count += 1;
        self.last_move = Some((r, c));

        // Execute captures: scan 8 directions for current-opp-opp-current pattern
        let mut captured_stones = [(0usize, 0usize); 8];
        let mut num_captured = 0usize;
        let ri = r as i32;
        let ci = c as i32;

        for &(dr, dc) in ALL_DIRS.iter() {
            let r1 = ri + dr;
            let c1 = ci + dc;
            let r2 = ri + 2 * dr;
            let c2 = ci + 2 * dc;
            let r3 = ri + 3 * dr;
            let c3 = ci + 3 * dc;

            if !Self::in_bounds(r3, c3) { continue; }
            if self.get(r1, c1) == self.opponent
                && self.get(r2, c2) == self.opponent
                && self.get(r3, c3) == self.current
            {
                // Capture the two opponent stones
                let opp_zi = self.opponent.zobrist_idx();
                let (ur1, uc1) = (r1 as usize, c1 as usize);
                let (ur2, uc2) = (r2 as usize, c2 as usize);
                self.cells[ur1][uc1] = Cell::Empty;
                self.cells[ur2][uc2] = Cell::Empty;
                self.hash ^= z.board[ur1 * 19 + uc1][opp_zi];
                self.hash ^= z.board[ur2 * 19 + uc2][opp_zi];
                captured_stones[num_captured] = (ur1, uc1);
                captured_stones[num_captured + 1] = (ur2, uc2);
                num_captured += 2;
                self.captures[self.current as usize] += 1;
                self.total_stones -= 2;
            }
        }

        // Switch player
        self.hash ^= z.player;
        let tmp = self.current;
        self.current = self.opponent;
        self.opponent = tmp;

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
        }
    }

    fn undo_move(&mut self, info: &UndoInfo) {
        // Swap players back
        let tmp = self.current;
        self.current = self.opponent;
        self.opponent = tmp;

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
    fn count_stones(&self, r: usize, c: usize, dr: i32, dc: i32, player: Cell) -> i32 {
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
    fn can_capture_at(&self, r: usize, c: usize, dr: i32, dc: i32, player: Cell) -> bool {
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
    fn evaluate_position(&self, r: usize, c: usize, player: Cell) -> i32 {
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

    /// Port of Gomoku::count_free_three for SearchBoard
    fn sb_count_free_three(&self, sign: i32, dx: i32, dy: i32, x0: i32, y0: i32, player: Cell) -> (i32, i32, bool) {
        let opp = player.opponent();
        let mut my_count = 0;
        let mut empty_count = 0;
        let mut i = 1i32;
        let mut hole = false;
        loop {
            let x = x0 + dx * i * sign;
            let y = y0 + dy * i * sign;
            if !Self::in_bounds(x, y) || self.get(x, y) == opp || empty_count == 2 {
                break;
            }
            if self.get(x, y) == player {
                if empty_count > 0 { hole = true; }
                my_count += 1;
            } else {
                empty_count += 1;
            }
            i += 1;
        }
        (my_count, empty_count, hole)
    }

    /// Check if placing current player at (x0,y0) creates a double-three
    fn sb_is_double_three(&self, x0: i32, y0: i32) -> bool {
        let dirs: [(i32,i32); 4] = [(1,-1),(1,0),(1,1),(0,1)];
        let player = self.current;
        let mut free_three_count = 0;
        for (dx, dy) in dirs {
            let (plus_my, plus_empty, plus_hole) = self.sb_count_free_three(1, dx, dy, x0, y0, player);
            let (minus_my, minus_empty, minus_hole) = self.sb_count_free_three(-1, dx, dy, x0, y0, player);
            if plus_my + minus_my == 2 && plus_empty + minus_empty >= 3 {
                let mut ok = true;
                if plus_hole && minus_empty < 2 { ok = false; }
                if minus_hole && plus_empty < 2 { ok = false; }
                // Replicate the adjustment logic
                let mut adj_plus_empty = plus_empty;
                let mut adj_minus_empty = minus_empty;
                if plus_hole && minus_empty == 2 { adj_minus_empty = 1; }
                if minus_hole && plus_empty == 2 { adj_plus_empty = 1; }
                let _ = (adj_plus_empty, adj_minus_empty); // used for pattern extent only
                if ok {
                    free_three_count += 1;
                    if free_three_count > 1 { return true; }
                }
            }
        }
        false
    }

    // ---- Terminal detection ----

    /// Check if `player` has 5+ in a row anywhere on the board
    fn has_five_in_row(&self, player: Cell) -> bool {
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
    fn sb_stone_in_capturable_pair(&self, x: i32, y: i32, player: Cell) -> bool {
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
    fn has_uncapturable_five(&self, player: Cell) -> bool {
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
    fn get_winner(&self) -> Option<Cell> {
        // 1. Check if the player who just moved (opponent) has >= 5 captures
        if self.captures[self.opponent as usize] >= 5 {
            return Some(self.opponent);
        }
        // 2. Check if current player (who didn't just move) has a five_row
        //    (the opponent's move might have created it via capture removal — rare but check)
        //    Actually in the original: check opponent_player's five_row first (step 2)
        //    After make_move: self.current was the opponent_player before the move.
        //    The original Gomoku::get_winner checks:
        //      - current_player captures >= 5 (before switch_player was called in make_next_state)
        //      - opponent_player five_row (which is the one who just had their patterns updated)
        //      - current_player uncapturable five
        //    After make_move switches, self.opponent = who just moved = old current_player.
        //    self.current = old opponent_player.
        //
        //    Mapping:
        //    old current_player captures >= 5 → self.opponent captures >= 5 (done above)
        //    old opponent_player five_row → self.current has five_in_row
        if self.has_five_in_row(self.current) {
            return Some(self.current);
        }
        //    old current_player uncapturable five → self.opponent uncapturable five
        if self.has_uncapturable_five(self.opponent) {
            return Some(self.opponent);
        }
        None
    }

    fn is_terminal(&self) -> bool {
        self.get_winner().is_some() || self.total_stones >= 361
    }

    // ---- Heuristic evaluation ----

    /// Scan the board once and tally Black's and White's pattern counts
    /// together, instead of scanning per color, to halve the number of
    /// full-board passes needed for evaluation.
    fn sb_scan_patterns(&self) -> (PatternCounts, PatternCounts) {
        let dirs: [(i32,i32); 4] = [(1,0),(0,1),(1,1),(1,-1)];

        let mut black = PatternCounts::default();
        let mut white = PatternCounts::default();

        // Scan runs: for each occupied cell, scan 4 positive directions.
        // Only count from the start of a run (predecessor != same color).
        for r in 0..19i32 {
            for c in 0..19i32 {
                let cell = self.get(r, c);
                if cell == Cell::Empty { continue; }
                let stats = if cell == Cell::Black { &mut black } else { &mut white };

                for &(dr, dc) in &dirs {
                    let pr = r - dr;
                    let pc = c - dc;
                    if Self::in_bounds(pr, pc) && self.get(pr, pc) == cell { continue; }

                    let mut count = 1i32;
                    let mut nr = r + dr;
                    let mut nc = c + dc;
                    while Self::in_bounds(nr, nc) && self.get(nr, nc) == cell {
                        count += 1;
                        nr += dr;
                        nc += dc;
                    }

                    if count >= 5 {
                        stats.five_rows += 1;
                        continue;
                    }

                    let open_before = Self::in_bounds(pr, pc) && self.get(pr, pc) == Cell::Empty;
                    let open_after = Self::in_bounds(nr, nc) && self.get(nr, nc) == Cell::Empty;

                    match count {
                        4 => {
                            if open_before && open_after {
                                stats.open_fours += 1;
                            } else if open_before || open_after {
                                stats.block_fours += 1;
                            }
                        }
                        3 => {
                            if open_before && open_after {
                                stats.open_threes += 1;
                            }
                        }
                        2 => {
                            if open_before && open_after {
                                stats.open_twos += 1;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        // Free threes: detect gap patterns _X_XX_ and _XX_X_ via sliding
        // 6-cell window, checked for both colors against the same read.
        for r in 0..19i32 {
            for c in 0..19i32 {
                for &(dr, dc) in &dirs {
                    let r5 = r + 5*dr;
                    let c5 = c + 5*dc;
                    if !Self::in_bounds(r5, c5) { continue; }

                    let w = [
                        self.get(r, c),
                        self.get(r + dr, c + dc),
                        self.get(r + 2*dr, c + 2*dc),
                        self.get(r + 3*dr, c + 3*dc),
                        self.get(r + 4*dr, c + 4*dc),
                        self.get(r5, c5),
                    ];
                    if w[0] != Cell::Empty || w[5] != Cell::Empty { continue; }

                    for (color, stats) in [(Cell::Black, &mut black), (Cell::White, &mut white)] {
                        // _X_XX_
                        if w[1] == color && w[2] == Cell::Empty && w[3] == color && w[4] == color {
                            stats.free_threes += 1;
                        }
                        // _XX_X_
                        if w[1] == color && w[2] == color && w[3] == Cell::Empty && w[4] == color {
                            stats.free_threes += 1;
                        }
                    }
                }
            }
        }

        (black, white)
    }

    fn sb_heuristic_evaluation(&self) -> i32 {
        let (black_pat, white_pat) = self.sb_scan_patterns();
        let black_score = black_pat.score(self.captures[Cell::Black as usize], self.current == Cell::Black);
        let white_score = white_pat.score(self.captures[Cell::White as usize], self.current == Cell::White);
        (black_score - white_score).clamp(-99_991, 99_991)
    }

    // ---- Candidate move generation ----

    fn get_candidate_moves(&mut self, depth: usize) -> Vec<(usize, usize)> {
        // Empty board → center
        if self.total_stones == 0 {
            return vec![(9, 9)];
        }

        // Radius-1 moves around existing stones, dedup with flat array
        let mut seen = [false; 361];
        for r in 0..19usize {
            for c in 0..19usize {
                if self.cells[r][c] != Cell::Empty {
                    let r_start = r.saturating_sub(RADIUS);
                    let r_end = (r + RADIUS + 1).min(19);
                    let c_start = c.saturating_sub(RADIUS);
                    let c_end = (c + RADIUS + 1).min(19);
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
            if self.sb_is_double_three(r as i32, c as i32) { continue; }
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
            moves.sort_by_cached_key(|&(r, c)| {
                std::cmp::Reverse(self.evaluate_position(r, c, self.current))
            });
        }

        moves
    }
}

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

const TT_SHARDS: usize = 64;
const TT_SHARD_MASK: u64 = (TT_SHARDS as u64) - 1;

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
}

// Kept alive for the process lifetime so entries survive across turns —
// most sub-positions from one turn's search recur in the next turn's tree.
static GLOBAL_TT: OnceLock<ShardedTT> = OnceLock::new();

fn shared_tt() -> &'static ShardedTT {
    GLOBAL_TT.get_or_init(|| ShardedTT::new(14))
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

const MAX_VALUE: i32 = 100_000;
const MIN_VALUE: i32 = -100_000;
const MAX_DEPTH: usize = 5;
const SHALLOW_ORDER_DEPTH: usize = 1;
const RADIUS : usize = 2;

pub const BOARD_SIZE: usize = 19;

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
fn sb_alphabeta(
    board: &mut SearchBoard,
    mut alpha: i32,
    mut beta: i32,
    is_max_player: bool,
    depth: usize,
    max_depth: usize,
    stats: &mut SearchStats,
    tt: &ShardedTT,
) -> (i32, Vec<(usize, usize)>) {
    stats.nodes_visited += 1;

    if board.is_terminal() {
        return (sb_state_value(board, depth), vec![]);
    }

    if depth == max_depth {
        return (board.sb_heuristic_evaluation(), vec![]);
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
                TTFlag::Exact => return (entry.value, vec![]),
                TTFlag::LowerBound => alpha = max(alpha, entry.value),
                TTFlag::UpperBound => beta = min(beta, entry.value),
            }
            if alpha >= beta {
                return (entry.value, vec![]);
            }
        }
    }

    let mut candidates = board.get_candidate_moves(depth);
    stats.internal_nodes += 1;
    stats.total_children += candidates.len() as u64;

    // Move ordering: put TT best move first
    if let Some(tt_move) = tt_best_move {
        if let Some(pos) = candidates.iter().position(|&m| m == tt_move) {
            candidates.swap(0, pos);
        }
    }

    let mut best_value = if is_max_player { MIN_VALUE - 1 } else { MAX_VALUE + 1 };
    let mut best_move_here: Option<(usize, usize)> = None;
    let mut best_pv: Vec<(usize, usize)> = vec![];
    let mut first = true;

    for &(move_r, move_c) in &candidates {
        stats.children_explored += 1;

        let undo = board.make_move(move_r, move_c);

        let (mut child_val, mut child_pv);
        if first {
            (child_val, child_pv) = sb_alphabeta(
                board, alpha, beta, !is_max_player, depth + 1, max_depth, stats, tt,
            );
            first = false;
        } else if is_max_player {
            (child_val, child_pv) = sb_alphabeta(
                board, alpha, alpha + 1, false, depth + 1, max_depth, stats, tt,
            );
            if child_val > alpha && child_val < beta {
                (child_val, child_pv) = sb_alphabeta(
                    board, alpha, beta, false, depth + 1, max_depth, stats, tt,
                );
            }
        } else {
            (child_val, child_pv) = sb_alphabeta(
                board, beta - 1, beta, true, depth + 1, max_depth, stats, tt,
            );
            if child_val < beta && child_val > alpha {
                (child_val, child_pv) = sb_alphabeta(
                    board, alpha, beta, true, depth + 1, max_depth, stats, tt,
                );
            }
        }

        board.undo_move(&undo);

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
            break;
        }
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

    (best_value, best_pv)
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

    let undo = board.make_move(x, y);

    let mut final_pv = vec![];
    let mut final_value = 0i32;
    for depth in 1..=max_depth {
        let mut stats = SearchStats::new();
        stats.max_depth = depth;
        let (value, child_pv) = sb_alphabeta(
            &mut board, MIN_VALUE, MAX_VALUE, !is_max_player,
            1, depth, &mut stats, tt,
        );
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

    let candidates = board.get_candidate_moves(0);
    let mut all_moves: Vec<(usize, usize, Option<i32>)> = candidates
        .into_iter()
        .map(|(r, c)| (r, c, None))
        .collect();

    let max_depth = if board.move_count < 4 { 3 } else { MAX_DEPTH };

    let mut best_move: Option<(usize, usize, i32)> = None;
    let mut final_stats = SearchStats::new();
    let mut depth_times: Vec<(usize, f64, u64)> = Vec::new();
    let tt = shared_tt();

    for depth in 1..=max_depth {
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
        {
            let m = &mut all_moves[0];
            let move_r = m.0;
            let move_c = m.1;
            stats.children_explored += 1;
            let branch_start = Instant::now();

            let undo = board.make_move(move_r, move_c);
            let (value, child_pv) = sb_alphabeta(
                &mut board, alpha, beta, !is_max_player, 1, depth, &mut stats, tt,
            );
            board.undo_move(&undo);

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

        // ---- Phase 2: Parallel YBWC for the remaining root moves ----
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
            )> = tasks
                .into_par_iter()
                .map(|((r, c), mut child_board)| {
                    let mut child_stats = SearchStats::new();
                    let branch_start = Instant::now();
                    let (v, pv) = if is_max_player {
                        let (mut v, mut pv) = sb_alphabeta(
                            &mut child_board,
                            parent_alpha,
                            parent_alpha + 1,
                            false,
                            1,
                            depth,
                            &mut child_stats,
                            tt,
                        );
                        if v > parent_alpha && v < parent_beta {
                            let r2 = sb_alphabeta(
                                &mut child_board,
                                parent_alpha,
                                parent_beta,
                                false,
                                1,
                                depth,
                                &mut child_stats,
                                tt,
                            );
                            v = r2.0;
                            pv = r2.1;
                        }
                        (v, pv)
                    } else {
                        let (mut v, mut pv) = sb_alphabeta(
                            &mut child_board,
                            parent_beta - 1,
                            parent_beta,
                            true,
                            1,
                            depth,
                            &mut child_stats,
                            tt,
                        );
                        if v < parent_beta && v > parent_alpha {
                            let r2 = sb_alphabeta(
                                &mut child_board,
                                parent_alpha,
                                parent_beta,
                                true,
                                1,
                                depth,
                                &mut child_stats,
                                tt,
                            );
                            v = r2.0;
                            pv = r2.1;
                        }
                        (v, pv)
                    };
                    let branch_elapsed = branch_start.elapsed().as_secs_f64();
                    ((r, c), v, pv, child_stats, branch_elapsed)
                })
                .collect();

            // Merge phase: serial best-update, stats merge, branch_times.
            for ((r, c), value, child_pv, child_stats, branch_elapsed) in results {
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
    (best_move, all_moves, final_stats)
}
