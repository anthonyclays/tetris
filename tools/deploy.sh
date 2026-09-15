#!/bin/bash
cargo build --release
strip target/release/frustration-tetris
scp target/x86_64-pc-windows-gnu/release/frustration-tetris.exe target/release/frustration-tetris anthony.clays.me:.
