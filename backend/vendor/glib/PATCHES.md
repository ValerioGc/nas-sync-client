# GLib 0.18.5 compatibility patch

## Purpose

This directory contains the published `glib` 0.18.5 crate under its original MIT
license. Tauri's GTK 3/WebKit dependency chain requires the 0.18 API; adding GLib
0.20 would not replace the older version used by those dependencies.

The local backport fixes **RUSTSEC-2024-0429 / GHSA-wrw7-89jp-8q8g** while preserving
API compatibility. The original license and copyright files are included.

## Patch details

In [`src/variant_iter.rs`](src/variant_iter.rs),
`VariantStrIter::impl_get` passes a mutable pointer reference (`&mut p`) to the C
output argument. The original code passed an immutable reference (`&p`) and then
allowed C to write through it, causing undefined behavior and possible crashes
with compiler optimizations.

The change follows the [upstream fix](https://github.com/gtk-rs/gtk-rs-core/pull/1343).

Compiler compatibility cleanup also makes existing lifetimes explicit in return
types and removes redundant parentheses around function trait objects. These
syntax changes address `mismatched_lifetime_syntaxes` and `unused_parens` warnings
on current Rust toolchains without changing runtime behavior. Keep other vendored
sources aligned with the upstream crate.

## Regression test

The [regression test](../../tests/glib_variant.rs) covers forward and backward
string iteration, `nth`, and `last`. Run it from the repository root in release
mode, where compiler optimizations expose the original bug.

```sh
cargo test --manifest-path backend/Cargo.toml --release --test glib_variant
```

## Maintenance

The Cargo override is declared in [`backend/Cargo.toml`](../../Cargo.toml).
Retain it until the GTK/Tauri dependency chain can use a fixed upstream release.
When replacing the backport, verify the actual dependency graph and rerun the
release-mode regression test before removing the override.
