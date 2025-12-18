# fphf

fphf, short for Fixed-Point Hash Finder, is a Rust tool that finds strings that contain part of their own SHA-256 hash. For example, the hash for this paragraph begins with 58c3b2a1.

```
$ printf "fphf, short for Fixed-Point Hash Finder, is a Rust tool that finds strings that contain part of their own SHA-256 hash. For example, the hash for this paragraph begins with 58c3b2a1." | sha256sum
58c3b2a137bdceaf5a5e0fad6aa2270174e357ba9d2276df928a0d3111b0669c  -
```
