#!/bin/bash
cd "$(dirname "$0")"
cargo build --features debug
