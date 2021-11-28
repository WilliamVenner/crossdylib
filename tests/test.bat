@echo off
echo Testing Debug
cargo build --manifest-path a1/Cargo.toml
cargo build --manifest-path b2/Cargo.toml
cargo build --manifest-path c3/Cargo.toml
cargo run --manifest-path main/Cargo.toml

echo Testing Release
cargo build --release --manifest-path a1/Cargo.toml
cargo build --release --manifest-path b2/Cargo.toml
cargo build --release --manifest-path c3/Cargo.toml
cargo run --release --manifest-path main/Cargo.toml