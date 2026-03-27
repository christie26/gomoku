use crate::{Gomoku, Stone};
use std::sync::OnceLock;

pub const BOARD_SIZE: usize = 19;
pub const TOTAL_CELLS: usize = BOARD_SIZE * BOARD_SIZE; // 361

const DIRECTIONS: [(i32, i32); 4] = [(1, 0), (0, 1), (1, 1), (1, -1)];
const ALL_DIRS: [(i32, i32); 8] = [
    (1, 0),
    (0, 1),
    (1, 1),
    (1, -1),
    (-1, 0),
    (0, -1),
    (-1, -1),
    (-1, 1),
];

// ---- Cell ----

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Cell {
    Empty = 0,
    Black = 1,
    White = 2,
}

impl Cell {
    #[inline]
    pub fn opponent(self) -> Cell {
        match self {
            Cell::Black => Cell::White,
            Cell::White => Cell::Black,
            Cell::Empty => Cell::Empty,
        }
    }

    #[inline]
    fn zobrist_index(self) -> usize {
        match self {
            Cell::Black => 0,
            Cell::White => 1,
            Cell::Empty => unreachable!(),
        }
    }
}

// ---- Zobrist ----

pub struct ZobristTable {
    pub piece_keys: [[[u64; 2]; BOARD_SIZE]; BOARD_SIZE],
    pub capture_keys: [[u64; 5]; 2],
    pub side_to_move: u64,
}

fn xorshift64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

impl ZobristTable {
    fn new() -> Self {
        let mut rng: u64 = 0x12345678_DEADBEEF;
        let mut table = ZobristTable {
            piece_keys: [[[0u64; 2]; BOARD_SIZE]; BOARD_SIZE],
            capture_keys: [[0u64; 5]; 2],
            side_to_move: 0,
        };
        for row in 0..BOARD_SIZE {
            for col in 0..BOARD_SIZE {
                for stone in 0..2 {
                    table.piece_keys[row][col][stone] = xorshift64(&mut rng);
                }
            }
        }
        for player in 0..2 {
            for count in 0..5 {
                table.capture_keys[player][count] = xorshift64(&mut rng);
            }
        }
        table.side_to_move = xorshift64(&mut rng);
        table
    }
}

pub static ZOBRIST: OnceLock<ZobristTable> = OnceLock::new();

pub fn zobrist() -> &'static ZobristTable {
    ZOBRIST.get_or_init(ZobristTable::new)
}

// ---- SearchState ----

#[derive(Clone)]
pub struct SearchState {
    pub cells: [Cell; TOTAL_CELLS],
    pub black_captures: u8,
    pub white_captures: u8,
    pub is_black_turn: bool,
    pub last_move: Option<(u8, u8)>,
    pub zobrist_hash: u64,
    pub move_count: u16,
}

pub struct UndoInfo {
    pub placed: (u8, u8),
    pub captured: [(u8, u8); 8],
    pub num_captured: u8,
    pub prev_hash: u64,
    pub prev_last_move: Option<(u8, u8)>,
    pub prev_captures: u8,
}

#[inline]
fn idx(row: usize, col: usize) -> usize {
    row * BOARD_SIZE + col
}

#[inline]
fn in_bounds(r: i32, c: i32) -> bool {
    r >= 0 && r < BOARD_SIZE as i32 && c >= 0 && c < BOARD_SIZE as i32
}

impl SearchState {
    pub fn from_gomoku(state: &Gomoku) -> SearchState {
        let mut cells = [Cell::Empty; TOTAL_CELLS];
        for row in 0..BOARD_SIZE {
            for col in 0..BOARD_SIZE {
                cells[idx(row, col)] = match state.board[row][col] {
                    Stone::Black => Cell::Black,
                    Stone::White => Cell::White,
                    Stone::Empty => Cell::Empty,
                };
            }
        }

        let black_caps = *state.capture_count.get(&Stone::Black).unwrap_or(&0) as u8;
        let white_caps = *state.capture_count.get(&Stone::White).unwrap_or(&0) as u8;
        let is_black_turn = state.current_player == Stone::Black;
        let last_move = state.current_move.map(|(r, c)| (r as u8, c as u8));

        let hash = Self::compute_full_hash(&cells, black_caps, white_caps, is_black_turn);

        SearchState {
            cells,
            black_captures: black_caps,
            white_captures: white_caps,
            is_black_turn,
            last_move,
            zobrist_hash: hash,
            move_count: state.move_count as u16,
        }
    }

    fn compute_full_hash(
        cells: &[Cell; TOTAL_CELLS],
        black_caps: u8,
        white_caps: u8,
        is_black_turn: bool,
    ) -> u64 {
        let z = zobrist();
        let mut hash: u64 = 0;
        for row in 0..BOARD_SIZE {
            for col in 0..BOARD_SIZE {
                let c = cells[idx(row, col)];
                if c != Cell::Empty {
                    hash ^= z.piece_keys[row][col][c.zobrist_index()];
                }
            }
        }
        let bc = (black_caps as usize).min(4);
        let wc = (white_caps as usize).min(4);
        hash ^= z.capture_keys[0][bc];
        hash ^= z.capture_keys[1][wc];
        if !is_black_turn {
            hash ^= z.side_to_move;
        }
        hash
    }

    #[inline]
    pub fn current_player(&self) -> Cell {
        if self.is_black_turn {
            Cell::Black
        } else {
            Cell::White
        }
    }

    #[inline]
    pub fn opponent_player(&self) -> Cell {
        if self.is_black_turn {
            Cell::White
        } else {
            Cell::Black
        }
    }

    #[inline]
    fn get(&self, r: i32, c: i32) -> Cell {
        self.cells[idx(r as usize, c as usize)]
    }

    #[inline]
    pub fn captures(&self, is_black: bool) -> u8 {
        if is_black {
            self.black_captures
        } else {
            self.white_captures
        }
    }

