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

    /// Quiet mode: only print the result string
    #[arg(short, long)]
    quiet: bool,

    /// Verbose mode: print detailed progress information
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum VerbosityLevel {
    Quiet,
    Normal,
    Verbose,
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

fn check(candidate: &str, template: &str) -> Option<(String, String)> {
    let msg = template.replace('#', candidate);
    let digest = Sha256::digest(&msg);
    let result = format!("{digest:x}");

    OPS_COUNT.fetch_add(1, Ordering::Relaxed);

    if result.starts_with(candidate) {
        FOUND.store(true, Ordering::SeqCst);
        Some((msg, result))
    } else {
        None
    }
}

fn format_hash_rate(hashes_per_sec: f64) -> String {
    if hashes_per_sec >= 1_000_000_000.0 {
        format!("{:.2} GH/s", hashes_per_sec / 1_000_000_000.0)
    } else if hashes_per_sec >= 1_000_000.0 {
        format!("{:.2} MH/s", hashes_per_sec / 1_000_000.0)
    } else if hashes_per_sec >= 1_000.0 {
        format!("{:.2} kH/s", hashes_per_sec / 1_000.0)
    } else {
        format!("{:.2} H/s", hashes_per_sec)
    }
}

fn format_time(seconds: u64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;

    if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, secs)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, secs)
    } else {
        format!("{}s", secs)
    }
}

fn solve(length: u8, template: &str, verbosity: VerbosityLevel) {
    // Let length <= 32.
    let base: u128 = 16;
    let max_count: u128 = base.pow(cast!(length, u32)); // Then max_count <= 16^32 = 2^128.
    let start_time: Instant = Instant::now();
    let start_time_arc = Arc::new(start_time);
    let num_threads = rayon::current_num_threads();

    // Print initial information based on verbosity
    match verbosity {
        VerbosityLevel::Verbose => {
            println!("Template: {}", template);
            println!("Digits to match: {}", length);
            println!("Search space: {} possible combinations", max_count);

            // Estimate time based on assumed hash rate (this is a rough estimate)
            let estimated_rate = 1_000_000.0 * num_threads as f64; // Rough estimate
            let estimated_seconds = (max_count as f64 / estimated_rate) as u64;
            println!("Estimated time: ~{}", format_time(estimated_seconds));
            println!("Threads available: {}\n", num_threads);
        }
        VerbosityLevel::Normal => {
            println!("Searching for {}-digit hash prefix match...", length);
        }
        VerbosityLevel::Quiet => {}
    }

    // Start status updater thread
    let start_time_clone = Arc::clone(&start_time_arc);
    let status_thread = if verbosity != VerbosityLevel::Quiet {
        Some(thread::spawn(move || {
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

                match verbosity {
                    VerbosityLevel::Verbose => {
                        print!(
                            "\rElapsed: {} | Remaining: ~{} | Hashes: {}/{} ({:.4}%) | Speed: {}",
                            format_time(elapsed as u64),
                            format_time(remaining_secs),
                            current_ops,
                            max_count,
                            progress_pct,
                            format_hash_rate(speed)
                        );
                    }
                    VerbosityLevel::Normal => {
                        print!(
                            "\r{:.1}% complete | Speed: {} | Elapsed: {}",
                            progress_pct,
                            format_hash_rate(speed),
                            format_time(elapsed as u64)
                        );
                    }
                    VerbosityLevel::Quiet => {}
                }
                std::io::stdout().flush().unwrap();
            }
        }))
    } else {
        None
    };

    // Use parallel iteration with early termination
    let result = (0..max_count).into_par_iter().find_map_any(|i| {
        // Check the flag periodically to allow early exit
        if FOUND.load(Ordering::Relaxed) {
            return None;
        }

        // Generate candidate (hash prefix)
        let candidate = to_fixed_hex(i, length);

        // Check the candidate
        check(&candidate, template)
    });

    // Signal status thread to stop and wait for it
    FOUND.store(true, Ordering::SeqCst);
    if let Some(handle) = status_thread {
        let _ = handle.join();
    }

    let total_ops = OPS_COUNT.load(Ordering::SeqCst);
    let elapsed = start_time.elapsed();

    // Print results based on verbosity
    match verbosity {
        VerbosityLevel::Quiet => {
            if let Some((msg, _)) = result {
                println!("{}", msg);
            }
        }
        VerbosityLevel::Normal => {
            if let Some((msg, _)) = result {
                println!("\n\nFound: {}", msg);
            } else {
                println!("\n\nNo match found after searching {} hashes.", total_ops);
            }
        }
        VerbosityLevel::Verbose => {
            println!("\n");
            if let Some((msg, hash)) = result {
                println!("=== MATCH FOUND ===");
                println!("Total time: {}", format_time(elapsed.as_secs()));
                println!("Total hashes searched: {}", total_ops);
                println!("Output string: {}", msg);
                println!("Full hash: {}", hash);
            } else {
                println!("=== NO MATCH FOUND ===");
                println!("Total time: {}", format_time(elapsed.as_secs()));
                println!("Total hashes searched: {}", total_ops);
                println!("Exhausted search space without finding a match.");
            }
        }
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

    // Determine verbosity level
    let verbosity = if args.quiet && args.verbose {
        eprintln!("Error: Cannot specify both --quiet and --verbose");
        std::process::exit(1);
    } else if args.quiet {
        VerbosityLevel::Quiet
    } else if args.verbose {
        VerbosityLevel::Verbose
    } else {
        VerbosityLevel::Normal
    };

    solve(args.digits, &args.text, verbosity);
}
