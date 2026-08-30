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

fn encode_short_coord((x, y): Position) -> String {
    format!("{}{}", (b'a' + x as u8) as char, y + 1)
}

fn encode_patterns(patterns: &[Pattern]) -> String {
    let patterns: Vec<String> = patterns
        .iter()
        .map(|p| {
            let coords: Vec<String> = p.iter().map(|&pos| encode_short_coord(pos)).collect();
            format!("[{}]", coords.join(", "))
        })
        .collect();
    format!("[{}]", patterns.join(", "))
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
                "{:?} idx={}: before expected: {}, what we got: {}",
                case.pattern, case.index,
                encode_patterns(&case.before), encode_patterns(actual_before)
            ));
            continue;
        }

        let actual_after = patterns_of(black_patterns(&game), kind);
        if *actual_after != case.after {
            failures.push(format!(
                "{:?} idx={}: after expected: {}, what we got: {}",
                case.pattern, case.index,
                encode_patterns(&case.after), encode_patterns(actual_after)
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
                Case::new("..XX..", 2, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]]),
                // ..XX.O
                Case::new("..XX.O", 2, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14"]]),
                Case::new("..XX.O", 3, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14"]]),
                Case::new("..XX.O", 5, vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]], 
                                       vec![vec!["j10", "j11", "j12", "j13", "j14"]]),
                // O.X.X.O
                Case::new("O.X.X.O", 0, vec![vec!["j10","j11", "j12", "j13", "j14", "j15"]], 
                                        vec![vec!["j11", "j12", "j13", "j14", "j15"]]),
                Case::new("O.X.X.O", 2, vec![], vec![vec!["j11", "j12", "j13", "j14", "j15"]]),
                // NOTE - we cannot detect 5 empty space
                // ..X.X..
                Case::new("..X.X..", 2, vec![], vec![vec!["j10","j11", "j12", "j13", "j14", "j15"]]),
                Case::new("..X.X..", 4, vec![], vec![vec!["j11", "j12", "j13", "j14", "j15", "j16"]]),
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
                // TODO
                // // X..XX
                // Case::new("X..XX", 0, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14"]]),
                // Case::new("X..XX", 3, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14"]]),
                // Case::new("X..XX", 4, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14"]]),
                // // X.X.X
                // Case::new("X.X.X", 0, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14"]]),
                // Case::new("X.X.X", 2, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14"]]),
                // Case::new("X.X.X", 4, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14"]]),
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
                Case::new("O.XXX..", 0, vec![vec!["j10", "j11", "j12", "j13", "j14", "j15", "j16"]], 
                                        vec![vec!["j11", "j12", "j13", "j14", "j15", "j16"]]),
                Case::new("O.XXX..", 2, vec![], vec![vec!["j11", "j12", "j13", "j14", "j15", "j16"]]),
                Case::new("O.XXX..", 3, vec![], vec![vec!["j11", "j12", "j13", "j14", "j15", "j16"]]),
                Case::new("O.XXX..", 4, vec![], vec![vec!["j11", "j12", "j13", "j14", "j15", "j16"]]),
                // .XX.X.
                Case::new(".XX.X.", 1, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]]),
                Case::new(".XX.X.", 2, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]]),
                Case::new(".XX.X.", 4, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]]),

                // Should not be counted as a free three
                // OXX.X.
                Case::new("OXX.X.", 0, vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]], vec![]),
                Case::new("OXX.X.", 1, vec![], vec![]),
                Case::new("OXX.X.", 2, vec![], vec![]),
                Case::new("OXX.X.", 4, vec![], vec![]),

                // OX.XX.
                Case::new("OX.XX.", 0, vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]], vec![]),
                Case::new("OX.XX.", 1, vec![], vec![]),
                Case::new("OX.XX.", 3, vec![], vec![]),
                Case::new("OX.XX.", 4, vec![], vec![]),
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
                Case::new("XXXXX", 0, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14"]]),
                Case::new("XXXXX", 1, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14"]]),
                Case::new("XXXXX", 2, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14"]]),
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
                Case::new("..XX.O", 5, vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]], 
                                       vec![vec!["j10", "j11", "j12", "j13", "j14"]]),
                // ..XX.O 
                Case::new("O.XX.O", 0, vec![vec!["j10", "j11", "j12", "j13", "j14"]], vec![vec!["j11", "j12", "j13", "j14"]]),
                Case::new(".OXX.O", 1, vec![vec!["j10", "j11", "j12", "j13", "j14"]], vec![]),
                Case::new("..XXOO", 4, vec![vec!["j10", "j11", "j12", "j13", "j14"]], vec![]),
                // O.X.X.O
                Case::new("OOX.X.O", 1, vec![vec!["j11", "j12", "j13", "j14", "j15"]], vec![]),
                Case::new("O.XOX.O", 3, vec![vec!["j11", "j12", "j13", "j14", "j15"]], vec![]),
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
                Case::new("OXXXO.", 4, vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]], vec![]),
                Case::new("OXXX.O", 5, vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]], vec![]),
                // OXX.X.
                Case::new("OXXOX.O", 3, vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]], vec![]),
                Case::new("OXX.XOO", 5, vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]], vec![]),
                // OX.XX.
                Case::new("OXOXX.O", 2, vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]], vec![]),
                Case::new("OX.XXOO", 5, vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]], vec![]),
                // O.XXX.O
                Case::new("OOXXX.O", 1, vec![vec!["j10", "j11", "j12", "j13", "j14", "j15", "j16"]], vec![]),
                // TODO
                // // X..XX
                // Case::new("OXO.XXO", 2, vec![vec!["j11", "j12", "j13", "j14", "j15"]], vec![]),
                // Case::new("OX.OXXO", 3, vec![vec!["j11", "j12", "j13", "j14", "j15"]], vec![]),
                // // X.X.X
                // Case::new("OXOX.XO", 2, vec![vec!["j11", "j12", "j13", "j14", "j15"]], vec![]),
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
                Case::new("O.XXX..", 0,  vec![vec!["j10", "j11", "j12", "j13", "j14", "j15", "j16"]], vec![vec!["j11", "j12", "j13", "j14", "j15", "j16"]]),
                Case::new(".OXXX..", 1,  vec![vec!["j10", "j11", "j12", "j13", "j14", "j15", "j16"]], vec![]),
                // O.XXX..
                Case::new("OOXXX..", 1,  vec![vec!["j11", "j12", "j13", "j14", "j15", "j16"]], vec![]),
                Case::new("O.XXXO.", 5,  vec![vec!["j11", "j12", "j13", "j14", "j15", "j16"]], vec![]),
                Case::new("O.XXX.O", 6,  vec![vec!["j11", "j12", "j13", "j14", "j15", "j16"]], vec![]),
                // .XX.X.
                Case::new("OXX.X.", 0,  vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]], vec![]),
                Case::new(".XXOX.", 3,  vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]], vec![]),
                Case::new(".XX.XO", 5,  vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]], vec![]),
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
                Case::new("OXXXXO", 0, vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]],vec![]),
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

