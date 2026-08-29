use super::*;

fn place(game: &mut Gomoku, player: Stone, x: i32, y: i32) {
    game.current_player = player;
    game.opponent_player = match player {
        Stone::Black => Stone::White,
        Stone::White => Stone::Black,
        Stone::Empty => panic!("Empty는 플레이어가 될 수 없다"),
    };
    let (result, _, _) = game.handle_move(x, y);
    assert_eq!(result, MoveResult::Valid, "({x},{y}) {:?} 착수가 실패함", player);
}
fn remove(game: &mut Gomoku, x: i32, y: i32) {
    assert_ne!(
        game.board[x as usize][y as usize],
        Stone::Empty,
        "({x},{y})에는 지울 돌이 없음"
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
        other => panic!("add_index={add_index}는 돌 자리가 아님: {other:?}"),
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

mod add_stone_remove_pattern {
    use super::*;

    #[test]
    fn open_two() {
        run_cases(
            setup_window_add_stone,
            PatternKind::OpenTwo,
            vec![
                // ..XX.. 
                Case { row: 5, base: 5, pattern: "..XXO.", index: 4,
                       before: vec![vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9), (5, 10)]], after: vec![] },
                // ..XX.O 
                Case { row: 5, base: 5, pattern: "..XXOO", index: 4,
                       before: vec![vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9)]], after: vec![] },
                Case { row: 5, base: 5, pattern: ".OXX.O", index: 2,
                       before: vec![vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9)]], after: vec![] },
                // O.X.X.O
                Case { row: 5, base: 5, pattern: "O.XOX.O", index: 3,
                       before: vec![vec![(5, 6), (5, 7), (5, 8), (5, 9), (5, 10)]], after: vec![] },
                Case { row: 5, base: 5, pattern: "OOX.X.O", index: 2,
                       before: vec![vec![(5, 6), (5, 7), (5, 8), (5, 9), (5, 10)]], after: vec![] },
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
                Case { row: 5, base: 4, pattern: "OXXXO.", index: 4,
                       before: vec![vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9)]]  , after: vec![]},
                Case { row: 5, base: 4, pattern: "OXXX.O", index: 5,
                       before: vec![vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9)]]  , after: vec![]},
                // XX.X.
                Case { row: 1, base: 5, pattern: "OXXOX.O", index: 3,
                       before: vec![vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)]] , after: vec![] },
                Case { row: 1, base: 5, pattern: "OXX.XOO", index: 5,
                       before: vec![vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)]] , after: vec![] },
                // X.XX.
                Case { row: 1, base: 5, pattern: "OXOXX.O", index: 2,
                       before: vec![vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)]] , after: vec![] },
                Case { row: 1, base: 5, pattern: "OX.XXOO", index: 5,
                       before: vec![vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)]] , after: vec![] },
                // .XXX.
                Case { row: 1, base: 5, pattern: "OOXXX.O", index: 1,
                       before: vec![vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)]] , after: vec![] },
                // X..XX
                Case { row: 1, base: 5, pattern: "OXO.XXO", index: 2,
                       before: vec![vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)]] , after: vec![] },
                Case { row: 1, base: 5, pattern: "OX.OXXO", index: 3,
                       before: vec![vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)]] , after: vec![] },
                // X.X.X
                Case { row: 1, base: 5, pattern: "OXOX.XO", index: 2,
                       before: vec![vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)]] , after: vec![] },
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
