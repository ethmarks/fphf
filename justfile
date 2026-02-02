alias b := build
alias i := install
alias r := run

build:
    cargo build --release --
install:
    cargo install --path .
run:
    cargo run --release --
