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
// `````````````````````````` INSTANCE AFFILIATES (IMPL) `````````````````````````
// ===============================================================================

//! This module generates hidden implementation-side affiliate items used
//! to realize affiliate relationships declared by trait-side expansion.
//! (See [`affiliates`](crate::traits::affiliates))
//!
//! Affiliate relationships are represented through typenum-backed counter
//! tuples and hidden associated types generated into each implementation.
//!
//! ## Why this exists
//!
//! Trait-side expansion defines affiliate contracts, but those contracts
//! alone do not provide concrete affiliated counter tuples.
//!
//! For example:
//!
//! ```ignore
//! trait K<Counters> {
//!     type NextAffiliateCounters;
//!     type NextAffiliateInstance:
//!         K<Self::NextAffiliateCounters>;
//! }
//! ```
//!
//! The trait declares that a next affiliate exists, but it does not define:
//!
//! - what the successor counters are,
//! - which implementation instance they refer to,
//! - how predecessor relationships are recovered,
//! - or how semantic affiliate topology should be resolved.
//!
//! This module supplies those implementation-side projections.
//!
//! ## What this module generates
//!
//! For every implementation instance, this module generates:
//!
//! - affiliate counter projections,
//! - affiliate instance projections,
//! - semantic navigation links,
//! - structural navigation links,
//! - and validation helpers.
//!
//! Together these items provide concrete implementations of the
//! affiliate contracts declared by trait-side expansion.
//!
//! ## Structural Affiliates
//!
//! Structural affiliates can be derived directly from the current
//! counter tuple.
//!
//! These projections are deterministic and require no additional
//! implementation knowledge.
//!
//! - [`ImplReverseAffiliateCounter`]
//!     Generates the reverse-affiliate counter tuple.
//!
//! - [`ImplCeilAffiliateCounter`]
//!     Generates the ceil-affiliate counter tuple.
//!
//! ## Semantic Affiliates
//!
//! Semantic affiliates describe relationships in the implementation
//! topology rather than direct counter transformations.
//!
//! These projections are constructed through implementation-side
//! navigation rules and may later be refined by additional expansion
//! stages.
//!
//! - [`ImplNextAffiliateCounter`]
//!     Generates an optimistic semantic successor projection.
//!
//! - [`ImplBackAffiliateCounter`]
//!     Reconstructs the semantic predecessor projection.
//!
//! - [`ImplFloorAffiliatesCounters`]
//!     Resolves canonical lineage representatives (per counter).
//!
//! ## Affiliate Instance Projections
//!
//! - [`ImplAffiliateInstances`]
//!     Supplies concrete definitions for every affiliate-instance
//!     associated type.
//!
//! All affiliate-instance projections resolve to the same implementing
//! Rust type (`Self`).
//!
//! Navigation occurs by changing the trait counter tuple rather than the
//! implementation type itself.
//!
//! ```ignore
//! type NextAffiliateInstance = Self;
//! ```
//!
//! together with:
//!
//! ```ignore
//! type NextAffiliateCounters = (U0, U0, U6);
//! ```
//!
//! allows:
//!
//! ```ignore
//! Self as MyTrait<Self::NextAffiliateCounters>
//! ```
//!
//! to refer to another trait instantiation while preserving the same
//! underlying Rust type.
//!
//! ## Affiliate Checker
//!
//! - [`ImplAffiliateCountersChecker`]
//!     Verifies affiliate consistency invariants.
//!
//! ```text
//! back(next(current)) == current
//! next(back(current)) == current
//! ```
//!
//! This helps ensure that semantic affiliates form a single canonical
//! implementation-instance chain.
//!
//! ## Structural vs Semantic Navigation
//!
//! Structural affiliates answer:
//!
//! ```text
//! What counter tuple can be derived mechanically?
//! ```
//!
//! Semantic affiliates answer:
//!
//! ```text
//! What neighboring implementation instance should this represent?
//! ```
//!
//! Together they form a compile-time navigation graph spanning all
//! reachable trait instances.
//!
//! ## Generated Items
//!
//! All generated items are:
//!
//! - hidden,
//! - deterministic,
//! - proc-macro-readable,
//! - and compile-time verifiable.
//!
//! These items are consumed by later expansion phases to reconstruct
//! affiliate topology, validate transitions, and enable recursive access
//! to associated items across related trait instances.
//!
//! ## What this enables
//!
//! Implementation-side affiliate expansion makes it possible to:
//!
//! - navigate to successor instances,
//! - navigate to predecessor instances,
//! - jump to structural projections,
//! - recover semantic representatives,
//! - build compile-time instance graphs,
//! - and recursively access associated items across affiliated trait
//!   instantiations.
//!
//! All of this works entirely at compile time without runtime state or
//! unstable language features.

// ===============================================================================
// ``````````````````````````````````` IMPORTS ```````````````````````````````````
// ===============================================================================

// --- Local-crate ---
use crate::{
    Extension, Extraction, Insertion, Instance, Transformation, Utilization,
    impls::{
        counters::*,
        errors::{AffiliateBugs, AffiliateErrors},
        utils::*,
    },
    traits::affiliates::*,
};

// --- Proc-macro Utilties ---
use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident};
use syn::{Expr, ImplItem, ImplItemConst, ImplItemType, ItemImpl, Type, parse_quote};

// --- Proc Suite ---
use proc_suite::{SupportCrate, misc::*};

// ===============================================================================
// ``````````````````````````` INSTANCE IMPL AFFILIATES ``````````````````````````
// ===============================================================================

/// Generates all implementation-side affiliate projections.
///
/// This transformation acts as the implementation-side counterpart of
/// trait-side affiliate expansion.
///
/// It materializes the concrete affiliate items required to satisfy the
/// affiliate contracts declared by the trait.
///
/// ## Generated Components
///
/// - [`ImplAffiliateInstances`]
///     Defines all affiliate-instance associated types.
///
/// - [`ImplReverseAffiliateCounter`]
///     Generates reverse-affiliate counters.
///
/// - [`ImplCeilAffiliateCounter`]
///     Generates ceil-affiliate counters.
///
/// - [`ImplBackAffiliateCounter`]
///     Resolves semantic predecessor counters.
///
/// - [`ImplNextAffiliateCounter`]
///     Generates optimistic semantic successor counters.
///
/// - [`ImplFloorAffiliatesCounters`]
///     Resolves semantic floor-representative counters (multiple).
///
/// - [`ImplAffiliateCountersChecker`]
///     Resolves semantic floor-representative counters (multiple).
///
/// ## Navigation Model
///
/// Together these generated items provide:
///
/// ```text
/// Reverse
/// Ceil <-> Floor
/// Back <-> Next
/// ```
///
/// navigation across affiliated trait instances while preserving the
/// same implementation type.
///
/// Structural affiliates are derived directly from the current counter
/// tuple, while semantic affiliates are resolved through
/// implementation-side affiliate topology.
///
/// This transformation therefore provides the concrete implementation
/// of all affiliate relationships declared by trait-side expansion.
#[derive(Debug, Clone)]
pub(crate) struct InstanceImplAffiliates;

