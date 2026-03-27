use faster_functions::{
    minimax,
    search_state::BOARD_SIZE,
    Gomoku,
};
use signal_hook::{
    consts::{SIGINT, SIGTERM},
    iterator::Signals,
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use std::time::{Duration, Instant};

fn main() {
    let mut game = Gomoku::new(BOARD_SIZE);

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
    // let r = running.clone();
    // Set up Ctrl-C handler
    // ctrlc::set_handler(move || {
    //     println!("\nReceived Ctrl-C, stopping...");
    //     r.store(false, Ordering::SeqCst);
    // })
    // .expect("Error setting Ctrl-C handler");

    let mut move_history = vec![];
    loop {
        let start = Instant::now();

        let game_clone = game.clone();
        let handle = thread::spawn(move || minimax::get_ai_move_iterative_deepening(&game_clone));

        let mut res = None;
        let mut moves = vec![];

        loop {
            if !running.load(Ordering::SeqCst) {
                println!("Abandoning current task...");
                break;
            }

            // Check if task completed
            if handle.is_finished() {
                (res, moves) = handle.join().unwrap();
                break;
            }

            thread::sleep(Duration::from_millis(10));
        }

        let Some((x, y, score)) = res else {
            println!("Played all possible valid moves or canceled");
            break;
        };
        let elapsed = start.elapsed().as_secs_f64();
        println!(
            "Turn {}: {} playing ({}, {}) - {}pts (took {:.2}s and tested {} moves)",
            durations.len() + 1,
            game.current_player,
            x,
            y,
            score,
            elapsed,
            moves.len()
        );
        durations.push(elapsed);
        game.handle_move(x as i32, y as i32);
        game.print_board(vec![(x, y)]);
        move_history.push((x, y, score));
        game.switch_player();
        if let Some(winner) = game.get_winner() {
            println!("{winner} won");
            break;
        }
    }

    game.print_board(vec![]);

    let count = durations.len();
    let sum: f64 = durations.iter().sum();
    let mean = sum / count as f64;
    let min = durations.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = durations.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let stddev = (durations.iter().map(|d| (d - mean).powi(2)).sum::<f64>() / count as f64).sqrt();

    println!("Iterations: {}", count);
    plot_bar_chart(&durations);
    println!("Mean duration: {:.3} s", mean);
    println!("Min duration: {:.3} s", min);
    println!("Max duration: {:.3} s", max);
    println!("Standard deviation: {:.3} s", stddev);

    println!(
        "move history: {}",
        move_history
            .iter()
            .map(|u| format!("{u:?}"))
            .collect::<Vec<String>>()
            .join("->")
    );
}

fn plot_bar_chart(values: &[f64]) {
    if values.is_empty() {
        println!("No data to plot");
        return;
    }

    // Find the maximum value for scaling
    let max_value = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);

    // Handle edge case where all values are 0 or negative
    if max_value <= 0.0 {
        println!("All values are zero or negative");
        return;
    }

    // Define bar width in characters
    let bar_width = 50;

    // Block characters for sub-character granularity (8 levels)
    let blocks = [" ", "▏", "▎", "▍", "▌", "▋", "▊", "▉", "█"];

    // Plot each value
    for (i, &value) in values.iter().enumerate() {
        if value <= 0.0 {
            println!("{:3}: {:6.2} ", i, value);
            continue;
        }

        // Calculate bar length with fractional part
        let normalized = (value / max_value) * bar_width as f64;
        let full_blocks = normalized.floor() as usize;
        let fractional = normalized - normalized.floor();

        // Choose partial block character based on fraction
        let partial_index = (fractional * 8.0).round() as usize;

        // Build the bar
        let mut bar = "█".repeat(full_blocks);
        if full_blocks < bar_width && partial_index > 0 && partial_index < 8 {
            bar.push_str(blocks[partial_index]);
        }

        println!("{:3}: {:6.2}s {}", i + 1, value, bar);
    }
}
