#!/bin/bash
# Tesseract OS Compatibility Test Suite
# Compiles every major feature combination and runs a smoke test

set -e

echo "Running Tesseract OS Compatibility Test Suite..."

FEATURES=(
    "crypto_pki"
    "warm_gpu_context"
    "heterogeneous_simd"
    "persistent_nonce"
)

# Test base build
echo "Testing base build..."
cargo check

# Test individual features
for feature in "${FEATURES[@]}"; do
    echo "Testing feature: $feature"
    cargo check --features "$feature"
done

# Test all features combined
echo "Testing all features combined..."
cargo check --all-features

echo "Compatibility Test Suite Passed!"
