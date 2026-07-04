# JS runtime parity inventory

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
| `match` / `matchAll` | implemented — `regexp::JsRegExpMatch` carrier via `JsRegExp::match_first` (non-`g` `match`), `JsRegExp::match_strings` (`g` `match`), and `JsRegExp::match_all` (`matchAll`, `TypeError` without `g`) |
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
| `find` | implemented — `array::dense::find` (cloned value, `None` for JS `undefined`), ABI `array_dense_find` |
| `findIndex` | implemented — `array::dense::find_index`, ABI `array_dense_find_index` |
| `findLast` | implemented — `array::dense::find_last`, ABI `array_dense_find_last` |
| `findLastIndex` | implemented — `array::dense::find_last_index`, ABI `array_dense_find_last_index` |
| `indexOf` | implemented — `array::dense::index_of`, ABI `array_dense_index_of` |
| `lastIndexOf` | implemented — `array::dense::last_index_of`, ABI `array_dense_last_index_of` |
| `join` | implemented — `array::dense::join`, ABI `array_dense_join` |
| `concat` | implemented — `array::dense::concat`, ABI `array_dense_concat` |
| `slice` | implemented — `array::dense::slice`, ABI `array_dense_slice` |
| `flat` | implemented (depth 1) — `array::dense::flat_one`, ABI `array_dense_flat_one`; arbitrary-depth `flat` is rejected-by-architecture (element types are static; nesting depth is a type, not a value) |
| `flatMap` | implemented (depth 1) — `array::dense::flat_map_one`, ABI `array_dense_flat_map_one` |

Sparse-array semantics live separately in `array::JsArray` (hole-preserving
carrier), mirroring the C# `JSArray` split.

## Math / Number

Nothing needed in rust-js: `std` `f64` plus the existing `math.rs` and
`number.rs` already cover the C# `Math.cs`/`Number.cs` surface used by the
backend. No delta.

## JSON

`json::parse` / `json::stringify` are implemented (ABI `json_parse` /
`json_stringify`) over the closed `JsValue` carrier. The `space` argument is
implemented as `json::stringify_with_indent` (ABI
`json_stringify_with_indent`): it takes a pre-resolved indent string — the
compiler lowers a numeric `space` to `" ".repeat(n)` clamped to 0..=10 and a
string `space` to its first 10 chars — and matches Node's
`JSON.stringify(value, null, space)` output byte-for-byte (empty indent =
compact form; `json::stringify_pretty` remains a compact-output alias).
`stringify` with a `replacer` function is rejected-by-architecture (no
function values in the closed carrier).

## Map / Set

Core surface (`get`/`set`/`has`/`add`/`delete`/`clear`/`keys`/`values`/
`entries`/`forEach`, SameValueZero keys) is implemented in `map.rs`/`set.rs`.
The ES2025 set-algebra methods are implemented on `JsSet` with JS
insertion-order and SameValueZero semantics, mirroring the C# `Set<T>`
surface: `union`, `intersection`, `difference`, `symmetric_difference`
(returning new `JsSet`s; receiver order first), and the predicates
`is_subset_of`, `is_superset_of`, `is_disjoint_from`.

## Date

UTC-based surface is implemented in `date.rs` (`now`, `from_millis`,
`get_time`, `value_of`, `to_iso_string`, `to_json` — the ISO string, `"null"`
for an invalid date — and the UTC getters). `Date.parse` is implemented as
`JsDate::parse` over the ISO 8601 subset Node accepts deterministically:
`YYYY-MM-DD` (UTC midnight) and `YYYY-MM-DDTHH:mm:ss(.sss)?(Z|±HH:MM)`;
everything else — including formats Node's legacy parser reads as local time
— is NaN. `Date.UTC` is implemented as `JsDate::utc` with JS argument
truncation, month/day overflow carry, the 1900-mapping of two-digit years,
and time-range clipping to NaN. Local-time getters/setters,
`getTimezoneOffset`, and the locale renderers present in the C# `Date.cs`
are requires-timezone-contract: the closed runtime has no IANA tzdata
source. UTC setters are requires-date-mutation-contract (the carrier is an
immutable millisecond value).

## RegExp

`regexp::JsRegExp` implements a closed, oracle-proven subset (see the module
docs in `crates/tsonic_rust_js/src/regexp/mod.rs` and the Node-generated
vectors in `tests/oracle/regexp-vectors.json`):

- implemented: literals, `.`, character classes with ranges/negation, class
  escapes `\d \D \w \W \s \S`, identity/control/hex escapes, greedy
  `* + ? {n} {n,} {n,m}`, `^ $`, alternation, capturing and non-capturing
  groups; flags `i g m`; operations `test`, `find_first`, `replace` (with
  `$$ $& $` $' $1..$99` substitution), `split`, `search`, `exec` (stateful
  `lastIndex` contract under `g`: UTF-16 code-unit progression, reset to 0 on
  no match; `last_index`/`set_last_index` accessors), `match_first`,
  `match_strings`, `match_all` (`TypeError` without `g`), and the flag
  getters `global`/`ignore_case`/`multiline`. Match results are carried by
  `JsRegExpMatch` (`text`, UTF-16 `index`, `input`, 1-based `group`,
  `group_count`).
- rejected-by-architecture (deterministic `SyntaxError` at construction):
  lazy quantifiers, backreferences, lookaround, named groups, `\b`/`\B`,
  `\p`/`\P`, `\c`, `\k`, `\u{...}`, flags `d s u v y`, quantifier bounds
  above 1000, and `split` over patterns with capturing groups (JS splices
  captures into the result; use `(?:...)`).
