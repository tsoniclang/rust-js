# Tsonic Rust JS Runtime

Rust JS surface runtime crate for Tsonic-emitted Rust.

The npm artifact `@tsonic/rust-js` owns the canonical JavaScript surface
runtime source tree. Installed Rust targets reference
`crates/tsonic_rust_js`; target packages do not copy this source. Generated
Cargo projects resolve the separately installed `@tsonic/rust-runtime` peer
through explicit runtime contributions and an explicit crates.io source patch;
the packages do not need to be physical filesystem siblings.

## Crate

- Package/crate: `tsonic_rust_js`
