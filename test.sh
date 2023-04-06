#!/bin/sh

./build.sh

echo ">> Integration tests"

cargo run --example integration-tests "./target/wasm32-unknown-unknown/release/near_payments.wasm" 