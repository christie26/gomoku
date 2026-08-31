use super::*;
use crate::search_board::{Cell, PatternCounts, PatternKind, SearchBoard};

/// Place a stone for `player` under the same preconditions
/// `Gomoku::handle_move` enforces: the cell is empty and the move is not a
/// double three.
fn place(board: &mut SearchBoard, player: Cell, x: i32, y: i32) {
    board.current = player;
    board.opponent = player.opponent();
    assert_eq!(
        board.cells[x as usize][y as usize],
        Cell::Empty,
        "({x},{y}) is already occupied"
    );
    assert!(
        !board.sb_is_double_three(x, y),
        "({x},{y}) {player:?} is a double three"
    );
    board.make_move(x as usize, y as usize);
}

fn remove(board: &mut SearchBoard, x: i32, y: i32) {
    board.remove_stone_raw(x as usize, y as usize);
}

/// Lay out `pattern` on row `row` from column `base`, holding back the stone at
/// `add_index`, then play it. Returns the board and Black's counts from just
/// before that last stone landed.
fn setup_window_add_stone(
    row: i32,
    base: i32,
    pattern: &str,
    add_index: usize,
) -> (SearchBoard, PatternCounts) {
    let mut board = SearchBoard::empty();
    for (i, ch) in pattern.chars().enumerate() {
        if i == add_index {
            continue;
        }
        match ch {
            'O' => place(&mut board, Cell::White, row, base + i as i32),
            'X' => place(&mut board, Cell::Black, row, base + i as i32),
            _ => {}
        }
    }
    let before = board.black_patterns;
    let last = match pattern.chars().nth(add_index) {
        Some('X') => Cell::Black,
        Some('O') => Cell::White,
        other => panic!("add_index={add_index} is not a stone: {other:?}"),
    };
    place(&mut board, last, row, base + add_index as i32);
    (board, before)
}

/// Lay out all of `pattern`, then lift the stone at `remove_index`. Returns the
/// board and Black's counts from just before the removal.
fn setup_window_remove_stone(
    row: i32,
    base: i32,
    pattern: &str,
    remove_index: usize,
) -> (SearchBoard, PatternCounts) {
    let mut board = SearchBoard::empty();
    for (i, ch) in pattern.chars().enumerate() {
        match ch {
            'O' => place(&mut board, Cell::White, row, base + i as i32),
            'X' => place(&mut board, Cell::Black, row, base + i as i32),
            _ => {}
        }
    }
    let before = board.black_patterns;
    remove(&mut board, row, base + remove_index as i32);
    (board, before)
}

/// One window, one stone played or lifted, and how many patterns of the kind
/// under test Black should hold either side of that change.
struct Case {
    row: i32,
    base: i32,
    pattern: &'static str,
    index: usize,
    before: i32,
    after: i32,
}

impl Case {
    /// Defaults to row j, starting at column 10 (`j10`).
    fn new(pattern: &'static str, index: usize, before: i32, after: i32) -> Self {
        Case { row: 9, base: 9, pattern, index, before, after }
    }
}

fn decode_short_coord(coord: &str) -> Position {
    let mut chars = coord.chars();
    let col_char = chars.next().unwrap().to_ascii_uppercase();
    let x = (col_char as i32) - ('A' as i32);
    let row_str = chars.as_str();
    let row_num: i32 = row_str.parse().expect("invalid row");
    assert!(row_num > 0);
    assert!(row_num < 20);
    let y = row_num - 1;

    (x, y)
}

fn encode_short_coord((x, y): Position) -> String {
    format!("{}{}", (b'a' + x as u8) as char, y + 1)
}

type Setup = fn(i32, i32, &str, usize) -> (SearchBoard, PatternCounts);

fn count_of(counts: &PatternCounts, kind: PatternKind) -> i32 {
    match kind {
        PatternKind::OpenTwo => counts.open_twos,
        PatternKind::OpenThree => counts.open_threes,
        PatternKind::FreeThree => counts.free_threes,
        PatternKind::OpenFour => counts.open_fours,
        PatternKind::BlockFour => counts.block_fours,
        PatternKind::FiveRow => counts.five_rows,
    }
}

