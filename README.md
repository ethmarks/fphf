# fphf

fphf, short for Fixed-Point Hash Finder, is a Rust tool that finds strings that contain part of their own SHA-256 hash. For example, the hash for this paragraph begins with 58c3b2a1.

```bash
❯ printf "fphf, short for Fixed-Point Hash Finder, is a Rust tool that finds strings that contain part of their own SHA-256 hash. For example, the hash for this paragraph begins with 58c3b2a1." | sha256sum
58c3b2a137bdceaf5a5e0fad6aa2270174e357ba9d2276df928a0d3111b0669c  -
```

## What It Does

fphf uses brute force to repeatedly guess-and-check candidates until it finds a string which contains its own truncated SHA-256 hash. It can be parameterized to work with any template string and any hash prefix length. It's written in Rust and uses [rayon](https://crates.io/crates/rayon) for multithreading to maximize performance.

## Why?

Fixed point hashes are similar in usefulness to bare-handed apple splitting. Seeing someone casually rip an apple in half seems like a superhuman feat of strength unless you know the technique. It's a fun but gimmicky party trick. Likewise, seeing something that required brute-forcing a SHA-256 hash seems like a staggering feat of computation unless you realize that the search space is actually only a few billion combinations.

The most practical use case I can think of for fphf is to get attention from tech people. The [first time I saw a fixed-point hash](https://mastodon.social/@susam/113465877615338066), I was completely blown away. If you want to include a fixed point hash in your bio, I can confidently say that at least some people will find it really cool.

## A note on authorship

fphf started as a clone of Susam Pal's [rust-sha-prefix-embed](https://github.com/susam/lab/tree/main/rust-sha-prefix-embed) tool. fphf has a much more advanced feature set, including multithreading and CLI parameters, but the concept and original source code belong to Susam.

Secondly, the majority (though not the entirety) of the code in this project was written using LLM agents. My role was that of an architect, reviewer, and tester, *not* that of a programmer. I attached a note in the description body of each commit that was generated primarily with an LLM.

## Installation

Make sure you have [Rust installed](https://rust-lang.org/tools/install/), clone the repo, and build it:

```bash
git clone https://github.com/ethmarks/fphf.git
cd fphf
cargo build --release
```

The binary will be available at `target/release/fphf`. The usage instructions below assume that you've moved the binary to a location on your PATH, but you could also use `./target/release/fphf` or `cargo run --release --`.

## Basic Usage

By default, fphf uses the template "The SHA-256 hash of this sentence begins with #." and searches for a hash with 7 digits.

```bash
❯ fphf
Searching for 7-digit hash prefix match...
50.1% searched | Speed: 19.19 MH/s | Elapsed: 7s

The SHA-256 hash of this sentence begins with b43c8b9.
```

## Options

### --text

The `--text` flag can be used to specify a custom template. Use an octothorpe (#) as a placeholder for the hash.

```bash
❯ fphf -t 'Hash: #'
Searching for 7-digit hash prefix match...
23.5% searched | Speed: 20.99 MH/s | Elapsed: 3s

Hash: 0386242
```

### --digits

The `--digits` flag can be used to specify the length of the hash. 

WARNING: The search space grows exponentially with O(16^n) complexity for n digits. I strongly recommend sticking to 10 or fewer digits because longer hashes will take a very, very long time to compute.

- 4 digits: 65,536 hashes (~instant)
- 6 digits: 16,777,216 hashes (seconds)
- 8 digits: 4,294,967,296 hashes (minutes)
- 10 digits: 1,099,511,627,776 hashes (hours to days)
- 30 digits: 1,329,227,995,784,915,872,903,807,060,280,344,576 hashes (effectively uncomputable)

```bash
❯ fphf -d 5
Searching for 5-digit hash prefix match...
47.3% searched | Speed: 495.29 kH/s | Elapsed: 1s

The SHA-256 hash of this sentence begins with 2f64e.
```

### --quiet

The `--quiet` flag can be used to silence all output other than the final string.

```bash
❯ fphf -q
The SHA-256 hash of this sentence begins with b43c8b9.
```

### --verbose

The `--verbose` flag can be used to provide detailed status information.

Note that the "Remaining" field displays the time remaining until the entire hash space has been searched, not the time until a fixed point hash has been found. Put another way, the Remaining field is the *maximum* time it'll take, but it'll quite possibly take less time.

```bash
❯ fphf -v
Template: The SHA-256 hash of this sentence begins with #######.
Digits to match: 7
Search space: 268435456 possible combinations
Threads available: 12

Elapsed: 1s | Remaining: ~10s | Hashes: 22701125/268435456 (8.4568%) | Speed: 22.66 MH/s
Elapsed: 2s | Remaining: ~9s | Hashes: 45375593/268435456 (16.9037%) | Speed: 22.67 MH/s
Elapsed: 3s | Remaining: ~8s | Hashes: 68044707/268435456 (25.3486%) | Speed: 22.67 MH/s
Elapsed: 4s | Remaining: ~7s | Hashes: 90510051/268435456 (33.7176%) | Speed: 22.62 MH/s
Elapsed: 5s | Remaining: ~7s | Hashes: 107706640/268435456 (40.1239%) | Speed: 21.53 MH/s
Elapsed: 6s | Remaining: ~6s | Hashes: 125779193/268435456 (46.8564%) | Speed: 20.94 MH/s
Elapsed: 7s | Remaining: ~6s | Hashes: 144521962/268435456 (53.8386%) | Speed: 20.62 MH/s


=== MATCH FOUND ===
Total time: 7s
Total hashes searched: 144521962
Output string: The SHA-256 hash of this sentence begins with b43c8b9.
Full hash: b43c8b96f151033a566e148d45c43aa84ba153ff9407397f23d5eb43112bb5e1
```

### --help

The `--help` flag can be used to view all available options.

```bash
❯ fphf -h
Find fixed-point hash strings for SHA-256

Usage: fphf [OPTIONS]

Options:
  -d, --digits <DIGITS>  Number of hex digits to match [default: 7]
  -t, --text <TEXT>      Text template with # as placeholder for the hash [default: "The SHA-256 hash of this sentence begins with #."]
  -q, --quiet            Quiet mode: only print the result string
  -v, --verbose          Verbose mode: print detailed progress information
  -h, --help             Print help
  -V, --version          Print version
```

## Search Space Exhaustion

If you try to find low-length fixed-point hashes, there's a decent chance that you'll get a "No match found" error at some point. What this means is that there aren't any fixed point hashes for the specified length and template. It's not that fphf was unable to find any, it's that they mathematically don't exist. It's like asking for a real-valued solution to `x^2+x+1=0`. 

Search space exhaustion is far less common on higher hash lengths, but if you insist on a low-length hash, you can tweak the template by adding or removing punctuation, rephrasing a word or two, or adjusting the capitalization. Because SHA-256 is so sensitive, even a tiny change will fundamentally alter the hash space, effectively giving you another shot at a low-length fixed-point hash.

```bash
❯ fphf -d 4
Searching for 4-digit hash prefix match...
100.0% searched | Speed: 65.45 kH/s | Elapsed: 1s

No match found after searching 65536 hashes.
```

## Verification

To verify the veracity of a fixed point hash, all you need to do is calculate the SHA-256 hash of the string and check to make sure that the first few digits match. You can do this a number of ways. 

The easiest method to calculate a SHA-256 hash is by using an online tool. Googling "sha256 online", I found [this webpage](https://emn178.github.io/online-tools/sha256.html). From what I can tell, it appears to be accurate.

My preferred method of calculating a SHA-256 hash is using the Linux [`sha256sum`](https://linux.die.net/man/1/sha256sum) CLI utility. If you choose this option, make *sure* that you pipe the text into `sha256sum` without adding a newline, as this will interfere with the result. `echo` *will not work*. I suggest using `printf` instead.

```bash
❯ fphf -q
The SHA-256 hash of this sentence begins with b43c8b9.

❯ printf 'The SHA-256 hash of this sentence begins with b43c8b9.' | sha256sum
b43c8b96f151033a566e148d45c43aa84ba153ff9407397f23d5eb43112bb5e1  -
```

## License

Licensed under an Apache 2.0 License. See [LICENSE](LICENSE) for more information.
