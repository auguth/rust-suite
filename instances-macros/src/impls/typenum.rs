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
// ```````````````````````` INSTANCE IMPL TYPENUM COUNTER ````````````````````````
// ===============================================================================

//! Rewrites extracted instance-counter const arguments into a synthesized
//! typenum carrier type argument.
//!
//! This transformation removes all extracted instance-counter const-generic
//! arguments from an implemented trait path and replaces them with a single
//! synthesized type argument representing the entire counter state.
//!
//! The generated type-level representation is used by later expansion stages
//! to perform instance-counter computation through typenum-based machinery on
//! stable Rust.
//!
//! ## Example
//!
//! Given:
//!
//! ```ignore
//! #[instance(0, 2)]
//! impl Example<4, T, 7> for MyType {}
//! ```
//!
//! extraction identifies:
//!
//! ```text
//! 4, 7
//! ```
//!
//! and this transformation rewrites the implemented trait path into:
//!
//! ```ignore
//! impl Example<(instances::U4, instances::U7), T> for MyType {}
//! ```
//!
//! where the synthesized tuple type becomes the type-level carrier used by
//! later proc-macro expansion stages.
//!
//! This mirrors the trait-side typenum transformation:
//!
//! ```ignore
//! trait Example<__TypeNumCounters, T> {}
//! ```
//!
//! allowing trait definitions and implementations to communicate instance
//! counters entirely through stable type-level representations rather than
//! const-generic arithmetic.

// ===============================================================================
// ``````````````````````````````````` IMPORTS ```````````````````````````````````
// ===============================================================================

// --- Proc-Macro crates ---
use proc_macro2::TokenStream;
use quote::format_ident;
use syn::{
    GenericArgument, ItemImpl, PathArguments, Type, parse_quote,
    punctuated::Punctuated,
    token::{Comma, For},
};

// --- Local Crate ---
use crate::{
    Extraction, Instance, Transformation, Utilization,
    impls::{
        counters::{CounterArgs, CounterArgsSlice},
        errors::{TypeNumBugs, TypeNumError, UtilityBugs},
        utils::{ImplTraitGenArgs, ImplTraitPath},
    },
};

// --- Proc Suite ---
use proc_suite::{SupportCrate, misc::*};

// ===============================================================================
// ``````````````````````` TYPENUM COUNTERS TRANSFORMATION ```````````````````````
// ===============================================================================

/// Rewrites extracted instance-counter arguments of an [`ItemImpl`] into a
/// synthesized typenum carrier type argument.
///
/// During extraction, [`CounterArgs`] identifies which const-generic
/// arguments participate in instance-counter generation. Those arguments are
/// then removed from the implemented trait path and replaced with a single
/// type argument encoding all counter values as typenum unsigned integers.
///
/// ## Generic Placement
///
/// The synthesized typenum carrier is inserted at the argument position
/// previously occupied by the first extracted counter argument.
///
/// This preserves the relative ordering of all remaining generic
/// arguments.
///
/// Example:
///
/// ```ignore
/// #[instance(0, 2)]
/// impl Example<4, T, 7, U> for MyType {}
/// ```
///
/// Extracted counters:
///
/// ```text
/// 4
/// 7
/// ```
///
/// Since `4` is the first extracted counter argument and occupies generic
/// index `0`, the synthesized typenum carrier is inserted at that same
/// position:
///
/// ```ignore
/// impl Example<(instances::U4, instances::U7), T, U> for MyType {}
/// ```
///
/// Likewise:
///
/// ```ignore
/// #[instance(1, 3)]
/// impl Example<T, 4, U, 7> for MyType {}
/// ```
///
/// becomes:
///
/// ```ignore
/// impl Example<T, (instances::U4, instances::U7), U> for MyType {}
/// ```
///
/// This preserves:
///
/// - generic argument ordering,
/// - argument indices of non-counter generics,
/// - deterministic placement of typenum counters,
/// - and consistency with the trait-side typenum transformation.
///
/// ## Example
///
/// Given:
///
/// ```ignore
/// #[instance(0, 2)]
/// impl Example<4, T, 7> for MyType {}
/// ```
///
/// extraction identifies:
///
/// ```text
/// 4 -> counter value
/// 7 -> counter value
/// ```
///
/// The transformation removes those counter arguments and injects:
///
/// ```ignore
/// impl Example<(instances::U4, instances::U7), T> for MyType {}
/// ```
///
/// Later expansion stages use this synthesized type-level representation to:
///
/// - perform stable type-level counter arithmetic,
/// - avoid unstable const-generic expression requirements,
/// - propagate instance-counter state through generated items,
/// - construct compile-time instance metadata,
/// - and drive typenum-based instance resolution.
///
/// This transformation is the implementation-side counterpart of
/// [`InstanceTraitTypeNumCounters`](crate::traits::typenum::InstanceTraitTypeNumCounters).
/// Together they convert both trait-side counter declarations and
/// impl-side counter values into a common typenum representation suitable
/// for stable Rust expansion.
#[derive(Clone, Debug)]
pub(crate) struct InstanceImplTypeNumCounters;

