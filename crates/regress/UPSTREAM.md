# Upstream provenance

This crate is derived from `ridiculousfish/regress` at exact commit
`1621a47cb0cf679a492ac8a88efa816d1640febb` (version `0.11.1`). The original
project is dual-licensed under MIT or Apache-2.0; both license texts are retained
in this directory.

Tsonic vendors the source because JavaScript RegExp execution must have a
deterministic, checked work budget. Product calls use the bounded APIs added by
Tsonic. Grammar, bytecode, matching, and Unicode behavior remain target-neutral
ECMAScript engine concerns; no Tsonic source names or Rust-target selection are
introduced into this crate.
