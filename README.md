cargo build --target wasm32-unknown-unknown --release
TMDB_API_KEY=your_key cargo test --test lookup_test -- --nocapture
