use super::*;

// 테스트 전용 헬퍼: player를 강제로 세팅하고 수를 둔다. Valid가 아니면 바로 실패시켜서
// (더블쓰리 등으로) 세팅 자체가 잘못됐을 때 조용히 넘어가지 않게 한다.
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

fn black_patterns(game: &Gomoku) -> &PlayerPatterns {
    game.patterns.get(&Stone::Black).unwrap()
}

// 테스트 보드 생성 헬퍼. 'X'=Black, 'O'=White(벽), '.'=빈칸.
// add_patterns_for_move는 "방금 둔 돌" 위치에서만 스캔하므로, new_index가 가리키는 'X'을
// 맨 마지막에 둬서 그게 classify()를 트리거하는 "새 수"가 되게 한다. 벽('O')은 항상 먼저 둔다.
// new_index를 두기 직전의 패턴 스냅샷도 같이 반환해서, 테스트가 "그 수가 실제로 패턴을
// 등록시켰는지"(이전엔 없다가 이후에 생겼는지)를 확인할 수 있게 한다.
// 캡처처럼 "감싸는 쪽/감싸이는 쪽" 순서 자체가 다른 케이스는 이 헬퍼로 표현 안 되니 수동으로 놓는다.
fn setup_window(row: i32, base: i32, pattern: &str, new_index: usize) -> (Gomoku, PlayerPatterns) {
    let mut game = Gomoku::new(19);
    for (i, ch) in pattern.chars().enumerate() {
        if ch == 'O' {
            place(&mut game, Stone::White, row, base + i as i32);
        } else if ch == 'X' && i != new_index {
            place(&mut game, Stone::Black, row, base + i as i32);
        }
    }
    let before = black_patterns(&game).clone();
    place(&mut game, Stone::Black, row, base + new_index as i32);
    (game, before)
}

