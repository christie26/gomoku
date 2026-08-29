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

fn setup_window_new_stone(row: i32, base: i32, pattern: &str, new_index: usize) -> (Gomoku, PlayerPatterns) {
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

mod new_move_add {
    use super::*;

    #[test]
    fn open_two() {
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
            let (game, before) = setup_window_new_stone(5, 5, pattern, new_index);
            if !before.open_two.is_empty() {
                failures.push(format!("{pattern:?} idx={new_index}: we found pattern before last move"));
                continue;
            }
            let actual = &black_patterns(&game).open_two;
            if *actual != vec![expected] {
                failures.push(format!("{pattern:?} idx={new_index}: different from expected: {actual:?}"));
            }
        }
        assert!(failures.is_empty(), "\n{}", failures.join("\n"));
    }

    #[test]
    fn open_three() {
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
            let (game, before) = setup_window_new_stone(row, base, pattern, new_index);
            if !before.open_three.is_empty() {
                failures.push(format!("{pattern:?} idx={new_index}: we found pattern before last move"));
                continue;
            }

            let actual = &black_patterns(&game).open_three;
            if *actual != vec![expected] {
                failures.push(format!("{pattern:?} idx={new_index}: different from expected: {actual:?}"));
            }
        }
        assert!(failures.is_empty(), "\n{}", failures.join("\n"));
    }

    #[test]
    fn free_three() {
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
            let (game, before) = setup_window_new_stone(row, base, pattern, new_index);
            if !before.free_three.is_empty() {
                failures.push(format!("{pattern:?} idx={new_index}: we found pattern before last move"));
                continue;
            }

            let actual = &black_patterns(&game).free_three;
            if *actual != vec![expected] {
                failures.push(format!("{pattern:?} idx={new_index}: different from expected: {actual:?}"));
            }
        }
        assert!(failures.is_empty(), "\n{}", failures.join("\n"));
    }

    #[test]
    fn block_four() {
        let cases: Vec<(i32, i32, &str, usize, Vec<Position>)> = vec![
            // O.XXXXO -> .XXXXO
            (5, 4, "O.XXXXO", 2, vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9), (5, 10)]),
            (5, 4, "O.XXXXO", 3, vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9), (5, 10)]),
            (5, 4, "O.XXXXO", 4, vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9), (5, 10)]),
            (5, 4, "O.XXXXO", 5, vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9), (5, 10)]),
            (5, 4, "O.XXXXO", 6, vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9), (5, 10)]),
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
            let (game, before) = setup_window_new_stone(row, base, pattern, new_index);
            if !before.block_four.is_empty() {
                failures.push(format!("{pattern:?} idx={new_index}: we found pattern before last move"));
                continue;
            }

            let actual = &black_patterns(&game).block_four;
            if *actual != vec![expected] {
                failures.push(format!("{pattern:?} idx={new_index}: different from expected: {actual:?}"));
            }
        }
        assert!(failures.is_empty(), "\n{}", failures.join("\n"));
    }

    #[test]
    fn open_four() {
        let cases: Vec<(i32, i32, &str, usize, Vec<Position>)> = vec![
            // O.XXXX.O
            (5, 4, "O.XXXX.O", 2, vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9), (5, 10)]),
            (5, 4, "O.XXXX.O", 3, vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9), (5, 10)]),
        ];

        let mut failures = Vec::new();
        for (row, base, pattern, new_index, expected) in cases {
            let (game, before) = setup_window_new_stone(row, base, pattern, new_index);
            if !before.open_four.is_empty() {
                failures.push(format!("{pattern:?} idx={new_index}: we found pattern before last move"));
                continue;
            }

            let actual = &black_patterns(&game).open_four;
            if *actual != vec![expected] {
                failures.push(format!("{pattern:?} idx={new_index}: different from expected: {actual:?}"));
            }
        }
        assert!(failures.is_empty(), "\n{}", failures.join("\n"));
    }

    #[test]
    fn five_row() {
        // XXXXX 완성 -> five_row 등록
        let (game, before) = setup_window_new_stone(5, 5, "XXXXX", 4);
        assert!(before.five_row.is_empty());

        let expected = vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9)];
        assert_eq!(black_patterns(&game).five_row, vec![expected]);
    }
}

mod new_move_remove {
  use super::*;
  #[test]
  fn block_four() {
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
      for (row, base, pattern, index, expected) in cases {
          let (mut game, _) = setup_window_remove_stone(row, base, pattern, index);
          if black_patterns(&game).block_four != vec![expected.clone()] {
              failures.push(format!(
                  "{pattern:?} idx={index}: 제거 이전에 등록되어 있어야 하는데 {:?}",
                  black_patterns(&game).block_four
              ));
              continue;
          }
  
          let actual = &black_patterns(&game).block_four;
          if !actual.is_empty() {
              failures.push(format!("{pattern:?} idx={index}: 제거 후에도 등록이 남아있음: {actual:?}"));
          }
      }
      assert!(failures.is_empty(), "\n{}", failures.join("\n"));
  }
}

