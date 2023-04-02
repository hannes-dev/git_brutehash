#! /usr/bin/env bash

./make_test_dir.sh

cargo build --release
cd test_dir
hyperfine "../target/release/brutecommit -d 00000 -t1" --warmup 3 --min-runs 10
cd ..
rm -rf ./test_dir