mod remove_stone_add_pattern {
    use super::*;

    #[test]
    fn open_two() {
        run_cases(
            setup_window_remove_stone,
            PatternKind::OpenTwo,
            vec![
                // Removing 'O' at index 4 turns "..XXO." into "..XX.."
                Case::new("..XXO.", 4, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]]),
                // Removing 'O' at index 4 turns "..XXOO" into "..XX.O"
                Case::new("..XXOO", 4, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14"]]),
                Case::new(".OXX.O", 1, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14"]]),
                // Removing 'O' at index 3 turns "O.XOX.O" into "O.X.X.O"
                Case::new("O.XOX.O", 3, vec![], vec![vec!["j11", "j12", "j13", "j14", "j15"]]),
                Case::new("OOX.X.O", 1, vec![], vec![vec!["j11", "j12", "j13", "j14", "j15"]]),
            ],
        );
    }

    #[test]
    fn open_three() {
        run_cases(
            setup_window_remove_stone,
            PatternKind::OpenThree,
            vec![
                // Removing 'O' at index 4 turns "OXXXO." into "OXXX.."

                // becomes OXXX..
                Case::new("OXXXO.", 4, vec![], vec![vec!["j11", "j12", "j13", "j14", "j15"]]),
                Case::new("OXXX.O", 5, vec![], vec![vec!["j11", "j12", "j13", "j14", "j15"]]),

                // becomes OXX.X.O
                Case::new("OXXOX.O", 3, vec![], vec![vec!["j11", "j12", "j13", "j14", "j15"]]),
                Case::new("OXX.XOO", 5, vec![], vec![vec!["j11", "j12", "j13", "j14", "j15"]]),
                Case::new("OXOXX.O", 2, vec![], vec![vec!["j11", "j12", "j13", "j14", "j15"]]),
                Case::new("OX.XXOO", 5, vec![], vec![vec!["j11", "j12", "j13", "j14", "j15"]]),

                // becomes O.XXX.O
                Case::new("OOXXX.O", 1, vec![], vec![vec!["j11", "j12", "j13", "j14", "j15"]]),
            ],
        );
    }

