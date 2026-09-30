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
// ```````````````````````````````````` TUPLES ```````````````````````````````````
// ===============================================================================

//! Type-level tuple counter access.
//!
//! This module provides [`CountersGet`] implementations for counter tuples
//! containing one through four type-level unsigned counters. The trait allows
//! an individual counter to be selected from a tuple by its type-level index.
//!
//! The tuple implementations are intended primarily for use by the instances
//! type-level machinery and generated code. They are not intended as a general
//! purpose tuple-access API for unrelated crate code.
//!
//! Their public visibility exists to allow the generated instances machinery
//! to resolve counter positions across the required Rust visibility
//! boundaries. The public signatures should therefore not be considered a
//! stable API.
//!
//! The trait and implementation signatures may be changed as the instances
//! system evolves. Code outside the instances machinery should not depend on
//! their names, generic parameter ordering, associated types, or other
//! signature details.

// ===============================================================================
// ```````````````````````````````````` IMPORT ```````````````````````````````````
// ===============================================================================

// --- Local Crate Re-Exports ---
use crate::*;

// ===============================================================================
// ````````````````````````````````` COUNTERS GET ````````````````````````````````
// ===============================================================================

pub trait CountersGet<Idx: Unsigned>
where
    Self: Marker,
{
    type Output: UnsignedTypeNum;
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// `````````````````````````````````` 1 COUNTER ``````````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

impl<A> CountersGet<U0> for (A,)
where
    A: UnsignedTypeNum,
{
    type Output = A;
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// `````````````````````````````````` 2 COUNTERS `````````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

impl<A, B> CountersGet<U0> for (A, B)
where
    A: UnsignedTypeNum,
    B: UnsignedTypeNum,
{
    type Output = A;
}

impl<A, B> CountersGet<U1> for (A, B)
where
    A: UnsignedTypeNum,
    B: UnsignedTypeNum,
{
    type Output = B;
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// `````````````````````````````````` 3 COUNTERS `````````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

impl<A, B, C> CountersGet<U0> for (A, B, C)
where
    A: UnsignedTypeNum,
    B: UnsignedTypeNum,
    C: UnsignedTypeNum,
{
    type Output = A;
}

impl<A, B, C> CountersGet<U1> for (A, B, C)
where
    A: UnsignedTypeNum,
    B: UnsignedTypeNum,
    C: UnsignedTypeNum,
{
    type Output = B;
}

impl<A, B, C> CountersGet<U2> for (A, B, C)
where
    A: UnsignedTypeNum,
    B: UnsignedTypeNum,
    C: UnsignedTypeNum,
{
    type Output = C;
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// `````````````````````````````````` 4 COUNTERS `````````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

impl<A, B, C, D> CountersGet<U0> for (A, B, C, D)
where
    A: UnsignedTypeNum,
    B: UnsignedTypeNum,
    C: UnsignedTypeNum,
    D: UnsignedTypeNum,
{
    type Output = A;
}

impl<A, B, C, D> CountersGet<U1> for (A, B, C, D)
where
    A: UnsignedTypeNum,
    B: UnsignedTypeNum,
    C: UnsignedTypeNum,
    D: UnsignedTypeNum,
{
    type Output = B;
}

impl<A, B, C, D> CountersGet<U2> for (A, B, C, D)
where
    A: UnsignedTypeNum,
    B: UnsignedTypeNum,
    C: UnsignedTypeNum,
    D: UnsignedTypeNum,
{
    type Output = C;
}

impl<A, B, C, D> CountersGet<U3> for (A, B, C, D)
where
    A: UnsignedTypeNum,
    B: UnsignedTypeNum,
    C: UnsignedTypeNum,
    D: UnsignedTypeNum,
{
    type Output = D;
}
