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
| `padStart` | implemented — `string::pad_start` / `pad_start_with`, ABI `js_string_pad_start` / `js_string_pad_start_with`; applies `ToLength`, uses the default space filler when omitted, and fails with `RangeError` at the closed runtime allocation limit |
| `padEnd` | implemented — `string::pad_end` / `pad_end_with`, ABI `js_string_pad_end` / `js_string_pad_end_with`; applies `ToLength`, uses the default space filler when omitted, and fails with `RangeError` at the closed runtime allocation limit |
| `repeat` | implemented — `string::repeat`, ABI `js_string_repeat` |
| `trimStart` | implemented — `string::trim_start`, ABI `js_string_trim_start` |
| `trimEnd` | implemented — `string::trim_end`, ABI `js_string_trim_end` |
| `at` | implemented — `string::at`, ABI `js_string_at` |
| `charAt` | implemented — `string::char_at`, ABI `js_string_char_at` |
| `charCodeAt` | implemented — `string::char_code_at`, ABI `js_string_char_code_at`; returns `NaN` outside the string |
| `codePointAt` | implemented — `string::code_point_at`, ABI `js_string_code_point_at` |
| `substring` / `substr` | implemented — omitted and supplied trailing arguments use separate closed entrypoints; numeric arguments use ECMAScript integer coercion |
| `lastIndexOf` | implemented — omitted and supplied positions use separate closed entrypoints and UTF-16 indexes |
| `replaceAll` | implemented — `string::replace_all`, including string-search replacement substitutions |
| `concat`, `valueOf`, `trimLeft`, `trimRight` | implemented — exact aliases and one borrowed string-slice concat ABI |
| `String.fromCharCode` / `String.fromCodePoint` | implemented — fallible UTF-16/code-point constructors over one numeric slice ABI |
| `match` / `matchAll` | implemented — `regexp::JsRegExpMatch` carrier via `JsRegExp::match_first` (non-`g` `match`), `JsRegExp::match_strings` (`g` `match`), and `JsRegExp::match_all` (`matchAll`, `TypeError` without `g`) |
| `localeCompare`, `toLocale*`, `normalize` | requires-icu-contract — locale/normalization tables are not part of the closed runtime |
| `isWellFormed` / `toWellFormed` | rejected-by-architecture — Rust `str` is always well-formed UTF-8; lone surrogates cannot be carried |

All other members on the C# `String` surface (`slice`, `indexOf`,
`startsWith`, `endsWith`, `includes`, `replace` (string form), `split`
(string form), `trim`, `toLowerCase`, `toUpperCase`, `raw`, and `length` as
`js_len`) are implemented in `string.rs`. Operations that would produce an
unpaired UTF-16 surrogate return a deterministic unsupported error because a
Rust `String` cannot represent that result.

## Array

| Member | Disposition |
| --- | --- |
| `find` / `findIndex` | implemented on `array::JsArray`; callbacks skip holes |
| `findLast` / `findLastIndex` | implemented on `array::JsArray`; callbacks skip holes |
| `includes` / `indexOf` | implemented on `array::JsArray` with hole-aware search semantics |
| `join` | implemented on `array::JsArray`; holes stringify as empty fields |
| `slice` | implemented on `array::JsArray` and preserves holes |
| `concat` | implemented on `array::JsArray` with exact `JsArrayConcatItem<T>` value/array alternatives; preserves holes and shallow-copies source arrays |
| `map` / `filter` / `reduce` / `some` / `every` | implemented on `array::JsArray` with initial-length callback bounds and live element reads |
| `Array.of` | implemented as `array::of`, ABI `array_of`; owns each exact element in one fixed Rust array before constructing the identity-preserving carrier |
| `Array.from(string)` | implemented as `array::from_string`, ABI `array_from_string`; iterates Unicode code points exactly |
| `Array.isArray` | implemented as `array::is_array_value`, ABI `array_is_array_value`, over the closed `JsValue` carrier |

