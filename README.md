# `@tsonic/rust-js`

Rust runtime implementation for Tsonic's explicitly selected JavaScript source
surface. The canonical crate is `tsonic_rust_js`; it depends on the installed
`@tsonic/rust-runtime` through explicit runtime contributions.

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
