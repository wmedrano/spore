#!/bin/bash

cargo build --all-targets
cargo nextest run
cargo doc
cargo test --doc
cargo clippy