impl<'a> Transformation<ItemImpl, CounterArgsSlice<'a>> for InstanceImplAffiliates {
    fn raw_transform(
        &self,
        transform: &mut ItemImpl,
        context: &CounterArgsSlice,
    ) -> Result<(), TokenStream> {
        ImplReverseAffiliateCounter::checked_extend(
            &ImplReverseAffiliateCounter,
            transform,
            context,
        )?;
        ImplCeilAffiliateCounter::checked_extend(&ImplCeilAffiliateCounter, transform, context)?;
        ImplBackAffiliateCounter::checked_extend(&ImplBackAffiliateCounter, transform, context)?;
        ImplNextAffiliateCounter::checked_extend(&ImplNextAffiliateCounter, transform, context)?;
        ImplFloorAffiliatesCounters::checked_extend(
            &ImplFloorAffiliatesCounters,
            transform,
            context,
        )?;
        ImplAffiliateInstances::checked_extend(&ImplAffiliateInstances, transform, context)?;
        ImplAffiliateCountersChecker::checked_extend(
            &ImplAffiliateCountersChecker,
            transform,
            context,
        )?;
        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &ItemImpl,
        context: Option<&CounterArgsSlice<'a>>,
    ) -> Result<(), TokenStream> {
        ImplReverseAffiliateCounter::validate_extend(
            &ImplReverseAffiliateCounter,
            None,
            transform,
            context,
        )?;
        ImplCeilAffiliateCounter::validate_extend(
            &ImplCeilAffiliateCounter,
            None,
            transform,
            context,
        )?;
        ImplBackAffiliateCounter::validate_extend(
            &ImplBackAffiliateCounter,
            None,
            transform,
            context,
        )?;
        ImplNextAffiliateCounter::validate_extend(
            &ImplNextAffiliateCounter,
            None,
            transform,
            context,
        )?;
        ImplFloorAffiliatesCounters::validate_extend(
            &ImplFloorAffiliatesCounters,
            None,
            transform,
            context,
        )?;
        ImplAffiliateInstances::validate_extend(&ImplAffiliateInstances, None, transform, context)?;
        ImplAffiliateCountersChecker::validate_extend(
            &ImplAffiliateCountersChecker,
            None,
            transform,
            context,
        )?;

        Ok(())
    }
}

// ===============================================================================
// ``````````````````````````` AFFILIATE INSTANCES ALL ```````````````````````````
// ===============================================================================

/// Extends an instance implementation with definitions for all
/// affiliate-instance associated types.
///
/// This phase acts as the implementation-side counterpart of the
/// affiliate-instance contracts generated by trait-side affiliate
/// expansion.
///
/// Trait-side expansion introduces a collection of associated types
/// representing neighboring affiliate instances. Each affiliate
/// instance is constrained to implement the same trait using its
/// corresponding affiliate-counter associated type.
///
/// ## Example
///
/// Trait-side expansion:
///
/// ```ignore
/// trait Example<__TypeNumCounters> {
///     type __ReverseAffiliateCounter;
///     type __CeilAffiliateCounter;
///     type __NextAffiliateCounter;
///     type __BackAffiliateCounter;
///     type __FloorAffiliatesCounters;
///
///     type __ReverseAffiliateInstance:
///         Example<Self::__ReverseAffiliateCounter>;
///
///     type __CeilAffiliateInstance:
///         Example<Self::__CeilAffiliateCounter>;
///
///     type __NextAffiliateInstance:
///         Example<Self::__NextAffiliateCounter>;
///
///     type __BackAffiliateInstance:
///         Example<Self::__BackAffiliateCounter>;
///
///     type __FloorAffiliatesInstances_0:
///         Example<Self::__FloorAffiliatesCounters_0>;
///
///     type __FloorAffiliatesInstances_1:
///         Example<Self::__FloorAffiliatesCounters_1>;
/// }
/// ```
///
/// This phase satisfies those contracts by assigning the current
/// implementation type to every affiliate-instance associated type:
///
/// ```ignore
/// impl Example<(U1, U2)> for MyType {
///     type __ReverseAffiliateInstance = MyType;
///     type __CeilAffiliateInstance = MyType;
///     type __NextAffiliateInstance = MyType;
///     type __BackAffiliateInstance = MyType;
///     type __FloorAffiliatesInstances_0 = MyType;
///     type __FloorAffiliatesInstances_1 = MyType;
/// }
/// ```
///
/// Although every affiliate instance resolves to the same
/// implementation type (`Self`), each associated type is coupled to a
/// different affiliate-counter associated type through the trait-side
/// bounds
///
/// Consequently, affiliate navigation does not occur by changing the
/// implementation type. Instead, navigation occurs by changing the
/// counter tuple used to instantiate the trait while reusing the same
/// implementation type.
///
/// This allows affiliate relationships to be expressed entirely through
/// trait generics while preserving a single implementing Rust type for
/// all reachable affiliate states.
#[derive(Debug, Clone)]
pub(super) struct ImplAffiliateInstances;

impl Insertion<ItemImpl> for Vec<ImplItemType> {
    fn raw_insert(&self, to: &mut ItemImpl, _: &()) -> Result<(), TokenStream> {
        for c in self {
            to.items.push(ImplItem::Type(c.clone()));
        }
        Ok(())
    }

    fn validate_inserted(
        _of: Option<&Self>,
        _to: &ItemImpl,
        _: Option<&()>,
    ) -> Result<(), TokenStream> {
        Ok(())
    }
}

