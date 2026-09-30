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
// ````````````````````````````` INSTANCE IDENTIFIERS ````````````````````````````
// ===============================================================================

//! Instance identifier metadata generation and validation.
//!
//! This module implements the identifier-reflection pipeline used by the
//! instance-trait expansion system.
//!
//! ## Why this exists
//!
//! Instance counters uniquely identify coordinates within an instance
//! hierarchy:
//!
//! ```text
//! (Category, Algorithm)
//!
//! ("crypto", "sha-256") -> (0, 0)
//! ("crypto", "sha-512") -> (0, 1)
//! ("image",  "png")     -> (1, 0)
//! ("image",  "jpeg")    -> (1, 1)
//! ("image",  "gif")     -> (1, 2)
//! ```
//!
//! Counter values identify position, but they do not describe meaning.
//!
//! For example:
//!
//! ```text
//! (0, 1)
//! ```
//!
//! identifies a coordinate, but not the semantic instance:
//!
//! ```text
//! ("crypto", "sha-512")
//! ```
//!
//! To bridge this gap, each counter value is assigned a user-provided
//! ASCII identifier. These identifiers become the semantic identity
//! layer of the instance system and are reflected into hidden trait
//! metadata for later expansion and validation.
//!
//! ## Identifier Model
//!
//! Every counter dimension exposes a semantic naming space:
//!
//! ```text
//! Counter 0
//!
//! 0 => "crypto"
//! 1 => "image"
//!
//! Counter 1
//!
//! 0 => "sha-256"
//! 1 => "sha-512"
//! 0 => "png"
//! 0 => "jpeg"
//! 0 => "gif"
//! ```
//!
//! This allows expansion stages to reason about instance meaning rather
//! than raw counter coordinates.
//!
//! Alongside the ASCII representation, a deterministic `u64` hash is
//! generated for each identifier. Hashes provide a compact fixed-size
//! representation suitable for uniqueness validation, collection
//! comparison, affiliate traversal, and historical reconstruction.
//!
//! ## Generated Metadata
//!
//! Starting from ASCII identifiers, the pipeline generates:
//!
//! - identifier reflection,
//! - identifier hashes,
//! - current identifier collections,
//! - historical identifier collections,
//! - collection cardinalities,
//! - typenum cardinalities,
//! - and affiliate projections.
//!
//! ```text
//! Identifier
//!
//!     |
//!     +-- Hash
//!     |
//!     +-- Collection Lengths
//!     |       |
//!     |       +-- usize
//!     |       +-- TypeNum
//!     |
//!     +-- Current Collections
//!     |       |
//!     |       +-- Identifier
//!     |       +-- Hash
//!     |
//!     +-- Historical Collections
//!             |
//!             +-- Identifier
//!             +-- Hash
//! ```
//!
//! ## Collection Models
//!
//! Current collections describe the identifiers visible from the current
//! coordinate.
//!
//! Example:
//!
//! ```text
//! ("crypto", "sha-512")
//!
//! Counter 0:
//!     ["crypto"]
//!
//! Counter 1:
//!     ["sha-256", "sha-512"]
//! ```
//!
//! Historical collections preserve all previously reachable hierarchy
//! state.
//!
//! Example:
//!
//! ```text
//! ("image", "gif")
//!
//! Counter 0:
//!     ["crypto", "image"]
//!
//! Counter 1:
//!     [
//!         ["sha-256", "sha-512", PAD],
//!         ["png", "jpeg", "gif"]
//!     ]
//! ```
//!
//! Historical metadata therefore preserves:
//!
//! - previously completed parent coordinates,
//! - their child histories,
//! - and the current coordinate.
//!
//! Padded cardinalities ensure that all hierarchy branches can be
//! represented within a common multidimensional layout.
//!
//! ## Type-Level Metadata
//!
//! Collection cardinalities are reflected both as constants and as
//! typenum-associated types.
//!
//! The typenum representation enables:
//!
//! - fixed-size `GenericArray` collections,
//! - compile-time dimension checking,
//! - nested historical layouts,
//! - and type-level validation contracts.
//!
//! ## Expansion Model
//!
//! Trait-side expansion defines identifier metadata contracts.
//! Implementation-side expansion supplies identifier values,
//! collections, historical data, cardinalities, and affiliate-derived
//! metadata. Validation then ensures both sides remain synchronized.
//!
//! ## Design Goal
//!
//! The primary purpose of this metadata is to ensure that instances
//! carry a semantic identity rather than being represented solely by
//! counter coordinates.
//!
//! Identifiers describe what an instance represents, while hashes,
//! collections, and historical views provide verifiable representations
//! of that identity.
//!
//! Collection metadata additionally exposes the surrounding identifier
//! space of any instance coordinate. Consequently, from a single
//! instance, expansion stages can recover:
//!
//! - instance meaning,
//! - neighboring instance identities,
//! - hierarchy history,
//! - identity uniqueness,
//! - and the underlying counter structure that produced them.
//!
//! In this sense, identifier collections act as semantic projections of
//! the counter hierarchy, allowing proc-macro expansion to reason about
//! instances through meaning rather than raw indexes.

// ===============================================================================
// ``````````````````````````````````` IMPORTS ```````````````````````````````````
// ===============================================================================

// --- Proc-Macro crates ---
use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident};
use syn::{
    Expr, Ident, ItemTrait, LitInt, TraitBound, TraitItem, TraitItemConst, TraitItemType, Type,
    TypeParamBound, parse_quote, punctuated::Punctuated, token::Plus,
};

// --- Local Crate ---
use crate::{
    Extension, Extraction, Insertion, Instance, Transformation,
    traits::{
        counters::*,
        errors::{AffiliateBugs, CounterParamBugs, IdentItemsExpansionErrors, IdentTraitItemsBugs},
        utils::*,
    },
};

// --- Proc-suite ---
use proc_suite::{
    SupportCrate,
    docs::{DocAttr, InsertDocs},
    misc::*,
};

// ===============================================================================
// `````````````````````````````````` IDENTS SET `````````````````````````````````
// ===============================================================================

/// Aggregates all instance-identifier associated types and constants generation
/// and validation.
///
/// This transformation serves as the central entry point for the
/// identifier-items generation and reflection pipeline. It installs the
/// complete set of identifier, identifier-hash, collection, collection-length,
/// and historical identifier metadata required by the instance system.
///
/// The generated metadata progresses through several layers:
///
/// - [`CounterIdent`]
///     User-provided ASCII identifiers for each counter dimension.
///
/// - [`IndexedCounterIdent`]
///     Index-addressable reflection of counter identifiers.
///
/// - [`CounterIdentHash`]
///     Deterministic `u64` hashes of reflected identifiers.
///
/// - [`CounterIdentCollectionLenTypeNum`]
///     Type-level collection cardinalities derived from
///     [`InstanceTraitTypeNumCounters`].
///
/// - [`CounterIdentCollectionGenArray`]
///     Fixed-size typenum-backed identifier collections.
///
/// - [`CounterIdentHashCollectionGenArray`]
///     Fixed-size typenum-backed identifier-hash collections.
///
/// - [`CounterIdentHistoricalCollectionLenTypeNum`]
///     Type-level padded historical collection cardinalities.
///
/// - [`CounterIdentHistoricalCollectionGenArray`]
///     Fixed-size typenum-backed historical identifier collections.
///
/// - [`CounterIdentHashHistoricalCollectionGenArray`]
///     Fixed-size typenum-backed historical identifier-hash collections.
///
/// - [`CounterIdentExpectedHashChecker`]
///     Parent Counter identifiers consistency validator (const-assertion)
///
/// Together these metadata layers provide a complete compile-time model
/// of identifier state, current collections, historical collections,
/// hash representations, and collection cardinalities for every counter
/// dimension and affiliate projection.
#[derive(Debug, Clone)]
pub(crate) struct InstanceTraitIdents;

