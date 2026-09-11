// Run the pure conversion/parser unit tests natively without linking lib.rs's
// Extism host imports. Runtime behavior is covered by lookup_test using the WASM.
#[path = "../src/tmdb.rs"]
mod tmdb;
#[path = "../src/convert.rs"]
mod convert;
