#!/bin/bash
cargo build --release --features="tetrominos"
strip target/release/gliumtetris
scp target/x86_64-pc-windows-gnu/release/gliumtetris.exe target/release/gliumtetris anthony.clays.me:.
