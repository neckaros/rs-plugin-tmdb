cargo build --target wasm32-unknown-unknown --release
TMDB_API_KEY=your_key cargo test --test lookup_test -- --nocapture


## Person types

Version 0.13.0 uses `rs-plugin-common-interfaces` 0.38.0 and emits canonical
person types as plain JSON strings (for example `"type": "Actor"`). TMDB
`Acting`, `Directing`, `Writing`, and `Production` departments map to `Actor`,
`Director`, `Writer`, and `Producer`. Unknown departments keep their exact labels
as custom types; empty departments are omitted. A `Sound` department does not
imply `Singer`.

Cast summaries use `Actor`; movies additionally include `Director` credits,
and shows include `Creator` credits from TMDB's `created_by` field. Full person lookup
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

Movie and show metadata include **all unique cast members**, sorted by TMDB's
cast `order` (lowest first). Missing order values come last; ties preserve provider
order. Repeated cast credits are merged into one person without a numerical limit.

Movies additionally include directors only. Shows additionally include creators
from TMDB's `created_by` field only. Other crew members are not imported unless
they are also cast members or qualify as a movie director or show creator.
People present in both cast and eligible crew appear once, with all their roles
and character names preserved.

This limits future imports. Existing people and relationships in a library
are not removed by refresh.

### Relationship credits

The plugin returns one object per selected person in `relations.peopleDetails`.
Each object contains the person summary plus optional `roles`, `characters`, and
integer `rank` fields. There are no parallel credit maps in plugin output.

Roles use canonical PersonType strings and include all mapped roles for that
selected person. Character names are nonblank and deduplicated. Rank preserves
TMDB's zero-based cast `order`; duplicate credits use the lowest known order.
Unknown ranks and character names are omitted. Crew selection is unchanged;
the cast has no numerical limit.

This uses common interfaces 0.40.0. Update the server before updating the plugin
so inline relationship fields are persisted, then refresh existing title credits.

## Relationship search

Version 0.17.0 uses `rs-plugin-common-interfaces` 0.41.0 and supports relationship
filters on movie and show lookup queries:

- `people` accepts a TMDB person ID (`tmdb-person`) or a name. An omitted `role`
  matches either cast or crew; a supplied role restricts the matching credit.
- `tags` accepts a TMDB genre ID (`tmdb-genre`) or an exact genre name. Returned
  genre tags expose the same `tmdb-genre:<id>` external ID.
- `series` on movie queries accepts a TMDB collection ID (`tmdb-collection`) or
  an exact collection name. Movie results expose that collection in
  `relations.seriesDetails`.

TMDB has no equivalent parent-franchise filter for TV shows, so a `series`
constraint on a show query returns no matches. Relationship filters can be used
alone to discover titles, or alongside a title/provider ID to constrain its
results.
