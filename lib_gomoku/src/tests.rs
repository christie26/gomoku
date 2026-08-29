use super::*;

fn place(game: &mut Gomoku, player: Stone, x: i32, y: i32) {
    game.current_player = player;
    game.opponent_player = match player {
        Stone::Black => Stone::White,
        Stone::White => Stone::Black,
        Stone::Empty => panic!("Empty is not player"),
    };
    let (result, _, _) = game.handle_move(x, y);
    assert_eq!(result, MoveResult::Valid, "({x},{y}) {:?} fail to handle_move", player);
}
fn remove(game: &mut Gomoku, x: i32, y: i32) {
    assert_ne!(
        game.board[x as usize][y as usize],
        Stone::Empty,
        "No stone to remove in ({x},{y})"
    );
    game.remove_stone(x, y);
}

fn black_patterns(game: &Gomoku) -> &PlayerPatterns {
    game.patterns.get(&Stone::Black).unwrap()
}

fn setup_window_add_stone(row: i32, base: i32, pattern: &str, add_index: usize) -> (Gomoku, PlayerPatterns) {
    let mut game = Gomoku::new(19);
    for (i, ch) in pattern.chars().enumerate() {
        if ch == 'O' && i != add_index{
            place(&mut game, Stone::White, row, base + i as i32);
        } else if ch == 'X' && i != add_index {
            place(&mut game, Stone::Black, row, base + i as i32);
        }
    }
    let before = black_patterns(&game).clone();
    let last = match pattern.chars().nth(add_index) {
        Some('X') => Stone::Black,
        Some('O') => Stone::White,
        other => panic!("add_index={add_index}is not stone: {other:?}"),
    };
    place(&mut game, last, row, base + add_index as i32);
    (game, before)
}

fn setup_window_remove_stone(row: i32, base: i32, pattern: &str, remove_index: usize) -> (Gomoku, PlayerPatterns) {
    let mut game = Gomoku::new(19);
    for (i, ch) in pattern.chars().enumerate() {
        if ch == 'O' {
            place(&mut game, Stone::White, row, base + i as i32);
        } else if ch == 'X' {
            place(&mut game, Stone::Black, row, base + i as i32);
        }
    }
    let before = black_patterns(&game).clone();
    remove(&mut game, row, base + remove_index as i32);
    (game, before)
}

struct Case {
    row: i32,
    base: i32,
    pattern: &'static str,
    index: usize,
    before: Vec<Pattern>,
    after: Vec<Pattern>,
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

impl Case {

    fn new(pattern: &'static str, index: usize, before: Vec<Vec<&str>>, after: Vec<Vec<&str>>) -> Self {
        let before = before.into_iter().map(|p| p.into_iter().map(decode_short_coord).collect()).collect();
        let after = after.into_iter().map(|p| p.into_iter().map(decode_short_coord).collect()).collect();

        // default j10
        Case { row: 9, base: 9, pattern, index, before, after }
    }

