#!/usr/bin/env bash
set -e

# Clean and rebuild with coverage
cargo clean
cargo llvm-cov --lib --no-report

# Run tests
cargo llvm-cov test --lib --no-report

# Generate report
cargo llvm-cov report --lib
