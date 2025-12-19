use clap::Parser;
use rayon::prelude::*;
use sha2::{Digest, Sha256};
use std::io::Write;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

/// Find fixed-point hash strings for SHA-256
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

static FOUND: AtomicBool = AtomicBool::new(false);
static OPS_COUNT: AtomicU64 = AtomicU64::new(0);

// Helper for high-speed hex writing without String allocations
#[inline(always)]
fn write_hex_bytes(buf: &mut [u8], mut n: u128, len: usize) {
    const HEX_CHARS: &[u8] = b"0123456789abcdef";
    for i in (0..len).rev() {
        buf[i] = HEX_CHARS[(n & 0xf) as usize];
        n >>= 4;
    }
}

// Helper for high-speed byte comparison
#[inline(always)]
fn check_match(digest: &[u8], expected_hex_prefix: &[u8]) -> bool {
    for (i, &expected_byte) in expected_hex_prefix.iter().enumerate() {
        let shift = if i % 2 == 0 { 4 } else { 0 };
        let nibble = (digest[i / 2] >> shift) & 0xf;
        let actual_hex_char = if nibble < 10 {
            b'0' + nibble
        } else {
            b'a' + (nibble - 10)
        };
        if actual_hex_char != expected_byte {
            return false;
        }
    }
    true
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
    let hash_placeholder_idx = template.find('#').expect("Template must contain #");

    let prefix = &template[..hash_placeholder_idx];
    let suffix = &template[hash_placeholder_idx + 1..];
    let mut template_bytes = Vec::new();
    template_bytes.extend_from_slice(prefix.as_bytes());
    template_bytes.extend_from_slice(&vec![b'0'; length as usize]);
    template_bytes.extend_from_slice(suffix.as_bytes());

    let max_count: u64 = 16u64.pow(length as u32);
    let start_time = Instant::now();
    let num_threads = rayon::current_num_threads();

    // Print initial information based on verbosity
    match verbosity {
        VerbosityLevel::Verbose => {
            println!(
                "Template: {}",
                template.replace('#', &"#".repeat(length as usize))
            );
            println!("Digits to match: {}", length);
            println!("Search space: {} possible combinations", max_count);
            println!("Threads available: {}\n", num_threads);
        }
        VerbosityLevel::Normal => {
            println!("Searching for {}-digit hash prefix match...", length);
        }
        VerbosityLevel::Quiet => {}
    }

    // Start status updater thread
    let status_thread = if verbosity != VerbosityLevel::Quiet {
        let start_clone = start_time;
        Some(thread::spawn(move || {
            while !FOUND.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_secs(1));
                let current_ops = OPS_COUNT.load(Ordering::Relaxed);
                let elapsed = start_clone.elapsed().as_secs_f64();
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
                        println!(
                            "Elapsed: {} | Remaining: ~{} | Hashes: {}/{} ({:.4}%) | Speed: {}",
                            format_time(elapsed as u64),
                            format_time(remaining_secs),
                            current_ops,
                            max_count,
                            progress_pct,
                            format_hash_rate(speed),
                        );
                    }
                    VerbosityLevel::Normal => {
                        print!(
                            "\r{:.1}% searched | Speed: {} | Elapsed: {}",
                            progress_pct,
                            format_hash_rate(speed),
                            format_time(elapsed as u64)
                        );
                    }
                    VerbosityLevel::Quiet => {}
                }
                let _ = std::io::stdout().flush();
            }
        }))
    } else {
        None
    };

    // High-performance loop
    let chunk_size: u64 = 2048;
    let result = (0..(max_count / chunk_size + 1))
        .into_par_iter()
        .find_map_any(|chunk_idx| {
            if FOUND.load(Ordering::Relaxed) {
                return None;
            }

            let mut hasher = Sha256::new();
            let mut local_buf = template_bytes.clone();
            let start = chunk_idx * chunk_size;
            let end = std::cmp::min(start + chunk_size, max_count);

            for i in start..end {
                write_hex_bytes(
                    &mut local_buf[hash_placeholder_idx..hash_placeholder_idx + length as usize],
                    i as u128,
                    length as usize,
                );

                hasher.update(&local_buf);
                let hash_result = hasher.finalize_reset();

                if check_match(
                    &hash_result,
                    &local_buf[hash_placeholder_idx..hash_placeholder_idx + length as usize],
                ) {
                    FOUND.store(true, Ordering::SeqCst);
                    return Some((
                        String::from_utf8_lossy(&local_buf).into_owned(),
                        format!("{:x}", hash_result),
                    ));
                }
            }
            OPS_COUNT.fetch_add(end - start, Ordering::Relaxed);
            None
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
                println!("\n\n{}", msg);
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

    // Warn about large search spaces
    if args.digits >= 11 {
        let base: u128 = 16;
        let search_space = base.pow(args.digits as u32);
        eprintln!(
            "WARNING: Searching for {} digits requires checking up to {} combinations.",
            args.digits, search_space
        );
        eprintln!("This may take an extremely long time or never complete.");
        eprintln!("Consider using fewer digits for a practical search.");
        eprintln!();
    }

    solve(args.digits, &args.text, verbosity);
}
