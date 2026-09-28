// Copyright 2023 Developers of the reconcile project.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! A small state-based last-write-wins register and Hybrid Logical Clock domain.
//!
//! [`Entry<T, V>`] is the generic register state. [`LwwRegister<V>`] is the recommended alias
//! using this crate's [`Timestamp`] ordering. The crate intentionally contains no networking,
//! membership, persistence, async runtime, storage codec, or wall-clock implementation.
//!
//! # Convergence contract
//!
//! Distinct conflicting writes must receive distinct stamps from one total order. With
//! [`Timestamp`], give every independently writing replica a distinct [`NodeId`] and use a
//! [`Clock`] implementation that satisfies [`assert_conformance`].
#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod clock;
pub mod entry;

pub use clock::{
    assert_conformance, AdmittedTime, Clock, ClockDrift, Hlc, LogicalCounter, NodeId, PhysicalTime,
    Timestamp, MAX_CLOCK_DRIFT,
};
pub use entry::{Entry, State};

/// An LWW register using this crate's Hybrid Logical Clock [`Timestamp`] as its stamp.
pub type LwwRegister<V> = Entry<Timestamp, V>;
