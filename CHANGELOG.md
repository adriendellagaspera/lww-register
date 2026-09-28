# Changelog

All notable changes to this crate are documented here.

## 0.6.0 - 2026-09-28

First release maintained from the standalone `adriendellagaspera/lww-register` repository.

### Changed

- narrowed the crate to reusable LWW-register and Hybrid Logical Clock primitives;
- moved reconcile-specific key/value bounds and persistence state out of this crate;
- made `serde` support optional behind the `serde` feature;
- re-exported the complete clock-domain surface from the crate root;
- added `LwwRegister<V>` as the timestamp-backed convenience alias;
- generalized `Entry::merge` from `T: Copy` to `T: Clone`;
- documented the unique-stamp requirement for CRDT convergence.

## 0.5.0

Historical release produced from the `reconcile-rs` repository.
