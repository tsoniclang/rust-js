# `@tsonic/rust-js`

Rust runtime implementation for Tsonic's explicitly selected JavaScript source
surface. The canonical crate is `tsonic_rust_js`; it depends on the installed
`@tsonic/rust-runtime` through explicit runtime contributions.

Indexed-record `Object.keys`, `Object.values`, and `Object.entries` project the
core record's native hash-table contents into dense arrays without copying an
intermediate array. `Object.assign` preserves the destination's reference
identity. These operations retain native enumeration order; they do not add
JavaScript property-order bookkeeping to every record mutation.

Canonical product documentation:

- [JavaScript source profile](https://github.com/tsoniclang/tsonic/blob/main/docs/reference/javascript-source-profile.md)
- [Rust JavaScript surface](https://github.com/tsoniclang/tsonic/blob/main/docs/reference/targets/rust/javascript-surface.md)
- [Rust support inventory](https://github.com/tsoniclang/tsonic/blob/main/docs/reference/targets/rust/support-inventory.md)

## Development

```sh
npm test
```

The runtime's Cargo workspace owns its conformance and differential tests.
Target packages reference `crates/tsonic_rust_js` directly rather than copying
runtime source.
