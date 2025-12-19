# fphf

fphf (Fixed-Point Hash Finder) is a Rust tool that finds strings that contain part of their own SHA-256 hash. For example, the hash for this paragraph begins with 80699e1.

```bash
❯ printf 'fphf (Fixed-Point Hash Finder) is a Rust tool that finds strings that contain part of their own SHA-256 hash. For example, the hash for this paragraph begins with 80699e1.' | sha256sum
80699e11b9c39c251f05efb875a8c7d0ad25beb87860fa52751c057370353969  -
```

## What It Does

fphf uses brute force to repeatedly guess-and-check candidates until it finds a string which contains its own truncated SHA-256 hash. It can be parameterized to work with any template string and any hash prefix length. It's written in Rust and uses [rayon](https://crates.io/crates/rayon) for multithreading to maximize performance.

## Why?

Fixed point hashes are similar in usefulness to bare-handed apple splitting. It's a fun but gimmicky party trick. Seeing someone casually rip an apple in half seems like a superhuman feat of strength unless you know the technique. Likewise, seeing something that required brute-forcing a SHA-256 hash seems like a staggering feat of computation unless you realize that the search space is actually only a few billion combinations.

The most practical use case I can think of for fphf is to get attention from tech people. The [first time I saw a fixed-point hash](https://mastodon.social/@susam/113465877615338066), I was completely blown away. If you want to include a fixed point hash in your bio, I can confidently say that at least some people will find it really cool.

## A note on authorship

fphf started as a clone of Susam Pal's [rust-sha-prefix-embed](https://github.com/susam/lab/tree/main/rust-sha-prefix-embed) tool. fphf has a much more advanced feature set, including multithreading and CLI parameters, but the concept and original source code belong to Susam.

Secondly, the majority (though not the entirety) of the code in this project was written using LLM agents. My role was that of an architect, reviewer, and tester, *not* that of a programmer. I attached a note in the description body of each commit that was generated primarily with an LLM.

## Installation

Make sure you have [Rust installed](https://rust-lang.org/tools/install/), clone the repo, and install it with Cargo.

```bash
git clone https://github.com/ethmarks/fphf.git
cd fphf
cargo install --path .
```

## Basic Usage

By default, fphf uses the template "The SHA-256 hash of this sentence begins with #." and searches for a hash with 7 digits.

```bash
❯ fphf
Searching for 7-digit hash prefix match...
58.0% searched | Speed: 155.56 MH/s | Elapsed: 1s

The SHA-256 hash of this sentence begins with b43c8b9.
```

## Options

### --text

The `--text` flag can be used to specify a custom template. Use an octothorpe (#) as a placeholder for the hash.

```bash
❯ fphf -t 'hello, dear reader! #'
Searching for 7-digit hash prefix match...
76.1% searched | Speed: 102.14 MH/s | Elapsed: 2s

hello, dear reader! fb32a98
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
❯ fphf -d 8
Searching for 8-digit hash prefix match...
79.6% searched | Speed: 170.97 MH/s | Elapsed: 20s

The SHA-256 hash of this sentence begins with 634f0510.
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

Elapsed: 1s | Remaining: ~0s | Hashes: 181817344/268435456 (67.7322%) | Speed: 181.69 MH/s
Elapsed: 2s | Remaining: ~0s | Hashes: 209453056/268435456 (78.0273%) | Speed: 104.69 MH/s


=== MATCH FOUND ===
Total time: 2s
Total hashes searched: 209453056
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

There's a decent chance that you'll get a "No match found" error from fphf at some point. What this means is that there aren't any fixed point hashes for the specified length and template. It's not that fphf was just unable to find any, it's that they mathematically don't exist. It's like asking for a real-valued solution to `x^2+x+1=0`. 

```bash
❯ fphf -t 'i want this particular template #' -d 5
Searching for 5-digit hash prefix match...
100.0% searched | Speed: 1.05 MH/s | Elapsed: 1s

No match found after searching 1048576 hashes.
```

You can overcome this by changing the desired hash length or by tweaking the template (adding or removing punctuation, rephrasing a word or two, adjusting capitalization, etc). Because SHA-256 is so sensitive, even a tiny change will fundamentally alter the hash space, effectively giving you another shot at a low-length fixed-point hash.

```bash
❯ fphf -t 'I want this particular template #' -d 5
Searching for 5-digit hash prefix match...
2.5% searched | Speed: 26.60 kH/s | Elapsed: 1s

I want this particular template 1cba2
```

## Verification

To verify a fixed-point hash, all you need to do is calculate the SHA-256 hash of the string and check to see if the first few digits match.

I recommend using my [fphv](https://github.com/ethmarks/fphv) (Fixed-Point Hash Verifier) tool for this. It's hosted at <https://fphv.vercel.app/>. Copy-paste the output from fphf into the "String" textbox, and it'll calculate the SHA-256 hash. If it's a valid fixed-point hash, it'll show a little check mark (✔) in the output box.

You can also verify fixed-point hashes using the Linux [`sha256sum`](https://linux.die.net/man/1/sha256sum) CLI utility. If you choose this option, make *sure* that you pipe the text into `sha256sum` without adding a newline, as it will interfere with the result hash otherwise. **`echo` will not work**. I suggest using `printf` instead.

```bash
❯ fphf -q
The SHA-256 hash of this sentence begins with b43c8b9.

❯ printf 'The SHA-256 hash of this sentence begins with b43c8b9.' | sha256sum
b43c8b96f151033a566e148d45c43aa84ba153ff9407397f23d5eb43112bb5e1  -
```

## License

Licensed under an Apache 2.0 License. See [LICENSE](LICENSE) for more information.
