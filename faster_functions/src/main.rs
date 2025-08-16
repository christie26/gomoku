use signal_hook::{consts::{SIGINT, SIGTERM}, iterator::Signals};
use faster_functions::{minimax, Gomoku};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use std::time::{Duration, Instant};

fn main() {
    let mut game = Gomoku::new(19);

    let mut durations = vec![];
    println!("Game start!");

    let running = Arc::new(AtomicBool::new(true));

    let r = running.clone();
    thread::spawn(move || {
            let mut signals = Signals::new(&[SIGINT, SIGTERM]).expect("Failed to create signals");
            for _ in signals.forever() {
                println!("\nReceived Ctrl-C, shutting down...");
                r.store(false, Ordering::SeqCst);
            }
        });
    let r = running.clone();
    // Set up Ctrl-C handler
    // ctrlc::set_handler(move || {
    //     println!("\nReceived Ctrl-C, stopping...");
    //     r.store(false, Ordering::SeqCst);
    // })
    // .expect("Error setting Ctrl-C handler");

    loop {
        let start = Instant::now();

        let game_clone = game.clone();
        let handle = thread::spawn(move || minimax::get_ai_move(&game_clone));

        let mut res = None;

        loop {
            if !running.load(Ordering::SeqCst) {
                println!("Abandoning current task...");
                break;
            }

            // Check if task completed
            if handle.is_finished() {
                (res, _) = handle.join().unwrap();
                break;
            }

            thread::sleep(Duration::from_millis(10));
        }

        let Some((x, y, score)) = res else {
            println!("Played all possible valid moves!!");
            break;
        };
        let elapsed = start.elapsed().as_secs_f64(); // convert to milliseconds
        durations.push(elapsed);
        println!("{} playing ({}, {}) - {}pts", game.current_player, x, y, score);
        game.print_board();
        game.handle_move(x as i32, y as i32);
        game.switch_player();
        if let Some(winner) = game.get_winner() {
            println!("{winner} won");
            break;
        }
    }

    game.print_board();

    let count = durations.len();
    let sum: f64 = durations.iter().sum();
    let mean = sum / count as f64;
    let min = durations.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = durations.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let stddev = (durations.iter().map(|d| (d - mean).powi(2)).sum::<f64>() / count as f64).sqrt();

    println!("Iterations: {}", count);
    println!("Mean duration: {:.3} s", mean);
    println!("Min duration: {:.3} s", min);
    println!("Max duration: {:.3} s", max);
    println!("Standard deviation: {:.3} s", stddev);
    if durations.len() >= 10 {
        println!("Mean duration of first 10 iterations: {:.3} s", durations[0..10].iter().sum::<f64>() / 10.0);
    }

    if durations.len() >= 20 {
        println!("Mean duration of first 20 iterations: {:.3} s", durations[0..20].iter().sum::<f64>() / 20.0);
    }

    if durations.len() >= 30 {
        println!("Mean duration of first 30 iterations: {:.3} s", durations[0..30].iter().sum::<f64>() / 30.0);
    }
}
