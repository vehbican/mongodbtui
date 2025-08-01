#!/bin/bash
echo "Building MongoDB TUI..."
cargo build --release

sudo cp target/release/mongodbtui /usr/local/bin/

echo " Installed! Run with: mongodbtui"
