//! Tactical regression tests for the search.
//!
//! These exist because unit tests on the pattern scanner cannot catch a search
//! that prunes away the only good move. Two real bugs were found this way:
//! a candidate cap that dropped the sole defensive move, and null-move pruning
//! that inverted the turn parity `get_winner` depends on. Both left the engine
//! scoring every move alike, so it picked one at random.
//!
//! Each case is run several times: the search is not deterministic (parallel
//! root split, and `RANDOMIZE_TIED_MOVES` picks among equal-scoring moves), and
//! both bugs showed up as *sometimes* playing a sensible move.

use lib_gomoku::minimax::{clear_transposition_table, get_ai_move_with_stats};
use lib_gomoku::{Gomoku, MoveResult};

const REPEATS: usize = 5;

fn position(moves: &[(i32, i32)]) -> Gomoku {
    let mut g = Gomoku::new(19);
    for &(x, y) in moves {
        let (r, _, _) = g.handle_move(x, y);
        assert_eq!(r, MoveResult::Valid, "setup move ({x},{y}) was rejected");
        g.switch_player();
    }
    g
}

/// Play `moves`, then require the engine to answer with one of `expected`,
/// every time.
fn expect_move(name: &str, moves: &[(i32, i32)], expected: &[(usize, usize)]) -> Vec<String> {
    let g = position(moves);
    let mut failures = Vec::new();
    for attempt in 0..REPEATS {
        clear_transposition_table();
        let (best, _, _) = pyo3::Python::with_gil(|_| get_ai_move_with_stats(&g));
        let (x, y, score) = best.expect("engine returned no move");
        if !expected.contains(&(x, y)) {
            failures.push(format!(
                "{name} attempt {attempt}: played ({x},{y}) score={score}, expected one of {expected:?}"
            ));
        }
    }
    failures
}

#[test]
fn tactics() {
    pyo3::prepare_freethreaded_python();
    let mut failures = Vec::new();

    // Black has four in a row and must finish it.
    failures.extend(expect_move(
        "complete-five",
        &[(9, 6), (2, 2), (9, 7), (2, 3), (9, 8), (2, 4), (9, 9), (15, 15)],
        &[(9, 5), (9, 10)],
    ));

    // White's four is already blocked at (9,5), so (9,10) is the only defence.
    // An *open* four would be unstoppable and makes a poor test.
    failures.extend(expect_move(
        "block-closed-four",
        &[(9, 5), (9, 6), (2, 2), (9, 7), (2, 3), (9, 8), (2, 4), (9, 9)],
        &[(9, 10)],
    ));

    // White's four has a hole in it; filling the hole is the only defence.
    failures.extend(expect_move(
        "block-split-four",
        &[(2, 2), (9, 6), (2, 3), (9, 7), (2, 4), (9, 9), (15, 15), (9, 10)],
        &[(9, 8)],
    ));

    // White has an open three. Ignoring it loses, so the reply must be on that
    // line. This is the case that exposed both search bugs.
    failures.extend(expect_move(
        "block-open-three",
        &[(2, 2), (9, 7), (2, 4), (9, 8), (15, 15), (9, 9)],
        &[(9, 5), (9, 6), (9, 10), (9, 11)],
    ));

    // Black's own open three: push it to a four rather than wander off.
    failures.extend(expect_move(
        "extend-own-three",
        &[(9, 7), (2, 2), (9, 8), (2, 3), (9, 9), (15, 15)],
        &[(9, 5), (9, 6), (9, 10), (9, 11)],
    ));

    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}
