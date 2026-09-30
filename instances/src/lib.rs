// SPDX-License-Identifier: MPL-2.0
//
// Part of Auguth Labs open-source softwares.
// Built for the Rust Programming Language Ecosystem.
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Copyright (c) 2026 Auguth Labs (OPC) Pvt Ltd, India

// ===============================================================================
// `````````````````````````````````` INSTANCES ``````````````````````````````````
// ===============================================================================

//! Instances (Low-Level Macros)
//! 
//! Module Doc (TBD)

// ===============================================================================
// ``````````````````````````````````` MODULES ```````````````````````````````````
// ===============================================================================

mod access;
mod affiliates;
mod bounds;
mod counters;
mod idents;
mod misc;

// ===============================================================================
// ````````````````````````````````` RE-EXPORTS ``````````````````````````````````
// ===============================================================================

// --- Instances Macros ---
pub use instances_macros::*;

// --- TypeNum ---
pub use typenum::{Bit, Diff, IsEqual, Max, Sum, Unsigned, consts::*};

// --- Gen-Arrays ---
pub use generic_array::{ArrayLength, GenericArray};

// --- Macro Paths ---
pub use access::*;
pub use affiliates::*;
pub use bounds::*;
pub use counters::*;
pub use idents::*;
pub use misc::*;

// --- Std & Core Ops ---
pub use core::{fmt, hash::Hash};
pub use std::{
    cmp::Eq,
    fmt::Debug,
    mem::MaybeUninit,
    ops::{Add, BitAnd, Sub},
};

// ===============================================================================
// ``````````````````````````````````` PUBLIC ````````````````````````````````````
// ===============================================================================

/// Errors that can occur while dynamically accessing an instance impl through a dynamic counter ident.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Error {
    /// The requested instance identifier does not exist.
    ///
    /// The instance identifier could not be found within the available counter range.
    OutOfRange,

    /// The requested instance identifier exists, but cannot be accessed at the
    /// requested depth.
    /// 
    /// This error can be avoided via setting the depth to `>` [`Error::Exhausted::expect`]
    Exhausted {
        /// Depth at which access was exhausted.
        depth: usize,

        /// Depth required to reach the resolved identifier.
        expect: usize,

        /// Counter-generic position being accessed (internal).
        index: usize,
    },
}