impl<'a> Transformation<ItemTrait, CounterParamsSlice<'a>> for InstanceTraitIdents {
    fn raw_transform(
        &self,
        transform: &mut ItemTrait,
        context: &CounterParamsSlice<'a>,
    ) -> Result<(), TokenStream> {
        CounterIdent::checked_extend(&CounterIdent, transform, context)?;
        IndexedCounterIdent::checked_extend(&IndexedCounterIdent, transform, context)?;
        CounterIdentHash::checked_extend(&CounterIdentHash, transform, context)?;
        CounterIdentCollectionLenTypeNum::checked_transform(
            &CounterIdentCollectionLenTypeNum,
            transform,
            context,
        )?;
        CounterIdentCollectionGenArray::checked_extend(
            &CounterIdentCollectionGenArray,
            transform,
            context,
        )?;
        CounterIdentHashCollectionGenArray::checked_extend(
            &CounterIdentHashCollectionGenArray,
            transform,
            context,
        )?;
        CounterIdentHistoricalCollectionLenTypeNum::checked_extend(
            &CounterIdentHistoricalCollectionLenTypeNum,
            transform,
            context,
        )?;
        CounterIdentHistoricalCollectionGenArray::checked_extend(
            &CounterIdentHistoricalCollectionGenArray,
            transform,
            context,
        )?;
        CounterIdentHashHistoricalCollectionGenArray::checked_extend(
            &CounterIdentHashHistoricalCollectionGenArray,
            transform,
            context,
        )?;
        CounterIdentExpectedHashChecker::checked_extend(
            &CounterIdentExpectedHashChecker,
            transform,
            &(),
        )?;
        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &ItemTrait,
        context: Option<&CounterParamsSlice<'a>>,
    ) -> Result<(), TokenStream> {
        CounterIdent::validate_extend(&CounterIdent, None, transform, context)?;
        IndexedCounterIdent::validate_extend(&IndexedCounterIdent, None, transform, context)?;
        CounterIdentHash::validate_extend(&CounterIdentHash, None, transform, context)?;
        CounterIdentCollectionLenTypeNum::validate_transform(
            &CounterIdentCollectionLenTypeNum,
            transform,
            context,
        )?;
        CounterIdentCollectionGenArray::validate_extend(
            &CounterIdentCollectionGenArray,
            None,
            transform,
            context,
        )?;
        CounterIdentHashCollectionGenArray::validate_extend(
            &CounterIdentHashCollectionGenArray,
            None,
            transform,
            context,
        )?;
        CounterIdentHistoricalCollectionLenTypeNum::validate_extend(
            &CounterIdentHistoricalCollectionLenTypeNum,
            None,
            transform,
            context,
        )?;
        CounterIdentHistoricalCollectionGenArray::validate_extend(
            &CounterIdentHistoricalCollectionGenArray,
            None,
            transform,
            context,
        )?;
        CounterIdentHashHistoricalCollectionGenArray::validate_extend(
            &CounterIdentHashHistoricalCollectionGenArray,
            None,
            transform,
            context,
        )?;
        CounterIdentExpectedHashChecker::validate_extend(
            &CounterIdentExpectedHashChecker,
            None,
            transform,
            None,
        )?;
        Ok(())
    }
}

// ===============================================================================
// ````````````````````````````` COUNTER ASCII IDENT `````````````````````````````
// ===============================================================================

/// Declares user-provided identifiers for instance counters.
///
/// Multiple implementations of the same instance trait may share the
/// exact same trait contract while representing semantically distinct
/// instances. Although counters uniquely identify those instances,
/// numeric counter values are often difficult to read and reason about,
/// especially for multi-counter instance spaces.
///
/// This metadata allows implementors to associate a human-readable
/// identifier with each counter dimension. Later expansion stages use
/// these identifiers to construct deterministic instance metadata,
/// hashes, collections, and historical identifier views.
///
/// ## Example
///
/// Given:
///
/// ```ignore
/// #[instance]
/// trait Service<const REGION: u8, const SHARD: u8> {
///     const REGION_IDENT: &'static [u8];
///     const SHARD_IDENT: &'static [u8];
/// }
/// ```
///
/// an implementation may provide:
///
/// ```ignore
/// #[impl_instance]
/// impl Service<...> for MyType {
///     const REGION_IDENT: &'static [u8] = b"us-east";
///     const SHARD_IDENT: &'static [u8] = b"payments";
/// }
/// ```
///
/// allowing the instance to be identified by the semantic tuple:
///
/// ```text
/// ("us-east", "payments")
/// ```
///
/// rather than only by its underlying counter values.
#[derive(Clone, Debug)]
pub(crate) struct CounterIdent;

pub(crate) const INSTANCE_COUNTER_IDENT: &'static str = "IDENT";