    #[inline]
    fn captures_mut(&mut self, is_black: bool) -> &mut u8 {
        if is_black {
            &mut self.black_captures
        } else {
            &mut self.white_captures
        }
    }

    // ---- make_move / unmake_move ----

    pub fn make_move(&mut self, row: u8, col: u8) -> UndoInfo {
        let z = zobrist();
        let r = row as usize;
        let c = col as usize;
        let me = self.current_player();
        let opp = self.opponent_player();
        let is_black = self.is_black_turn;

        let prev_hash = self.zobrist_hash;
        let prev_last_move = self.last_move;
        let prev_captures = self.captures(is_black);

        // Place stone
        self.cells[idx(r, c)] = me;
        self.zobrist_hash ^= z.piece_keys[r][c][me.zobrist_index()];

        // Check captures in 8 directions
        let mut captured = [(0u8, 0u8); 8];
        let mut num_captured: u8 = 0;

        for &(dx, dy) in &ALL_DIRS {
            let r1 = row as i32 + dx;
            let c1 = col as i32 + dy;
            let r2 = row as i32 + 2 * dx;
            let c2 = col as i32 + 2 * dy;
            let r3 = row as i32 + 3 * dx;
            let c3 = col as i32 + 3 * dy;

            if !in_bounds(r3, c3) {
                continue;
            }
            if !in_bounds(r2, c2) || !in_bounds(r1, c1) {
                continue;
            }

            if self.get(r1, c1) == opp
                && self.get(r2, c2) == opp
                && self.get(r3, c3) == me
            {
                // Capture the pair at (r1,c1) and (r2,c2)
                let (ur1, uc1) = (r1 as usize, c1 as usize);
                let (ur2, uc2) = (r2 as usize, c2 as usize);

                self.cells[idx(ur1, uc1)] = Cell::Empty;
                self.cells[idx(ur2, uc2)] = Cell::Empty;
                self.zobrist_hash ^= z.piece_keys[ur1][uc1][opp.zobrist_index()];
                self.zobrist_hash ^= z.piece_keys[ur2][uc2][opp.zobrist_index()];

                captured[num_captured as usize] = (r1 as u8, c1 as u8);
                captured[num_captured as usize + 1] = (r2 as u8, c2 as u8);
                num_captured += 2;
            }
        }

        // Update capture count
        let cap_pairs = num_captured / 2;
        if cap_pairs > 0 {
            // XOR out old capture key, XOR in new
            let old_cap = (self.captures(is_black) as usize).min(4);
            let player_idx = if is_black { 0 } else { 1 };
            self.zobrist_hash ^= z.capture_keys[player_idx][old_cap];

            *self.captures_mut(is_black) += cap_pairs;

            let new_cap = (self.captures(is_black) as usize).min(4);
            self.zobrist_hash ^= z.capture_keys[player_idx][new_cap];
        }

        // Switch side
        self.zobrist_hash ^= z.side_to_move;
        self.is_black_turn = !self.is_black_turn;
        self.last_move = Some((row, col));
        self.move_count += 1;

        UndoInfo {
            placed: (row, col),
            captured,
            num_captured,
            prev_hash,
            prev_last_move,
            prev_captures,
        }
    }

    pub fn unmake_move(&mut self, undo: UndoInfo) {
        // Switch player back
        self.is_black_turn = !self.is_black_turn;
        let is_black = self.is_black_turn;
        let opp = self.opponent_player();

        // Restore capture count
        *self.captures_mut(is_black) = undo.prev_captures;

        // Restore captured stones
        for i in 0..undo.num_captured as usize {
            let (cr, cc) = undo.captured[i];
            self.cells[idx(cr as usize, cc as usize)] = opp;
        }

        // Remove placed stone
        let (pr, pc) = undo.placed;
        self.cells[idx(pr as usize, pc as usize)] = Cell::Empty;

        // Restore hash and last_move
        self.zobrist_hash = undo.prev_hash;
        self.last_move = undo.prev_last_move;
        self.move_count -= 1;
    }

    // ---- is_valid_move ----

    pub fn is_valid_move(&self, row: i32, col: i32) -> bool {
        if !in_bounds(row, col) {
            return false;
        }
        if self.cells[idx(row as usize, col as usize)] != Cell::Empty {
            return false;
        }
        !self.is_double_three_move(row, col)
    }

    fn is_double_three_move(&self, row: i32, col: i32) -> bool {
        let mut free_three_count = 0;
        let me = self.current_player();
        let opp = self.opponent_player();

        for &(dx, dy) in &DIRECTIONS {
            let (plus_my, plus_empty, plus_hole) =
                self.count_free_three_dir(1, dx, dy, row, col, me, opp);
            let (minus_my, minus_empty, minus_hole) =
                self.count_free_three_dir(-1, dx, dy, row, col, me, opp);

            if plus_my + minus_my == 2 && plus_empty + minus_empty >= 3 {
                let mut adj_plus_empty = plus_empty;
                let mut adj_minus_empty = minus_empty;

                if plus_hole && minus_empty == 2 {
                    adj_minus_empty = 1;
                }
                if minus_hole && plus_empty == 2 {
                    adj_plus_empty = 1;
                }

                let _ = adj_plus_empty + plus_my;
                let _ = adj_minus_empty + minus_my;

                free_three_count += 1;
                if free_three_count > 1 {
                    return true;
                }
            }
        }
        false
    }

    fn count_free_three_dir(
        &self,
        sign: i32,
        dx: i32,
        dy: i32,
        x0: i32,
        y0: i32,
        me: Cell,
        opp: Cell,
    ) -> (i32, i32, bool) {
        let mut my_count = 0;
        let mut empty_count = 0;
        let mut hole = false;
        let mut i = 1;

        loop {
            let x = x0 + dx * i * sign;
            let y = y0 + dy * i * sign;

            if !in_bounds(x, y) || self.get(x, y) == opp || empty_count == 2 {
                break;
            }

            if self.get(x, y) == me {
                if empty_count > 0 {
                    hole = true;
                }
                my_count += 1;
            } else {
                empty_count += 1;
            }
            i += 1;
        }
        (my_count, empty_count, hole)
    }