fn run_cases(setup: Setup, kind: PatternKind, cases: Vec<Case>) {
    let mut failures = Vec::new();
    for case in cases {
        let (board, before) = setup(case.row, case.base, case.pattern, case.index);

        // Every case doubles as a check that the counts the board carried
        // incrementally still agree with a full rescan.
        let (black, white) = board.sb_scan_patterns();
        if board.black_patterns != black || board.white_patterns != white {
            failures.push(format!(
                "{:?} idx={}: incremental counts drifted from a full rescan\n  black held {:?}\n  black scan {:?}\n  white held {:?}\n  white scan {:?}",
                case.pattern, case.index,
                board.black_patterns, black, board.white_patterns, white,
            ));
            continue;
        }

        let actual_before = count_of(&before, kind);
        if actual_before != case.before {
            failures.push(format!(
                "{:?} idx={}: before expected {} {:?}, what we got: {}",
                case.pattern, case.index, case.before, kind, actual_before
            ));
            continue;
        }

        let actual_after = count_of(&board.black_patterns, kind);
        if actual_after != case.after {
            failures.push(format!(
                "{:?} idx={}: after expected {} {:?}, what we got: {}",
                case.pattern, case.index, case.after, kind, actual_after
            ));
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

mod decode_short_coords {
    use super::*;

    #[test]
    fn test_decode_short_coords() {
        assert_eq!(decode_short_coord("a1"), (0, 0));
        assert_eq!(decode_short_coord("s19"), (18, 18));
        assert_eq!(decode_short_coord("b5"), (1, 4));
        assert_eq!(decode_short_coord("f6"), (5, 5));
        assert_eq!(decode_short_coord("j10"), (9, 9));
    }

    #[test]
    fn test_encode_short_coords() {
        assert_eq!(encode_short_coord((0, 0)), "a1");
        assert_eq!(encode_short_coord((18, 18)), "s19");
        assert_eq!(encode_short_coord((5, 5)), "f6");
        assert_eq!(encode_short_coord((9, 9)), "j10");
    }

}

mod add_stone_add_pattern {
    use super::*;

    #[test]
    fn open_two() {
        run_cases(
            setup_window_add_stone,
            PatternKind::OpenTwo,
            vec![
                // ..XX..
                Case::new("..XX..", 2, 0, 1),
                // ..XX.O
                Case::new("..XX.O", 2, 0, 1),
                Case::new("..XX.O", 3, 0, 1),
                Case::new("..XX.O", 5, 1, 1),
                // O.X.X.O
                Case::new("O.X.X.O", 0, 1, 1),
                Case::new("O.X.X.O", 2, 0, 1),
                // NOTE - we cannot detect 5 empty space
                // ..X.X..
                Case::new("..X.X..", 2, 0, 1),
                Case::new("..X.X..", 4, 0, 1),
            ],
        );
    }

    #[test]
    fn open_three() {
        run_cases(
            setup_window_add_stone,
            PatternKind::OpenThree,
            vec![
                // OXXX..
                Case::new("OXXX..", 0, 0, 1),
                Case::new("OXXX..", 1, 0, 1),
                Case::new("OXXX..", 2, 0, 1),
                Case::new("OXXX..", 3, 0, 1),
                // OXX.X.
                Case::new("OXX.X.", 0, 0, 1),
                Case::new("OXX.X.", 1, 0, 1),
                Case::new("OXX.X.", 2, 0, 1),
                Case::new("OXX.X.", 4, 0, 1),
                // OX.XX.
                Case::new("OX.XX.", 0, 0, 1),
                Case::new("OX.XX.", 1, 0, 1),
                Case::new("OX.XX.", 3, 0, 1),
                Case::new("OX.XX.", 4, 0, 1),
                // O.XXX.O
                Case::new("O.XXX.O", 0, 0, 1),
                Case::new("O.XXX.O", 2, 0, 1),
                Case::new("O.XXX.O", 3, 0, 1),
                Case::new("O.XXX.O", 4, 0, 1),
                // TODO
                // // X..XX
                // Case::new("X..XX", 0, 0, 1),
                // Case::new("X..XX", 3, 0, 1),
                // Case::new("X..XX", 4, 0, 1),
                // // X.X.X
                // Case::new("X.X.X", 0, 0, 1),
                // Case::new("X.X.X", 2, 0, 1),
                // Case::new("X.X.X", 4, 0, 1),
            ],
        );
    }

    #[test]
    fn free_three() {
        run_cases(
            setup_window_add_stone,
            PatternKind::FreeThree,
            vec![
                // ..XXX..
                Case::new("..XXX..", 2, 0, 1),
                Case::new("..XXX..", 3, 0, 1),
                // O.XXX..
                Case::new("O.XXX..", 0, 1, 1),
                Case::new("O.XXX..", 2, 0, 1),
                Case::new("O.XXX..", 3, 0, 1),
                Case::new("O.XXX..", 4, 0, 1),
                // .XX.X.
                Case::new(".XX.X.", 1, 0, 1),
                Case::new(".XX.X.", 2, 0, 1),
                Case::new(".XX.X.", 4, 0, 1),

                // Should not be counted as a free three
                // OXX.X.
                Case::new("OXX.X.", 0, 1, 0),
                Case::new("OXX.X.", 1, 0, 0),
                Case::new("OXX.X.", 2, 0, 0),
                Case::new("OXX.X.", 4, 0, 0),

                // OX.XX.
                Case::new("OX.XX.", 0, 1, 0),
                Case::new("OX.XX.", 1, 0, 0),
                Case::new("OX.XX.", 3, 0, 0),
                Case::new("OX.XX.", 4, 0, 0),
            ],
        );
    }

    #[test]
    fn block_four() {
        run_cases(
            setup_window_add_stone,
            PatternKind::BlockFour,
            vec![
                // .XXXXO
                Case::new(".XXXXO", 1, 0, 1),
                Case::new(".XXXXO", 2, 0, 1),
                Case::new(".XXXXO", 3, 0, 1),
                Case::new(".XXXXO", 4, 0, 1),
                Case::new(".XXXXO", 5, 0, 1),
                // X.XXX
                Case::new("X.XXX", 0, 0, 1),
                Case::new("X.XXX", 2, 0, 1),
                Case::new("X.XXX", 3, 0, 1),
                Case::new("X.XXX", 4, 0, 1),
                // XX.XX
                Case::new("XX.XX", 0, 0, 1),
                Case::new("XX.XX", 1, 0, 1),
            ],
        );
    }

    #[test]
    fn open_four() {
        run_cases(
            setup_window_add_stone,
            PatternKind::OpenFour,
            vec![
                // .XXXX.
                Case::new(".XXXX.", 1, 0, 1),
                Case::new(".XXXX.", 2, 0, 1),
            ],
        );
    }

    #[test]
    fn five_row() {
        run_cases(
            setup_window_add_stone,
            PatternKind::FiveRow,
            vec![
                // XXXXX
                Case::new("XXXXX", 0, 0, 1),
                Case::new("XXXXX", 1, 0, 1),
                Case::new("XXXXX", 2, 0, 1),
            ],
        );
    }
}

mod add_stone_remove_pattern {
    use super::*;

    #[test]
    fn open_two() {
        run_cases(
            setup_window_add_stone,
            PatternKind::OpenTwo,
            vec![
                // ..XX.. 
                Case::new("..XXO.", 4, 1, 0),
                Case::new("..XX.O", 5, 1, 1),
                // ..XX.O 
                Case::new("O.XX.O", 0, 1, 1),
                Case::new(".OXX.O", 1, 1, 0),
                Case::new("..XXOO", 4, 1, 0),
                // O.X.X.O
                Case::new("OOX.X.O", 1, 1, 0),
                Case::new("O.XOX.O", 3, 1, 0),
            ],
        );
    }

    #[test]
    fn open_three() {
        run_cases(
            setup_window_add_stone,
            PatternKind::OpenThree,
            vec![
                // OXXX..
                Case::new("OXXXO.", 4, 1, 0),
                Case::new("OXXX.O", 5, 1, 0),
                // OXX.X.
                Case::new("OXXOX.O", 3, 1, 0),
                Case::new("OXX.XOO", 5, 1, 0),
                // OX.XX.
                Case::new("OXOXX.O", 2, 1, 0),
                Case::new("OX.XXOO", 5, 1, 0),
                // O.XXX.O
                Case::new("OOXXX.O", 1, 1, 0),
                // TODO
                // // X..XX
                // Case::new("OXO.XXO", 2, 1, 0),
                // Case::new("OX.OXXO", 3, 1, 0),
                // // X.X.X
                // Case::new("OXOX.XO", 2, 1, 0),
            ],
        );
    }

    #[test]
    fn free_three() {
        run_cases(
            setup_window_add_stone,
            PatternKind::FreeThree,
            vec![
                // ..XXX..
                Case::new("O.XXX..", 0, 1, 1),
                Case::new(".OXXX..", 1, 1, 0),
                // O.XXX..
                Case::new("OOXXX..", 1, 1, 0),
                Case::new("O.XXXO.", 5, 1, 0),
                Case::new("O.XXX.O", 6, 1, 0),
                // .XX.X.
                Case::new("OXX.X.", 0, 1, 0),
                Case::new(".XXOX.", 3, 1, 0),
                Case::new(".XX.XO", 5, 1, 0),
            ],
        );
    }

    #[test]
    fn block_four() {
        run_cases(
            setup_window_add_stone,
            PatternKind::BlockFour,
            vec![
                // .XXXXO
                Case::new("OXXXXO", 0, 1, 0),
                // X.XXX
                Case::new("XOXXX", 1, 1, 0),
                // XX.XX
                Case::new("XXOXX", 2, 1, 0),
            ],
        );
    }

    #[test]
    fn open_four() {
        run_cases(
            setup_window_add_stone,
            PatternKind::OpenFour,
            vec![
                // .XXXX.
                Case::new("OXXXX.", 0, 1, 0),
            ],
        );
    }

}

mod remove_stone_add_pattern {
    use super::*;

    #[test]
    fn open_two() {
        run_cases(
            setup_window_remove_stone,
            PatternKind::OpenTwo,
            vec![
                // ..XX.. 
                Case::new("..XXO.", 4, 0, 1),
                Case::new("..XX.O", 5, 1, 1),
                // ..XX.O 
                Case::new("O.XX.O", 0, 1, 1),
                Case::new(".OXX.O", 1, 0, 1),
                Case::new("..XXOO", 4, 0, 1),
                // O.X.X.O
                Case::new("OOX.X.O", 1, 0, 1),
                Case::new("O.XOX.O", 3, 0, 1),
            ],
        );
    }

    #[test]
    fn open_three() {
        run_cases(
            setup_window_remove_stone,
            PatternKind::OpenThree,
            vec![
                // OXXX..
                Case::new("OXXXO.", 4, 0, 1),
                Case::new("OXXX.O", 5, 0, 1),
                // OXX.X.
                // Placing the O at index 3 captures the XX pair, so the board
                // this removal starts from is `O..OX.O`, not `OXXOX.O`. Lifting
                // that O leaves Black a lone stone — no three to find.
                Case::new("OXXOX.O", 3, 0, 0),
                Case::new("OXX.XOO", 5, 0, 1),
                // OX.XX.
                Case::new("OXOXX.O", 2, 0, 1),
                Case::new("OX.XXOO", 5, 0, 1),
                // O.XXX.O
                Case::new("OOXXX.O", 1, 0, 1),
            ],
        );
    }

    #[test]
    fn free_three() {
        run_cases(
            setup_window_remove_stone,
            PatternKind::FreeThree,
            vec![
                // ..XXX..
                Case::new("O.XXX..", 0, 1, 1),
                Case::new(".OXXX..", 1, 0, 1),
                // O.XXX..
                Case::new("OOXXX..", 1, 0, 1),
                Case::new("O.XXXO.", 5, 0, 1),
                Case::new("O.XXX.O", 6, 0, 1),
                // .XX.X.
                Case::new("OXX.X.", 0, 0, 1),
                Case::new(".XXOX.", 3, 0, 1),
                Case::new(".XX.XO", 5, 0, 1),
            ],
        );
    }

    #[test]
    fn block_four() {
        run_cases(
            setup_window_remove_stone,
            PatternKind::BlockFour,
            vec![
                // .XXXXO
                Case::new("OXXXXO", 0, 0, 1),
                // X.XXX
                Case::new("XOXXX", 1, 0, 1),
                // XX.XX
                Case::new("XXOXX", 2, 0, 1),
            ],
        );
    }

    #[test]
    fn open_four() {
        run_cases(
            setup_window_remove_stone,
            PatternKind::OpenFour,
            vec![
                // .XXXX.
                Case::new("OXXXX.", 0, 0, 1),
            ],
        );
    }
}

mod remove_stone_remove_pattern {
    use super::*;

    #[test]
    fn open_two() {
        run_cases(
            setup_window_remove_stone,
            PatternKind::OpenTwo,
            vec![
                // ..XX..
                Case::new("..XX..", 2, 1, 0),
                // ..XX.O
                Case::new("..XX.O", 2, 1, 0),
                Case::new("..XX.O", 3, 1, 0),
                Case::new("..XX.O", 5, 1, 1),
                // O.X.X.O
                Case::new("O.X.X.O", 0, 1, 1),
                Case::new("O.X.X.O", 2, 1, 0),
                // NOTE - we cannot detect 5 empty space
                // ..X.X..
                Case::new("..X.X..", 2, 1, 0),
                Case::new("..X.X..", 4, 1, 0),
            ],
        );
    }

    #[test]
    fn open_three() {
        run_cases(
            setup_window_remove_stone,
            PatternKind::OpenThree,
            vec![
                // OXXX..
                Case::new("OXXX..", 0, 1, 0),
                Case::new("OXXX..", 1, 1, 0),
                Case::new("OXXX..", 2, 1, 0),
                Case::new("OXXX..", 3, 1, 0),
                // OXX.X.
                Case::new("OXX.X.", 0, 1, 0),
                Case::new("OXX.X.", 1, 1, 0),
                Case::new("OXX.X.", 2, 1, 0),
                Case::new("OXX.X.", 4, 1, 0),
                // OX.XX.
                Case::new("OX.XX.", 0, 1, 0),
                Case::new("OX.XX.", 1, 1, 0),
                Case::new("OX.XX.", 3, 1, 0),
                Case::new("OX.XX.", 4, 1, 0),
                // O.XXX.O
                Case::new("O.XXX.O", 0, 1, 0),
                Case::new("O.XXX.O", 2, 1, 0),
                Case::new("O.XXX.O", 3, 1, 0),
                Case::new("O.XXX.O", 4, 1, 0),
            ],
        );
    }

    #[test]
    fn free_three() {
        run_cases(
            setup_window_remove_stone,
            PatternKind::FreeThree,
            vec![
                // ..XXX..
                Case::new("..XXX..", 2, 1, 0),
                Case::new("..XXX..", 3, 1, 0),
                // O.XXX..
                Case::new("O.XXX..", 0, 1, 1),
                Case::new("O.XXX..", 2, 1, 0),
                Case::new("O.XXX..", 3, 1, 0),
                Case::new("O.XXX..", 4, 1, 0),
                // .XX.X.
                Case::new(".XX.X.", 1, 1, 0),
                Case::new(".XX.X.", 2, 1, 0),
                Case::new(".XX.X.", 4, 1, 0),
            ],
        );
    }

    #[test]
    fn block_four() {
        run_cases(
            setup_window_remove_stone,
            PatternKind::BlockFour,
            vec![
                // .XXXXO
                Case::new(".XXXXO", 1, 1, 0),
                Case::new(".XXXXO", 2, 1, 0),
                Case::new(".XXXXO", 3, 1, 0),
                Case::new(".XXXXO", 4, 1, 0),
                Case::new(".XXXXO", 5, 1, 0),
                // X.XXX
                Case::new("X.XXX", 0, 1, 0),
                Case::new("X.XXX", 2, 1, 0),
                Case::new("X.XXX", 3, 1, 0),
                Case::new("X.XXX", 4, 1, 0),
                // XX.XX
                Case::new("XX.XX", 0, 1, 0),
                Case::new("XX.XX", 1, 1, 0),
            ],
        );
    }

    #[test]
    fn open_four() {
        run_cases(
            setup_window_remove_stone,
            PatternKind::OpenFour,
            vec![
                // .XXXX.
                Case::new(".XXXX.", 1, 1, 0),
                Case::new(".XXXX.", 2, 1, 0),
            ],
        );
    }

    #[test]
    fn five_row() {
        run_cases(
            setup_window_remove_stone,
            PatternKind::FiveRow,
            vec![
                // XXXXX
                Case::new("XXXXX", 0, 1, 0),
                Case::new("XXXXX", 1, 1, 0),
                Case::new("XXXXX", 2, 1, 0),
            ],
        );
    }
}
