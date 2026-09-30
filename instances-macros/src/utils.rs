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
// ```````````````````````````` UTILITIES (CRATE-WIDE) ```````````````````````````
// ===============================================================================

//! Provides shared utility functions for generic parameter manipulation.

// ===============================================================================
// ``````````````````````````````````` IMPORTS ```````````````````````````````````
// ===============================================================================

// --- Proc-Macro Crates ---
use proc_macro2::TokenStream;
use syn::{GenericParam, Generics};

// --- Local-Crate ---
use crate::{MiscBugs, MiscErrors};

// ===============================================================================
// ````````````````````````````````` UTILITY FNS `````````````````````````````````
// ===============================================================================

/// Merges two generic parameter lists and their where clauses, rejecting
/// duplicate parameter names.
pub(crate) fn merge_generics(lhs: &Generics, rhs: &Generics) -> Result<Generics, TokenStream> {
    let mut out = lhs.clone();

    for rhs_param in &rhs.params {
        let rhs_ident = match rhs_param {
            GenericParam::Type(param) => &param.ident,

            GenericParam::Lifetime(param) => &param.lifetime.ident,

            GenericParam::Const(param) => &param.ident,
        };

        let duplicate = lhs.params.iter().find(|lhs_param| {
            let lhs_ident = match lhs_param {
                GenericParam::Type(param) => &param.ident,

                GenericParam::Lifetime(param) => &param.lifetime.ident,

                GenericParam::Const(param) => &param.ident,
            };

            lhs_ident == rhs_ident
        });

        if duplicate.is_some() {
            return Err(MiscErrors::DuplicateGenericParameter {
                ident: rhs_ident.clone(),
            }
            .into());
        }

        out.params.push(rhs_param.clone());
    }

    // Merge where clauses.
    match (&mut out.where_clause, &rhs.where_clause) {
        (Some(dst), Some(src)) => {
            dst.predicates.extend(src.predicates.iter().cloned());
        }

        (None, Some(src)) => {
            out.where_clause = Some(src.clone());
        }

        _ => {}
    }

    Ok(out)
}

/// Validates that all implementation generic parameters are present in the
/// merged generic parameter list.
pub(crate) fn validate_merged(
    generics: &Generics,
    impl_generics: &Generics,
) -> Result<(), TokenStream> {
    for impl_param in &impl_generics.params {
        let impl_ident = match impl_param {
            GenericParam::Type(param) => &param.ident,
            GenericParam::Lifetime(param) => &param.lifetime.ident,
            GenericParam::Const(param) => &param.ident,
        };

        let found = generics.params.iter().any(|param| {
            let ident = match param {
                GenericParam::Type(param) => &param.ident,
                GenericParam::Lifetime(param) => &param.lifetime.ident,
                GenericParam::Const(param) => &param.ident,
            };

            ident == impl_ident
        });

        if !found {
            return Err(MiscBugs::MergedGenericParameterMissing {}.into());
        }
    }

    Ok(())
}