    // ---- get_winner ----
    // Called AFTER make_move + player switch.
    // "current_player" is now the NEXT player to move.
    // The player who JUST moved is self.opponent_player() (the one whose move is self.last_move).

    pub fn get_winner(&self) -> Option<bool> {
        let (lrow, lcol) = self.last_move?;
        let just_moved = self.opponent_player();
        let just_moved_is_black = !self.is_black_turn;

        // 1. Check if just-moved player's captures >= 5
        if self.captures(just_moved_is_black) >= 5 {
            return Some(just_moved_is_black); // true = black wins
        }

        // 2. Check opponent's (current player's) five-in-a-row
        //    Scan from last_move for the opponent — but actually the opponent of just_moved
        //    is self.current_player(). We need to check if the CURRENT player (next to move)
        //    has a five-in-a-row (formed before, by previous moves).
        //    Actually: the pattern from lib.rs checks opponent_player five_row (opponent = the one who DIDN'T just move)
        //    In the original code, after handle_move + switch_player:
        //      - current_player = next to move
        //      - opponent_player = just moved
        //    And it checks opponent_player (just moved) first for captures, then
        //    checks "opponent" five_row which is actually the CURRENT player's fives.
        //    Wait, re-reading lib.rs:
        //      step 1: current_player captures >= 5 → current wins
        //      step 2: opponent's five_row → opponent wins
        //      step 3: current's five_row (non-capturable) → current wins
        //    But in lib.rs, AFTER handle_move + switch_player:
        //      current_player = NEXT to move
        //      opponent_player = JUST moved
        //    So "current" = next-to-move, "opponent" = just-moved.
        //    Step 1: check current(next-to-move) captures >= 5 → current wins
        //
        //    Wait that doesn't match either. Let me re-read more carefully.
        //    In lib.rs get_winner:
        //      - It checks self.current_player captures
        //      - It checks self.opponent_player five_row
        //      - It checks self.current_player five_row
        //    But this is called AFTER switch_player, so:
        //      current = next-to-move (NOT the one who just moved)
        //      opponent = just-moved
        //
        //    Hmm, but captures are incremented in handle_move BEFORE switch_player.
        //    So after handle_move, current_player = just-moved, capture_count is updated.
        //    After switch_player, current_player = next-to-move.
        //    So checking current_player captures after switch = checking next-to-move captures,
        //    which weren't changed. That seems wrong...
        //
        //    Actually no — looking at the original minimax flow:
        //      make_next_state: clone → handle_move → switch_player
        //    In handle_move, current_player places the stone and captures.
        //    After switch_player, current_player is now the next player.
        //    So get_winner() checks:
        //      1. current_player (= next-to-move) captures >= 5
        //         This is WRONG in the original unless it's checking the PREVIOUSLY accumulated captures.
        //         Actually wait — the captures were done by the just-moved player, stored under their stone color.
        //         After switch, current_player changed, so capture_count.get(&current_player) would get
        //         the NEXT player's captures, not the one who just moved!
        //
        //    Looking more carefully at the original code:
        //      capture_center increments capture_count for self.current_player (= just-moved, before switch)
        //      After switch_player, current_player = next-to-move
        //      get_winner checks current_player captures = next-to-move captures
        //
        //    This means the original checks if the NEXT player already had >= 5 captures from
        //    THEIR previous turns, not the just-moved player. That seems intentional:
        //    the just-moved player's captures were checked implicitly because they accumulated
        //    over their turns.
        //
        //    Actually I think both players' captures should be checked. But the original only
        //    checks current_player. Let me just faithfully port the semantics:
        //      1. current_player captures >= 5 → current wins (current = next-to-move = self.current_player())
        //      2. opponent (= just-moved) five-in-a-row → opponent wins
        //      3. current five-in-a-row (non-capturable) → current wins

        // Step 1 (port): current_player (next to move) captures >= 5
        let current_is_black = self.is_black_turn;
        if self.captures(current_is_black) >= 5 {
            return Some(current_is_black);
        }

        // Step 2: opponent (just-moved) five-in-a-row from last_move
        if self.has_five_in_a_row_from(lrow as i32, lcol as i32, just_moved) {
            return Some(just_moved_is_black);
        }

        // Step 3: current player five-in-a-row (non-capturable) — full board scan
        // This checks fives that existed from before (not from this move)
        let current = self.current_player();
        if self.has_uncapturable_five(current) {
            return Some(current_is_black);
        }

        None
    }

    fn has_five_in_a_row_from(&self, row: i32, col: i32, player: Cell) -> bool {
        for &(dx, dy) in &DIRECTIONS {
            let mut count = 1;
            for sign in &[1i32, -1] {
                let mut i = 1;
                loop {
                    let r = row + dx * i * sign;
                    let c = col + dy * i * sign;
                    if !in_bounds(r, c) || self.get(r, c) != player {
                        break;
                    }
                    count += 1;
                    i += 1;
                }
            }
            if count >= 5 {
                return true;
            }
        }
        false
    }

