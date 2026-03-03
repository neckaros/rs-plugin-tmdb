Always run a wasm release (cargo build --target wasm32-unknown-unknown --release) build before (and not in parallel) running test.
Running integration tests with: TMDB_API_KEY=your_key cargo test --test lookup_test -- --nocapture
