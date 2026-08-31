use lib_gomoku::{
    Gomoku, constants::{
        BOARD_SIZE, DEEP_RADIUS, DEEP_RADIUS_DEPTH, MAX_DEPTH, RADIUS, RANDOMIZE_TIED_MOVES,
        SHALLOW_ORDER_DEPTH, TIME_LIMIT_MS, TT_SIZE_BITS,
    }, minimax::{self, RunLogger, SearchStats, print_search_stats}, position_name,
};
use signal_hook::{
    consts::{SIGINT, SIGTERM},
    iterator::Signals,
};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use pyo3::Python;



/// Random 6-digit run id. Doubles as the run's tag — the constants
/// themselves are already recorded as separate columns in runs.csv, so the
/// tag just needs to be a short, unique handle for this run.
fn random_6_digit() -> u32 {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    let r = RandomState::new().build_hasher().finish();
    100_000 + (r % 900_000) as u32
}

fn build_tag() -> String {
    random_6_digit().to_string()
}

fn logs_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("logs")
}

fn append_csv_row(
    tag: &str,
    log_file_name: &str,
    count: usize,
    mean: f64,
    min: f64,
    max: f64,
    stddev: f64,
    winner: &str,
    move_count: usize,
) {
    let csv_path = logs_dir().join("runs.csv");
    let header = "timestamp,tag,max_depth,shallow_order_depth,radius,deep_radius_depth,deep_radius,time_limit_ms,randomize_tied_moves,tt_size_bits,iterations,mean_duration_s,min_duration_s,max_duration_s,stddev_duration_s,winner,move_count,log_file\n";
    let file_exists = csv_path.exists();
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&csv_path)
        .expect("failed to open runs.csv");
    if !file_exists {
        file.write_all(header.as_bytes()).ok();
    }
    let time_limit = match TIME_LIMIT_MS {
        Some(ms) => ms.to_string(),
        None => "none".to_string(),
    };
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let row = format!(
        "{},{},{},{},{},{},{},{},{},{},{},{:.3},{:.3},{:.3},{:.3},{},{},{}\n",
        ts,
        tag,
        MAX_DEPTH,
        SHALLOW_ORDER_DEPTH,
        RADIUS,
        DEEP_RADIUS_DEPTH,
        DEEP_RADIUS,
        time_limit,
        RANDOMIZE_TIED_MOVES,
        TT_SIZE_BITS,
        count,
        mean,
        min,
        max,
        stddev,
        winner,
        move_count,
        log_file_name,
    );
    file.write_all(row.as_bytes()).ok();
}

fn main() {
    pyo3::prepare_freethreaded_python();

    let mut game = Gomoku::new(BOARD_SIZE);

    let mut durations = vec![];

    let tag = build_tag();
    let dir = logs_dir();
    fs::create_dir_all(&dir).expect("failed to create logs dir");
    let log_file_name = format!("{tag}.log");
    let log_path = dir.join(&log_file_name);
    let mut logger = RunLogger::new(Some(&log_path)).expect("failed to create log file");

    logger.log(&format!("Game start! [tag={tag}]"));

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
    let mut winner: Option<String> = None;
    loop {
        let start = Instant::now();

        let game_clone = game.clone();
        let handle =
            thread::spawn(move || Python::with_gil(|_py| minimax::get_ai_move_with_stats(&game_clone)));

        let mut res = None;
        let mut moves = vec![];
        let mut stats = SearchStats::new();

        loop {
            if !running.load(Ordering::SeqCst) {
                logger.log("Abandoning current task...");
                break;
            }

            // Check if task completed
            if handle.is_finished() {
                (res, moves, stats) = handle.join().unwrap();
                break;
            }

            thread::sleep(Duration::from_millis(10));
        }

        let Some((x, y, score)) = res else {
            logger.log("Played all possible valid moves or canceled");
            break;
        };
        let elapsed = start.elapsed().as_secs_f64();
        logger.log(&format!(
            "Turn {}: {} playing {} - {}pts (took {:.2}s and tested {} moves)",
            durations.len() + 1,
            game.current_player,
            position_name(&(x as i32, y as i32)),
            score,
            elapsed,
            moves.len()
        ));
        print_search_stats(&stats, &mut logger);
        durations.push(elapsed);
        game.handle_move(x as i32, y as i32);
        game.print_board(vec![(x, y)]);
        move_history.push((x, y, score));
        game.switch_player();
        if let Some(w) = game.get_winner() {
            logger.log(&format!("{w} won"));
            winner = Some(w);
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

    logger.log(&format!("Iterations: {}", count));
    plot_bar_chart(&durations, &mut logger);
    logger.log(&format!("Mean duration: {:.3} s", mean));
    logger.log(&format!("Min duration: {:.3} s", min));
    logger.log(&format!("Max duration: {:.3} s", max));
    logger.log(&format!("Standard deviation: {:.3} s", stddev));

    logger.log(&format!(
        "move history: {}",
        move_history
            .iter()
            .map(|u| format!("{}", position_name(&(u.0 as i32, u.1 as i32))))
            .collect::<Vec<String>>()
            .join("->")
    ));

    append_csv_row(
        &tag,
        &log_file_name,
        count,
        mean,
        min,
        max,
        stddev,
        winner.as_deref().unwrap_or("none"),
        move_history.len(),
    );
}

fn plot_bar_chart(values: &[f64], logger: &mut RunLogger) {
    if values.is_empty() {
        logger.log("No data to plot");
        return;
    }

    // Find the maximum value for scaling
    let max_value = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);

    // Handle edge case where all values are 0 or negative
    if max_value <= 0.0 {
        logger.log("All values are zero or negative");
        return;
    }

    // Define bar width in characters
    let bar_width = 50;

    // Block characters for sub-character granularity (8 levels)
    let blocks = [" ", "▏", "▎", "▍", "▌", "▋", "▊", "▉", "█"];

    // Plot each value
    for (i, &value) in values.iter().enumerate() {
        if value <= 0.0 {
            logger.log(&format!("{:3}: {:6.2} ", i, value));
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

        logger.log(&format!("{:3}: {:6.2}s {}", i + 1, value, bar));
    }
}
