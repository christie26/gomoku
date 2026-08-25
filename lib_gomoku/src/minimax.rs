use crate::MoveResult;
use crate::{zobrist, Gomoku, Stone};

use linked_hash_set::LinkedHashSet;
use pyo3::prelude::*;
use rayon::prelude::*;

use std::cmp::{max, min};
use std::sync::{Arc, Mutex};
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

    fn sb_evaluate_player(&self, player: Cell, is_active: bool) -> i32 {
        let dirs: [(i32,i32); 4] = [(1,0),(0,1),(1,1),(1,-1)];

        let mut five_rows = 0i32;
        let mut open_fours = 0i32;
        let mut block_fours = 0i32;
        let mut open_threes = 0i32;
        let mut open_twos = 0i32;
        let mut free_threes = 0i32;

        // Scan runs: for each cell of `player`, scan 4 positive directions.
        // Only count from the start of a run (predecessor != player).
        for r in 0..19i32 {
            for c in 0..19i32 {
                if self.get(r, c) != player { continue; }

                for &(dr, dc) in &dirs {
                    // Skip if predecessor is same color (not start of run)
                    let pr = r - dr;
                    let pc = c - dc;
                    if Self::in_bounds(pr, pc) && self.get(pr, pc) == player { continue; }

                    // Count consecutive
                    let mut count = 1i32;
                    let mut nr = r + dr;
                    let mut nc = c + dc;
                    while Self::in_bounds(nr, nc) && self.get(nr, nc) == player {
                        count += 1;
                        nr += dr;
                        nc += dc;
                    }

                    if count >= 5 {
                        five_rows += 1;
                        continue;
                    }

                    // Check openness: before the run and after the run
                    let open_before = Self::in_bounds(pr, pc) && self.get(pr, pc) == Cell::Empty;
                    let open_after = Self::in_bounds(nr, nc) && self.get(nr, nc) == Cell::Empty;

                    match count {
                        4 => {
                            if open_before && open_after {
                                open_fours += 1;
                            } else if open_before || open_after {
                                block_fours += 1;
                            }
                        }
                        3 => {
                            if open_before && open_after {
                                open_threes += 1;
                            }
                        }
                        2 => {
                            if open_before && open_after {
                                open_twos += 1;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        // Free threes: detect gap patterns _X_XX_ and _XX_X_ via sliding 6-cell window
        for r in 0..19i32 {
            for c in 0..19i32 {
                for &(dr, dc) in &dirs {
                    // Check 6-cell window starting at (r,c)
                    let r5 = r + 5*dr;
                    let c5 = c + 5*dc;
                    if !Self::in_bounds(r5, c5) { continue; }
                    // Only check in positive directions from valid starts
                    if r < 0 || c < 0 { continue; }

                    let g = |i: i32| -> Cell {
                        self.get(r + i*dr, c + i*dc)
                    };

                    // Pattern _X_XX_: cells [empty, player, empty, player, player, empty]
                    if g(0) == Cell::Empty && g(1) == player && g(2) == Cell::Empty
                        && g(3) == player && g(4) == player && g(5) == Cell::Empty
                    {
                        free_threes += 1;
                    }
                    // Pattern _XX_X_: cells [empty, player, player, empty, player, empty]
                    if g(0) == Cell::Empty && g(1) == player && g(2) == player
                        && g(3) == Cell::Empty && g(4) == player && g(5) == Cell::Empty
                    {
                        free_threes += 1;
                    }
                }
            }
        }

        let captures = self.captures[player as usize];

        let mut score = 0i32;
        score += five_rows * 80_001;
        score += open_fours * 35_000;
        score += block_fours * 7_000;
        score += free_threes * 5_000;
        score += open_threes * 100;
        score += open_twos * 50;

        score += match captures {
            0 => 0,
            1 => 5_000,
            2 => 12_000,
            3 => 25_000,
            4 => 50_000,
            _ => 50_000,
        };

        let total_threes = open_threes + free_threes;
        if open_fours >= 2 { score += 40_000; }
        if open_fours >= 1 && block_fours >= 1 { score += 35_000; }
        if block_fours >= 2 { score += 30_000; }
        if open_fours >= 1 && total_threes >= 1 { score += 30_000; }
        if block_fours >= 1 && total_threes >= 1 { score += 20_000; }
        if total_threes >= 2 { score += 15_000; }
        if captures >= 4 && (block_fours >= 1 || open_fours >= 1) { score += 25_000; }

        if is_active {
            if open_fours >= 1 { score += 5_000; }
            if block_fours >= 1 { score += 3_000; }
            if captures >= 4 { score += 8_000; }
        }

        score
    }

    fn sb_heuristic_evaluation(&self) -> i32 {
        let black_score = self.sb_evaluate_player(Cell::Black, self.current == Cell::Black);
        let white_score = self.sb_evaluate_player(Cell::White, self.current == Cell::White);
        (black_score - white_score).clamp(-99_991, 99_991)
    }

    // ---- Candidate move generation ----

    fn get_candidate_moves(&self) -> Vec<(usize, usize)> {
        // Empty board → center
        if self.total_stones == 0 {
            return vec![(9, 9)];
        }

        // Radius-1 moves around existing stones, dedup with flat array
        let mut seen = [false; 361];
        for r in 0..19usize {
            for c in 0..19usize {
                if self.cells[r][c] != Cell::Empty {
                    let r_start = r.saturating_sub(1);
                    let r_end = (r + 2).min(19);
                    let c_start = c.saturating_sub(1);
                    let c_end = (c + 2).min(19);
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

        // Sort by move ordering heuristic
        moves.sort_by_cached_key(|&(r, c)| {
            std::cmp::Reverse(self.evaluate_position(r, c, self.current))
        });

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
const RADIUS : usize = 2;

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
  // TODO - add order of best move sort
    for player in [&state.opponent_player, &state.current_player] {
        let p = state.patterns.get(player).unwrap();
        for (pattern_type, patterns) in [
            ("block_four", &p.block_four),
            ("open_four", &p.open_four),
            ("open_three", &p.open_three),
            ("open_two", &p.open_two),
            ("free_three", &p.free_three),
        ] {
            for pattern in patterns {
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
pub fn get_candidate_moves(state: &Gomoku, radius: usize) -> Vec<(usize, usize)> {
    if state.count_empty_spots() as usize == state.size * state.size {
        return vec![(state.size / 2, state.size / 2)];
    }

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

fn make_next_state(state: &Gomoku, move_x: usize, move_y: usize) -> Gomoku {
    let mut new_state = state.clone_gomoku();
    new_state.handle_move(move_x.try_into().unwrap(), move_y.try_into().unwrap());
    new_state.switch_player();
    new_state
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

    let mut candidates = board.get_candidate_moves();
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
    let tt = ShardedTT::new(14);

    let undo = board.make_move(x, y);

    let mut final_pv = vec![];
    let mut final_value = 0i32;
    for depth in 1..=max_depth {
        let mut stats = SearchStats::new();
        stats.max_depth = depth;
        let (value, child_pv) = sb_alphabeta(
            &mut board, MIN_VALUE, MAX_VALUE, !is_max_player,
            1, depth, &mut stats, &tt,
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

    let candidates = board.get_candidate_moves();
    let mut all_moves: Vec<(usize, usize, Option<i32>)> = candidates
        .into_iter()
        .map(|(r, c)| (r, c, None))
        .collect();

    let max_depth = if board.move_count < 4 { 3 } else { MAX_DEPTH };

    let mut best_move: Option<(usize, usize, i32)> = None;
    let mut final_stats = SearchStats::new();
    let mut depth_times: Vec<(usize, f64, u64)> = Vec::new();
    let tt = ShardedTT::new(14);

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
                &mut board, alpha, beta, !is_max_player, 1, depth, &mut stats, &tt,
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
                            &tt,
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
                                &tt,
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
                            &tt,
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
                                &tt,
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