impl<'a> Extension<Vec<ImplItemType>, ItemImpl, CounterArgsSlice<'a>> for ImplAffiliateInstances {
    fn raw_extend(
        &self,
        towards: &ItemImpl,
        context: &CounterArgsSlice<'a>,
    ) -> Result<Vec<ImplItemType>, TokenStream> {
        let idents = AllAffiliateInstanceIdents::checked_extract(&context, &())?.0;
        let mut collect = Vec::new();
        let self_ty = &towards.self_ty;
        let ty: Type = parse_quote!(#self_ty);
        for ident in idents {
            let item = ImplItemType {
                attrs: proc_suite::internal_code(),
                ident,
                ty: ty.clone(),
                vis: syn::Visibility::Inherited,
                defaultness: None,
                type_token: Default::default(),
                generics: Default::default(),
                eq_token: Default::default(),
                semi_token: Default::default(),
            };
            collect.push(item);
        }
        Ok(collect)
    }

    fn validate_extend(
        &self,
        item: Option<&Vec<ImplItemType>>,
        towards: &ItemImpl,
        context: Option<&CounterArgsSlice<'a>>,
    ) -> Result<(), TokenStream> {
        let extracted;
        let counters = match context {
            Some(c) => c,
            None => {
                extracted = CounterArgs::checked_extract(towards, &None)?;
                extracted.as_slice().as_ref().as_ref()
            }
        };
        let idents = AllAffiliateInstanceIdents::checked_extract(&counters, &())?.0;
        let self_ty = &towards.self_ty;
        let ty: Type = parse_quote!(#self_ty);
        for ident in idents {
            validate_impl_types! {
                items: item,
                towards: towards,
                ident: ident,
                ty: ty,
                errors: {
                    not_found: AffiliateBugs::ImplAffiliateInstanceNotFound {},
                    wrong_ident: AffiliateBugs::ImplAffiliateInstanceWrongIdent {},
                    wrong_ty: AffiliateBugs::ImplAffiliateInstanceInvalidType {},
                    has_generics: AffiliateBugs::ImplAffiliateInstanceHasGenerics {},
                }
            };
        }
        Ok(())
    }
}

// ===============================================================================
// `````````````````````````` REVERSE AFFILIATE COUNTER ``````````````````````````
// ===============================================================================

/// Extends an instance implementation with the concrete reverse-affiliate
/// counter projection.
///
/// This phase acts as the implementation-side counterpart of
/// [`ReverseAffiliateCounters`].
///
/// Trait-side affiliate expansion declares the reverse-affiliate counter
/// contract:
///
/// ```ignore
/// trait Example<Counters> {
///     type ReverseAffiliateCounters;
///
///     type ReverseAffiliateInstance:
///         Example<Self::ReverseAffiliateCounters>;
/// }
/// ```
///
/// This phase fulfills that contract by computing the concrete reverse
/// counter tuple for the current implementation.
///
/// ## Reverse Semantics
///
/// Reverse locates the rightmost non-zero counter and decrements it.
///
/// Examples:
///
/// ```text
/// (5)       -> (4)
/// (3, 2)    -> (3, 1)
/// (3, 0)    -> (2, 0)
/// (0, 0, 0) -> (0, 0, 0)
/// ```
///
/// Reverse is intentionally saturating and never performs positional
/// borrowing.
///
/// ## Example
///
/// Given:
///
/// ```ignore
/// impl Example<(U3, U2, U5)> for MyType {}
/// ```
///
/// this phase generates:
///
/// ```ignore
/// impl Example<(U3, U2, U5)> for MyType {
///     type ReverseAffiliateCounters = (U3, U2, U4);
/// }
/// ```
///
/// Likewise:
///
/// ```ignore
/// impl Example<(U3, U2, U0)> for MyType {
///     type ReverseAffiliateCounters = (U3, U1, U0);
/// }
/// ```
///
/// The generated counter tuple satisfies the trait-side affiliate
/// contract and allows:
///
/// ```ignore
/// type ReverseAffiliateInstance:
///     Example<Self::ReverseAffiliateCounters>;
/// ```
///
/// to resolve to a reachable reverse-affiliated instance.
///
/// Together with [`ImplAffiliateInstances`], this provides the concrete
/// implementation-side projection required for reverse-affiliate
/// navigation.
///
/// Since reverse affiliates are structurally derivable from the current
/// counters, every implementation can generate its reverse-affiliate
/// counters deterministically without requiring additional user input.
#[derive(Debug, Clone)]
pub(super) struct ImplReverseAffiliateCounter;

impl<'a> Insertion<ItemImpl, CounterArgsSlice<'a>> for ImplItemType {
    fn raw_insert(&self, to: &mut ItemImpl, _: &CounterArgsSlice) -> Result<(), TokenStream> {
        to.items.push(ImplItem::Type(self.clone()));
        Ok(())
    }

    fn validate_inserted(
        _of: Option<&Self>,
        _to: &ItemImpl,
        _context: Option<&CounterArgsSlice<'a>>,
    ) -> Result<(), TokenStream> {
        Ok(())
    }
}

impl<'a> Extension<ImplItemType, ItemImpl, CounterArgsSlice<'a>> for ImplReverseAffiliateCounter {
    fn raw_extend(
        &self,
        _: &ItemImpl,
        context: &CounterArgsSlice<'a>,
    ) -> Result<ImplItemType, TokenStream> {
        let ident = gen_type_ident::<ReverseAffiliateCounters>();
        let ty = reverse_counters_typenum_arg(context)?;
        let item = ImplItemType {
            attrs: proc_suite::internal_code(),
            ident,
            ty,
            vis: syn::Visibility::Inherited,
            defaultness: None,
            type_token: Default::default(),
            generics: Default::default(),
            eq_token: Default::default(),
            semi_token: Default::default(),
        };
        Ok(item)
    }

    fn validate_extend(
        &self,
        item: Option<&ImplItemType>,
        towards: &ItemImpl,
        context: Option<&CounterArgsSlice<'a>>,
    ) -> Result<(), TokenStream> {
        let extracted;
        let counters = match context {
            Some(c) => c,
            None => {
                extracted = CounterArgs::checked_extract(towards, &None)?;
                extracted.as_slice().as_ref().as_ref()
            }
        };

        let ident = gen_type_ident::<ReverseAffiliateCounters>();
        let ty = reverse_counters_typenum_arg(counters)?;
        validate_impl_type! {
            item: item,
            towards: towards,
            ident: ident,
            ty: ty,
            errors: {
                not_found: AffiliateBugs::ImplReverseAffiliateCounterNotFound {},
                wrong_ident: AffiliateBugs::ImplReverseAffiliateCounterWrongIdent {},
                wrong_ty: AffiliateBugs::ImplReverseAffiliateCounterInvalidType {},
                has_generics: AffiliateBugs::ImplReverseAffiliateCounterHasGenerics {},
            }
        }
    }
}

/// Builds the typenum counter tuple of the reverse affiliate.
///
/// Examples:
///
/// ```text
/// (U3, U2, U5) -> (U3, U2, U4)
/// (U3, U2, U0) -> (U3, U1, U0)
/// (U1, U0, U0) -> (U0, U0, U0)
/// (U0, U0, U0) -> (U0, U0, U0)
/// ```
fn reverse_counters_typenum_arg<'a>(counters: CounterArgsSlice<'a>) -> Result<Type, TokenStream> {
    let mut collect = Vec::<Type>::new();
    let mut decremented = false;
    let crate_of = Instance::support_crate();

    for c in counters.iter().rev() {
        let mut lit = parse_pos_usize(&c.const_lit)?;

        if !decremented && lit > 0 {
            lit -= 1;
            decremented = true;
        }

        let typenum = format_ident!("U{lit}");
        collect.push(parse_quote!(#crate_of::#typenum));
    }

    if collect.is_empty() {
        return Err(AffiliateBugs::ExtractedReverseAffiliateCountersAreEmpty {}.into());
    }

    collect.reverse();

    Ok(parse_quote!((#(#collect,)*)))
}

// ===============================================================================
// ``````````````````````````` CEIL AFFILIATE COUNTER ````````````````````````````
// ===============================================================================

/// Extends an instance implementation with the concrete ceil-affiliate
/// counter projection.
///
/// This phase acts as the implementation-side counterpart of
/// [`CeilAffiliateCounters`].
///
/// Trait-side affiliate expansion declares the ceil-affiliate counter
/// contract:
///
/// ```ignore
/// trait Example<Counters> {
///     type CeilAffiliateCounters;
///
///     type CeilAffiliateInstance:
///         Example<Self::CeilAffiliateCounters>;
/// }
/// ```
///
/// This phase fulfills that contract by computing the concrete
/// ceil-affiliated counter tuple for the current implementation.
///
/// ## Ceil Semantics
///
/// Ceil locates the rightmost non-zero counter and resets it to zero.
///
/// Examples:
///
/// ```text
/// (0,0,1) -> (0,0,0)
/// (0,1,0) -> (0,0,0)
/// (1,0,0) -> (0,0,0)
/// (2,3,4) -> (2,3,0)
/// (5,7,0) -> (5,0,0)
/// (0,0,0) -> (0,0,0)
/// ```
///
/// This operation is intentionally structural rather than arithmetic.
///
/// ## Example
///
/// Given:
///
/// ```ignore
/// impl Example<(U5, U7, U0)> for MyType {}
/// ```
///
/// this phase generates:
///
/// ```ignore
/// impl Example<(U5, U7, U0)> for MyType {
///     type CeilAffiliateCounters = (U5, U0, U0);
/// }
/// ```
///
/// Likewise:
///
/// ```ignore
/// impl Example<(U2, U3, U4)> for MyType {
///     type CeilAffiliateCounters = (U2, U3, U0);
/// }
/// ```
///
/// The generated counter tuple satisfies the trait-side affiliate
/// contract and allows:
///
/// ```ignore
/// type CeilAffiliateInstance:
///     Example<Self::CeilAffiliateCounters>;
/// ```
///
/// to resolve to a structurally reachable ceil-affiliated instance.
///
/// Together with [`ImplAffiliateInstances`], this provides the concrete
/// implementation-side projection required for ceil-affiliate
/// navigation.
///
/// Since ceil affiliates are structurally derivable from the current
/// counters, every implementation can generate its ceil-affiliate
/// counters deterministically without requiring additional user input.
///
/// The resulting counter tuple therefore acts as the concrete
/// implementation-side witness of the trait-side
/// [`CeilAffiliateCounters`] contract.
#[derive(Debug, Clone)]
pub(super) struct ImplCeilAffiliateCounter;

impl<'a> Extension<ImplItemType, ItemImpl, CounterArgsSlice<'a>> for ImplCeilAffiliateCounter {
    fn raw_extend(
        &self,
        _: &ItemImpl,
        context: &CounterArgsSlice<'a>,
    ) -> Result<ImplItemType, TokenStream> {
        let ident = gen_type_ident::<CeilAffiliateCounters>();
        let ty = ceil_counters_typenum_arg(context)?;
        let item = ImplItemType {
            attrs: proc_suite::internal_code(),
            ident,
            ty,
            vis: syn::Visibility::Inherited,
            defaultness: None,
            type_token: Default::default(),
            generics: Default::default(),
            eq_token: Default::default(),
            semi_token: Default::default(),
        };
        Ok(item)
    }

    fn validate_extend(
        &self,
        item: Option<&ImplItemType>,
        towards: &ItemImpl,
        context: Option<&CounterArgsSlice<'a>>,
    ) -> Result<(), TokenStream> {
        let extracted;
        let counters = match context {
            Some(c) => c,
            None => {
                extracted = CounterArgs::checked_extract(towards, &None)?;
                extracted.as_slice().as_ref().as_ref()
            }
        };

        let ident = gen_type_ident::<CeilAffiliateCounters>();
        let ty = ceil_counters_typenum_arg(counters)?;
        validate_impl_type! {
            item: item,
            towards: towards,
            ident: ident,
            ty: ty,
            errors: {
                not_found: AffiliateBugs::ImplCeilAffiliateCounterNotFound {},
                wrong_ident: AffiliateBugs::ImplCeilAffiliateCounterWrongIdent {},
                wrong_ty: AffiliateBugs::ImplCeilAffiliateCounterInvalidType {},
                has_generics: AffiliateBugs::ImplCeilAffiliateCounterHasGenerics {},
            }
        }
    }
}

/// Builds the typenum counter tuple of the ceil affiliate.
///
/// Examples:
///
/// ```text
/// (U0, U0, U1) -> (U0, U0, U0)
/// (U0, U1, U0) -> (U0, U0, U0)
/// (U1, U0, U0) -> (U0, U0, U0)
/// (U2, U3, U4) -> (U2, U3, U0)
/// (U5, U7, U0) -> (U5, U0, U0)
/// (U0, U0, U0) -> (U0, U0, U0)
/// ```
fn ceil_counters_typenum_arg<'a>(counters: CounterArgsSlice<'a>) -> Result<Type, TokenStream> {
    let mut collect = Vec::<Type>::new();
    let mut zeroed = false;
    let crate_of = Instance::support_crate();

    for c in counters.iter().rev() {
        let mut lit = parse_pos_usize(&c.const_lit)?;

        if !zeroed && lit > 0 {
            lit = 0;
            zeroed = true;
        }

        let typenum = format_ident!("U{lit}");
        collect.push(parse_quote!(#crate_of::#typenum));
    }

    if collect.is_empty() {
        return Err(AffiliateBugs::ExtractedCeilAffiliateCountersAreEmpty {}.into());
    }

    collect.reverse();

    Ok(parse_quote!((#(#collect,)*)))
}

// ===============================================================================
// ``````````````````````````` NEXT AFFILIATE COUNTER ````````````````````````````
// ===============================================================================

/// Extends an instance implementation with the semantic successor
/// counter projection.
///
/// This phase acts as the implementation-side counterpart of
/// [`NextAffiliateCounters`].
///
/// Unlike [`ReverseAffiliateCounters`] and
/// [`CeilAffiliateCounters`], next affiliates are not derived through a
/// bounded structural transformation.
///
/// Instead, successor resolution is delegated to later expansion
/// stages.
///
/// ## Example
///
/// Trait-side expansion:
///
/// ```ignore
/// trait Example<Counters> {
///     type NextAffiliateCounters;
///
///     type NextAffiliateInstance:
///         Example<Self::NextAffiliateCounters>;
/// }
/// ```
///
/// This phase fulfills that contract by generating an optimistic
/// successor counter tuple.
///
/// ## Successor Semantics
///
/// The rightmost counter is incremented while all remaining counters
/// are preserved.
///
/// Examples:
///
/// ```text
/// (0,0,0) -> (0,0,1)
/// (0,0,5) -> (0,0,6)
/// (0,1,9) -> (0,1,10)
/// ```
///
/// Unlike a positional-number increment, this phase intentionally does
/// not perform carry propagation or dimensional normalization.
///
/// Instead, it assumes that a successor instance exists and allows
/// later proc-macro phases to decide how that successor should be
/// resolved.
///
/// ## Deferred Resolution
///
/// The generated successor may later be rewritten into:
///
/// ```text
/// Current
/// ```
///
/// indicating that the current instance is terminal.
///
/// Alternatively, a later phase may redirect the successor into a
/// higher-dimensional transition:
///
/// ```text
/// (0,0,9) -> (0,1,0)
/// ```
///
/// or any other implementation-defined successor topology.
///
/// This phase therefore does not determine whether a successor actually
/// exists.
///
/// It merely provides an optimistic successor candidate from which
/// later expansion stages can construct the final successor graph.
///
/// Together with [`ImplAffiliateInstances`], this provides the concrete
/// implementation-side value required to satisfy the trait-side
/// [`NextAffiliateCounters`] contract.
///
/// This phase therefore acts as the semantic-navigation counterpart to
/// [`ImplBackAffiliateCounter`], providing the forward edge from which
/// predecessor and floor relationships can later be reconstructed.
#[derive(Debug, Clone)]
pub(super) struct ImplNextAffiliateCounter;

impl<'a> Extension<ImplItemType, ItemImpl, CounterArgsSlice<'a>> for ImplNextAffiliateCounter {
    fn raw_extend(
        &self,
        _: &ItemImpl,
        context: &CounterArgsSlice<'a>,
    ) -> Result<ImplItemType, TokenStream> {
        let ident = gen_type_ident::<NextAffiliateCounters>();
        let ty = next_counters_typenum_arg(context)?;
        let item = ImplItemType {
            attrs: proc_suite::internal_code(),
            ident,
            ty,
            vis: syn::Visibility::Inherited,
            defaultness: None,
            type_token: Default::default(),
            generics: Default::default(),
            eq_token: Default::default(),
            semi_token: Default::default(),
        };
        Ok(item)
    }

    fn validate_extend(
        &self,
        item: Option<&ImplItemType>,
        towards: &ItemImpl,
        context: Option<&CounterArgsSlice<'a>>,
    ) -> Result<(), TokenStream> {
        let extracted;
        let counters = match context {
            Some(c) => c,
            None => {
                extracted = CounterArgs::checked_extract(towards, &None)?;
                extracted.as_slice().as_ref().as_ref()
            }
        };

        let ident = gen_type_ident::<NextAffiliateCounters>();
        let ty = next_counters_typenum_arg(counters)?;
        validate_impl_type! {
            item: item,
            towards: towards,
            ident: ident,
            ty: ty,
            errors: {
                not_found: AffiliateBugs::ImplNextAffiliateCounterNotFound {},
                wrong_ident: AffiliateBugs::ImplNextAffiliateCounterWrongIdent {},
                wrong_ty: AffiliateBugs::ImplNextAffiliateCounterInvalidType {},
                has_generics: AffiliateBugs::ImplNextAffiliateCounterHasGenerics {},
            }
        }
    }
}

/// Builds the optimistic successor typenum counter tuple.
///
/// Examples:
///
/// ```text
/// (U0,U0,U0) -> (U0,U0,U1)
/// (U0,U0,U9) -> (U0,U0,U10)
/// (U1,U5,U7) -> (U1,U5,U8)
/// ```
///
/// The rightmost counter is incremented without performing carry
/// propagation or dimensional normalization.
fn next_counters_typenum_arg<'a>(counters: CounterArgsSlice<'a>) -> Result<Type, TokenStream> {
    let mut collect = Vec::<Type>::new();
    let mut increased = false;
    let crate_of = Instance::support_crate();

    for c in counters.iter().rev() {
        let mut lit = parse_pos_usize(&c.const_lit)?;
        if !increased {
            lit = lit + 1;
            increased = true;
        };
        let typenum = format_ident!("U{lit}");
        collect.push(parse_quote!(#crate_of::#typenum));
    }
    if collect.is_empty() {
        return Err(AffiliateBugs::ExtractedCeilAffiliateCountersAreEmpty {}.into());
    }

    collect.reverse();

    Ok(parse_quote!((#(#collect,)*)))
}

// ===============================================================================
// ``````````````````````````` BACK AFFILIATE COUNTER ````````````````````````````
// ===============================================================================

/// Extends an instance implementation with the semantic predecessor
/// counter projection.
///
/// This phase acts as the implementation-side counterpart of
/// [`BackAffiliateCounters`].
///
/// Unlike [`ReverseAffiliateCounters`], which is derived directly from
/// the current counter tuple, back affiliates represent the canonical
/// predecessor relationship within the affiliate topology.
///
/// ## Back Semantics
///
/// Reverse answers:
///
/// ```text
/// What structurally reachable reverse instance exists?
/// ```
///
/// Back instead answers:
///
/// ```text
/// What instance logically precedes the current instance?
/// ```
///
/// These are often identical:
///
/// ```text
/// (0,0,5)
///     -> back = (0,0,4)
/// ```
///
/// but may diverge at dimensional rollover boundaries:
///
/// ```text
/// (0,0,3)
/// (0,1,0)
/// ```
///
/// where the predecessor of:
///
/// ```text
/// (0,1,0)
/// ```
///
/// is:
///
/// ```text
/// (0,0,3)
/// ```
///
/// rather than its structural reverse.
///
/// ## Resolution Logic
///
/// ```text
/// if current == global_min
///     current
///
/// else if next(reverse) == current
///     reverse
///
/// else
///     reverse.floor_of_rollover_dimension
/// ```
///
/// The rollover dimension is the counter dimension whose lineage most
/// recently terminated before the current dimension became active.
///
/// This allows predecessor recovery across dimensional boundaries while
/// preserving a canonical semantic ordering.
///
/// ## Relationship to Next
///
/// Back acts as the semantic counterpart of
/// [`ImplNextAffiliateCounter`].
///
/// Valid affiliate topologies are expected to preserve:
///
/// ```text
/// back(next(current)) = current
/// ```
///
/// and:
///
/// ```text
/// next(back(current)) = current
/// ```
///
/// whenever both neighboring semantic instances exist.
///
/// ## Example
///
/// ```text
/// (0,0,3)
///     -> next = (0,1,0)
///
/// therefore:
///
/// back((0,1,0)) = (0,0,3)
/// ```
///
/// Together with [`ImplAffiliateInstances`], this provides the concrete
/// implementation-side value required to satisfy the trait-side
/// [`BackAffiliateCounters`] contract.
#[derive(Debug, Clone)]
pub(super) struct ImplBackAffiliateCounter;

impl<'a> Extension<ImplItemType, ItemImpl, CounterArgsSlice<'a>> for ImplBackAffiliateCounter {
    fn raw_extend(
        &self,
        towards: &ItemImpl,
        context: &CounterArgsSlice<'a>,
    ) -> Result<ImplItemType, TokenStream> {
        let ident = gen_type_ident::<BackAffiliateCounters>();
        let ty = back_counters_typenum_arg(towards, context, None)?;
        let item = ImplItemType {
            attrs: proc_suite::internal_code(),
            ident,
            ty,
            vis: syn::Visibility::Inherited,
            defaultness: None,
            type_token: Default::default(),
            generics: Default::default(),
            eq_token: Default::default(),
            semi_token: Default::default(),
        };
        Ok(item)
    }

    fn validate_extend(
        &self,
        item: Option<&ImplItemType>,
        towards: &ItemImpl,
        context: Option<&CounterArgsSlice<'a>>,
    ) -> Result<(), TokenStream> {
        let extracted;
        let counters = match context {
            Some(c) => c,
            None => {
                extracted = CounterArgs::checked_extract(towards, &None)?;
                extracted.as_slice().as_ref().as_ref()
            }
        };

        let ident = gen_type_ident::<BackAffiliateCounters>();
        let ty = back_counters_typenum_arg(towards, counters, None)?;
        validate_impl_type! {
            item: item,
            towards: towards,
            ident: ident,
            ty: ty,
            errors: {
                not_found: AffiliateBugs::ImplBackAffiliateCounterNotFound {},
                wrong_ident: AffiliateBugs::ImplBackAffiliateCounterWrongIdent {},
                wrong_ty: AffiliateBugs::ImplBackAffiliateCounterInvalidType {},
                has_generics: AffiliateBugs::ImplBackAffiliateCounterHasGenerics {},
            }
        }
    }
}

/// Builds the semantic predecessor back affiliate typenum counter projection.
///
/// The resulting counter tuple is resolved through affiliate topology
/// rather than direct counter transformation.
///
/// `floor_counter` identifies the counter dimension whose floor
/// affiliate should be used when predecessor recovery crosses a
/// rollover boundary.
///
/// If no counter is supplied, the final counter dimension is used as a
/// fallback. This is sufficient for normal predecessor recovery because
/// direct adjacency:
///
/// ```text
/// next(reverse) == current
/// ```
///
/// is checked first and therefore takes precedence whenever the reverse
/// instance is the true predecessor.
///
/// ```text
/// if current == global_min
///     current
///
/// else if next(reverse) == current
///     reverse
///
/// else
///     floor_of(floor_counter_or_last_counter, reverse)
/// ```
pub(crate) fn back_counters_typenum_arg<'a>(
    impl_of: &ItemImpl,
    counters: CounterArgsSlice<'a>,
    floor_counter: Option<&CounterArg>,
) -> Result<Type, TokenStream> {
    let current_counters = ImplCountersTypeNum::checked_utilize(impl_of, &counters)?.0;

    if is_zeroth_instance(counters)? {
        return Ok(parse_quote!(#current_counters));
    }

    let crate_of = Instance::support_crate();

    let reverse = AssocTyExpr::checked_extract(
        &(impl_of, &gen_type_ident::<ReverseAffiliateCounters>()),
        &(),
    )?
    .0;

    let next_counter_ident = gen_type_ident::<NextAffiliateCounters>();
    let reverse_next = AssocOfAffiliateTyExpr::<ReverseAffiliateInstance, ReverseAffiliateCounters>::checked_extract(&(&impl_of, &next_counter_ident), &counters)?.0;

    let floor_counter = match floor_counter {
        Some(c) => c,
        None => {
            let Some(fallback) = counters.last() else {
                return Err(AffiliateBugs::CountersAreEmptyToFindLastCounter {}.into());
            };
            fallback
        }
    };

    let floor_counter_ident = gen_type_ident_with_suffix::<FloorAffiliatesCounters>(Some(
        floor_counter.generic_index.to_string().as_bytes(),
    ));
    let reverse_floor = AssocOfAffiliateTyExpr::<ReverseAffiliateInstance, ReverseAffiliateCounters>::checked_extract(&(&impl_of, &floor_counter_ident), &counters)?.0;

    let is_global_min = is_first_counter_typenum_bool(counters)?;

    let ty: Type = parse_quote!(
        <
            #current_counters as #crate_of::BackAffiliate<
                #reverse,
                #reverse_next,
                #reverse_floor,
                #is_global_min
            >
        >::Output
    );
    Ok(ty)
}

/// Builds a typenum bit/boolean (`B1` or `B0`) indicating whether the current counters
/// represent the global minimum instance.
fn is_first_counter_typenum_bool<'a>(counters: CounterArgsSlice<'a>) -> Result<Type, TokenStream> {
    let zeroth = is_zeroth_instance(counters)?;

    let crate_of = Instance::support_crate();
    if zeroth {
        return Ok(parse_quote!(#crate_of::B1));
    }
    Ok(parse_quote!(#crate_of::B0))
}

// ===============================================================================
// ``````````````````````````` FLOOR AFFILIATE COUNTER ````````````````````````````
// ===============================================================================

/// Extends an implementation with all per-dimension floor-affiliate
/// counter projections.
///
/// This is the implementation-side counterpart of
/// [`FloorAffiliatesCounters`].
///
/// Unlike structural affiliates, floor affiliates are not computed from
/// the current counter tuple alone. Each floor counter represents the
/// canonical terminal representative of a specific counter dimension,
/// and therefore depends on implementation topology rather than pure
/// counter arithmetic.
///
/// For every extracted counter dimension:
///
/// ```text
/// Counter 0 -> FloorAffiliateCounter0
/// Counter 1 -> FloorAffiliateCounter1
/// Counter 2 -> FloorAffiliateCounter2
/// ...
/// ```
///
/// this phase generates a corresponding floor projection.
///
/// ## Recursive Delegation
///
/// The generated projections are intentionally optimistic.
///
/// ```text
/// Floor_k(Current) = Floor_k(Next(Current))
/// ```
///
/// for every counter dimension `k`.
///
/// Thus initial expansion does not attempt to determine where a floor
/// lineage terminates. Instead, it delegates resolution to the next
/// affiliate instance, producing a recursive chain:
///
/// ```text
/// Current
///    -> Next
///         -> Next
///              -> ...
/// ```
///
/// ## Boundary Resolution
///
/// A later implementation-side proc-macro phase is responsible for
/// identifying dimension boundaries and replacing the generated
/// recursive projection with a concrete floor value.
///
/// ```ignore
/// type FloorAffiliateCounterK = CurrentCounters;
/// ```
///
/// indicates that the current instance is the floor representative for
/// dimension `K`.
///
/// Once such a boundary is introduced, all preceding recursive
/// delegations automatically converge to that representative through
/// normal associated-type resolution.
///
/// ## Example
///
/// Consider:
///
/// ```text
/// (0,0)
/// (0,1)
/// (0,2)
/// (1,0)
/// (2,0)
/// ```
///
/// If:
///
/// ```text
/// Floor_0((2,0)) = (2,0)
/// ```
///
/// then:
///
/// ```text
/// Floor_0((1,0)) = (2,0)
/// Floor_0((0,2)) = (2,0)
/// Floor_0((0,1)) = (2,0)
/// Floor_0((0,0)) = (2,0)
/// ```
///
/// Likewise, if:
///
/// ```text
/// Floor_1((0,2)) = (0,2)
/// ```
///
/// then:
///
/// ```text
/// Floor_1((0,1)) = (0,2)
/// Floor_1((0,0)) = (0,2)
/// ```
///
/// Each counter dimension therefore maintains its own independent floor
/// lineage and canonical representative.
///
/// Together with [`ImplAffiliateInstances`], this provides the concrete
/// implementation-side values required to satisfy the trait-side
/// [`FloorAffiliatesCounters`] contract.
#[derive(Debug, Clone)]
pub(super) struct ImplFloorAffiliatesCounters;

impl<'a> Insertion<ItemImpl> for ImplItemType {
    fn raw_insert(&self, to: &mut ItemImpl, _: &()) -> Result<(), TokenStream> {
        to.items.push(ImplItem::Type(self.clone()));
        Ok(())
    }

    fn validate_inserted(
        _of: Option<&Self>,
        _to: &ItemImpl,
        _: Option<&()>,
    ) -> Result<(), TokenStream> {
        Ok(())
    }
}

impl<'a> Extension<Vec<ImplItemType>, ItemImpl, CounterArgsSlice<'a>>
    for ImplFloorAffiliatesCounters
{
    fn raw_extend(
        &self,
        towards: &ItemImpl,
        context: &CounterArgsSlice<'a>,
    ) -> Result<Vec<ImplItemType>, TokenStream> {
        let mut collect = Vec::new();
        for c in context.iter() {
            let ident = gen_type_ident_with_suffix::<FloorAffiliatesCounters>(Some(
                c.generic_index.to_string().as_bytes(),
            ));
            let ty = floor_counters_typenum_arg(towards, context, c)?;
            let item = ImplItemType {
                attrs: proc_suite::internal_code(),
                ident,
                ty,
                vis: syn::Visibility::Inherited,
                defaultness: None,
                type_token: Default::default(),
                generics: Default::default(),
                eq_token: Default::default(),
                semi_token: Default::default(),
            };
            collect.push(item)
        }
        Ok(collect)
    }

    fn validate_extend(
        &self,
        item: Option<&Vec<ImplItemType>>,
        towards: &ItemImpl,
        context: Option<&CounterArgsSlice<'a>>,
    ) -> Result<(), TokenStream> {
        let extracted;
        let counters = match context {
            Some(c) => c,
            None => {
                extracted = CounterArgs::checked_extract(towards, &None)?;
                extracted.as_slice().as_ref().as_ref()
            }
        };
        for c in counters.iter() {
            let ident = gen_type_ident_with_suffix::<FloorAffiliatesCounters>(Some(
                c.generic_index.to_string().as_bytes(),
            ));
            let ty = floor_counters_typenum_arg(towards, counters, c)?;
            validate_impl_types! {
                items: item,
                towards: towards,
                ident: ident,
                ty: ty,
                errors: {
                    not_found: AffiliateBugs::ImplFloorAffiliatesCountersNotFound {},
                    wrong_ident: AffiliateBugs::ImplFloorAffiliatesCountersWrongIdent {},
                    wrong_ty: AffiliateBugs::ImplFloorAffiliatesCountersInvalidType {},
                    has_generics: AffiliateBugs::ImplFloorAffiliatesCountersHasGenerics {},
                }
            };
        }
        Ok(())
    }
}

/// Builds the floor-affiliate typenum counter projection.
///
/// ```text
/// Floor_n(Current) = Floor_n(Next(Current))
/// ```
///
/// The resulting projection delegates floor resolution to the next
/// affiliate instance.
fn floor_counters_typenum_arg<'a>(
    impl_of: &ItemImpl,
    counters: CounterArgsSlice<'a>,
    counter: &CounterArg,
) -> Result<Type, TokenStream> {
    let floor_counter_ident = gen_type_ident_with_suffix::<FloorAffiliatesCounters>(Some(
        counter.generic_index.to_string().as_bytes(),
    ));
    let next_floor =
        AssocOfAffiliateTyExpr::<NextAffiliateInstance, NextAffiliateCounters>::checked_extract(
            &(&impl_of, &floor_counter_ident),
            &counters,
        )?
        .0;

    let ty: Type = parse_quote!(#next_floor);
    Ok(ty)
}

// ===============================================================================
// `````````````````````````` AFFILIATE COUNTERS CHECKER `````````````````````````
// ===============================================================================

/// Extends an implementation with compile-time affiliate consistency
/// assertions.
///
/// This phase is the implementation-side counterpart of
/// [`AffiliateCountersChecker`].
///
/// The generated constant verifies that affiliate relationships form a
/// single canonical instance chain for the implemented `Self` type.
///
/// ## Verified Invariants
///
/// Every non-terminal instance must satisfy:
///
/// ```text
/// back(next(current)) == current
/// next(back(current)) == current
/// ```
///
/// ensuring that:
///
/// - `Next` and `Back` are mutual inverses,
/// - no instance is skipped,
/// - no duplicate predecessor/successor paths exist,
/// - and all instances belong to a single consistent ordering.
///
/// In addition to validating the current instance, non-zeroth
/// instances recursively evaluate the predecessor's
/// [`AffiliateCountersChecker`].
///
/// Consequently, successful evaluation of any instance implies that the
/// entire predecessor chain also satisfies affiliate consistency.
///
/// ## Special Cases
///
/// The global minimum instance:
///
/// ```text
/// (0,0,0,...)
/// ```
///
/// has no predecessor and therefore only verifies:
///
/// ```text
/// next(back(current)) == current
/// ```
///
/// Likewise, terminal instances may omit successor validation because
/// no semantic next instance exists beyond the boundary.
///
/// ## Why this exists
///
/// Affiliates such as:
///
/// - `Next`,
/// - `Back`,
/// - and per-dimension `Floor`,
///
/// are implementation-defined rather than purely structural.
///
/// Consequently, incorrect implementations could accidentally create:
///
/// ```text
/// loops,
/// forks,
/// skipped instances,
/// unreachable instances,
/// or inconsistent predecessor/successor chains
/// ```
///
/// while still producing syntactically valid affiliate projections.
///
/// This checker detects such rogue affiliate configurations during
/// compile-time evaluation and rejects them through const assertions.
///
/// Together with affiliate generation, this helps guarantee that all
/// instance implementations for a given `Self` participate in a single
/// canonical affiliate topology.
///
/// Validation propagates through predecessor affiliates, allowing a
/// terminal instance to indirectly validate the complete affiliate
/// lineage leading back to the global-minimum instance.
#[derive(Debug, Clone)]
pub(super) struct ImplAffiliateCountersChecker;

impl<'a> Extension<ImplItemConst, ItemImpl, CounterArgsSlice<'a>> for ImplAffiliateCountersChecker {
    fn raw_extend(
        &self,
        towards: &ItemImpl,
        context: &CounterArgsSlice<'a>,
    ) -> Result<ImplItemConst, TokenStream> {
        let ident = gen_const_ident::<AffiliateCountersChecker>();
        let ty = parse_quote!(());
        let expr = counters_invariants_checker(towards, context, false)?;
        let item = ImplItemConst {
            attrs: proc_suite::internal_code(),
            ident,
            expr,
            ty,
            vis: syn::Visibility::Inherited,
            defaultness: None,
            const_token: Default::default(),
            generics: Default::default(),
            colon_token: Default::default(),
            eq_token: Default::default(),
            semi_token: Default::default(),
        };
        Ok(item)
    }

    fn validate_extend(
        &self,
        item: Option<&ImplItemConst>,
        towards: &ItemImpl,
        context: Option<&CounterArgsSlice<'a>>,
    ) -> Result<(), TokenStream> {
        let extracted;
        let counters = match context {
            Some(c) => c,
            None => {
                extracted = CounterArgs::checked_extract(towards, &None)?;
                extracted.as_slice().as_ref().as_ref()
            }
        };

        let ident = gen_const_ident::<AffiliateCountersChecker>();
        let ty: Type = parse_quote!(());
        let expr = counters_invariants_checker(towards, counters, false)?;

        validate_impl_const! {
            item: item,
            towards: towards,
            ident: ident,
            ty: ty,
            expr: expr,
            errors: {
                not_found: AffiliateBugs::ImplAffiliateCountersCheckerNotFound {},
                wrong_ident: AffiliateBugs::ImplAffiliateCountersCheckerWrongIdent {},
                wrong_ty: AffiliateBugs::ImplAffiliateCountersCheckerInvalidType {},
                has_generics: AffiliateBugs::ImplAffiliateCountersCheckerHasGenerics {},
                invalid_expr: AffiliateBugs::ImplAffiliateCountersCheckerInvalidExpr {},
            }
        }
    }
}

/// Builds compile-time affiliate consistency assertions.
///
/// Verified invariants:
///
/// ```text
/// back(next(current)) == current
/// next(back(current)) == current
/// ```
///
/// For non-zeroth instances, the generated assertion also recursively
/// evaluates the predecessor's [`AffiliateCountersChecker`].
///
/// Global-minimum instances skip predecessor validation because no
/// predecessor exists.
///
/// Terminal instances skip successor validation because no semantic
/// successor exists.
///
/// In case if terminal instance is in itself the global-minimum the
/// entire validation is itself skipped.
///
/// Any violation indicates an invalid affiliate topology and triggers a
/// compile-time panic.
pub(crate) fn counters_invariants_checker<'a>(
    impl_of: &ItemImpl,
    counters: CounterArgsSlice<'a>,
    terminal: bool,
) -> Result<Expr, TokenStream> {
    let crate_of = Instance::support_crate();
    let current_ty = ImplCountersTypeNum::checked_utilize(impl_of, &counters)?.0;

    let next_counters_ident = gen_type_ident::<NextAffiliateCounters>();
    let back_next_expr =
        AssocOfAffiliateTyExpr::<BackAffiliateInstance, BackAffiliateCounters>::checked_extract(
            &(impl_of, &next_counters_ident),
            &counters,
        )?
        .0;
    let back_next_ty: Type = parse_quote!(#back_next_expr);
    let back_check: Type =
        parse_quote!(<#current_ty as #crate_of::CountersEqual<#back_next_ty>>::Output);

    let back_counters_ident = gen_type_ident::<BackAffiliateCounters>();
    let next_back_expr =
        AssocOfAffiliateTyExpr::<NextAffiliateInstance, NextAffiliateCounters>::checked_extract(
            &(impl_of, &back_counters_ident),
            &counters,
        )?
        .0;
    let next_back_ty: Type = parse_quote!(#next_back_expr);
    let next_check: Type =
        parse_quote!(<#current_ty as #crate_of::CountersEqual<#next_back_ty>>::Output);

    let self_ty = *impl_of.self_ty.clone();
    let trait_of = ImplTraitIdent::checked_utilize(impl_of, &())?.0.clone();

    let back_err = AffiliateErrors::RougueInstanceFoundBackNextFailed {
        trait_of: trait_of.clone(),
        self_ty: self_ty.clone(),
        impl_of: impl_of.clone(),
    }
    .to_string();
    let next_err = AffiliateErrors::RougueInstanceFoundNextBackFailed {
        trait_of,
        self_ty,
        impl_of: impl_of.clone(),
    }
    .to_string();

    let self_ident = gen_const_ident::<AffiliateCountersChecker>();
    let back_checker =
        AssocOfAffiliateTyExpr::<BackAffiliateInstance, BackAffiliateCounters>::checked_extract(
            &(impl_of, &self_ident),
            &counters,
        )?
        .0;

    if !terminal {
        let expr = match is_zeroth_instance(counters)? {
            true => parse_quote!(
                {
                    if !<#next_check as #crate_of::Bit>::BOOL {
                        panic!(#next_err)
                    } else {
                        ()
                    }
                }
            ),
            false => parse_quote!(
                {
                    if !<#back_check as #crate_of::Bit>::BOOL {
                        panic!(#back_err)
                    } else if !<#next_check as #crate_of::Bit>::BOOL {
                        panic!(#next_err)
                    } else {
                        let _ = #back_checker;
                        ()
                    }
                }
            ),
        };
        return Ok(expr);
    }

    let terminal_expr = match is_zeroth_instance(counters)? {
        true => parse_quote!({ () }),
        false => parse_quote!(
            {
                if !<#back_check as #crate_of::Bit>::BOOL {
                    panic!(#back_err)
                } else {
                    let _ = #back_checker;
                    ()
                }
            }
        ),
    };

    Ok(terminal_expr)
}