#[test]
fn register_from_move_open_two_variants() {
    let cases: Vec<(&str, usize, Option<Vec<Position>>)> = vec![
        // ..XX..
        ("..XX..", 2, Some(vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9), (5, 10)])),
        // ..XX.O
        ("..XX.O", 2, Some(vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9)])),
        ("..XX.O", 3, Some(vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9)])),
        // O.X.X.O
        ("O.X.X.O", 2, Some(vec![(5, 6), (5, 7), (5, 8), (5, 9), (5, 10)])),
    ];

    let mut failures = Vec::new();
    for (pattern, new_index, expected) in cases {
        let (game, before) = setup_window(5, 5, pattern, new_index);
        if !before.open_two.is_empty() {
            failures.push(format!("{pattern:?} idx={new_index}: 마지막 수 이전에 이미 등록됨"));
            continue;
        }

        let actual = &black_patterns(&game).open_two;
        match expected {
            Some(expected) => {
                if *actual != vec![expected] {
                    failures.push(format!("{pattern:?} idx={new_index}: 등록된 결과가 기대값과 다름: {actual:?}"));
                }
            }
            None => {
                if !actual.is_empty() {
                    failures.push(format!("{pattern:?} idx={new_index}: 등록되지 않아야 하는데 {actual:?}"));
                }
            }
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

#[test]
fn register_from_move_open_three_variants() {
    let cases: Vec<(i32, i32, &str, usize, Option<Vec<Position>>)> = vec![
        // XXX..
        (5, 4, "OXXX..", 1, Some(vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9)])),
        (5, 4, "OXXX..", 2, Some(vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9)])),
        (5, 4, "OXXX..", 3, Some(vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9)])),
        // XX.X.
        (1, 5, "OXX.X.O", 1, Some(vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)])),
        (1, 5, "OXX.X.O", 2, Some(vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)])),
        (1, 5, "OXX.X.O", 4, Some(vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)])),
        // X.XX.
        (1, 5, "OX.XX.O", 1, Some(vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)])),
        (1, 5, "OX.XX.O", 3, Some(vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)])),
        (1, 5, "OX.XX.O", 4, Some(vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)])),
        // .XXX.
        (1, 5, "O.XXX.O", 2, Some(vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)])),
        (1, 5, "O.XXX.O", 3, Some(vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)])),
        (1, 5, "O.XXX.O", 4, Some(vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)])),
        // X..XX
        (1, 5, "OX..XXO", 1, Some(vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)])),
        (1, 5, "OX..XXO", 4, Some(vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)])),
        (1, 5, "OX..XXO", 5, Some(vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)])),
        // X.X.X
        (1, 5, "OX.X.XO", 1, Some(vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)])),
        (1, 5, "OX.X.XO", 3, Some(vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)])),
        (1, 5, "OX.X.XO", 5, Some(vec![(1, 6), (1, 7), (1, 8), (1, 9), (1, 10)])),
    ];

    let mut failures = Vec::new();
    for (row, base, pattern, new_index, expected) in cases {
        let (game, before) = setup_window(row, base, pattern, new_index);
        if !before.open_three.is_empty() {
            failures.push(format!("{pattern:?} idx={new_index}: 마지막 수 이전에 이미 등록됨"));
            continue;
        }

        let actual = &black_patterns(&game).open_three;
        match expected {
            Some(expected) => {
                if *actual != vec![expected] {
                    failures.push(format!("{pattern:?} idx={new_index}: 등록된 결과가 기대값과 다름: {actual:?}"));
                }
            }
            None => {
                if !actual.is_empty() {
                    failures.push(format!("{pattern:?} idx={new_index}: 등록되지 않아야 하는데 {actual:?}"));
                }
            }
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

#[test]
fn register_from_move_free_three_variants() {
    let cases: Vec<(i32, i32, &str, usize, Vec<Position>)> = vec![
        // ..XXX..
        (5, 5, "..XXX..", 2, vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9), (5, 10), (5, 11)]),
        (5, 5, "..XXX..", 3, vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9), (5, 10), (5, 11)]),
        // .XXX..
        (5, 5, "O.XXX..O", 2, vec![(5, 6), (5, 7), (5, 8), (5, 9), (5, 10), (5, 11)]),
        (5, 5, "O.XXX..O", 3, vec![(5, 6), (5, 7), (5, 8), (5, 9), (5, 10), (5, 11)]),
        (5, 5, "O.XXX..O", 4, vec![(5, 6), (5, 7), (5, 8), (5, 9), (5, 10), (5, 11)]),
        // .XX.X.
        (5, 5, "O.XX.X.O", 2, vec![(5, 6), (5, 7), (5, 8), (5, 9), (5, 10), (5, 11)]),
        (5, 5, "O.XX.X.O", 3, vec![(5, 6), (5, 7), (5, 8), (5, 9), (5, 10), (5, 11)]),
        (5, 5, "O.XX.X.O", 5, vec![(5, 6), (5, 7), (5, 8), (5, 9), (5, 10), (5, 11)]),
    ];

    let mut failures = Vec::new();
    for (row, base, pattern, new_index, expected) in cases {
        let (game, before) = setup_window(row, base, pattern, new_index);
        if !before.free_three.is_empty() {
            failures.push(format!("{pattern:?} idx={new_index}: 마지막 수 이전에 이미 등록됨"));
            continue;
        }

        let actual = &black_patterns(&game).free_three;
        if *actual != vec![expected] {
            failures.push(format!("{pattern:?} idx={new_index}: 등록된 결과가 기대값과 다름: {actual:?}"));
        }
        if !black_patterns(&game).open_three.is_empty() {
            failures.push(format!("{pattern:?} idx={new_index}: free_three와 open_three가 동시에 등록됨"));
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

#[test]
fn register_from_move_block_four_variants() {
    let cases: Vec<(i32, i32, &str, usize, Vec<Position>)> = vec![
        // O.XXXXO -> .XXXXO
        (5, 4, "O.XXXXO", 2, vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9), (5, 10)]),
        (5, 4, "O.XXXXO", 3, vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9), (5, 10)]),
        (5, 4, "O.XXXXO", 4, vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9), (5, 10)]),
        (5, 4, "O.XXXXO", 5, vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9), (5, 10)]),
        // OX.XXXO -> X.XXX
        (5, 4, "OX.XXXO", 1, vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9)]),
        (5, 4, "OX.XXXO", 3, vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9)]),
        (5, 4, "OX.XXXO", 4, vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9)]),
        (5, 4, "OX.XXXO", 5, vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9)]),
        // OXX.XXO -> XX.XX
        (5, 4, "OXX.XXO", 1, vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9)]),
        (5, 4, "OXX.XXO", 2, vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9)]),
    ];

    let mut failures = Vec::new();
    for (row, base, pattern, new_index, expected) in cases {
        let (game, before) = setup_window(row, base, pattern, new_index);
        if !before.block_four.is_empty() {
            failures.push(format!("{pattern:?} idx={new_index}: 마지막 수 이전에 이미 등록됨"));
            continue;
        }

        let actual = &black_patterns(&game).block_four;
        if *actual != vec![expected] {
            failures.push(format!("{pattern:?} idx={new_index}: 등록된 결과가 기대값과 다름: {actual:?}"));
        }
        if !black_patterns(&game).open_four.is_empty() {
            failures.push(format!("{pattern:?} idx={new_index}: block_four와 open_four가 동시에 등록됨"));
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

#[test]
fn register_from_move_open_four_variants() {
    let cases: Vec<(i32, i32, &str, usize, Vec<Position>)> = vec![
        // O.XXXX.O
        (5, 4, "O.XXXX.O", 2, vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9), (5, 10)]),
        (5, 4, "O.XXXX.O", 3, vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9), (5, 10)]),
    ];

    let mut failures = Vec::new();
    for (row, base, pattern, new_index, expected) in cases {
        let (game, before) = setup_window(row, base, pattern, new_index);
        if !before.open_four.is_empty() {
            failures.push(format!("{pattern:?} idx={new_index}: 마지막 수 이전에 이미 등록됨"));
            continue;
        }

        let actual = &black_patterns(&game).open_four;
        if *actual != vec![expected] {
            failures.push(format!("{pattern:?} idx={new_index}: 등록된 결과가 기대값과 다름: {actual:?}"));
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

#[test]
fn register_from_move_five_row() {
    // XXXXX 완성 -> five_row 등록, 직전 open_four는 사라져야 한다 (이중 집계 방지).
    let (game, before) = setup_window(5, 5, "XXXXX", 4);
    assert!(before.five_row.is_empty());

    let expected = vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9)];
    assert_eq!(black_patterns(&game).five_row, vec![expected]);
    assert!(black_patterns(&game).open_four.is_empty());
    assert!(black_patterns(&game).block_four.is_empty());
}

