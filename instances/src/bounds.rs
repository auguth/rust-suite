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
// `````````````````````````````` TYPENUMS & BOUNDS ``````````````````````````````
// ===============================================================================

//! Type-level numeric bounds and typenum primitives.
//!
//! This module defines compile-time minimum and maximum bounds for the
//! supported signed and unsigned integer widths.
//!
//! [`BoundMin`] and [`BoundMax`] expose the minimum and maximum values of a
//! type-level integer domain through associated types. The concrete marker
//! types identify the integer width and signedness:
//!
//! - [`U8Type`] - Unsigned 8-bit integer domain, from [`U0`] to [`U255`].
//! - [`U16Type`] - Unsigned 16-bit integer domain, from [`U0`] to [`U65535`].
//! - [`U32Type`] - Unsigned 32-bit integer domain, from `U0` to [`U4294967295`].
//! - [`I8Type`] - Signed 8-bit integer domain, from [`N128`] to [`P127`].
//! - [`I16Type`] - Signed 16-bit integer domain, from [`N32768`] to [`P32767`].
//! - [`I32Type`] - Signed 32-bit integer domain, from [`N2147483648`] to [`P2147483647`].
//!
//! The bounds are represented entirely at the type level using typenum
//! integer types, allowing generic code to reason about numeric domains
//! without requiring runtime values.

// ===============================================================================
// ```````````````````````````````````` IMPORT ```````````````````````````````````
// ===============================================================================

// --- Local Crate Re-Exports ---
use crate::*;

// ===============================================================================
// `````````````````````````````````` BOUNDS MAX `````````````````````````````````
// ===============================================================================

/// Provides the maximum value of a type-level numeric domain.
pub trait BoundMax {
    type Max: Marker + PartialOrd;
}

/// Provides the minimum value of a type-level numeric domain.
pub trait BoundMin {
    type Min: Marker + PartialOrd;
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ``````````````````````````````````` UNSIGNED ``````````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// Type-level unsigned 8-bit integer domain.
pub struct U8Type;

/// Type-level unsigned 16-bit integer domain.
pub struct U16Type;

/// Type-level unsigned 32-bit integer domain.
pub struct U32Type;

impl BoundMin for U8Type {
    type Min = U0;
}

impl BoundMax for U8Type {
    type Max = U255;
}

impl BoundMin for U16Type {
    type Min = U0;
}

impl BoundMax for U16Type {
    type Max = U65535;
}

impl BoundMin for U32Type {
    type Min = U0;
}

impl BoundMax for U32Type {
    type Max = U4294967295;
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ``````````````````````````````````` INTEGER ```````````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// Type-level signed 8-bit integer domain.
pub struct I8Type;

/// Type-level signed 16-bit integer domain.
pub struct I16Type;

/// Type-level signed 32-bit integer domain.
pub struct I32Type;

impl BoundMax for I8Type {
    type Max = P127;
}

impl BoundMin for I8Type {
    type Min = N128;
}

impl BoundMax for I16Type {
    type Max = P32767;
}

impl BoundMin for I16Type {
    type Min = N32768;
}

impl BoundMax for I32Type {
    type Max = P2147483647;
}

impl BoundMin for I32Type {
    type Min = N2147483648;
}
