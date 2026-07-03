# JS parity delta (r11): tsonic_rust_js vs Tsonic.CSharp.Js

Scope of this comparison: the String, dense-Array, Math/Number, JSON, Map/Set,
Date, and RegExp surfaces of `crates/tsonic_rust_js` measured against
`Tsonic.CSharp.Js`. Dispositions used:

- **implemented** — present in `tsonic_rust_js` (module path given; ABI alias
  given where one exists in `abi.rs`).
- **rejected-by-architecture** — deliberately out of scope for the Rust
  backend's closed, statically-typed carrier model; callers get a
  deterministic error or the construct is simply not emitted.
- **requires-\<contract\>** — implementable only once the named runtime
  contract exists; tracked as a gap, not silently approximated.

## String

| Member | Disposition |
| --- | --- |
| `padStart` | implemented — `string::pad_start`, ABI `js_string_pad_start` |
| `padEnd` | implemented — `string::pad_end`, ABI `js_string_pad_end` |
| `repeat` | implemented — `string::repeat`, ABI `js_string_repeat` |
| `trimStart` | implemented — `string::trim_start`, ABI `js_string_trim_start` |
| `trimEnd` | implemented — `string::trim_end`, ABI `js_string_trim_end` |
| `at` | implemented — `string::at`, ABI `js_string_at` |
| `charAt` | implemented — `string::char_at`, ABI `js_string_char_at` |
| `codePointAt` | implemented — `string::code_point_at`, ABI `js_string_code_point_at` |
| `match` / `matchAll` | requires-regexp-match-carrier — the subset engine (`regexp::JsRegExp`) covers `test`/`replace`/`split`/`search`; a match-array carrier type is needed before `match`/`matchAll` can be emitted |
| `localeCompare`, `toLocale*`, `normalize` | requires-icu-contract — locale/normalization tables are not part of the closed runtime |
| `isWellFormed` / `toWellFormed` | rejected-by-architecture — Rust `str` is always well-formed UTF-8; lone surrogates cannot be carried |

All other members on the C# `String` surface (`slice`, `substring`, `substr`,
`indexOf`, `lastIndexOf`, `startsWith`, `endsWith`, `includes`, `replace`
(string form), `split` (string form), `trim`, `toLowerCase`, `toUpperCase`,
`fromCharCode`, `fromCodePoint`, `raw`, `length` as `js_len`) are implemented
in `string.rs`.

## Array (dense)

| Member | Disposition |
| --- | --- |
| `find` | implemented — `array::dense::find`, ABI `array_dense_find` |
| `findIndex` | implemented — `array::dense::find_index`, ABI `array_dense_find_index` |
| `indexOf` | implemented — `array::dense::index_of`, ABI `array_dense_index_of` |
| `lastIndexOf` | implemented — `array::dense::last_index_of`, ABI `array_dense_last_index_of` |
| `join` | implemented — `array::dense::join`, ABI `array_dense_join` |
| `concat` | implemented — `array::dense::concat`, ABI `array_dense_concat` |
| `slice` | implemented — `array::dense::slice`, ABI `array_dense_slice` |
| `flat` | implemented (depth 1) — `array::dense::flat_one`, ABI `array_dense_flat_one`; arbitrary-depth `flat` is rejected-by-architecture (element types are static; nesting depth is a type, not a value) |
| `findLast` / `findLastIndex` | rejected-by-architecture for the ABI ledger until the compiler emits them; trivially expressible as reversed `find`/`find_index` when needed |

Sparse-array semantics live separately in `array::JsArray` (hole-preserving
carrier), mirroring the C# `JSArray` split.

## Math / Number

Nothing needed in rust-js: `std` `f64` plus the existing `math.rs` and
`number.rs` already cover the C# `Math.cs`/`Number.cs` surface used by the
backend. No delta.

## JSON

`json::parse` / `json::stringify` are implemented (ABI `json_parse` /
`json_stringify`) over the closed `JsValue` carrier. `stringify` with a
`replacer`/`space` argument is requires-formatting-contract:
`json::stringify_pretty` currently normalizes to compact output.

## Map / Set

Core surface (`get`/`set`/`has`/`add`/`delete`/`clear`/`keys`/`values`/
`entries`/`forEach`, SameValueZero keys) is implemented in `map.rs`/`set.rs`.
The C# side additionally ships the ES2025 set-algebra methods (`union`,
`intersection`, `difference`, `symmetricDifference`, `isSubsetOf`,
`isSupersetOf`, `isDisjointFrom`): requires-set-algebra-emission — additive
and straightforward once the compiler emits them; not part of the current
closed surface.

## Date

UTC-based surface is implemented in `date.rs` (`now`, `from_millis`, `parse`,
`get_time`, `value_of`, `to_iso_string`, `to_json`, UTC getters). Local-time
getters/setters, `getTimezoneOffset`, and the locale renderers present in the
C# `Date.cs` are requires-timezone-contract: the closed runtime has no IANA
tzdata source. `Date.UTC`-style construction and UTC setters are
requires-date-mutation-contract (the current carrier is an immutable
millisecond value).

## RegExp

`regexp::JsRegExp` implements a closed, oracle-proven subset (see the module
docs in `crates/tsonic_rust_js/src/regexp/mod.rs` and the Node-generated
vectors in `tests/oracle/regexp-vectors.json`):

- implemented: literals, `.`, character classes with ranges/negation, class
  escapes `\d \D \w \W \s \S`, identity/control/hex escapes, greedy
  `* + ? {n} {n,} {n,m}`, `^ $`, alternation, capturing and non-capturing
  groups; flags `i g m`; operations `test`, `find_first`, `replace` (with
  `$$ $& $` $' $1..$99` substitution), `split`, `search`.
- rejected-by-architecture (deterministic `SyntaxError` at construction):
  lazy quantifiers, backreferences, lookaround, named groups, `\b`/`\B`,
  `\p`/`\P`, `\c`, `\k`, `\u{...}`, flags `d s u v y`, quantifier bounds
  above 1000, and `split` over patterns with capturing groups (JS splices
  captures into the result; use `(?:...)`).
- `exec`-style match objects and `lastIndex` statefulness are
  requires-regexp-match-carrier, as for `String.prototype.match`.
