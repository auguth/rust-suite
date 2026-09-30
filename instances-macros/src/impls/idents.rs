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
// ``````````````````````` INSTANCE IDENTIFIER COLLECTIONS ```````````````````````
// ===============================================================================

//! Generates implementation-side identifier topology metadata for instance impls.
//!
//! Trait-side identifier expansion declares identifier contracts, but
//! those declarations alone describe only the identifiers attached to
//! the current implementation instance.
//!
//! For example:
//!
//! ```text
//! crypto -> sha512
//! ```
//!
//! does not reveal:
//!
//! - previously observed identifiers,
//! - sibling identifier branches,
//! - identifier lineage,
//! - parent consistency,
//! - or identifier uniqueness.
//!
//! This module generates the metadata required to reconstruct and
//! validate that semantic topology.
//!
//! ## Example
//!
//! Consider the following instance lineage:
//!
//! ```text
//! (0,0) -> crypto -> sha256
//! (0,1) -> crypto -> sha512
//! (0,2) -> crypto -> blake3
//! ```
//!
//! The current instance:
//!
//! ```text
//! (0,2)
//! ```
//!
//! knows only:
//!
//! ```text
//! crypto -> blake3
//! ```
//!
//! This module generates collections that preserve:
//!
//! ```text
//! crypto
//! sha256
//! sha512
//! blake3
//! ```
//!
//! allowing the entire identifier history of the lineage to be
//! recovered at compile time.
//!
//! ## Historical Topology
//!
//! Historical collections accumulate identifier information while
//! traversing semantic predecessor relationships.
//!
//! Example:
//!
//! ```text
//! (0,0) -> sha256
//! (0,1) -> sha512
//! (0,2) -> blake3
//! ```
//!
//! produces:
//!
//! ```text
//! [
//!     Some(sha256),
//!     Some(sha512),
//!     Some(blake3),
//! ]
//! ```
//!
//! If a larger identifier space later appears:
//!
//! ```text
//! [
//!     Some(sha256),
//!     Some(sha512),
//!     Some(blake3),
//!     None,
//! ]
//! ```
//!
//! the collection is expanded and newly introduced coordinates are
//! padded with `None` until an identifier occupies that position.
//!
//! ## Parent Consistency
//!
//! Semantic predecessor transitions must preserve parent identifiers.
//!
//! Valid:
//!
//! ```text
//! Current:
//!     crypto -> sha512
//!
//! Back:
//!     crypto -> sha256
//! ```
//!
//! Invalid:
//!
//! ```text
//! Current:
//!     crypto -> sha512
//!
//! Back:
//!     image -> sha256
//! ```
//!
//! because the parent identifier changed while traversing the same
//! semantic lineage.
//!
//! ## Hash Validation
//!
//! Hash collections are generated alongside identifier collections.
//!
//! These hashes are used to:
//!
//! - detect duplicate identifiers,
//! - validate parent consistency,
//! - compare identifier lineages,
//! - and perform compile-time topology verification.
//!
//! ## Generated Metadata
//!
//! Collectively the generated items provide:
//!
//! - identifier collections,
//! - identifier hash collections,
//! - historical identifier collections,
//! - historical identifier hash collections,
//! - collection cardinalities,
//! - historical collection cardinalities,
//! - and lineage validation.
//!
//! Together they form the complete implementation-side identifier graph
//! used by affiliate traversal, historical reconstruction, uniqueness
//! checking, and compile-time semantic validation.

// ===============================================================================
// ``````````````````````````````````` IMPORTS ```````````````````````````````````
// ===============================================================================

// --- Local Crate ---
use crate::{
    Extension, Extraction, Insertion, Instance, Transformation, Utilization,
    impls::{
        counters::*,
        errors::{IdentBugs, IdentErrors, UtilityBugs},
        utils::*,
    },
    traits::{affiliates::*, idents::*},
};

// --- Proc Macro Crates ---
use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident};
use syn::{Expr, ImplItem, ImplItemConst, ImplItemType, ItemImpl, Type, parse_quote};

// --- Proc Suite ---
use proc_suite::{SupportCrate, misc::*};

// ===============================================================================
// ```````````````````````````` INSTANCE IMPL IDENTS `````````````````````````````
// ===============================================================================

/// Expands all implementation-side identifier metadata.
///
/// This transformation generates the complete identifier infrastructure
/// required by affiliate navigation, lineage reconstruction, historical
/// topology tracking, and identifier validation.
///
/// ## Generated Components
///
/// - [`ImplCounterIdentCollectionLenTypeNum`]
///     Current identifier-collection cardinalities.
///
/// - [`ImplCounterIdentCollectionGenArray`]
///     Current identifier collections.
///
/// - [`ImplCounterIdentHashCollectionGenArray`]
///     Current identifier-hash collections with uniqueness validation.
///
/// - [`ImplCounterIdentHistoricalCollectionLenTypeNum`]
///     Historical collection cardinalities.
///
/// - [`ImplCounterIdentHistoricalCollectionGenArray`]
///     Historical identifier collections.
///
/// - [`ImplCounterIdentHashHistoricalCollectionGenArray`]
///     Historical identifier-hash collections.
///
/// - [`ImplCounterIdentExpectedHashChecker`]
///     Parent-lineage consistency validation.
///
/// ## Result
///
/// Together these generated items provide:
///
/// ```text
/// Current identifiers
/// Historical identifiers
/// Identifier hashes
/// Historical identifier hashes
/// Collection cardinalities
/// Lineage validation
/// ```
///
/// forming the complete compile-time identifier topology associated with
/// an implementation instance.
#[derive(Debug, Clone)]
pub(crate) struct InstanceImplIdents;