Dense and sparse arrays use the same identity-preserving `array::JsArray`
carrier. Assignment and parameter passing clone only the reference handle;
mutations remain visible through every alias.

## Math / Number

Nothing needed in rust-js: `std` `f64` plus the existing `math.rs` and
`number.rs` already cover the C# `Math.cs`/`Number.cs` surface used by the
backend. No delta.

## JSON

`json::parse` / `json::stringify` are implemented (ABI `json_parse` /
`json_stringify`) over the closed `JsValue` carrier. Stringification returns
`Option<String>` so top-level `undefined` remains distinct from an empty JSON
string. Parsing and stringification enforce explicit input, output, depth, and
node/member limits; allocation or borrow failures are deterministic `JsError`
values. The `space` argument is
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

- implemented: literals, positive character classes with ranges capped at
  U+D7FF, class escapes `\d \w \s`, identity/control/hex escapes, greedy
  `* + ? {n} {n,} {n,m}`, `^ $`, alternation, capturing and non-capturing
  groups; flags `i g m`; operations `test`, `find_first`, `replace` (with
  `$$ $& $` $' $1..$99` substitution), `split`, `search`, `exec` (stateful
  `lastIndex` contract under `g`: UTF-16 code-unit progression, reset to 0 on
  no match; `last_index`/`set_last_index` accessors — writable `lastIndex`
  is exact for non-nullable patterns, including values that land between the
  two code units of a surrogate pair: no accepted atom can match a lone
  surrogate, so scanning from a mid-pair position is equivalent to scanning
  from the next char boundary, proven by the `set-lastindex` oracle
  vectors. Nullable patterns — ones that can match the empty string — reject
  manual `lastIndex` assignment with a deterministic `Unsupported` error:
  Node can match empty *at* a mid-pair position (`/a*/g` with
  `lastIndex = 1` on `"💚"` matches `""` at UTF-16 index 1), which no Rust
  `String` can express. Exec-driven `lastIndex` values always land on char
  boundaries, so the natural exec flow stays exact for nullable patterns
  too, proven by the nullable-over-astral `exec` oracle vector),
  `match_first`,
  `match_strings`, `match_all` (`TypeError` without `g`), and the flag
  getters `global`/`ignore_case`/`multiline`. Match results are carried by
  `JsRegExpMatch` (`text`, UTF-16 `index`, `input`, 1-based `group`,
  `group_count`). Matching operations are fallible and enforce a deterministic
  VM step budget derived from program and input size. Exhaustion reports a
  `RangeError` rather than returning a false no-match result or allowing
  adversarial backtracking to run without a bound.
- rejected-by-architecture (deterministic `SyntaxError` at construction):
  lazy quantifiers, backreferences, lookaround, named groups, `\b`/`\B`,
  `\p`/`\P`, `\c`, `\k`, `\u{...}`, flags `d s u v y`, quantifier bounds
  above 1000, and `split` over patterns with capturing groups (JS splices
  captures into the result; use `(?:...)`).
- also rejected at construction (code-unit-sensitive constructs): `.`,
  negated classes (`[^...]` and `\D \W \S`, inside or outside classes),
  class ranges reaching past U+D7FF, astral chars inside classes, and
  quantifiers on a bare astral literal. Their non-`u` semantics are defined
  over UTF-16 code units, not scalar values: Node's `/./.exec("😀")` yields
  a *lone high surrogate* — a string a Rust `String` cannot represent —
  negated/surrogate-range classes likewise match lone surrogates, and a
  quantifier after an astral literal binds to its trailing low surrogate
  only. Rejecting the constructs at construction is fail-closed and
  independent of the input searched (unlike a per-call guard). Positive BMP
  classes (ranges up to U+D7FF, singles up to U+FFFF), unquantified astral
  literals, and grouped-and-quantified astral literals (`(?:😀)+`, which
  repeat the whole surrogate pair) remain exact, proven by the oracle
  vectors.