impl<'a> Transformation<ItemImpl, CounterArgsSlice<'a>> for InstanceImplTypeNumCounters {
    fn raw_transform(
        &self,
        transform: &mut ItemImpl,
        context: &CounterArgsSlice<'a>,
    ) -> Result<(), TokenStream> {
        let mut trait_path = ImplTraitPath::checked_utilize(transform, &())?.0.clone();
        let Some(trait_segment) = trait_path.segments.last_mut() else {
            return Err(TypeNumBugs::TraitPathLastSegmentNotFound {}.into());
        };
        let PathArguments::AngleBracketed(angle) = &mut trait_segment.arguments else {
            return Err(TypeNumBugs::FunctionTraitsNotApplicable {}.into());
        };
        let generic_args = &mut angle.args;

        let old_generic_args = generic_args.clone();
        let mut rebuilt = Punctuated::<GenericArgument, Comma>::new();

        for (i, arg) in old_generic_args.iter().enumerate() {
            let mut removed = false;

            for c in context.iter() {
                if c.generic_index >= old_generic_args.len() {
                    return Err(TypeNumBugs::CounterArgExtractedTraitInvalidOnGenericsLen {}.into());
                }

                if c.generic_index == i {
                    removed = true;
                    break;
                }
            }

            if !removed {
                rebuilt.push(arg.clone());
            }
        }

        let Some(first_counter) = context.first() else {
            return Err(UtilityBugs::CounterArgsEmptyForFirstCounterAccess {}.into());
        };

        let ty = counters_typenum_arg(context)?;

        let idx = first_counter.generic_index;

        let available_len = rebuilt.len();
        if idx > available_len {
            return Err(TypeNumError::InvalidFirstCounter {
                lit: first_counter.const_lit.clone(),
                index: idx,
                len: available_len,
            }
            .into());
        }

        rebuilt.insert(idx, GenericArgument::Type(ty));

        *generic_args = rebuilt;
        transform.trait_ = Some((None, trait_path, For::default()));

        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &ItemImpl,
        context: Option<&CounterArgsSlice<'a>>,
    ) -> Result<(), TokenStream> {
        let extracted;
        let counters = match context {
            Some(c) => c,
            None => {
                extracted = CounterArgs::checked_extract(transform, &None)?;
                extracted.as_slice().as_ref().as_ref()
            }
        };
        let args = ImplTraitGenArgs::checked_utilize(transform, &())?.0;

        let ty = counters_typenum_arg(counters)?;
        let Some(first_counter) = counters.first() else {
            return Err(UtilityBugs::CounterArgsEmptyForFirstCounterAccess {}.into());
        };
        let Some(arg) = args.get(first_counter.generic_index) else {
            return Err(TypeNumBugs::PostTypeNumNoGenericsAvailable {}.into());
        };

        let GenericArgument::Type(t) = arg else {
            return Err(TypeNumBugs::PostTypeNumFirstGenericNotType {}.into());
        };

        if *t != ty {
            return Err(TypeNumBugs::PostTypeNumInvalidTypeArg {}.into());
        }

        Ok(())
    }
}

/// Builds the impl-side typenum counter tuple from extracted counter values.
///
/// Example:
/// `4, 7` -> `(instances::U4, instances::U7)`
pub(crate) fn counters_typenum_arg<'a>(
    counters: CounterArgsSlice<'a>,
) -> Result<Type, TokenStream> {
    let mut collect = Vec::new();
    for c in counters {
        let lit = parse_pos_usize(&c.const_lit)?;
        let crate_of = Instance::support_crate();
        let typenum = format_ident!("U{lit}");
        let ty: Type = parse_quote!(#crate_of::#typenum);
        collect.push(ty);
    }
    if collect.len() == 1 {
        let Some(only_counter) = collect.first() else {
            return Err(UtilityBugs::CounterArgsEmptyForFirstCounterAccess {}.into());
        };
        return Ok(parse_quote!(
            (#only_counter ,)));
    }
    Ok(parse_quote! {
        (#(#collect),*)
    })
}