impl<'a> Transformation<ItemImpl, CounterArgsSlice<'a>> for InstanceImplIdents {
    fn raw_transform(
        &self,
        transform: &mut ItemImpl,
        context: &CounterArgsSlice,
    ) -> Result<(), TokenStream> {
        ImplCounterIdentCollectionLenTypeNum::checked_extend(
            &ImplCounterIdentCollectionLenTypeNum,
            transform,
            context,
        )?;
        ImplCounterIdentCollectionGenArray::checked_extend(
            &ImplCounterIdentCollectionGenArray,
            transform,
            context,
        )?;
        ImplCounterIdentHashCollectionGenArray::checked_extend(
            &ImplCounterIdentHashCollectionGenArray,
            transform,
            context,
        )?;
        ImplCounterIdentHistoricalCollectionLenTypeNum::checked_extend(
            &ImplCounterIdentHistoricalCollectionLenTypeNum,
            transform,
            context,
        )?;
        ImplCounterIdentHistoricalCollectionGenArray::checked_extend(
            &ImplCounterIdentHistoricalCollectionGenArray,
            transform,
            context,
        )?;
        ImplCounterIdentHashHistoricalCollectionGenArray::checked_extend(
            &ImplCounterIdentHashHistoricalCollectionGenArray,
            transform,
            context,
        )?;
        ImplCounterIdentExpectedHashChecker::checked_extend(
            &ImplCounterIdentExpectedHashChecker,
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
        ImplCounterIdentCollectionLenTypeNum::validate_extend(
            &ImplCounterIdentCollectionLenTypeNum,
            None,
            transform,
            context,
        )?;
        ImplCounterIdentCollectionGenArray::validate_extend(
            &ImplCounterIdentCollectionGenArray,
            None,
            transform,
            context,
        )?;
        ImplCounterIdentHashCollectionGenArray::validate_extend(
            &ImplCounterIdentHashCollectionGenArray,
            None,
            transform,
            context,
        )?;
        ImplCounterIdentHistoricalCollectionLenTypeNum::validate_extend(
            &ImplCounterIdentHistoricalCollectionLenTypeNum,
            None,
            transform,
            context,
        )?;
        ImplCounterIdentHistoricalCollectionGenArray::validate_extend(
            &ImplCounterIdentHistoricalCollectionGenArray,
            None,
            transform,
            context,
        )?;
        ImplCounterIdentHashHistoricalCollectionGenArray::validate_extend(
            &ImplCounterIdentHashHistoricalCollectionGenArray,
            None,
            transform,
            context,
        )?;
        ImplCounterIdentExpectedHashChecker::validate_extend(
            &ImplCounterIdentExpectedHashChecker,
            None,
            transform,
            context,
        )?;
        Ok(())
    }
}

// ===============================================================================
// ``````````````` IMPL COUNTER IDENT COLLECTION LENGTH (TYPENUM) ````````````````
// ===============================================================================

/// Generates the current identifier-collection length as a typenum.
///
/// Each counter value owns a collection containing all identifier values
/// from the beginning of that counter lineage up to and including the
/// current counter.
///
/// Consequently, the collection length is always one greater than the
/// current counter value.
///
/// Example:
///
/// ```text
/// Counter = 0
/// Collection = [Id0]
/// Length = U1
/// ```
///
/// ```text
/// Counter = 2
/// Collection = [Id0, Id1, Id2]
/// Length = U3
/// ```
///
/// ```text
/// Counter = 5
/// Collection = [Id0, Id1, Id2, Id3, Id4, Id5]
/// Length = U6
/// ```
///
/// This typenum length is later used to construct fixed-size
/// `GenericArray` collections containing:
///
/// - identifier values,
/// - identifier hashes,
/// - historical identifier collections,
/// - and historical identifier hash collections.
///
/// The generated type therefore acts as the canonical compile-time
/// cardinality associated with the current counter's identifier
/// collection.
#[derive(Debug, Clone)]
pub(crate) struct ImplCounterIdentCollectionLenTypeNum;

impl<'a> Insertion<ItemImpl, CounterArgsSlice<'a>> for Vec<ImplItemType> {
    fn raw_insert(&self, to: &mut ItemImpl, _: &CounterArgsSlice) -> Result<(), TokenStream> {
        for c in self {
            to.items.push(ImplItem::Type(c.clone()));
        }
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

impl<'a> Extension<Vec<ImplItemType>, ItemImpl, CounterArgsSlice<'a>>
    for ImplCounterIdentCollectionLenTypeNum
{
    fn raw_extend(
        &self,
        _towards: &ItemImpl,
        context: &CounterArgsSlice<'a>,
    ) -> Result<Vec<ImplItemType>, proc_macro2::TokenStream> {
        let mut collect = Vec::new();
        for c in context.iter() {
            let ident = gen_type_ident_with_suffix::<CounterIdentCollectionLenTypeNum>(Some(
                c.generic_index.to_string().as_bytes(),
            ));
            let ty = counter_ident_collection_len_typenum(c)?;
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
            collect.push(item);
        }
        Ok(collect)
    }

    fn validate_extend(
        &self,
        item: Option<&Vec<ImplItemType>>,
        towards: &ItemImpl,
        context: Option<&CounterArgsSlice<'a>>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let extracted;
        let counters = match context {
            Some(c) => c,
            None => {
                extracted = CounterArgs::checked_extract(towards, &None)?;
                extracted.as_slice().as_ref().as_ref()
            }
        };

        for c in counters.iter() {
            let ident = gen_type_ident_with_suffix::<CounterIdentCollectionLenTypeNum>(Some(
                c.generic_index.to_string().as_bytes(),
            ));
            let ty = counter_ident_collection_len_typenum(c)?;

            validate_impl_types! {
                items: item,
                towards: towards,
                ident: ident,
                ty: ty,
                errors: {
                    not_found: IdentBugs::ImplCounterIdentCollectionLenTypeNumNotFound {},
                    wrong_ident: IdentBugs::ImplCounterIdentCollectionLenTypeNumWrongIdent {},
                    wrong_ty: IdentBugs::ImplCounterIdentCollectionLenTypeNumInvalidType {},
                    has_generics: IdentBugs::ImplCounterIdentCollectionLenTypeNumHasGenerics {},
                }
            };
        }
        Ok(())
    }
}

/// Generates the identifier-collection cardinality typenum.
///
/// ```text
/// 0 -> U1
/// 2 -> U3
/// 5 -> U6
/// ```
fn counter_ident_collection_len_typenum(counter: &CounterArg) -> Result<Type, TokenStream> {
    let crate_of = Instance::support_crate();
    let lit = parse_pos_usize(&counter.const_lit)? + 1;
    let typenum = format_ident!("U{lit}");
    let ty = parse_quote!(#crate_of::#typenum);
    Ok(ty)
}

// ===============================================================================
// `````````````````` IMPL COUNTER IDENT COLLECTION (GEN-ARRAY) ``````````````````
// ===============================================================================

/// Generates the concrete identifier collection for each counter
/// dimension.
///
/// The collection contains every identifier previously observed along
/// that counter lineage, ending with the current identifier.
///
/// ## Collection Growth
///
/// Collections are built incrementally through the semantic predecessor
/// chain.
///
/// ```text
/// Collection(Current)
///     = Collection(Back(Current)) + CurrentIdentifier
/// ```
///
/// Example:
///
/// ```text
/// Counter 0:
///
/// 0 -> [crypto]
/// 1 -> [crypto, image]
/// 2 -> [crypto, image, audio]
/// ```
///
/// Each instance appends its own identifier to the collection inherited
/// from its back affiliate.
///
/// ## Generated Metadata
///
/// For every counter dimension, a hidden associated constant is
/// generated.
///
/// ```ignore
/// const __IndexNCollection:
///     GenericArray<&'static [u8], Self::__IndexNCollectionLen>;
/// ```
///
/// The resulting collection provides a fixed-size compile-time view of
/// all identifiers reachable along that counter lineage up to and
/// including the current instance.
#[derive(Debug, Clone)]
pub(crate) struct ImplCounterIdentCollectionGenArray;

impl<'a> Extension<Vec<ImplItemConst>, ItemImpl, CounterArgsSlice<'a>>
    for ImplCounterIdentCollectionGenArray
{
    fn raw_extend(
        &self,
        towards: &ItemImpl,
        context: &CounterArgsSlice<'a>,
    ) -> Result<Vec<ImplItemConst>, proc_macro2::TokenStream> {
        let mut collect = Vec::new();
        for c in context.iter() {
            let ident = gen_const_ident_with_suffix::<CounterIdentCollectionGenArray>(Some(
                c.generic_index.to_string().as_bytes(),
            ));

            let ty = counter_ident_collection_gen_array_ty(towards, c)?;

            let expr = counter_ident_collection_gen_array_expr(towards, context, c)?;

            let item = ImplItemConst {
                attrs: proc_suite::internal_code(),
                ident,
                ty,
                expr,
                vis: syn::Visibility::Inherited,
                defaultness: None,
                generics: Default::default(),
                eq_token: Default::default(),
                semi_token: Default::default(),
                const_token: Default::default(),
                colon_token: Default::default(),
            };
            collect.push(item);
        }
        Ok(collect)
    }

    fn validate_extend(
        &self,
        item: Option<&Vec<ImplItemConst>>,
        towards: &ItemImpl,
        context: Option<&CounterArgsSlice<'a>>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let extracted;
        let counters = match context {
            Some(c) => c,
            None => {
                extracted = CounterArgs::checked_extract(towards, &None)?;
                extracted.as_slice().as_ref().as_ref()
            }
        };

        for c in counters.iter() {
            let ident = gen_const_ident_with_suffix::<CounterIdentCollectionGenArray>(Some(
                c.generic_index.to_string().as_bytes(),
            ));
            let ty = counter_ident_collection_gen_array_ty(towards, c)?;
            let expr = counter_ident_collection_gen_array_expr(towards, counters, c)?;

            validate_impl_consts! {
                items: item,
                towards: towards,
                ident: ident,
                ty: ty,
                expr: expr,
                errors: {
                    not_found: IdentBugs::ImplCounterIdentCollectionGenArrayNotFound {},
                    wrong_ident: IdentBugs::ImplCounterIdentCollectionGenArrayWrongIdent {},
                    wrong_ty: IdentBugs::ImplCounterIdentCollectionGenArrayInvalidType {},
                    has_generics: IdentBugs::ImplCounterIdentCollectionGenArrayHasGenerics {},
                    invalid_expr: IdentBugs::ImplCounterIdentCollectionGenArrayInvalidExpr {},
                }
            }
        }

        Ok(())
    }
}

/// Generates the fixed-size identifier collection type.
///
/// ```ignore
/// GenericArray<&'static [u8], CountersCollectionLen>
/// ```
///
/// where `CountersCollectionLen` is the current identifier-collection
/// cardinality via implemented [`ImplCounterIdentCollectionLenTypeNum`].
fn counter_ident_collection_gen_array_ty(
    impl_of: &ItemImpl,
    counter: &CounterArg,
) -> Result<Type, TokenStream> {
    let gen_idx_str = &counter.generic_index.to_string();
    let gen_idx_bytes = gen_idx_str.as_bytes();

    let crate_of = Instance::support_crate();
    let len_ident =
        gen_type_ident_with_suffix::<CounterIdentCollectionLenTypeNum>(Some(gen_idx_bytes));
    let len = AssocTyExpr::checked_extract(&(impl_of, &len_ident), &())?.0;
    let ty = parse_quote!(#crate_of::GenericArray<&'static [u8], #len>);

    Ok(ty)
}

/// Builds the identifier collection constant expression.
///
/// ```text
/// Collection(Current)
///     = Collection(Back(Current)) + CurrentIdentifier
/// ```
///
/// Base case:
///
/// ```text
/// Counter = 0
/// Collection = [CurrentIdentifier]
/// ```
fn counter_ident_collection_gen_array_expr<'a>(
    impl_of: &ItemImpl,
    counters: CounterArgsSlice<'a>,
    counter: &CounterArg,
) -> Result<Expr, TokenStream> {
    let lit = parse_pos_usize(&counter.const_lit)?;
    let gen_idx_str = &counter.generic_index.to_string();
    let gen_idx_bytes = gen_idx_str.as_bytes();
    let counter_ascii_ident =
        gen_const_ident_with_suffix::<IndexedCounterIdent>(Some(gen_idx_bytes));
    let counter_ident = AssocTyExpr::checked_extract(&(impl_of, &counter_ascii_ident), &())?.0;
    let crate_of = Instance::support_crate();
    match lit == 0 {
        true => Ok(parse_quote!({#crate_of::GenericArray::from_array([#counter_ident])})),
        false => {
            let len_assoc =
                gen_type_ident_with_suffix::<CounterIdentCollectionLenTypeNum>(Some(gen_idx_bytes));

            let old_len_expr = AssocOfAffiliateTyExpr::<
                BackAffiliateInstance,
                BackAffiliateCounters,
            >::checked_extract(&(impl_of, &len_assoc), &counters)?
            .0;
            let old_len_ty: Type = parse_quote!(#old_len_expr);

            let new_len_expr = AssocTyExpr::checked_extract(&(impl_of, &len_assoc), &())?.0;
            let new_len_ty: Type = parse_quote!(#new_len_expr);

            let collection_ident =
                gen_const_ident_with_suffix::<CounterIdentCollectionGenArray>(Some(gen_idx_bytes));

            let back_collection = AssocOfAffiliateTyExpr::<
                BackAffiliateInstance,
                BackAffiliateCounters,
            >::checked_extract(
                &(impl_of, &collection_ident), &counters
            )?
            .0;

            let expr = parse_quote!(
                {
                    #crate_of::counter_ident_generic_collection_append::<#old_len_ty, #new_len_ty>(
                        &(#back_collection),
                        #counter_ident,
                    )
                }
            );
            Ok(expr)
        }
    }
}

// ===============================================================================
// ```````````````` IMPL COUNTER IDENT HASH COLLECTION (GEN-ARRAY) ```````````````
// ===============================================================================

/// Generates the concrete identifier-hash collection for each counter
/// dimension.
///
/// The collection contains every identifier hash previously observed
/// along that counter lineage, ending with the current identifier hash.
///
/// ## Collection Growth
///
/// Collections are built incrementally through the semantic predecessor
/// chain.
///
/// ```text
/// Collection(Current)
///     = Collection(Back(Current)) + Hash(CurrentIdentifier)
/// ```
///
/// Example:
///
/// ```text
/// Counter 0:
///
/// 0 -> [H("crypto")]
/// 1 -> [H("crypto"), H("image")]
/// 2 -> [H("crypto"), H("image"), H("audio")]
/// ```
///
/// ## Uniqueness Validation
///
/// Unlike [`ImplCounterIdentCollectionGenArray`], hash collections
/// additionally enforce identifier uniqueness.
///
/// Before appending the current hash, the generated collection verifies
/// that the hash does not already exist within the predecessor
/// collection.
///
/// ```text
/// Hash(CurrentIdentifier)
///     not_in Collection(Back(Current))
/// ```
///
/// Duplicate hashes indicate duplicate identifiers within the same
/// counter lineage and therefore cause a compile-time panic.
///
/// ## Generated Metadata
///
/// For every counter dimension, a hidden associated constant is
/// generated.
///
/// ```ignore
/// const __IndexNHashCollection:
///     GenericArray<u64, Self::__IndexNCollectionLen>;
/// ```
///
/// The resulting collection provides a fixed-size compile-time view of
/// all identifier hashes reachable along that counter lineage while
/// simultaneously validating uniqueness.
#[derive(Debug, Clone)]
pub(crate) struct ImplCounterIdentHashCollectionGenArray;

impl<'a> Extension<Vec<ImplItemConst>, ItemImpl, CounterArgsSlice<'a>>
    for ImplCounterIdentHashCollectionGenArray
{
    fn raw_extend(
        &self,
        towards: &ItemImpl,
        context: &CounterArgsSlice<'a>,
    ) -> Result<Vec<ImplItemConst>, proc_macro2::TokenStream> {
        let mut collect = Vec::new();
        for c in context.iter() {
            let ident = gen_const_ident_with_suffix::<CounterIdentHashCollectionGenArray>(Some(
                c.generic_index.to_string().as_bytes(),
            ));

            let ty = counter_ident_hash_collection_gen_array_ty(towards, c)?;

            let expr = counter_ident_hash_collection_gen_array_expr(towards, context, c)?;

            let item = ImplItemConst {
                attrs: proc_suite::internal_code(),
                ident,
                ty,
                expr,
                vis: syn::Visibility::Inherited,
                defaultness: None,
                generics: Default::default(),
                eq_token: Default::default(),
                semi_token: Default::default(),
                const_token: Default::default(),
                colon_token: Default::default(),
            };
            collect.push(item);
        }
        Ok(collect)
    }

    fn validate_extend(
        &self,
        item: Option<&Vec<ImplItemConst>>,
        towards: &ItemImpl,
        context: Option<&CounterArgsSlice<'a>>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let extracted;
        let counters = match context {
            Some(c) => c,
            None => {
                extracted = CounterArgs::checked_extract(towards, &None)?;
                extracted.as_slice().as_ref().as_ref()
            }
        };

        for c in counters.iter() {
            let ident = gen_const_ident_with_suffix::<CounterIdentHashCollectionGenArray>(Some(
                c.generic_index.to_string().as_bytes(),
            ));
            let ty = counter_ident_hash_collection_gen_array_ty(towards, c)?;
            let expr = counter_ident_hash_collection_gen_array_expr(towards, counters, c)?;

            validate_impl_consts! {
                items: item,
                towards: towards,
                ident: ident,
                ty: ty,
                expr: expr,
                errors: {
                    not_found: IdentBugs::ImplCounterIdentHashCollectionGenArrayNotFound {},
                    wrong_ident: IdentBugs::ImplCounterIdentHashCollectionGenArrayWrongIdent {},
                    wrong_ty: IdentBugs::ImplCounterIdentHashCollectionGenArrayInvalidType {},
                    has_generics: IdentBugs::ImplCounterIdentHashCollectionGenArrayHasGenerics {},
                    invalid_expr: IdentBugs::ImplCounterIdentHashCollectionGenArrayInvalidExpr {},
                }
            }
        }

        Ok(())
    }
}

/// Generates the fixed-size identifier-hash collection type.
///
/// ```ignore
/// GenericArray<u64, CountersCollectionLen>
/// ```
///
/// where `CountersCollectionLen` is the current identifier-collection
/// cardinality via implemented [`ImplCounterIdentCollectionLenTypeNum`].
fn counter_ident_hash_collection_gen_array_ty(
    impl_of: &ItemImpl,
    counter: &CounterArg,
) -> Result<Type, TokenStream> {
    let gen_idx_str = &counter.generic_index.to_string();
    let gen_idx_bytes = gen_idx_str.as_bytes();

    let crate_of = Instance::support_crate();
    let len_ident =
        gen_type_ident_with_suffix::<CounterIdentCollectionLenTypeNum>(Some(gen_idx_bytes));
    let len = AssocTyExpr::checked_extract(&(impl_of, &len_ident), &())?.0;
    let ty = parse_quote!(#crate_of::GenericArray<u64, #len>);

    Ok(ty)
}

/// Builds the identifier-hash collection constant expression.
///
/// ```text
/// Collection(Current)
///     = Collection(Back(Current))
///       + Hash(CurrentIdentifier)
/// ```
///
/// Before appending, the current hash must not already exist within the
/// predecessor collection.
///
/// Base case:
///
/// ```text
/// Counter = 0
/// Collection = [Hash(CurrentIdentifier)]
/// ```
///
/// Duplicate hashes cause a compile-time panic.
fn counter_ident_hash_collection_gen_array_expr<'a>(
    impl_of: &ItemImpl,
    counters: CounterArgsSlice<'a>,
    counter: &CounterArg,
) -> Result<Expr, TokenStream> {
    let lit = parse_pos_usize(&counter.const_lit)?;
    let gen_idx_str = &counter.generic_index.to_string();
    let gen_idx_bytes = gen_idx_str.as_bytes();
    let counter_hash_ident = gen_const_ident_with_suffix::<CounterIdentHash>(Some(gen_idx_bytes));
    let counter_hash = AssocTyExpr::checked_extract(&(impl_of, &counter_hash_ident), &())?.0;
    let crate_of = Instance::support_crate();
    match lit == 0 {
        true => Ok(parse_quote!({#crate_of::GenericArray::from_array([#counter_hash])})),
        false => {
            let len_assoc =
                gen_type_ident_with_suffix::<CounterIdentCollectionLenTypeNum>(Some(gen_idx_bytes));

            let old_len_expr = AssocOfAffiliateTyExpr::<
                BackAffiliateInstance,
                BackAffiliateCounters,
            >::checked_extract(&(impl_of, &len_assoc), &counters)?
            .0;
            let old_len_ty: Type = parse_quote!(#old_len_expr);

            let new_len_expr = AssocTyExpr::checked_extract(&(impl_of, &len_assoc), &())?.0;
            let new_len_ty: Type = parse_quote!(#new_len_expr);

            let hash_collection_ident = gen_const_ident_with_suffix::<
                CounterIdentHashCollectionGenArray,
            >(Some(gen_idx_bytes));

            let back_collection = AssocOfAffiliateTyExpr::<
                BackAffiliateInstance,
                BackAffiliateCounters,
            >::checked_extract(
                &(impl_of, &hash_collection_ident), &counters
            )?
            .0;

            let err = IdentErrors::DuplicateCounterIdentFound {
                lit: counter.const_lit.clone(),
            }
            .to_string();
            let expr = parse_quote!(
                {
                    match #crate_of::counter_ident_generic_hash_collection_append::<#old_len_ty, #new_len_ty>(
                        &(#back_collection),
                        #counter_hash,
                    ) {
                        Some(c) => c,
                        None => panic!(#err)
                    }
                }
            );
            Ok(expr)
        }
    }
}

// ===============================================================================
// ````````````` IMPL COUNTER HISTORICAL COLLECTION LENGTH (TYPENUM) `````````````
// ===============================================================================

/// Generates the padded historical collection cardinalities for every
/// counter dimension.
///
/// Unlike normal collection lengths, historical lengths do not describe
/// the number of identifiers visible at the current instance.
///
/// Instead, they represent the largest collection cardinality observed
/// anywhere within the reachable history of that counter lineage.
///
/// ## Historical Collection Shape
///
/// Historical collections preserve every previously reachable branch of
/// a counter hierarchy.
///
/// Consider:
///
/// ```text
/// Counter 0:
///
///     crypto
///     image
///
/// Counter 1:
///
///     crypto -> [sha256, sha512]
///     image  -> [png, jpeg, gif]
/// ```
///
/// The visible collection lengths are:
///
/// ```text
/// crypto -> 2
/// image  -> 3
/// ```
///
/// A historical collection must preserve both branches:
///
/// ```text
/// [
///     [sha256, sha512, PAD],
///     [png,    jpeg,   gif]
/// ]
/// ```
///
/// Therefore the historical cardinality becomes:
///
/// ```text
/// max(2, 3) = 3
/// ```
///
/// rather than the cardinality of the current branch.
///
/// ## Historical Cardinality Propagation
///
/// Historical lengths are propagated through the predecessor chain.
///
/// ```text
/// HistoricalLen(Current)
///     = max(
///         CountersCollectionLen(Current),
///         HistoricalLen(Back(Current))
///       )
/// ```
///
/// Example:
///
/// ```text
/// Current collection length = 2
/// Previous historical max   = 5
///
/// Historical length         = 5
/// ```
///
/// This propagated maximum is later used by historical identifier and
/// identifier-hash collections to construct uniformly padded historical
/// views.
///
/// ## Generated Metadata
///
/// For every counter dimension, a hidden associated type is generated.
///
/// ```ignore
/// type __IndexNHistoricalCollectionLen = /* padded cardinality */;
/// ```
///
/// These lengths define the fixed-size dimensions used by all historical
/// collection metadata.
#[derive(Debug, Clone)]
pub(crate) struct ImplCounterIdentHistoricalCollectionLenTypeNum;

impl<'a> Extension<Vec<ImplItemType>, ItemImpl, CounterArgsSlice<'a>>
    for ImplCounterIdentHistoricalCollectionLenTypeNum
{
    fn raw_extend(
        &self,
        towards: &ItemImpl,
        context: &CounterArgsSlice<'a>,
    ) -> Result<Vec<ImplItemType>, proc_macro2::TokenStream> {
        let mut collect = Vec::new();
        for c in context.iter() {
            let ident = gen_type_ident_with_suffix::<CounterIdentHistoricalCollectionLenTypeNum>(
                Some(c.generic_index.to_string().as_bytes()),
            );
            let ty = match is_zeroth_instance(context)? {
                true => zeroth_counter_ident_historical_collection_len_typenum(towards, c)?,
                false => {
                    non_zeroth_counter_ident_historical_collection_len_typenum(towards, context, c)?
                }
            };
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
            collect.push(item);
        }
        Ok(collect)
    }

    fn validate_extend(
        &self,
        item: Option<&Vec<ImplItemType>>,
        towards: &ItemImpl,
        context: Option<&CounterArgsSlice<'a>>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let extracted;
        let counters = match context {
            Some(c) => c,
            None => {
                extracted = CounterArgs::checked_extract(towards, &None)?;
                extracted.as_slice().as_ref().as_ref()
            }
        };

        for c in counters.iter() {
            let ident = gen_type_ident_with_suffix::<CounterIdentHistoricalCollectionLenTypeNum>(
                Some(c.generic_index.to_string().as_bytes()),
            );
            let ty = match is_zeroth_instance(counters)? {
                true => zeroth_counter_ident_historical_collection_len_typenum(towards, c)?,
                false => non_zeroth_counter_ident_historical_collection_len_typenum(
                    towards, counters, c,
                )?,
            };

            validate_impl_types! {
                items: item,
                towards: towards,
                ident: ident,
                ty: ty,
                errors: {
                    not_found: IdentBugs::ImplCounterIdentHistoricalCollectionLenTypeNumNotFound {},
                    wrong_ident: IdentBugs::ImplCounterIdentHistoricalCollectionLenTypeNumWrongIdent {},
                    wrong_ty: IdentBugs::ImplCounterIdentHistoricalCollectionLenTypeNumInvalidType {},
                    has_generics: IdentBugs::ImplCounterIdentHistoricalCollectionLenTypeNumHasGenerics {},
                }
            };
        }
        Ok(())
    }
}

/// Generates the historical collection cardinality for the zeroth
/// instance.
///
/// Since no predecessor history exists, the historical cardinality is
/// simply the current collection cardinality.
///
/// ```text
/// HistoricalLen(Current)
///     = CountersCollectionLen(Current)
/// ```
fn zeroth_counter_ident_historical_collection_len_typenum(
    impl_of: &ItemImpl,
    counter: &CounterArg,
) -> Result<Type, TokenStream> {
    let gen_idx_str = &counter.generic_index.to_string();
    let gen_idx_bytes = gen_idx_str.as_bytes();

    let current_len_ident =
        gen_type_ident_with_suffix::<CounterIdentCollectionLenTypeNum>(Some(gen_idx_bytes));
    let current_len_expr = AssocTyExpr::checked_extract(&(impl_of, &current_len_ident), &())?.0;
    let ty = parse_quote!(#current_len_expr);
    Ok(ty)
}

/// Computes the historical collection cardinality for a non-zeroth
/// instance.
///
/// ```text
/// HistoricalLen(Current)
///     = max(
///         CountersCollectionLen(Current),
///         HistoricalLen(Back(Current))
///       )
/// ```
///
/// The resulting cardinality represents the largest collection length
/// encountered anywhere within the reachable history of the counter
/// lineage.
///
/// This value is later used to pad shorter historical branches so that
/// all historical collections share a common fixed-size shape.
fn non_zeroth_counter_ident_historical_collection_len_typenum<'a>(
    impl_of: &ItemImpl,
    counters: CounterArgsSlice<'a>,
    counter: &CounterArg,
) -> Result<Type, TokenStream> {
    let gen_idx_str = &counter.generic_index.to_string();
    let gen_idx_bytes = gen_idx_str.as_bytes();

    let current_len_ident =
        gen_type_ident_with_suffix::<CounterIdentCollectionLenTypeNum>(Some(gen_idx_bytes));
    let current_len_expr = AssocTyExpr::checked_extract(&(impl_of, &current_len_ident), &())?.0;
    let current_len_ty: Type = parse_quote!(#current_len_expr);

    let historical_len_ident = gen_type_ident_with_suffix::<
        CounterIdentHistoricalCollectionLenTypeNum,
    >(Some(gen_idx_bytes));
    let back_historical_len_expr =
        AssocOfAffiliateTyExpr::<BackAffiliateInstance, BackAffiliateCounters>::checked_extract(
            &(impl_of, &historical_len_ident),
            &counters,
        )?
        .0;
    let back_historical_len_ty: Type = parse_quote!(#back_historical_len_expr);

    let crate_of = Instance::support_crate();

    let ty = parse_quote!(
        <#current_len_ty as #crate_of::Max<#back_historical_len_ty>>::Output
    );
    Ok(ty)
}

// ===============================================================================
// ````````````` IMPL COUNTER IDENT HISTORICAL COLLECTION (GEN-ARRAY) ````````````
// ===============================================================================

/// Generates historical identifier collections for every counter
/// dimension.
///
/// Unlike [`CounterIdentCollectionGenArray`], which stores only the
/// identifiers reachable within the current counter lineage, historical
/// collections accumulate identifiers across the entire semantic
/// predecessor chain.
///
/// ## Construction
///
/// For non-zeroth instances the collection is derived from:
///
/// ```text
/// Historical(Back(Current))
/// ```
///
/// The predecessor collection is first expanded to the current
/// historical cardinalities.
///
/// Any newly introduced coordinates are padded with:
///
/// ```text
/// None
/// ```
///
/// indicating that no historical identifier has ever occupied that
/// coordinate.
///
/// The current identifier is then recorded at the coordinates
/// represented by the current counter values:
///
/// ```text
/// Historical(Current)[CurrentCounters]
///     = Some(CurrentIdentifier)
/// ```
///
/// ## Example
///
/// Starting from:
///
/// ```text
/// Historical(Back)
///
/// [
///     [Some(A), Some(B)],
///     [Some(C), None]
/// ]
/// ```
///
/// a larger historical shape may be required:
///
/// ```text
/// [
///     [Some(A), Some(B), None],
///     [Some(C), None,    None]
/// ]
/// ```
///
/// After recording the current identifier:
///
/// ```text
/// [
///     [Some(A), Some(B), None],
///     [Some(C), Some(D), None]
/// ]
/// ```
///
/// where:
///
/// ```text
/// Some(X)
///     => identifier historically existed at that coordinate
///
/// None
///     => coordinate exists in the historical shape but no identifier
///        has ever been recorded there
/// ```
///
/// Historical cardinalities are determined by
/// [`CounterIdentHistoricalCollectionLenTypeNum`].
#[derive(Debug, Clone)]
pub(crate) struct ImplCounterIdentHistoricalCollectionGenArray;

impl<'a> Extension<Vec<ImplItemConst>, ItemImpl, CounterArgsSlice<'a>>
    for ImplCounterIdentHistoricalCollectionGenArray
{
    fn raw_extend(
        &self,
        towards: &ItemImpl,
        context: &CounterArgsSlice<'a>,
    ) -> Result<Vec<ImplItemConst>, proc_macro2::TokenStream> {
        let mut collect = Vec::new();
        let tys = historical_collection_gen_array_nested_ty(
            towards,
            context,
            parse_quote!(&'static [u8]),
        )?;
        let exprs = counter_ident_historical_collection_gen_array_expr(towards, context)?;
        for ((c, ty), expr) in context.iter().zip(tys).zip(exprs) {
            let ident = gen_const_ident_with_suffix::<CounterIdentHistoricalCollectionGenArray>(
                Some(c.generic_index.to_string().as_bytes()),
            );

            let item = ImplItemConst {
                attrs: proc_suite::internal_code(),
                ident,
                ty,
                expr,
                vis: syn::Visibility::Inherited,
                defaultness: None,
                generics: Default::default(),
                eq_token: Default::default(),
                semi_token: Default::default(),
                const_token: Default::default(),
                colon_token: Default::default(),
            };
            collect.push(item);
        }
        Ok(collect)
    }

    fn validate_extend(
        &self,
        item: Option<&Vec<ImplItemConst>>,
        towards: &ItemImpl,
        context: Option<&CounterArgsSlice<'a>>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let extracted;
        let counters = match context {
            Some(c) => c,
            None => {
                extracted = CounterArgs::checked_extract(towards, &None)?;
                extracted.as_slice().as_ref().as_ref()
            }
        };

        let tys = historical_collection_gen_array_nested_ty(
            towards,
            counters,
            parse_quote!(&'static [u8]),
        )?;
        let exprs = counter_ident_historical_collection_gen_array_expr(towards, counters)?;
        for ((c, ty), expr) in counters.iter().zip(tys).zip(exprs) {
            let ident = gen_const_ident_with_suffix::<CounterIdentHistoricalCollectionGenArray>(
                Some(c.generic_index.to_string().as_bytes()),
            );

            validate_impl_consts! {
                items: item,
                towards: towards,
                ident: ident,
                ty: ty,
                expr: expr,
                errors: {
                    not_found: IdentBugs::ImplCounterIdentHistoricalCollectionGenArrayNotFound {},
                    wrong_ident: IdentBugs::ImplCounterIdentHistoricalCollectionGenArrayWrongIdent {},
                    wrong_ty: IdentBugs::ImplCounterIdentHistoricalCollectionGenArrayInvalidType {},
                    has_generics: IdentBugs::ImplCounterIdentHistoricalCollectionGenArrayHasGenerics {},
                    invalid_expr: IdentBugs::ImplCounterIdentHistoricalCollectionGenArrayInvalidExpr {},
                }
            }
        }

        Ok(())
    }
}

/// Builds the nested historical collection type for each counter.
///
/// Historical collections are represented as nested `GenericArray`s whose
/// dimensions correspond to the historical cardinality of every counter
/// leading up to the current counter.
///
/// ```text
/// Counter 0 -> [Option<T>; Len0]
///
/// Counter 1 -> [[Option<T>; Len1]; Len0]
///
/// Counter 2 -> [[[Option<T>; Len2]; Len1]; Len0]
/// ```
///
/// where each `LenN` is the historical cardinality associated with that
/// counter.
///
/// The resulting type is later used to store every historically observed
/// value indexed by counter coordinates.
fn historical_collection_gen_array_nested_ty<'a>(
    impl_of: &ItemImpl,
    context: CounterArgsSlice<'a>,
    raw_ty: Type,
) -> Result<Vec<Type>, TokenStream> {
    let mut collect = Vec::new();
    let self_ty = &impl_of.self_ty;
    let trait_path = ImplTraitPath::checked_utilize(impl_of, &())?.0;
    for (i, _) in context.iter().enumerate() {
        let mut counters = context.to_vec();
        let mut rev = counters.iter_mut().take(i + 1).rev();
        let Some(c) = rev.next() else {
            return Err(UtilityBugs::CounterArgsEmptyForFirstCounterAccess {}.into());
        };
        let len = gen_type_ident_with_suffix::<CounterIdentHistoricalCollectionLenTypeNum>(Some(
            c.generic_index.to_string().as_bytes(),
        ));
        let crate_of = Instance::support_crate();
        let mut ty =
            parse_quote!(#crate_of::GenericArray<Option<#raw_ty>, <#self_ty as #trait_path>::#len>);
        for c in rev {
            let len = gen_type_ident_with_suffix::<CounterIdentHistoricalCollectionLenTypeNum>(
                Some(c.generic_index.to_string().as_bytes()),
            );
            ty = parse_quote!(#crate_of::GenericArray<#ty, <#self_ty as #trait_path>::#len>);
        }
        collect.push(ty);
    }
    Ok(collect)
}

/// Builds historical identifier collections.
///
/// Zeroth instances construct an initial historical collection.
///
/// All other instances derive their collection from the semantic
/// predecessor and record the current identifier into the resulting
/// structure.
fn counter_ident_historical_collection_gen_array_expr<'a>(
    impl_of: &ItemImpl,
    counters: CounterArgsSlice<'a>,
) -> Result<Vec<Expr>, TokenStream> {
    match is_zeroth_instance(counters)? {
        true => {
            zeroth_instance_historical_collection_expr::<IndexedCounterIdent>(impl_of, counters)
        }
        false => non_zeroth_instance_historical_collection_expr::<
            IndexedCounterIdent,
            CounterIdentHistoricalCollectionGenArray,
        >(impl_of, counters, parse_quote!(&'static [u8])),
    }
}

/// Constructs the initial historical collection for the global minimum
/// instance.
///
/// Since no predecessor exists, the collection contains only the current
/// identifier placed at the current coordinates.
///
/// ```text
/// Historical(Current)
///     = Some(CurrentIdentifier)
/// ```
///
/// wrapped into the required number of nested dimensions.
///
/// Example:
///
/// ```text
/// Counter 0:
///     [Some(A)]
///
/// Counter 1:
///     [[Some(B)]]
/// ```
fn zeroth_instance_historical_collection_expr<'a, Item: 'static>(
    impl_of: &ItemImpl,
    counters: CounterArgsSlice<'a>,
) -> Result<Vec<Expr>, TokenStream> {
    let mut collect = Vec::new();

    for (i, c) in counters.iter().enumerate() {
        let assoc_ident =
            gen_const_ident_with_suffix::<Item>(Some(c.generic_index.to_string().as_bytes()));
        let assoc_expr = AssocTyExpr::checked_extract(&(impl_of, &assoc_ident), &())?.0;

        let crate_of = Instance::support_crate();

        let mut expr: Expr = parse_quote!(Some(#assoc_expr));

        for _ in 0..=i {
            expr = parse_quote!(
                #crate_of::GenericArray::from_array([#expr])
            );
        }

        collect.push(expr);
    }

    Ok(collect)
}

/// Extends the predecessor historical collection with the current
/// identifier.
///
/// The algorithm:
///
/// ```text
/// Historical(Current)
///     = Historical(Back(Current))
///
/// resize to current historical cardinalities
///
/// pad newly introduced slots with None
///
/// Historical(Current)[CurrentCounters]
///     = Some(CurrentIdentifier)
/// ```
///
/// `None` indicates that a coordinate exists within the historical shape
/// but no identifier has yet been recorded at that location.
///
/// Previously recorded entries are preserved while the current identifier
/// is written into its coordinate.
fn non_zeroth_instance_historical_collection_expr<'a, Item: 'static, Collection: 'static>(
    impl_of: &ItemImpl,
    counters: CounterArgsSlice<'a>,
    raw_ty: Type,
) -> Result<Vec<Expr>, TokenStream> {
    if counters.is_empty() {}
    let mut collect = Vec::new();
    for (i, c) in counters.iter().enumerate() {
        let assoc_ident =
            gen_const_ident_with_suffix::<Item>(Some(c.generic_index.to_string().as_bytes()));
        let assoc_expr = AssocTyExpr::checked_extract(&(impl_of, &assoc_ident), &())?.0;

        let historical_ident =
            gen_const_ident_with_suffix::<Collection>(Some(c.generic_index.to_string().as_bytes()));
        let back_collec = AssocOfAffiliateTyExpr::<BackAffiliateInstance, BackAffiliateCounters>::checked_extract(&(impl_of, &historical_ident), &counters)?.0;

        let counters_slc = counters.iter().take(i + 1);

        let crate_of = Instance::support_crate();
        let fn_ident = format_ident!("historical_collection_{}", &counters_slc.len());

        let mut old_lens = Vec::new();
        let mut new_lens = Vec::new();
        let mut indexes = Vec::new();

        for c_slc in counters_slc {
            indexes.push(&c_slc.const_lit);

            let gen_idx_str = c_slc.generic_index.to_string();
            let gen_idx_byt = gen_idx_str.as_bytes();

            let len_ident = gen_type_ident_with_suffix::<CounterIdentHistoricalCollectionLenTypeNum>(
                Some(gen_idx_byt),
            );
            let old_len_expr = AssocOfAffiliateTyExpr::<
                BackAffiliateInstance,
                BackAffiliateCounters,
            >::checked_extract(&(impl_of, &len_ident), &counters)?
            .0;
            let old_len_ty: Type = parse_quote!(#old_len_expr);

            let new_len_expr = AssocTyExpr::checked_extract(&(impl_of, &len_ident), &())?.0;
            let new_len_ty: Type = parse_quote!(#new_len_expr);

            old_lens.push(old_len_ty);
            new_lens.push(new_len_ty);
        }

        let expr = parse_quote!(
            {
                #crate_of::#fn_ident::<
                    #raw_ty,
                    #(#old_lens),* ,
                    #(#new_lens),*
                >(
                    &(#back_collec),
                    Some(#assoc_expr),
                    #(#indexes),*
                )

            }
        );

        collect.push(expr);
    }
    Ok(collect)
}

// ===============================================================================
// `````````` IMPL COUNTER IDENT HISTORICAL HASH COLLECTION (GEN-ARRAY) ``````````
// ===============================================================================

/// Generates historical identifier-hash collections for every counter
/// dimension.
///
/// This is the hash-based counterpart of
/// [`ImplCounterIdentHistoricalCollectionGenArray`].
///
/// Instead of storing historical identifier byte strings:
///
/// ```text
/// Option<&'static [u8]>
/// ```
///
/// the generated collections store:
///
/// ```text
/// Option<u64>
/// ```
///
/// where each hash is produced by [`CounterIdentHash`].
///
/// Construction follows the same historical expansion algorithm:
///
/// ```text
/// Historical(Current)
///     = Historical(Back(Current))
///
/// resize to current historical cardinalities
///
/// pad newly introduced slots with None
///
/// Historical(Current)[CurrentCounters]
///     = Some(CurrentIdentifierHash)
/// ```
///
/// The resulting collections provide fixed-size historical views of the
/// complete identifier-hash hierarchy and are primarily used for
/// compile-time validation, equality checks, and historical topology
/// reconstruction.
#[derive(Debug, Clone)]
pub(crate) struct ImplCounterIdentHashHistoricalCollectionGenArray;

impl<'a> Extension<Vec<ImplItemConst>, ItemImpl, CounterArgsSlice<'a>>
    for ImplCounterIdentHashHistoricalCollectionGenArray
{
    fn raw_extend(
        &self,
        towards: &ItemImpl,
        context: &CounterArgsSlice<'a>,
    ) -> Result<Vec<ImplItemConst>, proc_macro2::TokenStream> {
        let mut collect = Vec::new();
        let tys = historical_collection_gen_array_nested_ty(towards, context, parse_quote!(u64))?;
        let exprs = counter_ident_hash_historical_collection_gen_array_expr(towards, context)?;
        for ((c, ty), expr) in context.iter().zip(tys).zip(exprs) {
            let ident = gen_const_ident_with_suffix::<CounterIdentHashHistoricalCollectionGenArray>(
                Some(c.generic_index.to_string().as_bytes()),
            );

            let item = ImplItemConst {
                attrs: proc_suite::internal_code(),
                ident,
                ty,
                expr,
                vis: syn::Visibility::Inherited,
                defaultness: None,
                generics: Default::default(),
                eq_token: Default::default(),
                semi_token: Default::default(),
                const_token: Default::default(),
                colon_token: Default::default(),
            };
            collect.push(item);
        }
        Ok(collect)
    }

    fn validate_extend(
        &self,
        item: Option<&Vec<ImplItemConst>>,
        towards: &ItemImpl,
        context: Option<&CounterArgsSlice<'a>>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let extracted;
        let counters = match context {
            Some(c) => c,
            None => {
                extracted = CounterArgs::checked_extract(towards, &None)?;
                extracted.as_slice().as_ref().as_ref()
            }
        };

        let tys = historical_collection_gen_array_nested_ty(towards, counters, parse_quote!(u64))?;
        let exprs = counter_ident_hash_historical_collection_gen_array_expr(towards, counters)?;
        for ((c, ty), expr) in counters.iter().zip(tys).zip(exprs) {
            let ident = gen_const_ident_with_suffix::<CounterIdentHashHistoricalCollectionGenArray>(
                Some(c.generic_index.to_string().as_bytes()),
            );

            validate_impl_consts! {
                items: item,
                towards: towards,
                ident: ident,
                ty: ty,
                expr: expr,
                errors: {
                    not_found: IdentBugs::ImplCounterIdentHashHistoricalCollectionGenArrayNotFound {},
                    wrong_ident: IdentBugs::ImplCounterIdentHashHistoricalCollectionGenArrayWrongIdent {},
                    wrong_ty: IdentBugs::ImplCounterIdentHashHistoricalCollectionGenArrayInvalidType {},
                    has_generics: IdentBugs::ImplCounterIdentHashHistoricalCollectionGenArrayHasGenerics {},
                    invalid_expr: IdentBugs::ImplCounterIdentHashHistoricalCollectionGenArrayInvalidExpr {},
                }
            }
        }

        Ok(())
    }
}

/// Builds historical identifier-hash collections.
///
/// Zeroth instances construct an initial historical collection from the
/// current [`CounterIdentHash`].
///
/// Non-zeroth instances extend the predecessor historical collection,
/// resize it to the current historical cardinalities, pad newly
/// introduced coordinates with `None`, and record the current hash at
/// the current counter coordinates.
fn counter_ident_hash_historical_collection_gen_array_expr<'a>(
    impl_of: &ItemImpl,
    counters: CounterArgsSlice<'a>,
) -> Result<Vec<Expr>, TokenStream> {
    match is_zeroth_instance(counters)? {
        true => zeroth_instance_historical_collection_expr::<CounterIdentHash>(impl_of, counters),
        false => non_zeroth_instance_historical_collection_expr::<
            CounterIdentHash,
            CounterIdentHashHistoricalCollectionGenArray,
        >(impl_of, counters, parse_quote!(u64)),
    }
}

// ===============================================================================
// ```````````````````` COUNTERS IDENT EXPECTED HASH CHECKER `````````````````````
// ===============================================================================

/// Generates a const-assertion to verify that counter-identifier lineage
/// remains consistent across semantic predecessor transitions.
///
/// For every active counter dimension:
///
/// ```text
/// Current[i] > 0
/// ```
///
/// all parent dimensions:
///
/// ```text
/// 0 .. i
/// ```
///
/// must refer to the same identifiers in both the current instance and
/// its back affiliate.
///
/// ```text
/// Current:
///     (crypto, sha, 512)
///
/// Back:
///     (crypto, sha, 256)
/// ```
///
/// Since only the final dimension advanced:
///
/// ```text
/// crypto == crypto
/// sha    == sha
/// ```
///
/// the lineage is valid.
///
/// Conversely:
///
/// ```text
/// Current:
///     (crypto, sha, 512)
///
/// Back:
///     (image, sha, 256)
/// ```
///
/// is invalid because a parent identifier changed while traversing the
/// same semantic lineage.
///
/// ## Recursive Validation
///
/// Non-zeroth instances recursively evaluate the predecessor's
/// [`CounterIdentExpectedHashChecker`].
///
/// Consequently, successful validation of an instance implies that
/// identifier-lineage consistency holds for the entire predecessor
/// chain back to the global-minimum instance.
///
/// ## Why hashes are used
///
/// Comparing identifier hashes instead of identifier strings allows the
/// invariant to be evaluated entirely in const contexts while avoiding
/// repeated string comparisons.
///
/// The generated constant performs a compile-time assertion and emits a
/// panic when parent identifier lineage is inconsistent.
///
/// This guarantees that semantic predecessor relationships preserve the
/// identifier hierarchy expected by the instance graph.
#[derive(Debug, Clone)]
pub(super) struct ImplCounterIdentExpectedHashChecker;

impl<'a> Extension<ImplItemConst, ItemImpl, CounterArgsSlice<'a>>
    for ImplCounterIdentExpectedHashChecker
{
    fn raw_extend(
        &self,
        towards: &ItemImpl,
        context: &CounterArgsSlice<'a>,
    ) -> Result<ImplItemConst, TokenStream> {
        let ident = gen_const_ident::<CounterIdentExpectedHashChecker>();
        let expr = counter_expected_ident_hash_checker_expr(towards, context)?;
        let ty = parse_quote!(());
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
        let ident = gen_const_ident::<CounterIdentExpectedHashChecker>();
        let expr = counter_expected_ident_hash_checker_expr(towards, counters)?;
        let ty: Type = parse_quote!(());

        validate_impl_const! {
            item: item,
            towards: towards,
            ident: ident,
            ty: ty,
            expr: expr,
            errors: {
                not_found: IdentBugs::ImplCounterIdentExpectedHashCheckerNotFound {},
                wrong_ident: IdentBugs::ImplCounterIdentExpectedHashCheckerWrongIdent {},
                wrong_ty: IdentBugs::ImplCounterIdentExpectedHashCheckerNotUnitType {},
                has_generics: IdentBugs::ImplCounterIdentExpectedHashCheckerHasGenerics {},
                invalid_expr: IdentBugs::ImplCounterIdentExpectedHashCheckerInvalidExpr {},
            }
        }
    }
}

/// Generates the compile-time parent-lineage consistency assertion.
///
/// For every counter dimension whose value is non-zero:
///
/// ```text
/// Current[i] > 0
/// ```
///
/// all parent identifier hashes:
///
/// ```text
/// Current[0..i)
/// ```
///
/// must match the corresponding hashes of:
///
/// ```text
/// Back(Current)
/// ```
///
/// Non-zeroth instances additionally recurse into the predecessor's
/// [`CounterIdentExpectedHashChecker`].
///
/// Otherwise compilation fails with a lineage-consistency panic.
fn counter_expected_ident_hash_checker_expr<'a>(
    impl_of: &ItemImpl,
    counters: CounterArgsSlice<'a>,
) -> Result<Expr, TokenStream> {
    if is_zeroth_instance(counters)? {
        return Ok(parse_quote!(()));
    };

    let mut collect = Vec::new();
    for c in counters {
        let lit = &c.const_lit;
        collect.push(lit)
    }
    let counters_slice_expr: Expr = parse_quote!(&[#(#collect),*]);

    let mut current_hashes = Vec::new();
    let mut back_hashes = Vec::new();

    for c in counters {
        let hash_ident = gen_const_ident_with_suffix::<CounterIdentHash>(Some(
            c.generic_index.to_string().as_bytes(),
        ));

        let current_hash_expr = AssocTyExpr::checked_extract(&(impl_of, &hash_ident), &())?.0;
        let back_hash_expr = AssocOfAffiliateTyExpr::<BackAffiliateInstance, BackAffiliateCounters>::checked_extract(&(impl_of, &hash_ident), &counters)?.0;

        current_hashes.push(current_hash_expr);
        back_hashes.push(back_hash_expr);
    }

    let current_hashes_expr: Expr = parse_quote!(&[#(#current_hashes),*]);
    let back_hashes_expr: Expr = parse_quote!(&[#(#back_hashes),*]);

    let self_ident = gen_const_ident::<CounterIdentExpectedHashChecker>();
    let back_checker =
        AssocOfAffiliateTyExpr::<BackAffiliateInstance, BackAffiliateCounters>::checked_extract(
            &(impl_of, &self_ident),
            &counters,
        )?
        .0;

    let crate_of = Instance::support_crate();

    let err = IdentErrors::ParentCountersIdentsNotConsistent {}.to_string();

    if is_zeroth_instance(counters)? {
        let expr = parse_quote!(
            {
                if !#crate_of::counters_hash_consistency_checker(
                        #counters_slice_expr,
                        #current_hashes_expr,
                        #back_hashes_expr,
                ) {
                    panic!(#err)
                } else {
                    ()
                }
            }
        );
        return Ok(expr);
    }

    let expr = parse_quote!(
        {
            if !#crate_of::counters_hash_consistency_checker(
                    #counters_slice_expr,
                    #current_hashes_expr,
                    #back_hashes_expr,
            ) {
                panic!(#err)
            } else {
                let _ = #back_checker;
                ()
            }
        }
    );
    Ok(expr)
}