    fn with_row_base(&mut self, row_base: &str) {
        let (row, base) = decode_short_coord(row_base);
        self.row = row;
        self.base = base;
    }
}

type Setup = fn(i32, i32, &str, usize) -> (Gomoku, PlayerPatterns);

fn patterns_of(patterns: &PlayerPatterns, kind: PatternKind) -> &Vec<Pattern> {
    match kind {
        PatternKind::OpenTwo => &patterns.open_two,
        PatternKind::OpenThree => &patterns.open_three,
        PatternKind::FreeThree => &patterns.free_three,
        PatternKind::OpenFour => &patterns.open_four,
        PatternKind::BlockFour => &patterns.block_four,
        PatternKind::FiveRow => &patterns.five_row,
    }
}

fn run_cases(setup: Setup, kind: PatternKind, cases: Vec<Case>) {
    let mut failures = Vec::new();
    for case in cases {
        let (game, before) = setup(case.row, case.base, case.pattern, case.index);

        let actual_before = patterns_of(&before, kind);
        if *actual_before != case.before {
            failures.push(format!(
                "{:?} idx={}: before expected: {:?}, what we got: {actual_before:?}",
                case.pattern, case.index, case.before
            ));
            continue;
        }

        let actual_after = patterns_of(black_patterns(&game), kind);
        if *actual_after != case.after {
            failures.push(format!(
                "{:?} idx={}: after expected: {:?}, what we got: {actual_after:?}",
                case.pattern, case.index, case.after
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
                Case::new("..XX..", 2, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]]),
                // ..XX.O
                Case::new("..XX.O", 2, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14"]]),
                Case::new("..XX.O", 3, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14"]]),
                Case::new("..XX.O", 5, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14"]]),
                // O.X.X.O
                Case::new("O.X.X.O", 0, vec![], vec![vec!["j11", "j12", "j13", "j14", "j15"]]),
                Case::new("O.X.X.O", 2, vec![], vec![vec!["j11", "j12", "j13", "j14", "j15"]]),
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
                Case::new("OXXX..", 0, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]]),
                Case::new("OXXX..", 1, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]]),
                Case::new("OXXX..", 2, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]]),
                Case::new("OXXX..", 3, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]]),
                // OXX.X.
                Case::new("OXX.X.", 0, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]]),
                Case::new("OXX.X.", 1, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]]),
                Case::new("OXX.X.", 2, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]]),
                Case::new("OXX.X.", 4, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]]),
                // OX.XX.
                Case::new("OX.XX.", 0, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]]),
                Case::new("OX.XX.", 1, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]]),
                Case::new("OX.XX.", 3, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]]),
                Case::new("OX.XX.", 4, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]]),
                // O.XXX.O
                Case::new("O.XXX.O", 0, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14", "j15", "j16"]]),
                Case::new("O.XXX.O", 2, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14", "j15", "j16"]]),
                Case::new("O.XXX.O", 3, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14", "j15", "j16"]]),
                Case::new("O.XXX.O", 4, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14", "j15", "j16"]]),
                // X..XX
                Case::new("X..XX", 0, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14"]]),
                Case::new("X..XX", 3, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14"]]),
                Case::new("X..XX", 4, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14"]]),
                // X.X.X
                Case::new("X.X.X", 0, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14"]]),
                Case::new("X.X.X", 2, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14"]]),
                Case::new("X.X.X", 4, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14"]]),
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
                Case::new("..XXX..", 2, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14", "j15", "j16"]]),
                Case::new("..XXX..", 3, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14", "j15", "j16"]]),
                // O.XXX..
                Case::new("O.XXX..", 0, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14", "j15", "j16"]]),
                Case::new("O.XXX..", 2, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14", "j15", "j16"]]),
                Case::new("O.XXX..", 3, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14", "j15", "j16"]]),
                Case::new("O.XXX..", 4, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14", "j15", "j16"]]),
                // .XX.X.
                Case::new(".XX.X.", 1, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]]),
                Case::new(".XX.X.", 2, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]]),
                Case::new(".XX.X.", 4, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]]),
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
                Case::new(".XXXXO", 1, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]]),
                Case::new(".XXXXO", 2, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]]),
                Case::new(".XXXXO", 3, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]]),
                Case::new(".XXXXO", 4, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]]),
                Case::new(".XXXXO", 5, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]]),
                // X.XXX
                Case::new("X.XXX", 0, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14"]]),
                Case::new("X.XXX", 2, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14"]]),
                Case::new("X.XXX", 3, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14"]]),
                Case::new("X.XXX", 4, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14"]]),
                // XX.XX
                Case::new("XX.XX", 0, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14"]]),
                Case::new("XX.XX", 1, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14"]]),
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
                Case::new(".XXXX.", 1, vec![], vec![vec!["j10","j11", "j12", "j13", "j14", "j15"]]),
                Case::new(".XXXX.", 2, vec![], vec![vec!["j10","j11", "j12", "j13", "j14", "j15"]]),
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
                Case::new("XXXXX", 4, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14"]]),
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
                Case::new("..XXO.", 4, vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]], vec![]),
                // ..XX.O 
                Case::new("..XXOO", 4, vec![vec!["j10", "j11", "j12", "j13", "j14"]], vec![]),
                Case::new(".OXX.O", 2, vec![vec!["j10", "j11", "j12", "j13", "j14"]], vec![]),
                // O.X.X.O
                Case::new("O.XOX.O", 3, vec![vec!["j11", "j12", "j13", "j14", "j15"]], vec![]),
                Case::new("OOX.X.O", 2, vec![vec!["j11", "j12", "j13", "j14", "j15"]], vec![]),
            ],
        );
    }

    #[test]
    fn open_three() {
        run_cases(
            setup_window_add_stone,
            PatternKind::OpenThree,
            vec![
                // XXX..
                Case::new("OXXXO.", 4, vec![vec!["j11", "j12", "j13", "j14", "j15"]], vec![]),
                Case::new("OXXX.O", 5, vec![vec!["j11", "j12", "j13", "j14", "j15"]], vec![]),
                // XX.X.
                Case::new("OXXOX.O", 3, vec![vec!["j11", "j12", "j13", "j14", "j15"]], vec![]),
                Case::new("OXX.XOO", 5, vec![vec!["j11", "j12", "j13", "j14", "j15"]], vec![]),
                // X.XX.
                Case::new("OXOXX.O", 2, vec![vec!["j11", "j12", "j13", "j14", "j15"]], vec![]),
                Case::new("OX.XXOO", 5, vec![vec!["j11", "j12", "j13", "j14", "j15"]], vec![]),
                // .XXX.
                Case::new("OOXXX.O", 1, vec![vec!["j11", "j12", "j13", "j14", "j15"]], vec![]),
                // X..XX
                Case::new("OXO.XXO", 2, vec![vec!["j11", "j12", "j13", "j14", "j15"]], vec![]),
                Case::new("OX.OXXO", 3, vec![vec!["j11", "j12", "j13", "j14", "j15"]], vec![]),
                // X.X.X
                Case::new("OXOX.XO", 2, vec![vec!["j11", "j12", "j13", "j14", "j15"]], vec![]),
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
                Case::new("O.XXX..", 0,  vec![vec!["j10", "j11", "j12", "j13", "j14", "j15", "j16"]], vec![]),
                Case::new(".OXXX..", 1,  vec![vec!["j10", "j11", "j12", "j13", "j14", "j15", "j16"]], vec![]),
                // .XXX..
                Case::new("OOXXX..O", 2,  vec![vec!["j11", "j12", "j13", "j14", "j15", "j16"]], vec![]),
                Case::new("O.XXXO.O", 5,  vec![vec!["j11", "j12", "j13", "j14", "j15", "j16"]], vec![]),
                Case::new("O.XXX.OO", 6,  vec![vec!["j11", "j12", "j13", "j14", "j15", "j16"]], vec![]),
                // .XX.X.
                Case::new("OOXX.X.O", 1,  vec![vec!["j11", "j12", "j13", "j14", "j15", "j16"]], vec![]),
                Case::new("O.XXOX.O", 4,  vec![vec!["j11", "j12", "j13", "j14", "j15", "j16"]], vec![]),
                Case::new("O.XX.XOO", 6,  vec![vec!["j11", "j12", "j13", "j14", "j15", "j16"]], vec![]),
            ],
        );
    }

    #[test]
    fn block_four() {
        run_cases(
            setup_window_add_stone,
            PatternKind::BlockFour,
            vec![
                // O.XXXX
                Case::new("OXXXX", 0, vec![vec!["j10", "j11", "j12", "j13", "j14"]],vec![]),
                // X.XXX
                Case::new("XOXXX", 1, vec![vec!["j10", "j11", "j12", "j13", "j14"]],vec![]),
                // XX.XX
                Case::new("XXOXX", 2, vec![vec!["j10", "j11", "j12", "j13", "j14"]],vec![]),
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
                Case::new("OXXXX.", 0, vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]], vec![]),
            ],
        );
    }

}

