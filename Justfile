[private]
default:
    just --list

install:
    cargo install --path .

build:
    cargo build --release
