use clap::Parser;
use rayon::prelude::*;
use sha2::{Digest, Sha256};
use std::io::Write;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

/// Find partial hash collisions for SHA-256
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Number of hex digits to match
    #[arg(short, long, default_value_t = 7)]
    digits: u8,

    /// Text template with # as placeholder for the hash
    #[arg(
        short,
        long,
        default_value = "The SHA-256 hash of this sentence begins with #."
    )]
    text: String,
}

macro_rules! cast {
    ($value: expr, $target_type: ty) => {
        <$target_type>::try_from($value).unwrap()
    };
}

static FOUND: AtomicBool = AtomicBool::new(false);
static OPS_COUNT: AtomicU64 = AtomicU64::new(0);

fn to_fixed_hex(n: u128, length: u8) -> String {
    format!("{n:0>width$x}", width = length as usize)
}

fn check(candidate: &str, template: &str) -> bool {
    let msg = template.replace('#', candidate);
    let digest = Sha256::digest(&msg);
    let result = format!("{digest:x}");

    OPS_COUNT.fetch_add(1, Ordering::Relaxed);

    if result.starts_with(candidate) {
        FOUND.store(true, Ordering::SeqCst);
        println!("\n{msg}\n{result}\n");
        true
    } else {
        false
    }
}

fn solve(length: u8, template: &str) {
    // Let length <= 32.
    let base: u128 = 16;
    let max_count: u128 = base.pow(cast!(length, u32)); // Then max_count <= 16^32 = 2^128.
    let start_time: Instant = Instant::now();
    let start_time_arc = Arc::new(start_time);

    println!("solving for length {length} with {max_count} arrangements");
    println!("Threads: {} (CPU cores)", rayon::current_num_threads());

    // Start status updater thread
    let start_time_clone = Arc::clone(&start_time_arc);
    let status_thread = thread::spawn(move || {
        while !FOUND.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_secs(1));
            let current_ops = OPS_COUNT.load(Ordering::Relaxed);
            let elapsed = start_time_clone.elapsed().as_secs_f64();
            let speed = if elapsed > 0.0 {
                current_ops as f64 / elapsed
            } else {
                0.0
            };

            let progress_pct = (current_ops as f64 / max_count as f64) * 100.0;
            let remaining_secs = if speed > 0.0 {
                ((max_count as f64 - current_ops as f64) / speed) as u64
            } else {
                0
            };
            print!(
                "\r[{:.0}s of ~{}s] Hashes: {} ({:.4}%) | Speed: {:.0} H/s",
                elapsed, remaining_secs, current_ops, progress_pct, speed
            );
            std::io::stdout().flush().unwrap();
        }
    });

    // Use parallel iteration with early termination
    (0..max_count).into_par_iter().find_any(|&i| {
        // Check the flag periodically to allow early exit
        if FOUND.load(Ordering::Relaxed) {
            return false;
        }

        // Generate candidate (hash prefix)
        let candidate = to_fixed_hex(i, length);

        // Check the candidate
        check(&candidate, template)
    });

    // Signal status thread to stop and wait for it
    FOUND.store(true, Ordering::SeqCst);
    let _ = status_thread.join();

    let total_ops = OPS_COUNT.load(Ordering::SeqCst);
    if u128::from(total_ops) == max_count {
        println!("\nExhausted search space without finding a match.");
    }
}

fn main() {
    let args = Args::parse();

    // Validate that template contains the placeholder
    if !args.text.contains('#') {
        eprintln!("Error: Template must contain '#' placeholder for the hash");
        std::process::exit(1);
    }

    // Validate digits range
    if args.digits == 0 || args.digits > 32 {
        eprintln!("Error: Digits must be between 1 and 32");
        std::process::exit(1);
    }

    solve(args.digits, &args.text);
}