//TODO: Etienne does thsi
mod remove_stone_add_pattern {
    use super::*;

    #[test]
    fn open_two() {
        run_cases(
            setup_window_add_stone,
            PatternKind::OpenTwo,
            vec![
                // ..XX..
                Case { row: 5, base: 5, pattern: "..XX..", index: 2,
                       before: vec![], after: vec![vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9), (5, 10)]] },
                // ..XX.O
                Case { row: 5, base: 5, pattern: "..XX.O", index: 2,
                       before: vec![], after: vec![vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9)]] },
                Case { row: 5, base: 5, pattern: "..XX.O", index: 3,
                       before: vec![], after: vec![vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9)]] },
                // O.X.X.O
                Case { row: 5, base: 5, pattern: "O.X.X.O", index: 2,
                       before: vec![], after: vec![vec![(5, 6), (5, 7), (5, 8), (5, 9), (5, 10)]] },
            ],
        );
    }

    #[test]
    fn open_three() {
        run_cases(
            setup_window_add_stone,
            PatternKind::OpenThree,
            vec![
                // XXX..
                Case { row: 5, base: 4, pattern: "OXXX..", index: 1,
                       before: vec![], after: vec![vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9)]] },
                Case { row: 5, base: 4, pattern: "OXXX..", index: 2,
                       before: vec![], after: vec![vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9)]] },
                Case { row: 5, base: 4, pattern: "OXXX..", index: 3,
                       before: vec![], after: vec![vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9)]] },
                // XX.X.
                Case { row: 1, base: 5, pattern: "OXX.X.O", index: 1,
                       before: vec![], after: vec![vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)]] },
                Case { row: 1, base: 5, pattern: "OXX.X.O", index: 2,
                       before: vec![], after: vec![vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)]] },
                Case { row: 1, base: 5, pattern: "OXX.X.O", index: 4,
                       before: vec![], after: vec![vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)]] },
                // X.XX.
                Case { row: 1, base: 5, pattern: "OX.XX.O", index: 1,
                       before: vec![], after: vec![vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)]] },
                Case { row: 1, base: 5, pattern: "OX.XX.O", index: 3,
                       before: vec![], after: vec![vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)]] },
                Case { row: 1, base: 5, pattern: "OX.XX.O", index: 4,
                       before: vec![], after: vec![vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)]] },
                // .XXX.
                Case { row: 1, base: 5, pattern: "O.XXX.O", index: 2,
                       before: vec![], after: vec![vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)]] },
                Case { row: 1, base: 5, pattern: "O.XXX.O", index: 3,
                       before: vec![], after: vec![vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)]] },
                Case { row: 1, base: 5, pattern: "O.XXX.O", index: 4,
                       before: vec![], after: vec![vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)]] },
                // X..XX
                Case { row: 1, base: 5, pattern: "OX..XXO", index: 1,
                       before: vec![], after: vec![vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)]] },
                Case { row: 1, base: 5, pattern: "OX..XXO", index: 4,
                       before: vec![], after: vec![vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)]] },
                Case { row: 1, base: 5, pattern: "OX..XXO", index: 5,
                       before: vec![], after: vec![vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)]] },
                // X.X.X
                Case { row: 1, base: 5, pattern: "OX.X.XO", index: 1,
                       before: vec![], after: vec![vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)]] },
                Case { row: 1, base: 5, pattern: "OX.X.XO", index: 3,
                       before: vec![], after: vec![vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)]] },
                Case { row: 1, base: 5, pattern: "OX.X.XO", index: 5,
                       before: vec![], after: vec![vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)]] },
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
                Case { row: 5, base: 5, pattern: "..XXX..", index: 2,
                       before: vec![], after: vec![vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9), (5, 10), (5, 11)]] },
                Case { row: 5, base: 5, pattern: "..XXX..", index: 3,
                       before: vec![], after: vec![vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9), (5, 10), (5, 11)]] },
                // .XXX..
                Case { row: 5, base: 5, pattern: "O.XXX..O", index: 2,
                       before: vec![], after: vec![vec![(5, 6), (5, 7), (5, 8), (5, 9), (5, 10), (5, 11)]] },
                Case { row: 5, base: 5, pattern: "O.XXX..O", index: 3,
                       before: vec![], after: vec![vec![(5, 6), (5, 7), (5, 8), (5, 9), (5, 10), (5, 11)]] },
                Case { row: 5, base: 5, pattern: "O.XXX..O", index: 4,
                       before: vec![], after: vec![vec![(5, 6), (5, 7), (5, 8), (5, 9), (5, 10), (5, 11)]] },
                // .XX.X.
                Case { row: 5, base: 5, pattern: "O.XX.X.O", index: 2,
                       before: vec![], after: vec![vec![(5, 6), (5, 7), (5, 8), (5, 9), (5, 10), (5, 11)]] },
                Case { row: 5, base: 5, pattern: "O.XX.X.O", index: 3,
                       before: vec![], after: vec![vec![(5, 6), (5, 7), (5, 8), (5, 9), (5, 10), (5, 11)]] },
                Case { row: 5, base: 5, pattern: "O.XX.X.O", index: 5,
                       before: vec![], after: vec![vec![(5, 6), (5, 7), (5, 8), (5, 9), (5, 10), (5, 11)]] },
            ],
        );
    }

    #[test]
    fn block_four() {
        run_cases(
            setup_window_add_stone,
            PatternKind::BlockFour,
            vec![
                // O.XXXXO -> .XXXXO
                Case { row: 5, base: 4, pattern: "O.XXXXO", index: 2,
                       before: vec![], after: vec![vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9), (5, 10)]] },
                Case { row: 5, base: 4, pattern: "O.XXXXO", index: 3,
                       before: vec![], after: vec![vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9), (5, 10)]] },
                Case { row: 5, base: 4, pattern: "O.XXXXO", index: 4,
                       before: vec![], after: vec![vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9), (5, 10)]] },
                Case { row: 5, base: 4, pattern: "O.XXXXO", index: 5,
                       before: vec![], after: vec![vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9), (5, 10)]] },
                // OX.XXXO -> X.XXX
                Case { row: 5, base: 4, pattern: "OX.XXXO", index: 1,
                       before: vec![], after: vec![vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9)]] },
                Case { row: 5, base: 4, pattern: "OX.XXXO", index: 3,
                       before: vec![], after: vec![vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9)]] },
                Case { row: 5, base: 4, pattern: "OX.XXXO", index: 4,
                       before: vec![], after: vec![vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9)]] },
                Case { row: 5, base: 4, pattern: "OX.XXXO", index: 5,
                       before: vec![], after: vec![vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9)]] },
                // OXX.XXO -> XX.XX
                Case { row: 5, base: 4, pattern: "OXX.XXO", index: 1,
                       before: vec![], after: vec![vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9)]] },
                Case { row: 5, base: 4, pattern: "OXX.XXO", index: 2,
                       before: vec![], after: vec![vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9)]] },
            ],
        );
    }

    #[test]
    fn open_four() {
        run_cases(
            setup_window_add_stone,
            PatternKind::OpenFour,
            vec![
                // O.XXXX.O
                Case { row: 5, base: 4, pattern: "O.XXXX.O", index: 2,
                       before: vec![], after: vec![vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9), (5, 10)]] },
                Case { row: 5, base: 4, pattern: "O.XXXX.O", index: 3,
                       before: vec![], after: vec![vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9), (5, 10)]] },
            ],
        );
    }

    #[test]
    fn five_row() {
        run_cases(
            setup_window_add_stone,
            PatternKind::FiveRow,
            vec![
                // XXXXX 완성 -> five_row 등록
                Case { row: 5, base: 5, pattern: "XXXXX", index: 4,
                       before: vec![], after: vec![vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9)]] },
            ],
        );
    }
}