    fn has_uncapturable_five(&self, player: Cell) -> bool {
        // Scan all cells for five-in-a-rows of `player`
        for row in 0..BOARD_SIZE as i32 {
            for col in 0..BOARD_SIZE as i32 {
                if self.get(row, col) != player {
                    continue;
                }
                // Only check rightward/downward directions to avoid double-counting
                for &(dx, dy) in &DIRECTIONS {
                    let mut count = 0;
                    let mut all_uncapturable = true;
                    for i in 0..5 {
                        let r = row + dx * i;
                        let c = col + dy * i;
                        if !in_bounds(r, c) || self.get(r, c) != player {
                            all_uncapturable = false;
                            break;
                        }
                        count += 1;
                        if self.stone_in_capturable_pair(r, c, player) {
                            all_uncapturable = false;
                            break;
                        }
                    }
                    if count == 5 && all_uncapturable {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn stone_in_capturable_pair(&self, x: i32, y: i32, player: Cell) -> bool {
        let opp = player.opponent();
        for &(dx, dy) in &DIRECTIONS {
            // Check (x,y)-(x+dx,y+dy) as a pair
            let nx = x + dx;
            let ny = y + dy;
            if in_bounds(nx, ny) && self.get(nx, ny) == player {
                if self.is_pair_capturable(x, y, nx, ny, player, opp) {
                    return true;
                }
            }
            // Check (x-dx,y-dy)-(x,y) as a pair
            let px = x - dx;
            let py = y - dy;
            if in_bounds(px, py) && self.get(px, py) == player {
                if self.is_pair_capturable(px, py, x, y, player, opp) {
                    return true;
                }
            }
        }
        false
    }

    fn is_pair_capturable(
        &self,
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        _player: Cell,
        opp: Cell,
    ) -> bool {
        let dx = (x1 - x0).clamp(-1, 1);
        let dy = (y1 - y0).clamp(-1, 1);

        let lx = x0 - dx;
        let ly = y0 - dy;
        let rx = x1 + dx;
        let ry = y1 + dy;

        // Pattern A: OPP - P0 - P1 - EMPTY
        let pattern_a = in_bounds(lx, ly)
            && self.get(lx, ly) == opp
            && in_bounds(rx, ry)
            && self.get(rx, ry) == Cell::Empty;

        // Pattern B: EMPTY - P0 - P1 - OPP
        let pattern_b = in_bounds(lx, ly)
            && self.get(lx, ly) == Cell::Empty
            && in_bounds(rx, ry)
            && self.get(rx, ry) == opp;

        pattern_a || pattern_b
    }

    // ---- is_draw ----

    pub fn is_draw(&self) -> bool {
        self.cells.iter().all(|c| *c != Cell::Empty)
    }

    // ---- Terminal check (combined) ----
    // Returns Some(score) if terminal, None otherwise.
    // Score: positive = black advantage, negative = white advantage.

    pub fn terminal_value(&self) -> Option<i32> {
        if let Some(winner_is_black) = self.get_winner() {
            if winner_is_black {
                return Some(100_000);
            } else {
                return Some(-100_000);
            }
        }
        if self.is_draw() {
            return Some(0);
        }
        None
    }

    // ---- Heuristic evaluation ----
    // Line-scanning pattern counter over all 112 lines (19 rows + 19 cols + 37 diags + 37 anti-diags)

    pub fn heuristic_eval(&self) -> i32 {
        let mut black_counts = PatternCounts::default();
        let mut white_counts = PatternCounts::default();

        // Scan rows
        for row in 0..BOARD_SIZE {
            self.scan_line_consecutive(row, 0, 0, 1, BOARD_SIZE, &mut black_counts, &mut white_counts);
        }
        // Scan columns
        for col in 0..BOARD_SIZE {
            self.scan_line_consecutive(0, col, 1, 0, BOARD_SIZE, &mut black_counts, &mut white_counts);
        }
        // Scan diagonals (top-left to bottom-right)
        for start in 0..BOARD_SIZE {
            let len = BOARD_SIZE - start;
            // Top edge going down-right
            self.scan_line_consecutive(0, start, 1, 1, len, &mut black_counts, &mut white_counts);
            // Left edge going down-right (skip main diagonal)
            if start > 0 {
                self.scan_line_consecutive(start, 0, 1, 1, len, &mut black_counts, &mut white_counts);
            }
        }
        // Scan anti-diagonals (top-right to bottom-left)
        for start in 0..BOARD_SIZE {
            let len = start + 1;
            // Top edge going down-left
            self.scan_line_consecutive(0, start, 1, -1, len, &mut black_counts, &mut white_counts);
            // Right edge going down-left (skip corner)
            if start < BOARD_SIZE - 1 {
                let len = BOARD_SIZE - start;
                self.scan_line_consecutive(start, BOARD_SIZE - 1, 1, -1, len, &mut black_counts, &mut white_counts);
            }
        }

        // Also scan for gap patterns (free threes with gap)
        self.scan_gap_patterns(&mut black_counts, &mut white_counts);

        let black_score = self.compute_score(&black_counts, true);
        let white_score = self.compute_score(&white_counts, false);

        // Vulnerability penalty: penalize sides with capturable pairs
        let black_vuln = self.count_vulnerable_pairs(true);
        let white_vuln = self.count_vulnerable_pairs(false);
        let black_vuln_penalty = black_vuln * match self.white_captures {
            n if n >= 3 => 15_000,
            2 => 8_000,
            1 => 4_000,
            _ => 2_000,
        } as i32;
        let white_vuln_penalty = white_vuln * match self.black_captures {
            n if n >= 3 => 15_000,
            2 => 8_000,
            1 => 4_000,
            _ => 2_000,
        } as i32;

        (black_score - black_vuln_penalty) - (white_score - white_vuln_penalty)
    }

    fn scan_line_consecutive(
        &self,
        start_row: usize,
        start_col: usize,
        dr: i32,
        dc: i32,
        length: usize,
        black: &mut PatternCounts,
        white: &mut PatternCounts,
    ) {
        if length < 5 {
            return;
        }

        // Collect cells along this line
        let mut line = Vec::with_capacity(length);
        let mut r = start_row as i32;
        let mut c = start_col as i32;
        for _ in 0..length {
            line.push(self.cells[idx(r as usize, c as usize)]);
            r += dr;
            c += dc;
        }

        // Find consecutive runs
        let mut i = 0;
        while i < line.len() {
            if line[i] == Cell::Empty {
                i += 1;
                continue;
            }
            let player = line[i];
            let run_start = i;
            while i < line.len() && line[i] == player {
                i += 1;
            }
            let run_len = i - run_start;

            // Check open ends
            let open_left = run_start > 0 && line[run_start - 1] == Cell::Empty;
            let open_right = i < line.len() && line[i] == Cell::Empty;

            let counts = if player == Cell::Black { &mut *black } else { &mut *white };

            match run_len {
                n if n >= 5 => counts.five_row += 1,
                4 => {
                    if open_left && open_right {
                        counts.open_four += 1;
                    } else if open_left || open_right {
                        counts.block_four += 1;
                    }
                }
                3 => {
                    if open_left && open_right {
                        counts.open_three += 1;
                    }
                }
                2 => {
                    if open_left && open_right {
                        counts.open_two += 1;
                    }
                }
                _ => {}
            }
        }
    }

    fn scan_gap_patterns(
        &self,
        black: &mut PatternCounts,
        white: &mut PatternCounts,
    ) {
        // Scan for gap patterns like _X_XX_, _XX_X_ (free threes with a gap)
        // These are "free_three" patterns that aren't caught by consecutive run scanning

        // Scan rows
        for row in 0..BOARD_SIZE {
            self.scan_line_gaps(row, 0, 0, 1, BOARD_SIZE, black, white);
        }
        // Scan columns
        for col in 0..BOARD_SIZE {
            self.scan_line_gaps(0, col, 1, 0, BOARD_SIZE, black, white);
        }
        // Scan diagonals
        for start in 0..BOARD_SIZE {
            let len = BOARD_SIZE - start;
            self.scan_line_gaps(0, start, 1, 1, len, black, white);
            if start > 0 {
                self.scan_line_gaps(start, 0, 1, 1, len, black, white);
            }
        }
        // Scan anti-diagonals
        for start in 0..BOARD_SIZE {
            let len = start + 1;
            self.scan_line_gaps(0, start, 1, -1, len, black, white);
            if start < BOARD_SIZE - 1 {
                let len = BOARD_SIZE - start;
                self.scan_line_gaps(start, BOARD_SIZE - 1, 1, -1, len, black, white);
            }
        }
    }

    fn scan_line_gaps(
        &self,
        start_row: usize,
        start_col: usize,
        dr: i32,
        dc: i32,
        length: usize,
        black: &mut PatternCounts,
        white: &mut PatternCounts,
    ) {
        if length < 6 {
            return;
        }

        let mut line = Vec::with_capacity(length);
        let mut r = start_row as i32;
        let mut c = start_col as i32;
        for _ in 0..length {
            line.push(self.cells[idx(r as usize, c as usize)]);
            r += dr;
            c += dc;
        }

        // Sliding window of 6 to find _X_XX_ and _XX_X_ patterns
        for i in 0..length.saturating_sub(5) {
            let w = &line[i..i + 6];

            // Pattern _X_XX_: Empty, Player, Empty, Player, Player, Empty
            if w[0] == Cell::Empty && w[5] == Cell::Empty && w[2] == Cell::Empty {
                if w[1] != Cell::Empty && w[1] == w[3] && w[1] == w[4] {
                    let counts = if w[1] == Cell::Black { &mut *black } else { &mut *white };
                    counts.free_three += 1;
                }
            }
            // Pattern _XX_X_: Empty, Player, Player, Empty, Player, Empty
            if w[0] == Cell::Empty && w[5] == Cell::Empty && w[3] == Cell::Empty {
                if w[1] != Cell::Empty && w[1] == w[2] && w[1] == w[4] {
                    let counts = if w[1] == Cell::Black { &mut *black } else { &mut *white };
                    counts.free_three += 1;
                }
            }
        }
    }

    fn compute_score(&self, counts: &PatternCounts, is_black: bool) -> i32 {
        let capture_count = self.captures(is_black) as i32;
        let capture_score = match capture_count {
            0 => 0,
            1 => 5_000,
            2 => 10_000,
            3 => 25_000,
            4 => 80_000,
            _ => 100_000, // 5 = win, handled by terminal check
        };
        let five_row_score = counts.five_row * 80001;
        let open_four_score = counts.open_four * 40000;
        let free_three_score = counts.free_three * 15000;
        let open_three_score = (counts.open_three - counts.free_three).max(0) * 3000;
        let block_four_score = counts.block_four * 4000;
        let open_two_score = counts.open_two * 100;

        capture_score
            + five_row_score
            + open_four_score
            + free_three_score
            + open_three_score
            + block_four_score
            + open_two_score
    }

    // ---- get_candidate_moves ----

    pub fn get_candidate_moves(&self, radius: usize) -> Vec<(u8, u8)> {
        self.get_candidate_moves_scored(radius)
            .into_iter()
            .map(|(r, c, _)| (r, c))
            .collect()
    }

    /// Like get_candidate_moves but also returns the score for each move.
    pub fn get_candidate_moves_scored(&self, radius: usize) -> Vec<(u8, u8, i32)> {
        // Check if board is empty
        let has_stone = self.cells.iter().any(|c| *c != Cell::Empty);
        if !has_stone {
            let center = BOARD_SIZE as u8 / 2;
            return vec![(center, center, 0)];
        }

        let mut seen = [false; TOTAL_CELLS];

        // Find all empty cells within radius of any occupied cell
        for row in 0..BOARD_SIZE {
            for col in 0..BOARD_SIZE {
                if self.cells[idx(row, col)] == Cell::Empty {
                    continue;
                }
                let start_row = row.saturating_sub(radius);
                let end_row = (row + radius + 1).min(BOARD_SIZE);
                let start_col = col.saturating_sub(radius);
                let end_col = (col + radius + 1).min(BOARD_SIZE);

                for r in start_row..end_row {
                    for c in start_col..end_col {
                        seen[idx(r, c)] = true;
                    }
                }
            }
        }

        // Collect valid moves
        let mut moves: Vec<(u8, u8, i32)> = Vec::new();
        for i in 0..TOTAL_CELLS {
            if !seen[i] || self.cells[i] != Cell::Empty {
                continue;
            }
            let row = (i / BOARD_SIZE) as i32;
            let col = (i % BOARD_SIZE) as i32;
            if !self.is_valid_move(row, col) {
                continue;
            }
            let score = self.score_move(row as u8, col as u8);
            moves.push((row as u8, col as u8, score));
        }

        // Sort by score descending
        moves.sort_by(|a, b| b.2.cmp(&a.2));
        moves
    }

    // ---- evaluate_position ----

    pub fn evaluate_position(&self, row: usize, col: usize, player: Cell) -> i32 {
        let mut score = 0;

        for &(dx, dy) in &DIRECTIONS {
            let mut count = 1i32;
            count += self.count_stones_dir(row, col, dx, dy, player);
            count += self.count_stones_dir(row, col, -dx, -dy, player);

            score += match count {
                n if n >= 5 => 100_000,
                4 => 10_000,
                3 => 1_000,
                2 => 100,
                _ => 0,
            };

            if self.can_capture(row, col, dx, dy, player) {
                score += 50_000;
            }
            if self.can_capture(row, col, -dx, -dy, player) {
                score += 50_000;
            }
        }

        score
    }

    fn count_stones_dir(&self, row: usize, col: usize, dx: i32, dy: i32, player: Cell) -> i32 {
        let mut count = 0;
        for i in 1..5 {
            let r = row as i32 + dx * i;
            let c = col as i32 + dy * i;
            if !in_bounds(r, c) || self.get(r, c) != player {
                break;
            }
            count += 1;
        }
        count
    }

    // ---- Threat-aware move scoring ----

    /// Bidirectional line scan for both colors from an empty candidate cell.
    /// Returns (mc, mo, m_gap, mc_consec, oc, oo, o_gap, oc_consec):
    ///   mc/oc:        total stones of my/opp color in the line (including across gap)
    ///   mo/oo:        open ends (0, 1, or 2)
    ///   m_gap/o_gap:  whether the pattern has exactly one internal gap
    ///   mc_consec/oc_consec: consecutive stones (no gap) including the candidate cell
    pub fn count_line_both(
        &self,
        me: Cell,
        opp: Cell,
        row: i32,
        col: i32,
        dr: i32,
        dc: i32,
    ) -> (i32, i32, bool, i32, i32, i32, bool, i32) {
        let sz = BOARD_SIZE as i32;

        let mut mc = 1i32; // count candidate cell as "mine"
        let mut mo = 0i32;
        let mut m_gap_pos = false;
        let mut m_gap_neg = false;
        let mut mc_pos = 0i32;
        let mut mc_neg = 0i32;

        let mut oc = 1i32; // count candidate cell as "opp" (hypothetical)
        let mut oo = 0i32;
        let mut o_gap_pos = false;
        let mut o_gap_neg = false;
        let mut oc_pos = 0i32;
        let mut oc_neg = 0i32;

        // Positive direction
        {
            let mut r = row + dr;
            let mut c = col + dc;
            let mut my_active = true;
            let mut my_consec = true;
            let mut opp_active = true;
            let mut opp_consec = true;

            while (my_active || opp_active) && r >= 0 && r < sz && c >= 0 && c < sz {
                let cell = self.get(r, c);

                if cell == me {
                    if my_active {
                        mc += 1;
                        if my_consec {
                            mc_pos += 1;
                        }
                    }
                    if opp_active {
                        opp_active = false;
                    }
                } else if cell == opp {
                    if opp_active {
                        oc += 1;
                        if opp_consec {
                            oc_pos += 1;
                        }
                    }
                    if my_active {
                        my_active = false;
                    }
                } else {
                    // Empty cell
                    if my_active {
                        if !m_gap_pos {
                            my_consec = false;
                            let nr = r + dr;
                            let nc = c + dc;
                            if nr >= 0 && nr < sz && nc >= 0 && nc < sz && self.get(nr, nc) == me {
                                m_gap_pos = true;
                            } else {
                                mo += 1;
                                my_active = false;
                            }
                        } else {
                            mo += 1;
                            my_active = false;
                        }
                    }
                    if opp_active {
                        if !o_gap_pos {
                            opp_consec = false;
                            let nr = r + dr;
                            let nc = c + dc;
                            if nr >= 0 && nr < sz && nc >= 0 && nc < sz && self.get(nr, nc) == opp {
                                o_gap_pos = true;
                            } else {
                                oo += 1;
                                opp_active = false;
                            }
                        } else {
                            oo += 1;
                            opp_active = false;
                        }
                    }
                }

                r += dr;
                c += dc;
            }
        }

        // Negative direction
        {
            let mut r = row - dr;
            let mut c = col - dc;
            let mut my_active = true;
            let mut my_consec = true;
            let mut opp_active = true;
            let mut opp_consec = true;

            while (my_active || opp_active) && r >= 0 && r < sz && c >= 0 && c < sz {
                let cell = self.get(r, c);

                if cell == me {
                    if my_active {
                        mc += 1;
                        if my_consec {
                            mc_neg += 1;
                        }
                    }
                    if opp_active {
                        opp_active = false;
                    }
                } else if cell == opp {
                    if opp_active {
                        oc += 1;
                        if opp_consec {
                            oc_neg += 1;
                        }
                    }
                    if my_active {
                        my_active = false;
                    }
                } else {
                    // Empty cell
                    if my_active {
                        if !m_gap_neg {
                            my_consec = false;
                            let nr = r - dr;
                            let nc = c - dc;
                            if nr >= 0 && nr < sz && nc >= 0 && nc < sz && self.get(nr, nc) == me {
                                m_gap_neg = true;
                            } else {
                                mo += 1;
                                my_active = false;
                            }
                        } else {
                            mo += 1;
                            my_active = false;
                        }
                    }
                    if opp_active {
                        if !o_gap_neg {
                            opp_consec = false;
                            let nr = r - dr;
                            let nc = c - dc;
                            if nr >= 0 && nr < sz && nc >= 0 && nc < sz && self.get(nr, nc) == opp {
                                o_gap_neg = true;
                            } else {
                                oo += 1;
                                opp_active = false;
                            }
                        } else {
                            oo += 1;
                            opp_active = false;
                        }
                    }
                }

                r -= dr;
                c -= dc;
            }
        }

        let mc_consec = 1 + mc_pos + mc_neg;
        let oc_consec = 1 + oc_pos + oc_neg;
        let m_gap = m_gap_pos || m_gap_neg;
        let o_gap = o_gap_pos || o_gap_neg;
        (mc, mo, m_gap, mc_consec, oc, oo, o_gap, oc_consec)
    }

    /// Count how many capture pairs placing `player` at (row,col) would create.
    fn count_captures_at(&self, row: i32, col: i32, player: Cell) -> i32 {
        let mut count = 0;
        for &(dx, dy) in &ALL_DIRS {
            if self.can_capture(row as usize, col as usize, dx, dy, player) {
                count += 1;
            }
        }
        count
    }

    /// Count how many of my pairs become capturable if I place at (row,col).
    /// Pattern: opp-ME(here)-ally-empty or empty-ally-ME(here)-opp
    fn count_vulnerability_at(&self, row: i32, col: i32, player: Cell) -> i32 {
        let opp = player.opponent();
        let mut vuln = 0;
        for &(dx, dy) in &ALL_DIRS {
            // Check pattern: OPP ME(here) ALLY EMPTY → opp can capture at EMPTY
            {
                let r1 = row - dx; let c1 = col - dy;
                let r2 = row + dx; let c2 = col + dy;
                let r3 = row + 2*dx; let c3 = col + 2*dy;
                if in_bounds(r1, c1) && in_bounds(r2, c2) && in_bounds(r3, c3)
                    && self.get(r1, c1) == opp && self.get(r2, c2) == player && self.get(r3, c3) == Cell::Empty {
                    vuln += 1;
                }
            }
            // Check pattern: EMPTY ALLY ME(here) OPP → opp can capture at EMPTY
            {
                let r1 = row + dx; let c1 = col + dy;
                let r2 = row - dx; let c2 = col - dy;
                let r3 = row - 2*dx; let c3 = col - 2*dy;
                if in_bounds(r1, c1) && in_bounds(r2, c2) && in_bounds(r3, c3)
                    && self.get(r1, c1) == opp && self.get(r2, c2) == player && self.get(r3, c3) == Cell::Empty {
                    vuln += 1;
                }
            }
        }
        vuln
    }

    /// Count vulnerable pairs for a given side (pairs that can be captured).
    /// A pair is vulnerable if: OPP/EMPTY - ME - ME - OPP/EMPTY with at least one OPP.
    fn count_vulnerable_pairs(&self, is_black: bool) -> i32 {
        let me = if is_black { Cell::Black } else { Cell::White };
        let opp = me.opponent();
        let mut count = 0;
        for row in 0..BOARD_SIZE {
            for col in 0..BOARD_SIZE {
                if self.cells[idx(row, col)] != me { continue; }
                for &(dx, dy) in &ALL_DIRS {
                    let r2 = row as i32 + dx;
                    let c2 = col as i32 + dy;
                    let r_cap = row as i32 - dx;
                    let c_cap = col as i32 - dy;
                    let r_end = row as i32 + 2*dx;
                    let c_end = col as i32 + 2*dy;
                    if in_bounds(r2, c2) && in_bounds(r_cap, c_cap) && in_bounds(r_end, c_end)
                        && self.get(r2, c2) == me
                        && (self.get(r_cap, c_cap) == opp || self.get(r_cap, c_cap) == Cell::Empty)
                        && (self.get(r_end, c_end) == opp || self.get(r_end, c_end) == Cell::Empty)
                        && (self.get(r_cap, c_cap) == opp || self.get(r_end, c_end) == opp)
                    {
                        count += 1;
                    }
                }
            }
        }
        count / 2 // each pair counted from both stones
    }

    /// Threat-aware move scoring. Scans both colors in 4 directions, then applies
    /// a priority ladder so that wins, blocks, forks, and captures are ordered correctly.
    pub fn score_move(&self, row: u8, col: u8) -> i32 {
        let me = self.current_player();
        let opp = self.opponent_player();
        let r = row as i32;
        let c = col as i32;

        let mut my_five = false;
        let mut opp_five = false;
        let mut my_open_four_count = 0i32;
        let mut opp_open_four_count = 0i32;
        let mut my_closed_four_count = 0i32;
        let mut opp_closed_four_count = 0i32;
        let mut my_open_three_count = 0i32;
        let mut opp_open_three_count = 0i32;
        let mut my_closed_three_count = 0i32;
        let mut opp_closed_three_count = 0i32;
        let mut my_two_score = 0i32;
        let mut my_developing_dirs = 0i32;
        let mut opp_developing_dirs = 0i32;

        for &(dr, dc) in &DIRECTIONS {
            let (mc, mo, mc_gap, mc_consec, oc, oo, oc_gap, oc_consec) =
                self.count_line_both(me, opp, r, c, dr, dc);

            // My patterns
            if mc_consec >= 5 {
                my_five = true;
            } else if mc >= 5 && mc_gap {
                my_open_four_count += 1; // gap-five = one move from winning
            }
            if mc == 4 {
                if mo == 2 {
                    my_open_four_count += 1;
                } else if mo == 1 {
                    my_closed_four_count += 1;
                }
            }
            if mc == 3 && mo == 2 {
                my_open_three_count += 1;
            }
            if mc == 3 && mo == 1 {
                my_closed_three_count += 1;
            }
            if mc == 2 {
                my_two_score += if mo == 2 { 500 } else if mo == 1 { 150 } else { 0 };
            }

            // Opponent patterns (what this move blocks)
            if oc_consec >= 5 {
                opp_five = true;
            } else if oc >= 5 && oc_gap {
                opp_open_four_count += 1;
            }
            if oc == 4 {
                if oo == 2 {
                    opp_open_four_count += 1;
                } else if oo == 1 {
                    opp_closed_four_count += 1;
                }
            }
            if oc == 3 && oo == 2 {
                opp_open_three_count += 1;
            }
            if oc == 3 && oo == 1 {
                opp_closed_three_count += 1;
            }
            if oc == 2 && oo == 2 {
                my_two_score += 200; // blocking opponent's open two has value
            }

            // Multi-directional development
            if mc >= 2 && mo >= 1 {
                my_developing_dirs += 1;
            }
            if oc >= 2 && oo >= 1 {
                opp_developing_dirs += 1;
            }
        }

        let my_total_fours = my_open_four_count + my_closed_four_count;
        let opp_total_fours = opp_open_four_count + opp_closed_four_count;

        // === Priority ladder ===

        // Immediate wins
        if my_five {
            return 900_000;
        }
        if opp_five {
            return 895_000;
        }

        // Capture wins
        let my_cap_count = self.count_captures_at(r, c, me);
        let my_caps = self.captures(self.is_black_turn) as i32;
        if my_cap_count > 0 && my_caps + my_cap_count >= 5 {
            return 890_000;
        }
        let opp_cap_count = self.count_captures_at(r, c, opp);
        let opp_caps = self.captures(!self.is_black_turn) as i32;
        if opp_cap_count > 0 && opp_caps + opp_cap_count >= 5 {
            return 885_000;
        }

        // My forks
        if my_total_fours >= 2 {
            return 880_000;
        }
        if my_total_fours >= 1 && my_open_three_count >= 1 {
            return 878_000;
        }

        // Single open four (unstoppable without capture)
        if my_open_four_count >= 1 {
            return 870_000;
        }

        // Block opponent forks
        if opp_total_fours >= 2 {
            return 868_000;
        }
        if opp_total_fours >= 1 && opp_open_three_count >= 1 {
            return 866_000;
        }
        if opp_open_four_count >= 1 {
            return 860_000;
        }

        // Capture urgency (opponent near capture win)
        if opp_cap_count > 0 && opp_caps >= 3 {
            return 855_000;
        }
        if opp_cap_count > 0 && opp_caps >= 2 {
            return 845_000;
        }

        // Double open three
        if my_open_three_count >= 2 {
            return 840_000;
        }
        if opp_open_three_count >= 2 {
            return 838_000;
        }

        // Single forcing threats
        if my_closed_four_count >= 1 {
            return 830_000;
        }
        if opp_closed_four_count >= 1 {
            return 820_000;
        }
        if my_open_three_count >= 1 {
            return 810_000;
        }
        if opp_open_three_count >= 1 {
            return 800_000;
        }

        // Closed threes (still above tactical threshold)
        if my_closed_three_count >= 1 {
            return 795_000;
        }
        if opp_closed_three_count >= 1 {
            return 790_000;
        }

        // My captures (always above tactical threshold)
        if my_cap_count > 0 {
            let base = match my_caps + my_cap_count {
                n if n >= 5 => 890_000, // already handled above
                4 => 850_000,
                3 => 842_000,
                _ => if my_caps >= 1 { 812_000 } else { 805_000 },
            };
            return base + my_cap_count * 1_000;
        }

        // Block opponent captures (scaled by urgency)
        // Must stay above 800K tactical threshold to avoid pruning
        if opp_cap_count > 0 {
            let base = match opp_caps {
                n if n >= 3 => 855_000, // already handled above, fallthrough safety
                2 => 845_000,
                1 => 825_000,
                _ => 815_000, // even at 0 caps, losing a pair is significant
            };
            return base + opp_cap_count * 1_000;
        }

        // Vulnerability penalty: penalize moves that create capturable pairs
        let vuln = self.count_vulnerability_at(r, c, me);
        let opp_cap_progress = self.captures(!self.is_black_turn) as i32;
        let vuln_penalty = vuln * match opp_cap_progress {
            n if n >= 3 => 150_000,
            2 => 80_000,
            1 => 40_000,
            _ => 20_000,
        };

        // Quiet move fallback: center + proximity + two-score + development
        let center = (BOARD_SIZE / 2) as i32;
        let dist = (r - center).abs() + (c - center).abs();
        let center_bonus = (18 - dist) * 25;

        let mut proximity = 0i32;
        for &(dr, dc) in &DIRECTIONS {
            for sign in [-1i32, 1] {
                let nr = r + dr * sign;
                let nc = c + dc * sign;
                if in_bounds(nr, nc) && self.get(nr, nc) == me {
                    proximity += 200;
                }
            }
        }

        let development_bonus = match my_developing_dirs {
            0..=1 => 0,
            2 => 50_000,
            _ => 100_000,
        };
        let disruption_bonus = match opp_developing_dirs {
            0..=1 => 0,
            2 => 30_000,
            _ => 80_000,
        };

        (center_bonus + proximity + my_two_score + development_bonus + disruption_bonus - vuln_penalty).max(0)
    }

    fn can_capture(&self, row: usize, col: usize, dx: i32, dy: i32, player: Cell) -> bool {
        let opp = player.opponent();
        let r1 = row as i32 + dx;
        let c1 = col as i32 + dy;
        let r2 = row as i32 + 2 * dx;
        let c2 = col as i32 + 2 * dy;
        let r3 = row as i32 + 3 * dx;
        let c3 = col as i32 + 3 * dy;

        in_bounds(r1, c1)
            && in_bounds(r2, c2)
            && in_bounds(r3, c3)
            && self.get(r1, c1) == opp
            && self.get(r2, c2) == opp
            && self.get(r3, c3) == player
    }
}

#[derive(Default)]
struct PatternCounts {
    five_row: i32,
    open_four: i32,
    block_four: i32,
    open_three: i32,
    free_three: i32,
    open_two: i32,
}