impl<'a> Extension<Vec<TraitItemConst>, ItemTrait, CounterParamsSlice<'a>> for CounterIdent {
    fn raw_extend(
        &self,
        towards: &ItemTrait,
        context: &CounterParamsSlice<'a>,
    ) -> Result<Vec<TraitItemConst>, TokenStream> {
        let mut collect = Vec::new();
        for c in context.iter().map(|c| &c.const_param.ident) {
            let ident = format_ident!("{c}_{INSTANCE_COUNTER_IDENT}");
            let mut item = TraitItemConst {
                attrs: vec![parse_quote!(#[allow(dead_code)])],
                ident,
                default: None,
                ty: parse_quote!(&'static [u8]),
                const_token: Default::default(),
                generics: Default::default(),
                colon_token: Default::default(),
                semi_token: Default::default(),
            };
            let trait_ident = &towards.ident;
            item.insert_docs(vec![
                DocAttr::Raw("Instance Counter ".to_string()),
                DocAttr::Inline("const".to_string()),
                DocAttr::Raw(" generic param ".to_string()),
                DocAttr::Inline(format!("{c}")),
                DocAttr::Raw(" ASCII identifier for trait ".to_string()),
                DocAttr::Ref(format!("{trait_ident}")),
            ]);
            collect.push(item);
        }
        Ok(collect)
    }

    fn validate_extend(
        &self,
        item: Option<&Vec<TraitItemConst>>,
        towards: &ItemTrait,
        context: Option<&CounterParamsSlice<'a>>,
    ) -> Result<(), TokenStream> {
        let extracted;
        let counters = match context {
            Some(c) => c,
            None => {
                extracted = CounterParams::checked_extract(towards, &None)?;
                extracted.as_slice().as_ref().as_ref()
            }
        };
        let exp_ty: Type = parse_quote!(&'static [u8]);
        for c in counters.iter().map(|c| &c.const_param.ident) {
            let ident = format_ident!("{c}_{INSTANCE_COUNTER_IDENT}");
            validate_trait_consts! {
                items: item,
                towards: towards,
                ident: ident,
                ty: exp_ty,
                expr: None => {
                    not_found: IdentTraitItemsBugs::CounterIdentNotFound {},
                    wrong_ident: IdentTraitItemsBugs::CounterIdentWrongIdent {},
                    wrong_ty: IdentTraitItemsBugs::CounterIdentWrongType {},
                    has_generics: IdentTraitItemsBugs::CounterIdentHasGenerics {},
                    has_expr: IdentTraitItemsBugs::CounterIdentHasExpr {},
                }
            }
        }
        Ok(())
    }
}

// ===============================================================================
// `````````````````````` COUNTER INDEXED ASCII IDENT (META) `````````````````````
// ===============================================================================

/// Reflects counter identifiers by generic index.
///
/// Counter generic identifiers exist only on the trait declaration.
/// After counter extraction and transformation, impl-side expansion
/// cannot reliably recover the original generic names because proc
/// macros operate on independent syntax trees and do not share semantic
/// information.
///
/// This metadata creates a hidden index-addressable view of every
/// user-provided counter identifier by mapping each counter's generic
/// index to its corresponding [`CounterIdent`] constant.
///
/// ## Example
///
/// Given:
///
/// ```ignore
/// #[instance]
/// trait Service<const REGION: u8, const SHARD: u8> {
///     const REGION_IDENT: &'static [u8];
///     const SHARD_IDENT: &'static [u8];
/// }
/// ```
///
/// the generated metadata is conceptually equivalent to:
///
/// ```ignore
/// #[instance]
/// trait Service<...> {
///     #[doc(hidden)]
///     const __INDEX_0_IDENT: &'static [u8] = Self::REGION_IDENT;
///
///     #[doc(hidden)]
///     const __INDEX_1_IDENT: &'static [u8] = Self::SHARD_IDENT;
/// }
/// ```
///
/// Later expansion stages can therefore reference identifiers through
/// generic indexes without needing access to the original counter
/// generic names (especially impl side).
///
/// The generated identifier is deterministically derived from this phase
/// type itself, making it stable while remaining hidden from external
/// crate APIs.
#[derive(Clone, Debug)]
pub(crate) struct IndexedCounterIdent;

impl<'a> Extension<Vec<TraitItemConst>, ItemTrait, CounterParamsSlice<'a>> for IndexedCounterIdent {
    fn raw_extend(
        &self,
        _towards: &ItemTrait,
        context: &CounterParamsSlice<'a>,
    ) -> Result<Vec<TraitItemConst>, TokenStream> {
        let mut collect = Vec::new();
        for (i, c) in context
            .iter()
            .map(|c| (&c.generic_index, &c.const_param.ident))
        {
            let ident = gen_const_ident_with_suffix::<Self>(Some(i.to_string().as_bytes()));
            let orig = format_ident!("{c}_{INSTANCE_COUNTER_IDENT}");
            let item = TraitItemConst {
                attrs: proc_suite::internal_code(),
                ident,
                default: Some((Default::default(), parse_quote!(Self::#orig))),
                ty: parse_quote!(&'static [u8]),
                const_token: Default::default(),
                generics: Default::default(),
                colon_token: Default::default(),
                semi_token: Default::default(),
            };
            collect.push(item);
        }
        Ok(collect)
    }

    fn validate_extend(
        &self,
        item: Option<&Vec<TraitItemConst>>,
        towards: &ItemTrait,
        context: Option<&CounterParamsSlice<'a>>,
    ) -> Result<(), TokenStream> {
        let extracted;
        let counters = match context {
            Some(c) => c,
            None => {
                extracted = CounterParams::checked_extract(towards, &None)?;
                extracted.as_slice().as_ref().as_ref()
            }
        };
        for (i, c) in counters
            .iter()
            .map(|c| (&c.generic_index, &c.const_param.ident))
        {
            let ident = gen_const_ident_with_suffix::<Self>(Some(i.to_string().as_bytes()));
            let orig = format_ident!("{c}_{INSTANCE_COUNTER_IDENT}");
            let exp_ty: Type = parse_quote!(&'static [u8]);
            let exp_expr: Expr = parse_quote!(Self::#orig);
            validate_trait_consts! {
                items: item,
                towards: towards,
                ident: ident,
                ty: exp_ty,
                expr: Some(exp_expr) => {
                    not_found: IdentTraitItemsBugs::IndexedCounterIdentNotFound {},
                    wrong_ident: IdentTraitItemsBugs::IndexedCounterIdentWrongIdent {},
                    wrong_ty: IdentTraitItemsBugs::IndexedCounterIdentWrongType {},
                    has_generics: IdentTraitItemsBugs::IndexedCounterIdentHasGenerics {},
                    missing_expr: IdentTraitItemsBugs::IndexedCounterIdentExprNotFound {},
                    invalid_expr: IdentTraitItemsBugs::IndexedCounterIdentInvalidExpr {},
                }
            }
        }
        Ok(())
    }
}

// ===============================================================================
// `````````````````````````````` COUNTER IDENT HASH `````````````````````````````
// ===============================================================================

/// Computes deterministic hashes for reflected counter identifiers.
///
/// Counter identifiers are user-provided byte slices whose lengths are not
/// known at compile time. While this flexibility makes identifiers easy to
/// author, it also makes direct const-time comparison and validation
/// cumbersome.
///
/// This metadata converts every reflected counter identifier into a fixed
/// width `u64` hash, allowing later expansion stages to perform equality,
/// uniqueness, and historical consistency checks entirely within const
/// contexts.
///
/// ## Why this exists
///
/// Instance identifiers form a multidimensional namespace. Identifiers that
/// belong to the same parent coordinate are expected to share the same
/// identifier history for all preceding dimensions, while identifiers within
/// the same dimension must remain distinct.
///
/// For example:
///
/// ```text
/// ("crypto", "sha-256")
/// ("crypto", "sha-512")
/// ```
///
/// Here:
///
/// - `"crypto"` must be identical because both instances belong to the same
///   first dimension,
/// - `"sha-256"` and `"sha-512"` must be distinct because they identify
///   different second-dimension instances.
///
/// These relationships are validated through hashed identifiers rather than
/// variable-length byte slices.
///
/// ## Generated Metadata
///
/// For each counter identifier:
///
/// ```ignore
/// #[instance]
/// trait Service<const REGION: u8, const SHARD: u8> {
///     const REGION_IDENT: &'static [u8];
///     const SHARD_IDENT: &'static [u8];
/// }
/// ```
///
/// a hidden hash constant is generated:
///
/// ```ignore
/// #[instance]
/// trait Service<...> {
///     #[doc(hidden)]
///     const __INDEX_0_HASH: u64 = hash(REGION_IDENT);
///
///     #[doc(hidden)]
///     const __INDEX_1_HASH: u64 = hash(SHARD_IDENT);
/// }
/// ```
///
/// allowing subsequent metadata generation and validation to operate on
/// fixed-size values.
#[derive(Clone, Debug)]
pub(crate) struct CounterIdentHash;

impl<'a> Extension<Vec<TraitItemConst>, ItemTrait, CounterParamsSlice<'a>> for CounterIdentHash {
    fn raw_extend(
        &self,
        towards: &ItemTrait,
        context: &CounterParamsSlice<'a>,
    ) -> Result<Vec<TraitItemConst>, TokenStream> {
        let mut collect = Vec::new();
        for (i, c) in context
            .iter()
            .map(|c| (&c.generic_index, &c.const_param.ident))
        {
            let ident = gen_const_ident_with_suffix::<Self>(Some(i.to_string().as_bytes()));
            let indexed_ident =
                gen_const_ident_with_suffix::<IndexedCounterIdent>(Some(i.to_string().as_bytes()));
            let counter_ident = format_ident!("{c}_{INSTANCE_COUNTER_IDENT}");
            let trait_ident = &towards.ident;
            let expr = ident_hash_expr(&counter_ident, &indexed_ident, trait_ident);
            let item = TraitItemConst {
                attrs: proc_suite::internal_code(),
                ident,
                default: Some((Default::default(), expr)),
                ty: parse_quote!(u64),
                const_token: Default::default(),
                generics: Default::default(),
                colon_token: Default::default(),
                semi_token: Default::default(),
            };
            collect.push(item);
        }
        Ok(collect)
    }

    fn validate_extend(
        &self,
        item: Option<&Vec<TraitItemConst>>,
        towards: &ItemTrait,
        context: Option<&CounterParamsSlice<'a>>,
    ) -> Result<(), TokenStream> {
        let extracted;
        let counters = match context {
            Some(c) => c,
            None => {
                extracted = CounterParams::checked_extract(towards, &None)?;
                extracted.as_slice().as_ref().as_ref()
            }
        };
        for (i, c) in counters
            .iter()
            .map(|c| (&c.generic_index, &c.const_param.ident))
        {
            let ident = gen_const_ident_with_suffix::<Self>(Some(i.to_string().as_bytes()));
            let indexed_ident =
                gen_const_ident_with_suffix::<IndexedCounterIdent>(Some(i.to_string().as_bytes()));
            let counter_ident = format_ident!("{c}_{INSTANCE_COUNTER_IDENT}");
            let trait_ident = &towards.ident;
            let exp_expr = ident_hash_expr(&counter_ident, &indexed_ident, trait_ident);
            let exp_ty: Type = parse_quote!(u64);
            validate_trait_consts! {
                items: item,
                towards: towards,
                ident: ident,
                ty: exp_ty,
                expr: Some(exp_expr) => {
                    not_found: IdentTraitItemsBugs::CounterIdentHashNotFound {},
                    wrong_ident: IdentTraitItemsBugs::CounterIdentHashWrongIdent {},
                    wrong_ty: IdentTraitItemsBugs::CounterIdentHashWrongType {},
                    has_generics: IdentTraitItemsBugs::CounterIdentHashHasGenerics {},
                    missing_expr: IdentTraitItemsBugs::CounterIdentHashExprNotFound {},
                    invalid_expr: IdentTraitItemsBugs::CounterIdentHashInvalidExpr {},
                }
            }
        }
        Ok(())
    }
}

/// Generates the const expression used to compute a counter-identifier hash.
///
/// The generated expression validates that the reflected identifier is
/// non-empty before hashing it into a fixed-size `u64` value.
fn ident_hash_expr(counter_ident: &Ident, indexed_ident: &Ident, trait_ident: &Ident) -> Expr {
    let crate_of = Instance::support_crate();
    let error = IdentItemsExpansionErrors::CounterIdentHashNullIdentProvided {
        counter: counter_ident.clone(),
        trait_of: trait_ident.clone(),
    }
    .to_string();
    parse_quote! {{
        if Self::#indexed_ident.is_empty() {
            panic!(#error);
        } else {
            #crate_of::hash_ident(Self::#indexed_ident)
        }
    }}
}

// ===============================================================================
// `````````````````` COUNTER IDENT COLLECTION LENGTH (TYPENUM) ``````````````````
// ===============================================================================

/// Declares hidden associated types representing identifier-collection
/// lengths for each counter dimension.
///
/// Every extracted counter dimension contributes one hidden associated
/// type whose value represents the compile-time length of the identifier
/// collection visible at the current instance.
///
/// Identifier collections are instance-relative and parent-scoped. Each
/// collection contains every identifier observed within the current
/// parent coordinate from the first instance of a dimension up to and
/// including the current instance.
///
/// ## Example
///
/// Consider:
///
/// ```text
/// (CATEGORY, ALGORITHM)
///
/// ("crypto", "sha-256") -> (0, 0)
/// ("crypto", "sha-512") -> (0, 1)
/// ("crypto", "blake3")  -> (0, 2)
///
/// ("image",  "png")     -> (1, 0)
/// ("image",  "jpeg")    -> (1, 1)
/// ```
///
/// At:
///
/// ```text
/// ("crypto", "sha-512") -> (0, 1)
/// ```
///
/// the algorithm collection is:
///
/// ```text
/// ["sha-256", "sha-512"]
/// ```
///
/// while at:
///
/// ```text
/// ("image", "jpeg") -> (1, 1)
/// ```
///
/// it becomes:
///
/// ```text
/// ["png", "jpeg"]
/// ```
///
/// since collections are scoped to their local parent coordinate rather
/// than the entire instance space.
///
/// ## Generated metadata
///
/// One hidden associated type is generated for every extracted counter:
///
/// ```ignore
/// #[instance]
/// trait Service<__TypeNumCounters> {
///     #[doc(hidden)]
///     type __INDEX_0_COLLECTION_LEN: ArrayLength;
///
///     #[doc(hidden)]
///     type __INDEX_1_COLLECTION_LEN: ArrayLength;
/// }
/// ```
///
/// These associated types act as compile-time placeholders for the
/// cardinality of each identifier collection. Later transformation
/// phases resolve them to concrete array lengths when materializing
/// identifier collections, identifier-hash collections, and related
/// metadata.
#[derive(Clone, Debug)]
pub(crate) struct CounterIdentCollectionLenTypeNum;

impl<'a> Insertion<ItemTrait, CounterParamsSlice<'a>> for Vec<TraitItemType> {
    fn raw_insert(
        &self,
        to: &mut ItemTrait,
        _: &CounterParamsSlice<'a>,
    ) -> Result<(), TokenStream> {
        for t in self {
            to.items.push(TraitItem::Type(t.clone()));
        }
        Ok(())
    }

    fn validate_inserted(
        _: Option<&Self>,
        _: &ItemTrait,
        _: Option<&CounterParamsSlice<'a>>,
    ) -> Result<(), TokenStream> {
        Ok(())
    }
}

impl<'a> Transformation<ItemTrait, CounterParamsSlice<'a>> for CounterIdentCollectionLenTypeNum {
    fn raw_transform(
        &self,
        transform: &mut ItemTrait,
        context: &CounterParamsSlice<'a>,
    ) -> Result<(), proc_macro2::TokenStream> {
        Self::checked_extend(&self, transform, context)?;
        AllAffiliateCountersCollectionLenBounds::checked_transform(
            &AllAffiliateCountersCollectionLenBounds,
            transform,
            context,
        )?;
        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &ItemTrait,
        context: Option<&CounterParamsSlice<'a>>,
    ) -> Result<(), proc_macro2::TokenStream> {
        Self::validate_extend(&self, None, transform, context)?;
        AllAffiliateCountersCollectionLenBounds::validate_transform(
            &AllAffiliateCountersCollectionLenBounds,
            transform,
            context,
        )?;
        Ok(())
    }
}

impl<'a> Extension<Vec<TraitItemType>, ItemTrait, CounterParamsSlice<'a>>
    for CounterIdentCollectionLenTypeNum
{
    fn raw_extend(
        &self,
        _towards: &ItemTrait,
        context: &CounterParamsSlice<'a>,
    ) -> Result<Vec<TraitItemType>, TokenStream> {
        let mut collect = Vec::new();
        for index in context.iter().map(|c| &c.generic_index) {
            let mut bounds = Punctuated::<TypeParamBound, Plus>::new();
            let crate_of = Instance::support_crate();
            let array_len_bound = parse_quote!(#crate_of::ArrayLength);
            bounds.push(TypeParamBound::Trait(array_len_bound));

            let ident = gen_type_ident_with_suffix::<Self>(Some(index.to_string().as_bytes()));
            let item = TraitItemType {
                attrs: proc_suite::internal_code(),
                ident,
                default: None,
                type_token: Default::default(),
                generics: Default::default(),
                colon_token: Default::default(),
                bounds,
                semi_token: Default::default(),
            };
            collect.push(item);
        }
        Ok(collect)
    }

    fn validate_extend(
        &self,
        item: Option<&Vec<TraitItemType>>,
        towards: &ItemTrait,
        context: Option<&CounterParamsSlice<'a>>,
    ) -> Result<(), TokenStream> {
        let extracted;
        let counters = match context {
            Some(c) => c,
            None => {
                extracted = CounterParams::checked_extract(towards, &None)?;
                extracted.as_slice().as_ref().as_ref()
            }
        };
        for gen_idx in counters.iter().map(|c| &c.generic_index) {
            let ident = gen_type_ident_with_suffix::<Self>(Some(gen_idx.to_string().as_bytes()));

            let crate_of = Instance::support_crate();
            let array_len_bound = parse_quote!(#crate_of::ArrayLength);

            let bounds = &[array_len_bound];

            validate_trait_types! {
                items: item,
                towards: towards,
                ident: ident,
                bounds: bounds => {
                    not_found: IdentTraitItemsBugs::CounterIdentCollectionLenTypeNumNotFound {},
                    wrong_ident: IdentTraitItemsBugs::CounterIdentCollectionLenTypeNumWrongIdent {},
                    has_generics: IdentTraitItemsBugs::CounterIdentCollectionLenTypeNumHasGenerics {},
                    bound_not_found: IdentTraitItemsBugs::CounterIdentCollectionLenTypeNumBoundNotFound {},
                }
            }
        }
        Ok(())
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ``````````````` ALL AFFILIATE COUNTERS COLLECTION LENGTH BOUNDS ```````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// Ensures all affiliate counter projections can derive identifier-
/// collection lengths for every counter dimension.
///
/// Identifier-collection metadata is generated not only for the current
/// instance represented by [`InstanceTraitTypeNumCounters`], but also for every
/// affiliate instance reachable through the affiliate graph.
///
/// Affiliates represent alternative counter coordinates describing
/// structurally or semantically related trait instances.
///
/// Examples include:
///
/// - reverse affiliates,
/// - ceil affiliates,
/// - next affiliates,
/// - back affiliates,
/// - and floor affiliates.
///
/// Each affiliate exposes its own affiliated counter tuple through a
/// generated associated type.
///
/// ## Why this exists
///
/// Collection-length metadata is derived through the support crate's
/// `CountersCollectionLen` trait:
///
/// ```ignore
/// <Counters as CountersCollectionLen<Ui>>::Output
/// ```
///
/// The primary counter tuple already receives these bounds through
/// [`CounterTypeNumCollectionLenBounds`].
///
/// However, affiliate counter tuples participate in the same identifier
/// reflection, identifier collection, identifier-hash collection, and
/// historical metadata generation pipeline.
///
/// Consequently, every affiliate counter projection must also be capable
/// of computing collection cardinalities for every counter dimension.
///
/// ## Generated Bounds
///
/// For a trait with two counters:
///
/// ```ignore
/// #[instance]
/// trait Service<const REGION: u8, const SHARD: u8> {}
/// ```
///
/// and an affiliate counter projection (may-have its own bounds):
///
/// ```ignore
/// type NextAffiliateCounters : .... ;
/// ```
///
/// the generated metadata is conceptually equivalent to:
///
/// ```ignore
/// type NextAffiliateCounters:
///     CountersCollectionLen<U0>
///     + CountersCollectionLen<U1> + ....;
/// ```
///
/// The same bounds are propagated (added) to every generated affiliate counter
/// projection.
///
/// ## Relationship to Collection Metadata
///
/// These bounds allow affiliate counter tuples to participate in the
/// generation of:
///
/// - identifier collections,
/// - identifier-hash collections,
/// - historical identifier collections,
/// - historical identifier-hash collections,
/// - collection-length metadata,
/// - and typenum collection-length metadata.
///
/// Without these bounds, affiliate projections could not derive the
/// collection cardinalities required to materialize fixed-size metadata
/// collections.
///
/// ## Compile-Time Validation
///
/// The generated bounds act both as capabilities and invariants.
///
/// Every affiliate counter projection is required to provide
/// `CountersCollectionLen` implementations for every counter index. If an
/// affiliate counter type cannot derive collection cardinalities, trait
/// generation fails before identifier metadata is emitted.
///
/// This guarantees that the entire affiliate graph remains structurally
/// compatible with the identifier-reflection pipeline rather than only
/// the current instance represented by [`InstanceTraitTypeNumCounters`].
#[derive(Clone, Debug)]
struct AllAffiliateCountersCollectionLenBounds;

impl<'a> Transformation<ItemTrait, CounterParamsSlice<'a>>
    for AllAffiliateCountersCollectionLenBounds
{
    fn raw_transform(
        &self,
        transform: &mut ItemTrait,
        context: &CounterParamsSlice<'a>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let afl_counters = AllAffiliateCounterIdents::checked_extract(&context, &())?.0;
        let crate_of = Instance::support_crate();
        let mut collect = Punctuated::<TypeParamBound, Plus>::new();
        for (i, _) in context.iter().enumerate() {
            let lit_int: LitInt = parse_quote!(#i);
            let typenum_idx = format_ident!("U{}", lit_int.base10_digits());
            collect.push(parse_quote!(
                #crate_of::CountersCollectionLen<
                    #crate_of::#typenum_idx
                >
            ));
        }
        for ident in afl_counters {
            let mut found = false;
            for item in transform.items.iter_mut() {
                let TraitItem::Type(t) = item else {
                    continue;
                };
                if t.ident != ident {
                    continue;
                }
                t.bounds.extend(collect.clone());
                found = true
            }
            if !found {
                return Err(AffiliateBugs::AffiliateCounterNotFound {}.into());
            }
        }
        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &ItemTrait,
        context: Option<&CounterParamsSlice<'a>>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let extracted;
        let counters = match context {
            Some(c) => c,
            None => {
                extracted = CounterParams::checked_extract(transform, &None)?;
                extracted.as_slice().as_ref().as_ref()
            }
        };

        let afl_counters = AllAffiliateCounterIdents::checked_extract(&counters, &())?.0;
        let bounds = {
            let crate_of = Instance::support_crate();
            let mut collect = Vec::new();
            for (i, _) in context.iter().enumerate() {
                let lit_int: LitInt = parse_quote!(#i);
                let typenum_idx = format_ident!("U{}", lit_int.base10_digits());
                let bound: TraitBound = parse_quote!(
                    #crate_of::CountersCollectionLen<
                        #crate_of::#typenum_idx
                    >
                );
                collect.push(bound);
            }
            collect
        };
        let def_items: Option<&Vec<TraitItemType>> = None;
        for ident in afl_counters {
            validate_trait_types! {
                items: def_items,
                towards: transform,
                ident: ident,
                bounds: &bounds => {
                    not_found: IdentTraitItemsBugs::AllAffiliateCountersCollectionLenBoundsNotFound {},
                    wrong_ident: IdentTraitItemsBugs::AllAffiliateCountersCollectionLenBoundsWrongIdent {},
                    has_generics: IdentTraitItemsBugs::AllAffiliateCountersCollectionLenBoundsHasGenerics {},
                    bound_not_found: IdentTraitItemsBugs::AllAffiliateCountersCollectionLenBoundsBoundNotFound {},
                }
            }
        }
        Ok(())
    }
}

// ===============================================================================
// ```````````````````````````` COLLECTION UTILITIES `````````````````````````````
// ===============================================================================

macro_rules! validate_expect_collection_consts {
    (
        $self_ty:ty,
        $item:expr,
        $towards:expr,
        $context:expr,
        |$gen_idx:ident| $exp_ty:expr,
        {
            not_found: $not_found:expr,
            wrong_ident: $wrong_ident:expr,
            wrong_ty: $wrong_ty:expr,
            has_generics: $has_generics:expr,
            has_expr: $has_expr:expr,
        }
    ) => {{
        let extracted;

        let counters = match $context {
            Some(c) => c,
            None => {
                extracted = CounterParams::checked_extract($towards, &None)?;
                extracted.as_slice().as_ref().as_ref()
            }
        };

        for $gen_idx in counters.iter().map(|c| &c.generic_index) {
            let ident =
                gen_const_ident_with_suffix::<$self_ty>(Some($gen_idx.to_string().as_bytes()));

            let exp_ty: Type = $exp_ty;

            validate_trait_consts! {
                items: $item,
                towards: $towards,
                ident: ident,
                ty: exp_ty,
                expr: None => {
                    not_found: $not_found,
                    wrong_ident: $wrong_ident,
                    wrong_ty: $wrong_ty,
                    has_generics: $has_generics,
                    has_expr: $has_expr,
                }
            }
        }
    }};
}

fn collect_expect_collection_consts<'a, T: 'static, F>(
    indexes: impl Iterator<Item = &'a usize>,
    mut ty_for: F,
) -> Vec<TraitItemConst>
where
    F: FnMut(&usize) -> syn::Type,
{
    let mut collect = Vec::new();

    for gen_idx in indexes {
        let ident = gen_const_ident_with_suffix::<T>(Some(gen_idx.to_string().as_bytes()));

        let item = TraitItemConst {
            attrs: proc_suite::internal_code(),
            ident,
            default: None,
            ty: ty_for(gen_idx),
            const_token: Default::default(),
            generics: Default::default(),
            colon_token: Default::default(),
            semi_token: Default::default(),
        };

        collect.push(item);
    }

    collect
}

// ===============================================================================
// ```````````````````` COUNTER IDENT COLLECTION (GEN-ARRAY) `````````````````````
// ===============================================================================

/// Declares identifier collections as fixed-size typenum-backed generic
/// arrays.
///
/// This collection constant expose identifier collections visible
/// at the current instance.
///
/// - [`CounterIdentCollectionGenArray`]
///     Represents collections as fixed-size `GenericArray`s whose
///     cardinality is encoded directly in the type system.
///
/// ## Why this exists
///
/// Ideally, identifier collections would be represented as fixed-size
/// arrays whose lengths are derived directly from generated collection
/// metadata.
///
/// ```ignore
/// const __IDENT_COLLECTION_INDEX_0:
///     [&'static [u8]; Self::__COLLECTION_LEN_INDEX_0];
/// ```
///
/// However, expressing array lengths through generated associated
/// metadata requires unstable generic-const-expression support.
///
/// To remain fully compatible with stable Rust, collection cardinalities
/// are instead reflected through
/// [`CounterIdentCollectionLenTypeNum`] and materialized using
/// `GenericArray`.
///
/// This provides a fixed-size collection whose cardinality remains
/// compile-time verified without relying on nightly-only language
/// features.
///
/// ## Generated Metadata
///
/// For every counter dimension, a hidden associated constant is
/// generated.
///
/// ```ignore
/// #[instance]
/// trait Service<InstanceTraitTypeNumCounters> {
///     #[doc(hidden)]
///     const __INDEX_0_IDENT_COLLECTION_GEN_ARRAY:
///         GenericArray<
///             &'static [u8],
///             Self::__Index0CollectionLen
///         >;
///
///     #[doc(hidden)]
///     const __INDEX_1_IDENT_COLLECTION_GEN_ARRAY:
///         GenericArray<
///             &'static [u8],
///             Self::__Index1CollectionLen
///         >;
/// }
/// ```
///
/// where:
///
/// ```ignore
/// #[doc(hidden)]
/// type __Index0CollectionLen:
///     Exact<
///         <InstanceTraitTypeNumCounters as CountersCollectionLen<U0>>::Output
///     > + ArrayLength;
///
/// #[doc(hidden)]
/// type __Index1CollectionLen:
///     Exact<
///         <InstanceTraitTypeNumCounters as CountersCollectionLen<U1>>::Output
///     > + ArrayLength;
/// ```
///
/// The collection-length types are generated by
/// [`CounterIdentCollectionLenTypeNum`] and encode the exact
/// cardinality of the identifier collection visible at the current
/// instance.
///
/// The collection contents themselves are supplied by
/// implementation-side expansion and validated against the generated
/// collection-length metadata.
#[derive(Clone, Debug)]
pub(crate) struct CounterIdentCollectionGenArray;

impl<'a> Extension<Vec<TraitItemConst>, ItemTrait, CounterParamsSlice<'a>>
    for CounterIdentCollectionGenArray
{
    fn raw_extend(
        &self,
        _towards: &ItemTrait,
        context: &CounterParamsSlice<'a>,
    ) -> Result<Vec<TraitItemConst>, proc_macro2::TokenStream> {
        let collect = collect_expect_collection_consts::<Self, _>(
            context.iter().map(|c| &c.generic_index),
            |gen_idx| {
                let len = gen_type_ident_with_suffix::<CounterIdentCollectionLenTypeNum>(Some(
                    gen_idx.to_string().as_bytes(),
                ));
                let crate_of = Instance::support_crate();

                parse_quote!(#crate_of::GenericArray<&'static [u8], Self::#len>)
            },
        );

        Ok(collect)
    }

    fn validate_extend(
        &self,
        item: Option<&Vec<TraitItemConst>>,
        towards: &ItemTrait,
        context: Option<&CounterParamsSlice<'a>>,
    ) -> Result<(), proc_macro2::TokenStream> {
        validate_expect_collection_consts!(
            Self,
            item,
            towards,
            context,
            |gen_idx| {
                let len =
                    gen_type_ident_with_suffix::<CounterIdentCollectionLenTypeNum>(
                        Some(gen_idx.to_string().as_bytes()),
                    );

                let crate_of = Instance::support_crate();

                parse_quote!(
                    #crate_of::GenericArray<&'static [u8], Self::#len>
                )
            },
            {
                not_found: IdentTraitItemsBugs::CounterIdentCollectionGenArrayNotFound {},
                wrong_ident: IdentTraitItemsBugs::CounterIdentCollectionGenArrayWrongIdent {},
                wrong_ty: IdentTraitItemsBugs::CounterIdentCollectionGenArrayWrongType {},
                has_generics: IdentTraitItemsBugs::CounterIdentCollectionGenArrayHasGenerics {},
                has_expr: IdentTraitItemsBugs::CounterIdentCollectionGenArrayHasExpr {},
            }
        );

        Ok(())
    }
}

// ===============================================================================
// `````````````````` COUNTER IDENT-HASH COLLECTION (GEN-ARRAY) ``````````````````
// ===============================================================================

/// Declares identifier-hash collections as fixed-size typenum-backed
/// generic arrays.
///
/// This collection exposes parent-scoped identifier-hash
/// collections visible at the current instance.
///
/// - [`CounterIdentHashCollectionGenArray`]
///     Represents collections as fixed-size `GenericArray`s whose
///     cardinality is encoded directly in the type system.
///
/// ## Why this exists
///
/// Stable Rust cannot express fixed-size arrays whose lengths are
/// derived from generated associated metadata without unstable
/// generic-const-expression support.
///
/// To provide fixed-size identifier-hash collections on stable Rust,
/// collection cardinalities are reflected through
/// [`CounterIdentCollectionLenTypeNum`] and used as the length
/// parameter of a `GenericArray`.
///
/// ## Generated Metadata
///
/// For every counter dimension, a hidden associated constant is
/// generated.
///
/// ```ignore
/// #[doc(hidden)]
/// const __INDEX_0_IDENT_HASH_COLLECTION_GEN_ARRAY:
///     GenericArray<u64, Self::__Index0CollectionLen>;
///
/// #[doc(hidden)]
/// const __INDEX_1_IDENT_HASH_COLLECTION_GEN_ARRAY:
///     GenericArray<u64, Self::__Index1CollectionLen>;
/// ```
///
/// where the generated collection-length types originate from
/// [`CounterIdentCollectionLenTypeNum`].
#[derive(Clone, Debug)]
pub(crate) struct CounterIdentHashCollectionGenArray;

impl<'a> Extension<Vec<TraitItemConst>, ItemTrait, CounterParamsSlice<'a>>
    for CounterIdentHashCollectionGenArray
{
    fn raw_extend(
        &self,
        _towards: &ItemTrait,
        context: &CounterParamsSlice<'a>,
    ) -> Result<Vec<TraitItemConst>, proc_macro2::TokenStream> {
        let collect = collect_expect_collection_consts::<Self, _>(
            context.iter().map(|c| &c.generic_index),
            |gen_idx| {
                let len = gen_type_ident_with_suffix::<CounterIdentCollectionLenTypeNum>(Some(
                    gen_idx.to_string().as_bytes(),
                ));
                let crate_of = Instance::support_crate();

                parse_quote!(#crate_of::GenericArray<u64, Self::#len>)
            },
        );

        Ok(collect)
    }

    fn validate_extend(
        &self,
        item: Option<&Vec<TraitItemConst>>,
        towards: &ItemTrait,
        context: Option<&CounterParamsSlice<'a>>,
    ) -> Result<(), proc_macro2::TokenStream> {
        validate_expect_collection_consts!(
            Self,
            item,
            towards,
            context,
            |gen_idx| {
                let len =
                    gen_type_ident_with_suffix::<CounterIdentCollectionLenTypeNum>(
                        Some(gen_idx.to_string().as_bytes()),
                    );

                let crate_of = Instance::support_crate();

                parse_quote!(
                    #crate_of::GenericArray<u64, Self::#len>
                )
            },
            {
                not_found: IdentTraitItemsBugs::CounterIdentHashCollectionGenArrayNotFound {},
                wrong_ident: IdentTraitItemsBugs::CounterIdentHashCollectionGenArrayWrongIdent {},
                wrong_ty: IdentTraitItemsBugs::CounterIdentHashCollectionGenArrayWrongType {},
                has_generics: IdentTraitItemsBugs::CounterIdentHashCollectionGenArrayHasGenerics {},
                has_expr: IdentTraitItemsBugs::CounterIdentHashCollectionGenArrayHasExpr {},
            }
        );
        Ok(())
    }
}

// ===============================================================================
// `````````````` COUNTER IDENT HISTORICAL COLLECTION LEN (TYPENUM) ``````````````
// ===============================================================================

/// Declares type-level historical collection cardinalities for a counter
/// dimension.
///
/// [`CounterIdentHistoricalCollectionLenTypeNum`] exposes the
/// padded historical cardinality as a typenum-associated type.
///
/// ## Why this exists
///
/// Historical identifier collections preserve the complete reachable
/// hierarchy accumulated up to the current instance, including:
///
/// - previously completed parent coordinates,
/// - their associated child histories,
/// - and the current coordinate being constructed.
///
/// Because historical collections are multidimensional and padded, many
/// generated metadata structures require their cardinalities to be
/// available at the type level.
///
/// This metadata provides those cardinalities as typenum values so they
/// can participate in trait bounds, type-level computation, and generic
/// array construction.
///
/// ## Relationship to Historical Length Constants
///
/// ```ignore
/// #[doc(hidden)]
/// const __INDEX_0_HISTORICAL_COLLECTION_LEN: usize = 3;
/// ```
///
/// is mirrored by:
///
/// ```ignore
/// #[doc(hidden)]
/// type __Index0HistoricalCollectionLenType = U3;
/// ```
///
/// Both represent the same padded historical cardinality. The former is
/// consumed in const contexts while the latter is consumed in type-level
/// contexts.
///
/// ## Generic Array Integration
///
/// Every generated historical cardinality type is required to implement
/// the support crate's re-exported generic-array's `ArrayLength` bound,
/// along with general typenum `Unsigned` bound:
///
/// ```ignore
/// type __Index0HistoricalCollectionLen:
///     ArrayLength + Unsigned;
/// ```
///
/// This allows historical identifier collections, historical
/// identifier-hash collections, and other historical metadata to be
/// represented as fixed-size `GenericArray`s whose dimensions are
/// derived from historical typenum cardinalities.
///
/// ## Implementation Note
///
/// The actual historical cardinality cannot be derived from trait-side
/// metadata alone. The associated type is therefore intended to be supplied by
/// implementation-side expansion after the complete reachable instance
/// hierarchy has been analyzed.
///
/// ## Generated Metadata
///
/// For every counter dimension, a hidden associated type is generated.
///
/// ```ignore
/// #[doc(hidden)]
/// type __Index0HistoricalCollectionLen:
///     ArrayLength + Unsigned;
///
/// #[doc(hidden)]
/// type __Index1HistoricalCollectionLen:
///     ArrayLength + Unsigned;
/// ```
///
/// These types are later consumed by historical collection metadata to
/// construct fixed-size typenum-backed representations of historical
/// identifier hierarchies.
#[derive(Clone, Debug)]
pub(crate) struct CounterIdentHistoricalCollectionLenTypeNum;

impl<'a> Extension<Vec<TraitItemType>, ItemTrait, CounterParamsSlice<'a>>
    for CounterIdentHistoricalCollectionLenTypeNum
{
    fn raw_extend(
        &self,
        _towards: &ItemTrait,
        context: &CounterParamsSlice<'a>,
    ) -> Result<Vec<TraitItemType>, proc_macro2::TokenStream> {
        let mut collect = Vec::new();
        for gen_idx in context.iter().map(|c| &c.generic_index) {
            let mut bounds = Punctuated::<TypeParamBound, Plus>::new();
            let crate_of = Instance::support_crate();
            let array_len_bound = parse_quote!(#crate_of::ArrayLength);
            bounds.push(TypeParamBound::Trait(array_len_bound));
            let unsigned_bound = parse_quote!(#crate_of::Unsigned);
            bounds.push(TypeParamBound::Trait(unsigned_bound));
            let ident = gen_type_ident_with_suffix::<Self>(Some(gen_idx.to_string().as_bytes()));
            let item = TraitItemType {
                attrs: proc_suite::internal_code(),
                ident,
                default: None,
                type_token: Default::default(),
                generics: Default::default(),
                colon_token: Default::default(),
                bounds,
                semi_token: Default::default(),
            };
            collect.push(item);
        }
        Ok(collect)
    }

    fn validate_extend(
        &self,
        item: Option<&Vec<TraitItemType>>,
        towards: &ItemTrait,
        context: Option<&CounterParamsSlice<'a>>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let extracted;
        let counters = match context {
            Some(c) => c,
            None => {
                extracted = CounterParams::checked_extract(towards, &None)?;
                extracted.as_slice().as_ref().as_ref()
            }
        };
        for gen_idx in counters.iter().map(|c| &c.generic_index) {
            let ident = gen_type_ident_with_suffix::<Self>(Some(gen_idx.to_string().as_bytes()));
            let crate_of = Instance::support_crate();
            let array_len_bound = parse_quote!(#crate_of::ArrayLength);
            let unsigned_bound = parse_quote!(#crate_of::Unsigned);
            let bounds = &[array_len_bound, unsigned_bound];
            validate_trait_types! {
                items: item,
                towards: towards,
                ident: ident,
                bounds: bounds => {
                    not_found: IdentTraitItemsBugs::CounterIdentHistoricalCollectionLenTypeNumNotFound {},
                    wrong_ident: IdentTraitItemsBugs::CounterIdentHistoricalCollectionLenTypeNumWrongIdent {},
                    has_generics: IdentTraitItemsBugs::CounterIdentHistoricalCollectionLenTypeNumHasGenerics {},
                    bound_not_found: IdentTraitItemsBugs::CounterIdentHistoricalCollectionLenTypeNumBoundNotFound {},
                }
            }
        }
        Ok(())
    }
}

// ===============================================================================
// ``````````````````````` HISTORICAL COLLECTION UTILITIES ```````````````````````
// ===============================================================================

macro_rules! validate_historical_collection {
    (
        $nested_ty_fn:ident,
        $self_ty:ty,
        $item:expr,
        $towards:expr,
        $context:expr,
        $base_ty:expr,
        errors => {
            not_found: $not_found:expr,
            wrong_ident: $wrong_ident:expr,
            wrong_ty: $wrong_ty:expr,
            has_generics: $has_generics:expr,
            has_expr: $has_expr:expr,
        }
    ) => {{
        let extracted;

        let counters = match $context {
            Some(c) => c,
            None => {
                extracted = CounterParams::checked_extract($towards, &None)?;
                extracted.as_slice().as_ref().as_ref()
            }
        };

        let types = $nested_ty_fn(counters, $base_ty)?;

        for (gen_idx, exp_ty) in counters.iter().map(|c| &c.generic_index).zip(types) {
            let ident =
                gen_const_ident_with_suffix::<$self_ty>(Some(gen_idx.to_string().as_bytes()));

            validate_trait_consts! {
                items: $item,
                towards: $towards,
                ident: ident,
                ty: exp_ty,
                expr: None => {
                    not_found: $not_found,
                    wrong_ident: $wrong_ident,
                    wrong_ty: $wrong_ty,
                    has_generics: $has_generics,
                    has_expr: $has_expr,
                }
            }
        }
    }};
}

fn collect_historical_collection<'a, T: 'static, F>(
    context: CounterParamsSlice<'a>,
    base_ty: Type,
    nested_ty_fn: F,
) -> Result<Vec<TraitItemConst>, TokenStream>
where
    F: FnOnce(CounterParamsSlice<'a>, Type) -> Result<Vec<Type>, TokenStream>,
{
    let types = nested_ty_fn(context, base_ty)?;

    Ok(context
        .iter()
        .map(|c| &c.generic_index)
        .zip(types)
        .map(|(gen_idx, ty)| TraitItemConst {
            attrs: proc_suite::internal_code(),
            ident: gen_const_ident_with_suffix::<T>(Some(gen_idx.to_string().as_bytes())),
            default: None,
            ty,
            const_token: Default::default(),
            generics: Default::default(),
            colon_token: Default::default(),
            semi_token: Default::default(),
        })
        .collect())
}

fn historical_collection_gen_array_nested_ty<'a>(
    context: CounterParamsSlice<'a>,
    base_ty: Type,
) -> Result<Vec<Type>, TokenStream> {
    let mut collect = Vec::new();
    for (i, _) in context.iter().enumerate() {
        let mut counters = context.to_vec();
        let mut rev = counters.iter_mut().take(i + 1).rev();
        let Some(c) = rev.next() else {
            return Err(CounterParamBugs::NoValidCountersProvided {}.into());
        };
        let len = gen_type_ident_with_suffix::<CounterIdentHistoricalCollectionLenTypeNum>(Some(
            c.generic_index.to_string().as_bytes(),
        ));
        let crate_of = Instance::support_crate();
        let mut ty = parse_quote!(#crate_of::GenericArray<#base_ty, Self::#len>);
        for c in rev {
            let len = gen_type_ident_with_suffix::<CounterIdentHistoricalCollectionLenTypeNum>(
                Some(c.generic_index.to_string().as_bytes()),
            );
            ty = parse_quote!(#crate_of::GenericArray<#ty, Self::#len>);
        }
        collect.push(ty);
    }
    Ok(collect)
}

// ===============================================================================
// ``````````````` COUNTER IDENT HISTORICAL COLLECTION (GEN-ARRAY) ```````````````
// ===============================================================================

/// Declares historical identifier collections as fixed-size typenum-backed
/// generic arrays.
///
/// This collection exposes the historical identifier hierarchy.
///
/// - [`CounterIdentHistoricalCollectionGenArray`]
///     Represents historical collections using nested `GenericArray`s
///     whose dimensions are encoded directly in the type system.
///
/// ## Historical Collection Shape
///
/// Historical collections preserve the complete hierarchy accumulated
/// across previously reachable instance coordinates.
///
/// For counters:
///
/// ```text
/// [C0, C1, C2, ...]
/// ```
///
/// the generated collection types become:
///
/// ```text
/// C0 -> GenericArray<T, Len0>
///
/// C1 -> GenericArray<
///           GenericArray<T, Len1>,
///           Len0
///       >
///
/// C2 -> GenericArray<
///           GenericArray<
///               GenericArray<T, Len2>,
///               Len1
///           >,
///           Len0
///       >
/// ```
///
/// where:
///
/// - `T` is the base collection element type,
/// - `LenN` is the padded historical cardinality represented by
///   [`CounterIdentHistoricalCollectionLenTypeNum`].
///
/// Unlike normal collection cardinalities, historical cardinalities do
/// not represent the identifiers visible at the current instance.
///
/// Instead, they represent the padded dimensions required to preserve
/// the complete reachable history of a counter hierarchy, including:
///
/// - previously completed parent coordinates,
/// - their associated child histories,
/// - and the current coordinate being constructed.
///
/// ## Example
///
/// Consider:
///
/// ```text
/// (Category, Algorithm)
///
/// ("crypto", "sha-256")
/// ("crypto", "sha-512")
///
/// ("image",  "png")
/// ("image",  "jpeg")
/// ("image",  "gif")
/// ```
///
/// At:
///
/// ```text
/// ("image", "gif")
/// ```
///
/// the historical collections are:
///
/// ```text
/// Counter 0:
///     [
///         "crypto",
///         "image"
///     ]
///
/// Counter 1:
///     [
///         ["sha-256", "sha-512"],
///         ["png", "jpeg", "gif"]
///     ]
/// ```
///
/// The algorithm histories have different cardinalities:
///
/// ```text
/// crypto -> 2
/// image  -> 3
/// ```
///
/// Therefore the historical algorithm dimension must be padded to:
///
/// ```text
/// 3
/// ```
///
/// yielding:
///
/// ```text
/// [
///     [Some("sha-256"), Some("sha-512"), None],
///     [Some("png"),     Some("jpeg"),    Some("gif")]
/// ]
/// ```
///
/// Missing historical entries are represented as `None`, allowing all
/// historical branches to share a uniform compile-time shape while
/// preserving the distinction between real identifiers and padding.
///
/// The generated `GenericArray` hierarchy preserves this historical
/// structure while encoding the padded historical cardinalities directly
/// into the type system.
///
/// ## Why this exists
///
/// Historical collections ideally would be represented as fixed-size
/// nested arrays whose dimensions are derived from historical collection
/// metadata.
///
/// Stable Rust cannot express those relationships through associated
/// metadata without unstable `generic_const_exprs`.
///
/// By using:
/// - [`CounterIdentHistoricalCollectionLenTypeNum`],
/// - and `GenericArray`,
///
/// the instance-trait system can construct fully sized historical
/// collections on stable Rust while preserving compile-time dimension
/// information.
///
/// ## Implementation Note
///
/// The collection contents and padded historical cardinalities are
/// supplied by implementation-side expansion.
///
/// Trait-side generation defines the expected historical collection
/// shape and cardinality contracts, while implementation-side expansion
/// computes the complete reachable instance hierarchy and populates the
/// generated collections.
///
/// ## Generated Metadata
///
/// For every counter dimension, a hidden associated constant is
/// generated.
///
/// ```ignore
/// #[doc(hidden)]
/// const __Index0HistoricalCollection:
///     GenericArray<
///         Option<&'static [u8]>,
///         Self::__Index0HistoricalCollectionLen
///     >;
///
/// #[doc(hidden)]
/// const __Index1HistoricalCollection:
///     GenericArray<
///         GenericArray<
///             Option<&'static [u8]>,
///             Self::__Index1HistoricalCollectionLen
///         >,
///         Self::__Index0HistoricalCollectionLen
///     >;
/// ```
///
/// These collections provide fixed-size historical views of the complete
/// identifier hierarchy, with every dimension sized according to its
/// padded historical cardinality.
///
/// Real historical identifiers are represented as `Some(...)`, while
/// padded historical space is represented as `None`.
#[derive(Clone, Debug)]
pub(crate) struct CounterIdentHistoricalCollectionGenArray;

impl<'a> Extension<Vec<TraitItemConst>, ItemTrait, CounterParamsSlice<'a>>
    for CounterIdentHistoricalCollectionGenArray
{
    fn raw_extend(
        &self,
        _towards: &ItemTrait,
        context: &CounterParamsSlice<'a>,
    ) -> Result<Vec<TraitItemConst>, proc_macro2::TokenStream> {
        collect_historical_collection::<Self, _>(
            context,
            parse_quote!(Option<&'static [u8]>),
            historical_collection_gen_array_nested_ty,
        )
    }

    fn validate_extend(
        &self,
        item: Option<&Vec<TraitItemConst>>,
        towards: &ItemTrait,
        context: Option<&CounterParamsSlice<'a>>,
    ) -> Result<(), proc_macro2::TokenStream> {
        validate_historical_collection!(
            historical_collection_gen_array_nested_ty,
            Self,
            item,
            towards,
            context,
            parse_quote!(Option<&'static [u8]>),
            errors => {
                not_found: IdentTraitItemsBugs::CounterIdentHistoricalCollectionGenArrayNotFound {},
                wrong_ident: IdentTraitItemsBugs::CounterIdentHistoricalCollectionGenArrayWrongIdent {},
                wrong_ty: IdentTraitItemsBugs::CounterIdentHistoricalCollectionGenArrayWrongType {},
                has_generics: IdentTraitItemsBugs::CounterIdentHistoricalCollectionGenArrayHasGenerics {},
                has_expr: IdentTraitItemsBugs::CounterIdentHistoricalCollectionGenArrayHasExpr {},
            }
        );
        Ok(())
    }
}

// ===============================================================================
// ```````````` COUNTER IDENT-HASH HISTORICAL COLLECTION (GEN-ARRAY) `````````````
// ===============================================================================

/// Declares historical identifier-hash collections as fixed-size
/// typenum-backed generic arrays.
///
/// This collection preserves the historical identifier-hash
/// hierarchy.
///
/// - [`CounterIdentHashHistoricalCollectionGenArray`]
///     Represents historical hash collections using nested
///     `GenericArray`s whose dimensions are encoded directly in the type
///     system.
///
/// ## Historical Collection Shape
///
/// Historical collections preserve the complete hierarchy accumulated
/// across previously reachable instance coordinates.
///
/// For counters:
///
/// ```text
/// [C0, C1, C2, ...]
/// ```
///
/// the generated collection types become:
///
/// ```text
/// C0 -> GenericArray<T, Len0>
///
/// C1 -> GenericArray<
///           GenericArray<T, Len1>,
///           Len0
///       >
///
/// C2 -> GenericArray<
///           GenericArray<
///               GenericArray<T, Len2>,
///               Len1
///           >,
///           Len0
///       >
/// ```
///
/// where:
///
/// - `T` is the historical identifier hash element type,
/// - `LenN` is the padded historical cardinality represented by
///   [`CounterIdentHistoricalCollectionLenTypeNum`].
///
/// Unlike normal collection cardinalities, these lengths do not
/// represent identifiers visible at the current instance.
///
/// Instead, they represent the padded dimensions required to preserve
/// the complete reachable history of the identifier hierarchy,
/// including:
///
/// - previously completed parent coordinates,
/// - their associated child histories,
/// - and the current coordinate being constructed.
///
/// ## Example
///
/// Given the historical identifier hierarchy:
///
/// ```text
/// [
///     ["sha-256", "sha-512"],
///     ["png", "jpeg", "gif"]
/// ]
/// ```
///
/// this metadata exposes:
///
/// ```text
/// [
///     [hash("sha-256"), hash("sha-512")],
///     [hash("png"), hash("jpeg"), hash("gif")]
/// ]
/// ```
///
/// The algorithm histories have different cardinalities:
///
/// ```text
/// crypto -> 2
/// image  -> 3
/// ```
///
/// Therefore the historical algorithm dimension must be padded to:
///
/// ```text
/// 3
/// ```
///
/// yielding:
///
/// ```text
/// [
///     [Some(hash("sha-256")), Some(hash("sha-512")), None],
///     [Some(hash("png")),     Some(hash("jpeg")),    Some(hash("gif"))]
/// ]
/// ```
///
/// Missing historical entries are represented as `None`, allowing all
/// historical branches to share a uniform compile-time shape while
/// preserving the distinction between real historical hashes and
/// padding.
///
/// The generated `GenericArray` hierarchy preserves this historical
/// structure while encoding the padded historical cardinalities directly
/// into the type system.
///
/// ## Why this exists
///
/// Historical identifier hashes provide a fixed-size representation of
/// historical identifier hierarchies that is well suited for:
///
/// - compile-time equality checks,
/// - uniqueness validation,
/// - affiliate-history traversal,
/// - historical reconstruction,
/// - and cross-instance verification.
///
/// Stable Rust cannot express nested fixed-size arrays whose dimensions
/// are derived from associated metadata without unstable
/// `generic_const_exprs`.
///
/// By using:
/// - [`CounterIdentHistoricalCollectionLenTypeNum`],
/// - and `GenericArray`,
///
/// the instance-trait system can construct fully sized historical hash
/// collections while remaining entirely compatible with stable Rust.
///
/// ## Implementation Note
///
/// The collection contents and padded historical cardinalities are
/// supplied by implementation-side expansion.
///
/// Trait-side generation defines the expected collection shape and
/// cardinality contracts, while implementation-side expansion computes
/// the complete reachable instance hierarchy and populates the generated
/// collections.
///
/// ## Generated Metadata
///
/// For every counter dimension, a hidden associated constant is
/// generated.
///
/// ```ignore
/// #[doc(hidden)]
/// const __Index0HistoricalHashCollection:
///     GenericArray<
///         Option<u64>,
///         Self::__Index0HistoricalCollectionLen
///     >;
///
/// #[doc(hidden)]
/// const __Index1HistoricalHashCollection:
///     GenericArray<
///         GenericArray<
///             Option<u64>,
///             Self::__Index1HistoricalCollectionLen
///         >,
///         Self::__Index0HistoricalCollectionLen
///     >;
/// ```
///
/// These collections provide fixed-size historical views of the complete
/// identifier-hash hierarchy, with every dimension sized according to
/// its padded historical cardinality.
///
/// Real historical identifier hashes are represented as `Some(...)`,
/// while padded historical space is represented as `None`.
#[derive(Clone, Debug)]
pub(crate) struct CounterIdentHashHistoricalCollectionGenArray;

impl<'a> Extension<Vec<TraitItemConst>, ItemTrait, CounterParamsSlice<'a>>
    for CounterIdentHashHistoricalCollectionGenArray
{
    fn raw_extend(
        &self,
        _towards: &ItemTrait,
        context: &CounterParamsSlice<'a>,
    ) -> Result<Vec<TraitItemConst>, proc_macro2::TokenStream> {
        collect_historical_collection::<Self, _>(
            context,
            parse_quote!(Option<u64>),
            historical_collection_gen_array_nested_ty,
        )
    }

    fn validate_extend(
        &self,
        item: Option<&Vec<TraitItemConst>>,
        towards: &ItemTrait,
        context: Option<&CounterParamsSlice<'a>>,
    ) -> Result<(), proc_macro2::TokenStream> {
        validate_historical_collection!(
            historical_collection_gen_array_nested_ty,
            Self,
            item,
            towards,
            context,
            parse_quote!(Option<u64>),
            errors => {
                not_found: IdentTraitItemsBugs::CounterIdentHashHistoricalCollectionGenArrayNotFound {},
                wrong_ident: IdentTraitItemsBugs::CounterIdentHashHistoricalCollectionGenArrayWrongIdent {},
                wrong_ty: IdentTraitItemsBugs::CounterIdentHashHistoricalCollectionGenArrayWrongType {},
                has_generics: IdentTraitItemsBugs::CounterIdentHashHistoricalCollectionGenArrayHasGenerics {},
                has_expr: IdentTraitItemsBugs::CounterIdentHashHistoricalCollectionGenArrayHasExpr {},
            }
        );
        Ok(())
    }
}

// ===============================================================================
// ```````````````````````` COUNTER IDENT EXPECTED HASHES ````````````````````````
// ===============================================================================

/// Declares a hidden-assoc-constant (validation hook) used to verify identifier lineage
/// consistency across parent counter dimensions.
///
/// Counter identifiers form a hierarchical namespace. Instances that
/// differ only in a deeper counter dimension are expected to preserve
/// the identifier history of all preceding dimensions.
///
/// Example:
///
/// ```text
/// (0,0,0) -> ("crypto", "sha", "256")
/// (0,0,1) -> ("crypto", "sha", "512")
/// ```
///
/// Since both instances belong to the same parent coordinate:
///
/// ```text
/// (0,0)
/// ```
///
/// their parent identifiers must remain identical:
///
/// ```text
/// Counter 0 -> "crypto"
/// Counter 1 -> "sha"
/// ```
///
/// During implementation-side expansion, deterministic
/// [`CounterIdentHash`] values are generated for every reflected
/// identifier. The generated implementation then compares the expected
/// parent identifier hashes against those of the back-affiliate
/// instance.
///
/// This validation guarantees that sibling instances
/// (via [`affiliates`](crate::traits::affiliates)) cannot diverge in
/// any ancestor identifier dimension while still sharing the same
/// parent coordinate.
///
/// ```text
/// current.parent_hashes
///     ==
/// back(current).parent_hashes
/// ```
///
/// for all preserved parent dimensions.
///
/// The implementation-side expansion materializes these checks through
/// compile-time assertions and binds the result to this hidden unit
/// constant.
///
/// Conceptually the impl-side is expected to assert:
///
/// ```ignore
/// const __COUNTER_IDENT_EXPECTED_HASH_CHECKER: () = {
///     const_assert!(parent_hashes_are_consistent);
/// };
/// ```
///
/// As a result, malformed identifier hierarchies are rejected during
/// compilation before identifier collections, affiliate metadata, or
/// historical identifier structures are generated.
#[derive(Clone, Debug)]
pub(crate) struct CounterIdentExpectedHashChecker;

impl Extension<TraitItemConst, ItemTrait> for CounterIdentExpectedHashChecker {
    fn raw_extend(&self, _: &ItemTrait, _: &()) -> Result<TraitItemConst, TokenStream> {
        let ident = gen_const_ident::<Self>();
        let ty: Type = parse_quote!(());
        let item = TraitItemConst {
            attrs: proc_suite::internal_code(),
            ident,
            default: None,
            ty,
            const_token: Default::default(),
            generics: Default::default(),
            colon_token: Default::default(),
            semi_token: Default::default(),
        };
        Ok(item)
    }

    fn validate_extend(
        &self,
        item: Option<&TraitItemConst>,
        towards: &ItemTrait,
        _: Option<&()>,
    ) -> Result<(), TokenStream> {
        let ident = gen_const_ident::<Self>();
        let expected_ty: Type = parse_quote!(());

        validate_trait_const! {
            item: item,
            towards: towards,
            ident: ident,
            ty: expected_ty,
            expr: None => {
                not_found: IdentTraitItemsBugs::CounterIdentExpectedHashCheckerNotFound {},
                wrong_ident: IdentTraitItemsBugs::CounterIdentExpectedHashCheckerWrongIdent {},
                wrong_ty: IdentTraitItemsBugs::CounterIdentExpectedHashCheckerNotUnitType {},
                has_generics: IdentTraitItemsBugs::CounterIdentExpectedHashCheckerHasGenerics {},
                has_expr: IdentTraitItemsBugs::CounterIdentExpectedHashCheckerHasExpr {},
            }
        }
    }
}
