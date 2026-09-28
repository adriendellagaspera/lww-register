# lww-register

A small state-based last-write-wins register with Hybrid Logical Clock (HLC) primitives.

The crate provides:

- `Entry<T, V>` and `State<V>` for generic LWW state;
- `LwwRegister<V>` as an `Entry<Timestamp, V>` convenience alias;
- HLC `Timestamp`, `Hlc`, `NodeId`, and related primitives;
- an object-safe `Clock` port plus `assert_conformance` for clock adapters.

It deliberately does **not** provide networking, membership, persistence, storage codecs, async
runtime integration, or a wall-clock implementation.

## Install

```toml
[dependencies]
lww-register = "0.6"
```

Serialization is optional:

```toml
lww-register = { version = "0.6", features = ["serde"] }
```

## Example

```rust
use lww_register::{
    Hlc, LogicalCounter, LwwRegister, NodeId, PhysicalTime, Timestamp,
};

fn stamp(ms: u64, node: u64) -> Timestamp {
    Timestamp::new(
        Hlc::new(PhysicalTime::from_millis(ms), LogicalCounter::ZERO),
        NodeId::new(node),
    )
}

let older = LwwRegister::present(stamp(10, 1), "old");
let newer = LwwRegister::present(stamp(11, 2), "new");

assert_eq!(older.merge(&newer).value(), Some(&"new"));
assert_eq!(newer.merge(&older).value(), Some(&"new"));
```

## Convergence requirement

`Entry::merge` chooses the strictly greater stamp. If two distinct conflicting writes reuse the
same stamp, an LWW register has no deterministic winner and merge is not commutative for that
collision.

For the provided `Timestamp` type, use a distinct `NodeId` for every independently writing live
replica and a `Clock` implementation that satisfies `assert_conformance`.

## Provenance

`lww-register` was extracted from
[`reconcile-rs`](https://github.com/adriendellagaspera/reconcile-rs). Version 0.6.0 is the first
release maintained from this standalone repository.

## License

MIT OR Apache-2.0.