    #[test]
    fn free_three() {
        run_cases(
            setup_window_remove_stone,
            PatternKind::FreeThree,
            vec![
                // Removing 'O' at index 0 turns "O.XXX.." into "..XXX.."
                Case::new("O.XXX..", 0, vec![vec!["j11", "j12", "j13", "j14", "j15", "j16"]], vec![vec!["j10", "j11", "j12", "j13", "j14", "j15", "j16"]]),
                Case::new(".OXXX..", 1, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14", "j15", "j16"]]),
                Case::new("OOXXX..O", 1, vec![], vec![vec!["j11", "j12", "j13", "j14", "j15", "j16"]]),
                Case::new("O.XXXO.O", 5, vec![], vec![vec!["j11", "j12", "j13", "j14", "j15", "j16"]]),
                Case::new("O.XXOX.O", 4, vec![], vec![vec!["j11", "j12", "j13", "j14", "j15"]]),
            ],
        );
    }

    #[test]
    fn block_four() {
        run_cases(
            setup_window_remove_stone,
            PatternKind::BlockFour,
            vec![
                // Removing 'O' at index 0 turns "OXXXX" into ".XXXX"
                // .XXXXO
                Case::new("OXXXXO", 0, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]]),
                // X.XXX
                Case::new("XOXXX", 1, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14"]]),
                // XX.XX
                Case::new("XXOXX", 2, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14"]]),
            ],
        );
    }

    #[test]
    fn open_four() {
        run_cases(
            setup_window_remove_stone,
            PatternKind::OpenFour,
            vec![
                // Removing 'O' at index 0 turns "OXXXX." into ".XXXX."
                Case::new("OXXXX.", 0, vec![], vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]]),
            ],
        );
    }

    #[test]
    fn five_row() {
        run_cases(
            setup_window_remove_stone,
            PatternKind::FiveRow,
            vec![],
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
                Case::new("..XX..", 2, vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]], vec![]),
                // ..XX.O
                Case::new("..XX.O", 2, vec![vec!["j10", "j11", "j12", "j13", "j14"]], vec![]),
                Case::new("..XX.O", 3, vec![vec!["j10", "j11", "j12", "j13", "j14"]], vec![]),
                Case::new("..XX.O", 5, vec![vec!["j10", "j11", "j12", "j13", "j14"]], 
                                       vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]]),
                // O.X.X.O
                Case::new("O.X.X.O", 0, vec![vec!["j11", "j12", "j13", "j14", "j15"]], 
                                        vec![vec!["j10","j11", "j12", "j13", "j14", "j15"]]),
                Case::new("O.X.X.O", 2, vec![vec!["j11", "j12", "j13", "j14", "j15"]], vec![]),
                // NOTE - we cannot detect 5 empty space
                // ..X.X..
                Case::new("..X.X..", 2, vec![vec!["j10","j11", "j12", "j13", "j14", "j15"]], vec![]),
                Case::new("..X.X..", 4, vec![vec!["j11", "j12", "j13", "j14", "j15", "j16"]], vec![]),
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
                Case::new("OXXX..", 0, vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]], vec![]),
                Case::new("OXXX..", 1, vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]], vec![]),
                Case::new("OXXX..", 2, vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]], vec![]),
                Case::new("OXXX..", 3, vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]], vec![]),
                // OXX.X.
                Case::new("OXX.X.", 0, vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]], vec![]),
                Case::new("OXX.X.", 1, vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]], vec![]),
                Case::new("OXX.X.", 2, vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]], vec![]),
                Case::new("OXX.X.", 4, vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]], vec![]),
                // OX.XX.
                Case::new("OX.XX.", 0, vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]], vec![]),
                Case::new("OX.XX.", 1, vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]], vec![]),
                Case::new("OX.XX.", 3, vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]], vec![]),
                Case::new("OX.XX.", 4, vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]], vec![]),
                // O.XXX.O
                Case::new("O.XXX.O", 0, vec![vec!["j10", "j11", "j12", "j13", "j14", "j15", "j16"]], vec![]),
                Case::new("O.XXX.O", 2, vec![vec!["j10", "j11", "j12", "j13", "j14", "j15", "j16"]], vec![]),
                Case::new("O.XXX.O", 3, vec![vec!["j10", "j11", "j12", "j13", "j14", "j15", "j16"]], vec![]),
                Case::new("O.XXX.O", 4, vec![vec!["j10", "j11", "j12", "j13", "j14", "j15", "j16"]], vec![]),
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
                Case::new("..XXX..", 2, vec![vec!["j10", "j11", "j12", "j13", "j14", "j15", "j16"]], vec![]),
                Case::new("..XXX..", 3, vec![vec!["j10", "j11", "j12", "j13", "j14", "j15", "j16"]], vec![]),
                // O.XXX..
                Case::new("O.XXX..", 0, vec![vec!["j11", "j12", "j13", "j14", "j15", "j16"]], 
                                        vec![vec!["j10", "j11", "j12", "j13", "j14", "j15", "j16"]]),
                Case::new("O.XXX..", 2, vec![vec!["j11", "j12", "j13", "j14", "j15", "j16"]], vec![]),
                Case::new("O.XXX..", 3, vec![vec!["j11", "j12", "j13", "j14", "j15", "j16"]], vec![]),
                Case::new("O.XXX..", 4, vec![vec!["j11", "j12", "j13", "j14", "j15", "j16"]], vec![]),
                // .XX.X.
                Case::new(".XX.X.", 1, vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]], vec![]),
                Case::new(".XX.X.", 2, vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]], vec![]),
                Case::new(".XX.X.", 4, vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]], vec![]),
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
                Case::new(".XXXXO", 1, vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]], vec![]),
                Case::new(".XXXXO", 2, vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]], vec![]),
                Case::new(".XXXXO", 3, vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]], vec![]),
                Case::new(".XXXXO", 4, vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]], vec![]),
                Case::new(".XXXXO", 5, vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]], vec![]),
                // X.XXX
                Case::new("X.XXX", 0, vec![vec!["j10", "j11", "j12", "j13", "j14"]], vec![]),
                Case::new("X.XXX", 2, vec![vec!["j10", "j11", "j12", "j13", "j14"]], vec![]),
                Case::new("X.XXX", 3, vec![vec!["j10", "j11", "j12", "j13", "j14"]], vec![]),
                Case::new("X.XXX", 4, vec![vec!["j10", "j11", "j12", "j13", "j14"]], vec![]),
                // XX.XX
                Case::new("XX.XX", 0, vec![vec!["j10", "j11", "j12", "j13", "j14"]], vec![]),
                Case::new("XX.XX", 1, vec![vec!["j10", "j11", "j12", "j13", "j14"]], vec![]),
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
                Case::new(".XXXX.", 1, vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]], vec![]),
                Case::new(".XXXX.", 2, vec![vec!["j10", "j11", "j12", "j13", "j14", "j15"]], vec![]),
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
                Case::new("XXXXX", 0, vec![vec!["j10", "j11", "j12", "j13", "j14"]], vec![]),
                Case::new("XXXXX", 1, vec![vec!["j10", "j11", "j12", "j13", "j14"]], vec![]),
                Case::new("XXXXX", 2, vec![vec!["j10", "j11", "j12", "j13", "j14"]], vec![]),
            ],
        );
    }
}
