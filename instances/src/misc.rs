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
// ``````````````````````````````` MISCELLAENEOUS ````````````````````````````````
// ===============================================================================

//! Miscellaneous type-level utilities.
//!
//! This module contains foundational types and traits used across the
//! type-level system, including the global metadata carrier, exact type
//! equality, type-level numeric and boolean markers, and the common [`Marker`]
//! bound shared by type-level values.

// ===============================================================================
// ```````````````````````````````````` IMPORT ```````````````````````````````````
// ===============================================================================

// --- Local Crate Re-Exports ---
use crate::*;

// ===============================================================================
// ``````````````````````````````````` STRUCTS ```````````````````````````````````
// ===============================================================================

/// Global type-level carrier for metadata across instance boundaries.
///
/// `Global` provides the common type-level context through which generated
/// access traits carry and recover metadata between the declaration
/// side and definition side of an instance.
///
/// The declaration side and definition side of instances intentionally do not
/// use the same types. Their metadata is instead carried through access traits
/// anchored to `Global`. This provides a common type-level channel between otherwise
/// distinct types while remaining within Rust's coherence and soundness rules.
///
/// The metadata is encoded in implementations of the crate-owned access traits,
/// while their trait parameters identify the instance context, identifier path,
/// generic position, and other compile-time information required to resolve it.
///
/// `Global` therefore represents the global metadata channel of the instance
/// system rather than any particular instance.
pub struct Global;

// ===============================================================================
// ```````````````````````````````````` TRAITS ```````````````````````````````````
// ===============================================================================

/// Requires the implementing type to be exactly `T`.
///
/// This is used in bounds to require an exact concrete type rather than merely
/// a type that satisfies a trait bound. For example, `Exact<Type>` can be used
/// to require an implementation to provide `Type` itself.
///
/// When `T` is an associated type, the implementing type must still be the
/// concrete type that resolves to that associated type.
///
/// ```ignore
/// T: Exact<Type>
/// ```
///
/// This prevents the implementation from substituting another type that merely
/// satisfies the surrounding bounds.
pub trait Exact<T> {}

impl<T> Exact<T> for T {}

/// Type-level unsigned integer with array-length semantics.
pub trait UnsignedTypeNum: ArrayLength + 'static + Unsigned + Marker {}

impl<T> UnsignedTypeNum for T where T: ArrayLength + Unsigned + Marker {}

/// Type-level boolean value.
pub trait BoolTypeNum: Bit + Marker {}

impl<T> BoolTypeNum for T where T: Bit + Marker {}

/// Type-level marker value.
///
/// This trait is ultimately intended for marker types, such as zero-sized
/// marker structs used to represent type-level state.
///
/// It is also implemented by type-level values used throughout the type-level
/// system, particularly typenum constants and counter tuples.
pub trait Marker:
    'static
    + Eq
    + PartialEq
    + Copy
    + Clone
    + Default
    + Debug
    + Ord
    + PartialOrd
    + Hash
    + Send
    + Sync
    + Sized
{
}

impl<T> Marker for T where
    T: 'static
        + Eq
        + PartialEq
        + Copy
        + Clone
        + Default
        + Debug
        + Ord
        + PartialOrd
        + Hash
        + Send
        + Sync
        + Sized
{
}
