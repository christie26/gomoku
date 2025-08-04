use std::collections::HashMap;

use pyo3::{intern, prelude::*};

// fn is_valid_move(board: Gomoku, x: i64, y: i64) {
//
// }

/// Formats the sum of two numbers as string.
// #[pyfunction]
// fn count_free_three(sign: usize, dx: usize, dy: usize, x0: usize, y0: usize) -> PyResult<(usize, usize, bool)> {
//     let mut my_count = 0;
//     let mut empty_count = 0;
//     let mut i = 1;
//     let mut hole = false;
//
//     loop {
//         let x = x0 + dx * i * sign;
//         let y = y0 + dy * i * sign;
//
//         if !self.is_on_board(x, y)
//             || self.board[x as usize][y as usize] == self.opponent_player
//             || empty_count == 2
//         {
//             break;
//         }
//
//         if self.board[x as usize][y as usize] == self.current_player {
//             if empty_count > 0 {
//                 hole = true;
//             }
//             my_count += 1;
//         } else {
//             empty_count += 1;
//         }
//
//         i += 1;
//     }
//
//     Ok((my_count, empty_count, hole))
// }

struct TableFormatter<'a>(&'a Vec<Vec<String>>);

impl<'a> std::fmt::Debug for TableFormatter<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.is_empty() {
            return write!(f, "[]");
        }

        writeln!(f)?;
        for row in self.0 {
            write!(f, "    ")?; // Indent for nice formatting
            for cell in row {
                write!(f, "{}", cell)?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

impl std::fmt::Debug for Gomoku {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MyStruct")
            .field("size", &self.size)
            .field("board", &TableFormatter(&self.board))
            .field("current_player", &self.current_player)
            .field("opponent_player", &self.opponent_player)
            // .field("capture_count", &self.capture_count)
            // .field("free_three_list", &self.free_three_list)
            // .field("five_row", &self.five_row)
            // .field("open_two", &self.open_two)
            // .field("open_three", &self.open_three)
            // .field("open_four", &self.open_four)
            // .field("win_capture_count", &self.win_capture_count)
            // .field("current_move", &self.current_move)
            .finish()
    }
}

type Coord = (usize, usize);

#[derive(Clone)]
pub struct Gomoku {
    pub size: usize,

    pub board: Vec<Vec<String>>,

    pub current_player: String,

    pub opponent_player: String,

    // pub capture_count: HashMap<String, usize>,

    // pub free_three_list: HashMap<String, Vec<Vec<(Coord, Coord, Coord, Coord, Coord, Coord)>>>,
    //
    // pub open_two: HashMap<String, Vec<(Coord, Coord, Coord, Coord)>>,
    //
    // pub open_three: HashMap<String, Vec<(Coord, Coord, Coord, Coord, Coord)>>,
    //
    // pub open_four: HashMap<String, Vec<(Coord, Coord, Coord, Coord, Coord, Coord)>>,
    //
    // pub five_row: HashMap<String, Vec<(Coord, Coord, Coord, Coord, Coord)>>,
    //
    // pub win_capture_count: usize,
    //
    // pub current_move: (Option<usize>, Option<usize>),
}

// #[pyfunction
// fn gomoku_test2(g: &Gomoku) -> bool {
//     println!("{g:?}");
//     false
// }

fn extract_coord_dict<'b, T>(py_dict: PyObject, py: Python<'b>) -> PyResult<HashMap<String, Vec<T>>>
where
    T: FromPyObject<'b> + std::fmt::Debug,
{
    let dict = py_dict.downcast_bound::<pyo3::types::PyDict>(py)?;
    let mut result = HashMap::new();

    for (key, value) in dict {
        let player: String = key.extract()?;
        // println!("Getting coords: {value:?}");
        let coords: Vec<T> = value.extract()?;
        // println!("Gotten coords: {coords:?}");
        result.insert(player, coords);
    }

    Ok(result)
}
#[pyfunction]
fn gomoku_test(py_gomoku: PyObject, x: i32, y: i32, py: Python<'_>) -> PyResult<usize> {
    let size: usize = py_gomoku.getattr(py, intern!(py, "size"))?.extract(py)?;
    let board: Vec<Vec<String>> = py_gomoku.getattr(py, intern!(py, "board"))?.extract(py)?;
    let current_player: String = py_gomoku
        .getattr(py, intern!(py, "current_player"))?
        .extract(py)?;
    let opponent_player: String = py_gomoku
        .getattr(py, intern!(py, "opponent_player"))?
        .extract(py)?;
    // let capture_count: HashMap<String, usize> = py_gomoku
    //     .getattr(py, intern!(py, "capture_count"))?
    //     .extract(py)?;
    // let win_capture_count: usize = py_gomoku
    //     .getattr(py, intern!(py, "win_capture_count"))?
    //     .extract(py)?;
    // // println!("current move");
    // let current_move: (Option<usize>, Option<usize>) = py_gomoku
    //     .getattr(py, intern!(py, "current_move"))?
    //     .extract(py)?;
    // // println!("current move clear");
    //
    // // println!("Extracting five_row");
    // let five_row = extract_coord_dict(py_gomoku.getattr(py, intern!(py, "five_row"))?, py)?;
    // // println!("Extracting open_four");
    // let open_four = extract_coord_dict(py_gomoku.getattr(py, intern!(py, "open_four"))?, py)?;
    // // println!("Extracting open_three");
    // let open_three = extract_coord_dict(py_gomoku.getattr(py, intern!(py, "open_three"))?, py)?;
    // // println!("Extracting open_two");
    // let open_two = extract_coord_dict(py_gomoku.getattr(py, intern!(py, "open_two"))?, py)?;
    // // println!("Extracting free_three_list");
    // let free_three_list =
    //     extract_coord_dict(py_gomoku.getattr(py, intern!(py, "free_three_list"))?, py)?;

    let g = Gomoku {
        size,
        board,
        current_player,
        opponent_player,
        // capture_count,
        // free_three_list,
        // five_row,
        // open_two,
        // open_three,
        // open_four,
        // win_capture_count,
        // current_move,
    };
    // println!("All clear");
    // println!("g -> {g:#?}");
    Ok(is_valid_move(g, x, y))
}

