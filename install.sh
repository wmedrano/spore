#!/bin/sh

cargo build --release
mkdir -p $HOME/.spore/bin
cp ./target/release/spore $HOME/.spore/bin/spore
