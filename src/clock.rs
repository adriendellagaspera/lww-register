// Copyright 2023 Developers of the reconcile project.
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Hybrid Logical Clock domain types.
//!
//! [`Timestamp`] orders writes by `(physical, logical, node_id)`. [`AdmittedTime`] represents a
//! physical reading admitted under the drift policy before it can advance [`Hlc`]. [`Clock`] is
//! the injectable clock port; [`assert_conformance`] checks the runtime monotonicity contract.
//! This crate does not read wall-clock time; adapters do.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

mod admitted;
mod hlc;
mod primitives;
mod timestamp;

/// A **duration** in milliseconds: how far a clock reading may lead another before it is suspect.
/// Not [`PhysicalTime`], which is an **instant**: a budget is never comparable to an instant.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClockDrift(u64);

/// Default maximum a remote clock may lead physical time before its [`PhysicalTime`] is clamped.
/// One hour: orders of magnitude above ordinary clock skew, still finite. Without a cap, an
/// untrusted timestamp near `u64::MAX` can pin local clock state there permanently.
/// Concrete [`Clock`] adapters may choose a different admission budget.
pub const MAX_CLOCK_DRIFT: ClockDrift = ClockDrift::from_millis(3_600_000); // 1 hour

/// The **physical time** of a [`Timestamp`]: an instant, in milliseconds since the Unix epoch.
/// Arithmetic on it is narrow and saturating, so no call site reasons about wrapping.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PhysicalTime(u64);

/// The **logical counter** of a [`Timestamp`]: disambiguates events in one [`PhysicalTime`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct LogicalCounter(u32);

/// Replica identity used as the deterministic final tie-break in [`Timestamp`] ordering.
///
/// Independently writing replicas must use distinct IDs. An ID collision can mint identical
/// timestamps for distinct conflicting writes and therefore prevent convergence.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct NodeId(u64);

/// A remote physical-time reading admitted to the local clock state under the drift policy.
/// Obtainable only via [`clamped_to_drift`](AdmittedTime::clamped_to_drift) or
/// [`trusted`](AdmittedTime::trusted) — no public field, no `Default`, no `From<PhysicalTime>`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AdmittedTime {
    physical: PhysicalTime,
    clamped: bool,
}

/// A Hybrid Logical Clock reading: `(physical, logical)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Hlc {
    /// Physical time: the instant last observed by the clock.
    physical: PhysicalTime,
    /// Logical counter: disambiguates events sharing the same `physical`.
    logical: LogicalCounter,
}

/// The LWW ordering key: `(physical, logical, node_id)`.
///
/// Field declaration order defines the derived [`Ord`] conflict order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Timestamp {
    /// The clock reading: what time it was, as this node's clock sees time.
    hlc: Hlc,
    /// Identity of the node that minted this timestamp; provides the deterministic tie-break.
    node_id: NodeId,
}

/// Clock port for minting and observing [`Timestamp`] values.
///
/// The implementation owns physical-time reads and this writer's [`NodeId`]. The concrete
/// [`Timestamp`] return type keeps the trait object-safe. Use [`assert_conformance`] to check a
/// clock implementation's required ordering behavior.
pub trait Clock: Send + Sync + 'static {
    /// Mint a strictly-monotonic local timestamp for a write or an outgoing message.
    fn now(&self) -> Timestamp;
    /// This node's identity, as stamped onto every timestamp minted. Costs no counter tick.
    fn node_id(&self) -> NodeId;
    /// Advance past a peer's timestamp, so a subsequent [`now`](Clock::now) is ordered after it.
    /// Holds for stamps within a bounded lead: an implementation may clamp beyond
    /// [`MAX_CLOCK_DRIFT`] via [`AdmittedTime::clamped_to_drift`]. The [`Timestamp`] order and the
    /// strict-`>` merge are unaffected.
    fn observe(&self, remote: Timestamp);
    /// Advance past a stamp **this node itself authored**, for example one restored from durable
    /// state, so the next [`now`](Clock::now) outranks it.
    /// Implementations must **not** clamp here — the one caller entitled to
    /// [`AdmittedTime::trusted`] — or a backward clock step re-introduces own-write shadowing. No
    /// default body: delegating to [`observe`](Clock::observe) is only sound for a clamp-free
    /// adapter, and a default silently makes that the fallback for every adapter that clamps,
    /// including one written after this trait. Stating the clamp policy explicitly, every time, is
    /// the point. [`assert_conformance`] checks it holds.
    fn observe_trusted(&self, remote: Timestamp);
}

/// Assert the invariants required by the [`Clock`] contract.
///
/// The check exercises strict local monotonicity, observation of a remote timestamp, and trusted
/// observation beyond the drift budget. A conforming clock must make the next [`Clock::now`]
/// strictly greater in each case.
///
/// # Panics
///
/// Panics when the clock violates one of these invariants.
pub fn assert_conformance<C: Clock>(clock: &C) {
    // 1. `now` alone must be strictly monotonic.
    let mut prev = clock.now();
    for _ in 0..1_000 {
        let next = clock.now();
        assert!(
            next > prev,
            "Clock::now() must be strictly monotonic: {next:?} is not > {prev:?}"
        );
        prev = next;
    }

    // 2. A modest, in-budget lead must be chased by `observe`, not ignored.
    let modest_future = Timestamp::new(
        Hlc::new(
            prev.physical()
                .saturating_add(ClockDrift::from_millis(1_000)),
            LogicalCounter::ZERO,
        ),
        NodeId::new(clock.node_id().get().wrapping_add(1)),
    );
    clock.observe(modest_future);
    let after_observe = clock.now();
    assert!(
        after_observe > modest_future,
        "Clock::observe(t) must be followed by a now() > t for an in-budget t: {after_observe:?} \
         is not > {modest_future:?}"
    );

    // 3. `observe_trusted` must never clamp, even for a stamp far beyond `MAX_CLOCK_DRIFT`.
    let far_future = Timestamp::new(
        Hlc::new(
            after_observe
                .physical()
                .saturating_add(MAX_CLOCK_DRIFT)
                .saturating_add(ClockDrift::from_millis(1)),
            LogicalCounter::ZERO,
        ),
        clock.node_id(),
    );
    clock.observe_trusted(far_future);
    let after_trusted = clock.now();
    assert!(
        after_trusted > far_future,
        "Clock::observe_trusted(t) must never clamp: now() must be > t even for t far beyond \
         MAX_CLOCK_DRIFT, or a backward wall-clock step across a restart can shadow this node's \
         own pre-restart writes: {after_trusted:?} is not > {far_future:?}"
    );
}

#[cfg(test)]
mod tests;