const VALID: usize = 0;
const OUT_OF_BOARD: usize = 1;
const NOT_EMPTY: usize = 2;
const DOUBLE_THREE: usize = 3;

fn is_valid_move(g: Gomoku, x: i32, y: i32) -> usize {
    if !is_on_board(x, y, g.size) {
        OUT_OF_BOARD
    } else if g.board[x as usize][y as usize] != "." {
        NOT_EMPTY
    } else if is_double_three_move(g, x as usize, y as usize) {
        DOUBLE_THREE
    } else {
        VALID
    }
}

fn is_double_three_move(mut g: Gomoku, x: usize, y: usize) -> bool {
    g.board[x][y] = g.current_player.clone();
    let new_free_threes = get_free_threes_from_move(&g, x as i32, y as i32);
    g.board[x][y] = ".".to_string();

    if new_free_threes.len() > 1 {
        true
    } else {
        false
    }
}

fn get_free_threes_from_move(g: &Gomoku, x0: i32, y0: i32) -> Vec<Vec<(usize, usize)>> {
    let mut new_free_threes = vec![];

    for (dx, dy) in [(1, -1), (1, 0), (1, 1), (0, 1)] {
        let (plus_my, mut plus_empty, plus_hole) = count_free_three(g, 1, x0, y0, dx, dy);
        let (minus_my, mut minus_empty, minus_hole) = count_free_three(g, -1, x0, y0, dx, dy);

        if plus_my + minus_my == 2 && plus_empty + minus_empty >= 3 {
            if plus_hole && minus_empty == 2 {
                minus_empty = 1;
            }
            if minus_hole && plus_empty == 2 {
                plus_empty = 1;
            }
            let plus_end = plus_empty + plus_my;
            let minus_end = minus_empty + minus_my;
            let mut n = vec![];
            for i in -minus_end..=plus_end {
                n.push(((x0 + dx * i) as usize, (y0 + dy * i) as usize));
            }
            new_free_threes.push(n);
        }
    }

    new_free_threes
}

fn count_free_three(g: &Gomoku, sign: i32, x0: i32, y0: i32, dx: i32, dy: i32) -> (i32, i32, bool) {
    let mut my_count = 0;
    let mut empty_count = 0;
    let mut i = 1;
    let mut hole = false;
    loop {
        let (x, y) = (x0 + dx * i * sign, y0 + dy * i * sign);
        if !is_on_board(x, y, g.size)
            || g.board[x as usize][y as usize] == g.opponent_player
            || empty_count == 2
        {
            break;
        }

        if g.board[x as usize][y as usize] == g.current_player {
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

#[pyfunction]
fn is_on_board(x: i32, y: i32, size: usize) -> bool {
    0 <= x && (x as usize) < size && 0 <= y && (y as usize) < size
}

/// A Python module implemented in Rust.
#[pymodule]
fn faster_functions(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // m.add_function(wrap_pyfunction!(count_free_three, m)?)?;
    m.add_function(wrap_pyfunction!(is_on_board, m)?)?;
    m.add_function(wrap_pyfunction!(gomoku_test, m)?)?;
    Ok(())
}
//
// #[cfg(test)]
// mod tests {
//     use super::*;
//
//     #[test]
//     fn test_is_on_baord_exec_time() {
//         let start = std::time::Instant::now();
//         let mut total = 0;
//         let iters = 100000;
//         for i in 0..iters {
//             is_on_board(i, i * i, i * i * i);
//         }
//         let elapsed = start.elapsed();
//         let per_iter = elapsed.as_secs_f64() / (iters as f64);
//         println!("elapsed: {elapsed:?}, per iter: {per_iter:e}");
//         assert!(per_iter < 1e-15);
//     }
// }
