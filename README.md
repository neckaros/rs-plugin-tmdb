cargo build --target wasm32-unknown-unknown --release
TMDB_API_KEY=your_key cargo test --test lookup_test -- --nocapture


## Person types

Version 0.13.0 uses `rs-plugin-common-interfaces` 0.38.0 and emits canonical
person types as plain JSON strings (for example `"type": "Actor"`). TMDB
`Acting`, `Directing`, `Writing`, and `Production` departments map to `Actor`,
`Director`, `Writer`, and `Producer`. Unknown departments keep their exact labels
as custom types; empty departments are omitted. A `Sound` department does not
imply `Singer`.

Cast summaries use `Actor`; the included director, writer/screenplay, producer,
and creator credits use their corresponding canonical types. Full person lookup
uses the person's primary department. Existing cast-first deduplication is
preserved when the same person appears in both cast and crew.

To verify person-type changes, build the plugin before running the pure
conversion/parser tests and the live WASM lookup:

```sh
cargo build --target wasm32-unknown-unknown --release
cargo test --test conversion_test
cargo test --test lookup_test test_lookup_person_type_uses_canonical_string
```

The live lookup requires TMDB network access. The conversion test target includes
only the pure modules; native `cargo test --lib` also links Extism host imports
that are normally provided by the WASM runtime.


## Cast selection during refresh

Movie and show metadata include at most **10 unique cast members**, sorted by
TMDB's cast `order` (lowest first). Missing order values come last; ties and
missing values retain provider order. Duplicate credits do not consume extra
slots. Filtering happens before the server fetches full person details.

Selected directors, writers/screenplay writers, producers, and creators are
included separately from the cast limit, so the total can exceed 10 people.
A person present in both selected cast and crew is returned once. A person
outside the cast cutoff can still be included for a selected crew job.

This limits future imports. Existing people and relationships in a library
are not removed by refresh.