//TODO: Etienne does thsi
mod remove_stone_remove_pattern {
    use super::*;

    #[test]
    fn open_two() {
        run_cases(
            setup_window_add_stone,
            PatternKind::OpenTwo,
            vec![
                // ..XX..
                Case { row: 5, base: 5, pattern: "..XX..", index: 2,
                       before: vec![], after: vec![vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9), (5, 10)]] },
                // ..XX.O
                Case { row: 5, base: 5, pattern: "..XX.O", index: 2,
                       before: vec![], after: vec![vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9)]] },
                Case { row: 5, base: 5, pattern: "..XX.O", index: 3,
                       before: vec![], after: vec![vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9)]] },
                // O.X.X.O
                Case { row: 5, base: 5, pattern: "O.X.X.O", index: 2,
                       before: vec![], after: vec![vec![(5, 6), (5, 7), (5, 8), (5, 9), (5, 10)]] },
            ],
        );
    }

    #[test]
    fn open_three() {
        run_cases(
            setup_window_add_stone,
            PatternKind::OpenThree,
            vec![
                // XXX..
                Case { row: 5, base: 4, pattern: "OXXX..", index: 1,
                       before: vec![], after: vec![vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9)]] },
                Case { row: 5, base: 4, pattern: "OXXX..", index: 2,
                       before: vec![], after: vec![vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9)]] },
                Case { row: 5, base: 4, pattern: "OXXX..", index: 3,
                       before: vec![], after: vec![vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9)]] },
                // XX.X.
                Case { row: 1, base: 5, pattern: "OXX.X.O", index: 1,
                       before: vec![], after: vec![vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)]] },
                Case { row: 1, base: 5, pattern: "OXX.X.O", index: 2,
                       before: vec![], after: vec![vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)]] },
                Case { row: 1, base: 5, pattern: "OXX.X.O", index: 4,
                       before: vec![], after: vec![vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)]] },
                // X.XX.
                Case { row: 1, base: 5, pattern: "OX.XX.O", index: 1,
                       before: vec![], after: vec![vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)]] },
                Case { row: 1, base: 5, pattern: "OX.XX.O", index: 3,
                       before: vec![], after: vec![vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)]] },
                Case { row: 1, base: 5, pattern: "OX.XX.O", index: 4,
                       before: vec![], after: vec![vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)]] },
                // .XXX.
                Case { row: 1, base: 5, pattern: "O.XXX.O", index: 2,
                       before: vec![], after: vec![vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)]] },
                Case { row: 1, base: 5, pattern: "O.XXX.O", index: 3,
                       before: vec![], after: vec![vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)]] },
                Case { row: 1, base: 5, pattern: "O.XXX.O", index: 4,
                       before: vec![], after: vec![vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)]] },
                // X..XX
                Case { row: 1, base: 5, pattern: "OX..XXO", index: 1,
                       before: vec![], after: vec![vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)]] },
                Case { row: 1, base: 5, pattern: "OX..XXO", index: 4,
                       before: vec![], after: vec![vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)]] },
                Case { row: 1, base: 5, pattern: "OX..XXO", index: 5,
                       before: vec![], after: vec![vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)]] },
                // X.X.X
                Case { row: 1, base: 5, pattern: "OX.X.XO", index: 1,
                       before: vec![], after: vec![vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)]] },
                Case { row: 1, base: 5, pattern: "OX.X.XO", index: 3,
                       before: vec![], after: vec![vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)]] },
                Case { row: 1, base: 5, pattern: "OX.X.XO", index: 5,
                       before: vec![], after: vec![vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)]] },
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
                Case { row: 5, base: 5, pattern: "..XXX..", index: 2,
                       before: vec![], after: vec![vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9), (5, 10), (5, 11)]] },
                Case { row: 5, base: 5, pattern: "..XXX..", index: 3,
                       before: vec![], after: vec![vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9), (5, 10), (5, 11)]] },
                // .XXX..
                Case { row: 5, base: 5, pattern: "O.XXX..O", index: 2,
                       before: vec![], after: vec![vec![(5, 6), (5, 7), (5, 8), (5, 9), (5, 10), (5, 11)]] },
                Case { row: 5, base: 5, pattern: "O.XXX..O", index: 3,
                       before: vec![], after: vec![vec![(5, 6), (5, 7), (5, 8), (5, 9), (5, 10), (5, 11)]] },
                Case { row: 5, base: 5, pattern: "O.XXX..O", index: 4,
                       before: vec![], after: vec![vec![(5, 6), (5, 7), (5, 8), (5, 9), (5, 10), (5, 11)]] },
                // .XX.X.
                Case { row: 5, base: 5, pattern: "O.XX.X.O", index: 2,
                       before: vec![], after: vec![vec![(5, 6), (5, 7), (5, 8), (5, 9), (5, 10), (5, 11)]] },
                Case { row: 5, base: 5, pattern: "O.XX.X.O", index: 3,
                       before: vec![], after: vec![vec![(5, 6), (5, 7), (5, 8), (5, 9), (5, 10), (5, 11)]] },
                Case { row: 5, base: 5, pattern: "O.XX.X.O", index: 5,
                       before: vec![], after: vec![vec![(5, 6), (5, 7), (5, 8), (5, 9), (5, 10), (5, 11)]] },
            ],
        );
    }

    #[test]
    fn block_four() {
        run_cases(
            setup_window_add_stone,
            PatternKind::BlockFour,
            vec![
                // O.XXXXO -> .XXXXO
                Case { row: 5, base: 4, pattern: "O.XXXXO", index: 2,
                       before: vec![], after: vec![vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9), (5, 10)]] },
                Case { row: 5, base: 4, pattern: "O.XXXXO", index: 3,
                       before: vec![], after: vec![vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9), (5, 10)]] },
                Case { row: 5, base: 4, pattern: "O.XXXXO", index: 4,
                       before: vec![], after: vec![vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9), (5, 10)]] },
                Case { row: 5, base: 4, pattern: "O.XXXXO", index: 5,
                       before: vec![], after: vec![vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9), (5, 10)]] },
                // OX.XXXO -> X.XXX
                Case { row: 5, base: 4, pattern: "OX.XXXO", index: 1,
                       before: vec![], after: vec![vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9)]] },
                Case { row: 5, base: 4, pattern: "OX.XXXO", index: 3,
                       before: vec![], after: vec![vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9)]] },
                Case { row: 5, base: 4, pattern: "OX.XXXO", index: 4,
                       before: vec![], after: vec![vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9)]] },
                Case { row: 5, base: 4, pattern: "OX.XXXO", index: 5,
                       before: vec![], after: vec![vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9)]] },
                // OXX.XXO -> XX.XX
                Case { row: 5, base: 4, pattern: "OXX.XXO", index: 1,
                       before: vec![], after: vec![vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9)]] },
                Case { row: 5, base: 4, pattern: "OXX.XXO", index: 2,
                       before: vec![], after: vec![vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9)]] },
            ],
        );
    }

    #[test]
    fn open_four() {
        run_cases(
            setup_window_add_stone,
            PatternKind::OpenFour,
            vec![
                // O.XXXX.O
                Case { row: 5, base: 4, pattern: "O.XXXX.O", index: 2,
                       before: vec![], after: vec![vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9), (5, 10)]] },
                Case { row: 5, base: 4, pattern: "O.XXXX.O", index: 3,
                       before: vec![], after: vec![vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9), (5, 10)]] },
            ],
        );
    }

    #[test]
    fn five_row() {
        run_cases(
            setup_window_add_stone,
            PatternKind::FiveRow,
            vec![
                // XXXXX 완성 -> five_row 등록
                Case { row: 5, base: 5, pattern: "XXXXX", index: 4,
                       before: vec![], after: vec![vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9)]] },
            ],
        );
    }
}
