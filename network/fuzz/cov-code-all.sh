
COVERAGE_TARGET_DIR=target/aarch64-apple-darwin/coverage/aarch64-apple-darwin/release/

cargo-fuzz coverage fuzz_peer_store -- \
    --ignore $CARGO_HOME/*
cargo cov \
    -- \
    show $COVERAGE_TARGET_DIR/fuzz_peer_store \
    --ignore-filename-regex="$CARGO_HOME/*" \
    --ignore-filename-regex="error/*" \
    --ignore-filename-regex="util/*" \
    --ignore-filename-regex="pow/*" \
    --ignore-filename-regex="resource/*" \
    --ignore-filename-regex="spec/*" \
    -instr-profile=coverage/fuzz_peer_store/coverage.profdata \
    --format=html \
> 'fuzz_peer_store.html'