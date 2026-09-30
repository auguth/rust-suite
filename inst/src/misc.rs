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
// ````````````````````````````````` INST MISC ```````````````````````````````````
// ===============================================================================

//! Module Comment (TBD)

// ===============================================================================
// `````````````````````````````` RE-ENTRANT MACROS ``````````````````````````````
// ===============================================================================

#[macro_export]
macro_rules! __instance_direct_internal_only {
    ([$($args:tt)*] $expr:expr) => {
        $crate::instance_direct_access!(
            #[instance_access($($args)*)]
            $expr
        )
    };
}

#[macro_export]
macro_rules! __instance_get_internal_only {
    ($expr:expr $(, $args:tt)* $(,)?) => {
        $crate::instance_get_access!(
            #[instance_access()]
            $expr $(, $args)*
        )
    };
}
