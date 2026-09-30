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
// ```````````````````````````````` INSTANCE TRAIT ```````````````````````````````
// ===============================================================================

//! Instance-trait expansion.
//!
//! This module implements the core transformation performed by the
//! `#[instance]` trait macro.
//!
//! The expansion converts a user-facing trait containing instance
//! counters into a fully reflected compile-time representation capable
//! of describing instance identity, hierarchy structure, adjacent
//! instances, historical state, and affiliate relationships.
//!
//! ## Mental Model
//!
//! An instance trait begins as a normal trait containing one or more
//! counter parameters:
//!
//! ```ignore
//! #[instance]
//! trait Service<
//!     const CATEGORY: u8,
//!     const ALGORITHM: u8,
//! > {}
//! ```
//!
//! Conceptually, the counters define coordinates within an instance
//! hierarchy:
//!
//! ```text
//! (CATEGORY, ALGORITHM)
//!
//! ("crypto", "sha-256") -> (0, 0)
//! ("crypto", "sha-512") -> (0, 1)
//!
//! ("image",  "png")     -> (1, 0)
//! ("image",  "jpeg")    -> (1, 1)
//! ("image",  "gif")     -> (1, 2)
//! ```
//!
//! Each coordinate represents a distinct instance.
//!
//! The purpose of instance-trait expansion is to transform those raw
//! counters into a metadata model capable of describing the entire
//! instance space.
//!
//! ## Expansion Pipeline
//!
//! Expansion proceeds through four major stages:
//!
//! ```text
//! Counter Extraction
//!         |
//!         +-- InstanceTraitMeta
//!         |
//!         +-- InstanceTraitTypeNumCounters
//!         |
//!         +-- Affiliates
//!         |
//!         +-- InstanceTraitIdents
//! ```
//!
//! Each stage contributes a different aspect of the final instance
//! model.
//!
//! ## Counter Extraction
//!
//! Expansion begins by extracting and validating all instance-counter
//! parameters declared on the trait.
//!
//! The extracted counters become the canonical description of the
//! instance coordinate system and drive all subsequent metadata
//! generation.
//!
//! ## TypeNum Counter Replacement
//!
//! Stable Rust cannot yet express many of the required type-level
//! relationships through const-generic expressions alone.
//!
//! To enable compile-time computation and metadata generation, the
//! original const counters are reflected into a hidden typenum-based
//! representation used throughout the expansion pipeline.
//!
//! ```text
//! (CATEGORY, ALGORITHM)
//!
//! becomes
//!
//! InstanceTraitTypeNumCounters
//! ```
//!
//! while preserving access to the original counter information through
//! reflected metadata.
//!
//! This allows expansion to perform type-level operations such as:
//!
//! - collection cardinality computation,
//! - dimension indexing,
//! - historical layout construction,
//! - affiliate traversal,
//! - and compile-time validation.
//!
//! ## Affiliate Expansion
//!
//! Affiliates expose projections of the current instance coordinate onto
//! related coordinates within the same hierarchy.
//!
//! Conceptually, affiliates provide metadata capable of navigating from
//! one instance to adjacent or related instances without requiring
//! direct access to the original counter declarations.
//!
//! Affiliate metadata therefore forms the basis for:
//!
//! - hierarchy traversal,
//! - parent relationships,
//! - child relationships,
//! - adjacent instance discovery,
//! - and historical reconstruction.
//!
//! ## Identifier Expansion
//!
//! Counter coordinates identify position but do not describe meaning.
//!
//! For example:
//!
//! ```text
//! (0, 1)
//! ```
//!
//! identifies an instance but does not communicate:
//!
//! ```text
//! ("crypto", "sha-512")
//! ```
//!
//! To solve this, every counter value may be assigned a user-provided
//! ASCII identifier.
//!
//! These identifiers become the semantic identity layer of the instance
//! system.
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
//! 1 => "jpeg"
//! 2 => "gif"
//! ```
//!
//! Expansion reflects these identifiers into hidden metadata so later
//! stages can reason about instances through meaning rather than raw
//! counter values.
//!
//! ## Identifier Hashes
//!
//! Every identifier additionally receives a deterministic `u64` hash.
//!
//! The ASCII identifier remains the canonical semantic representation,
//! while the hash provides a compact fixed-size representation suitable
//! for:
//!
//! - uniqueness validation,
//! - collection comparison,
//! - affiliate validation,
//! - historical reconstruction,
//! - and compile-time consistency checking.
//!
//! ## Collections
//!
//! Identifier metadata is further expanded into collection views.
//!
//! Collections provide a complete representation of the identifier space
//! visible from a particular instance coordinate.
//!
//! For:
//!
//! ```text
//! ("crypto", "sha-512")
//! ```
//!
//! the current collections become:
//!
//! ```text
//! Counter 0:
//!     ["crypto"]
//!
//! Counter 1:
//!     ["sha-256", "sha-512"]
//! ```
//!
//! These collections are not merely identifier lists.
//!
//! They are semantic projections of the underlying counter hierarchy and
//! therefore also describe the exact counter progression represented by
//! those identifiers:
//!
//! ```text
//! ["sha-256", "sha-512"]
//!     |
//!     +-- [0, 1]
//! ```
//!
//! From a single coordinate, expansion can therefore recover both
//! neighboring identifiers and the corresponding counter structure.
//!
//! ## Historical Collections
//!
//! Current collections describe only the identifiers visible from the
//! current coordinate.
//!
//! Historical collections preserve the complete hierarchy accumulated
//! across previously reachable coordinates.
//!
//! ```text
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
//! Historical metadata preserves:
//!
//! - parent history,
//! - child history,
//! - adjacent hierarchy state,
//! - and the current coordinate.
//!
//! Padded cardinalities ensure all branches can be represented within a
//! common multidimensional layout.
//!
//! ## Type-Level Metadata
//!
//! Collection cardinalities are reflected both as runtime constants and
//! as typenum-associated types.
//!
//! These type-level representations enable:
//!
//! - fixed-size `GenericArray` collections,
//! - compile-time dimension checking,
//! - nested historical layouts,
//! - and type-level validation contracts.
//!
//! ## Sum Types
//!
//! `#[sum]` turns an instance-dependent associated type into an enum-like
//! boundary through reciprocal `From`/`Into` bounds, allowing the same logical
//! type to be represented by different instance implementations.
//!
//! This provides a canonical terminal anchor through which the type can cross
//! generic boundaries and be converted back to the required instance.
//!
//! ## Documentation vs Compilation Targets
//!
//! Two separate expansion targets exist.
//!
//! [`InstanceTrait`] is used during normal compilation:
//!
//! ```text
//! cfg(not(doc))
//! ```
//!
//! and generates the complete internal metadata representation,
//! including:
//!
//! - typenum counters,
//! - affiliates,
//! - metadata reflection,
//! - identifier metadata,
//! - collection metadata,
//! - and validation contracts.
//!
//! [`InstanceTraitDocTarget`] is used when documentation is generated:
//!
//! ```text
//! cfg(doc)
//! ```
//!
//! and produces a simplified user-facing view focused on:
//!
//! - expected counter types,
//! - identifier requirements,
//! - and implementation guidance.
//!
//! This prevents internal expansion details from appearing in generated
//! API documentation while still documenting the user-visible contract.
//!
//! ## Implementation Model
//!
//! The instance trait itself defines metadata contracts and reflection
//! structures.
//!
//! Those contracts are later satisfied by a corresponding instance-impl
//! macro which supplies:
//!
//! - identifier values,
//! - identifier hashes,
//! - collection contents,
//! - historical collections,
//! - collection cardinalities,
//! - affiliate metadata,
//! - and other instance-specific state.
//!
//! Validation then ensures the implementation accurately satisfies every
//! contract generated by trait-side expansion.
//!
//! ## Result
//!
//! The expanded instance trait becomes a fully reconstructable
//! compile-time model of:
//!
//! - instance identity,
//! - semantic naming,
//! - hierarchy structure,
//! - adjacent instances,
//! - affiliate relationships,
//! - collection state,
//! - historical state,
//! - collection cardinalities,
//! - and type-level dimensions.
//!
//! Consequently, later proc-macro stages can reason about instances
//! using semantic metadata rather than raw counter coordinates alone.

// ===============================================================================
// ``````````````````````````````````` MODULES ```````````````````````````````````
// ===============================================================================

pub(crate) mod affiliates;
pub(crate) mod counters;
mod errors;
pub(crate) mod idents;
pub(crate) mod meta;
pub(crate) mod sum;
pub(crate) mod typenum;
pub(crate) mod utils;

// ===============================================================================
// ``````````````````````````````````` IMPORTS ```````````````````````````````````
// ===============================================================================

// --- Local-crate ---
use crate::{
    Extension, Extraction, InstanceArgs, Transformation,
    traits::{
        affiliates::*,
        counters::*,
        idents::*,
        meta::*,
        sum::{InstanceSumTypes, SumAttrRemoval},
        typenum::*,
    },
};

// --- Proc-macro Utilties ---
use proc_macro2::TokenStream;
use syn::ItemTrait;

// ===============================================================================
// ```````````````````````````````` INSTANCE TRAIT ```````````````````````````````
// ===============================================================================

/// Expands the internal instance-trait representation.
///
/// See module comment ([`traits`](crate::traits)) for semantic explanation.
///
/// This transformation acts as the primary orchestration entry point for
/// instance-trait expansion. It extracts counter metadata and applies
/// all compile-time transformations required by the instance system.
///
/// The generated trait is intended for normal compilation
/// (`cfg(not(doc))`) and contains the complete internal metadata model
/// consumed by implementation-side expansion (via a compatible impl macro)
/// and validation.
///
/// The expansion pipeline delegates to:
///
/// - [`InstanceTraitMeta`] for trait metadata reflection (see [`meta`]),
/// - [`InstanceTraitTypeNumCounters`] for typenum-based counter replacement (see [`typenum`]),
/// - [`InstanceSumTypes`] for terminal-anchored sum-type boundaries.
/// - [`InstanceTraitAffiliates`] for affiliate-counter projections (see [`affiliates`]),
/// - [`InstanceTraitIdents`] for semantic instance-identifier metadata (see [`idents`]).
/// - [`CumulatedConstChecker`] for cumulating independent const assertions.
///
/// Together these transformations convert the user-facing instance trait
/// into its fully expanded internal representation.
#[derive(Clone, Debug)]
pub(crate) struct InstanceTrait;

impl Transformation<ItemTrait, Option<&InstanceArgs>> for InstanceTrait {
    fn raw_transform(
        &self,
        transform: &mut ItemTrait,
        context: &Option<&InstanceArgs>,
    ) -> Result<(), TokenStream> {
        let counters = CounterParams::checked_extract(transform, context)?;
        let counters_slc = counters.as_slice();
        InstanceTraitTypeNumCounters::checked_transform(
            &InstanceTraitTypeNumCounters,
            transform,
            &counters_slc,
        )?;
        InstanceSumTypes::checked_transform(&InstanceSumTypes, transform, &counters_slc)?;
        InstanceTraitMeta::checked_transform(&InstanceTraitMeta, transform, &counters_slc)?;
        InstanceTraitAffiliates::checked_transform(
            &InstanceTraitAffiliates,
            transform,
            &counters_slc,
        )?;
        InstanceTraitIdents::checked_transform(&InstanceTraitIdents, transform, &counters_slc)?;
        CumulatedConstChecker::checked_extend(&CumulatedConstChecker, transform, &())?;
        SumAttrRemoval::checked_transform(&SumAttrRemoval, transform, &())?;
        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &ItemTrait,
        context: Option<&Option<&InstanceArgs>>,
    ) -> Result<(), TokenStream> {
        let context = match context {
            Some(Some(c)) => Some(*c),
            _ => None,
        };
        let counters = CountersParamMetaExtraction::checked_extract(transform, &context)?.0;
        let counters_slc = counters.as_slice();
        InstanceTraitTypeNumCounters::validate_transform(
            &InstanceTraitTypeNumCounters,
            transform,
            Some(&counters_slc),
        )?;
        InstanceSumTypes::validate_transform(&InstanceSumTypes, transform, Some(&counters_slc))?;
        InstanceTraitMeta::validate_transform(&InstanceTraitMeta, transform, Some(&counters_slc))?;
        InstanceTraitAffiliates::validate_transform(
            &InstanceTraitAffiliates,
            transform,
            Some(&counters_slc),
        )?;
        InstanceTraitIdents::validate_transform(
            &InstanceTraitIdents,
            transform,
            Some(&counters_slc),
        )?;
        CumulatedConstChecker::validate_extend(&CumulatedConstChecker, None, transform, None)?;
        SumAttrRemoval::validate_transform(&SumAttrRemoval, transform, None)?;
        Ok(())
    }
}

// ===============================================================================
// ````````````````````````````` INSTANCE TRAIT (DOC) ````````````````````````````
// ===============================================================================

/// Expands the documentation-facing instance-trait representation.
///
/// This transformation is used when generating documentation
/// (`cfg(doc)`).
///
/// Unlike [`InstanceTrait`], which produces the complete internal trait
/// representation used during compilation, this transformation emits a
/// simplified documentation-oriented view intended for end users.
///
/// The generated documentation trait exposes only the public contracts
/// relevant to trait implementors while hiding the majority of the
/// internal metadata required by the instance expansion pipeline.
///
/// The documentation view includes:
///
/// - [`CountersTy`] to re-expose the expected counter type contract,
/// - [`CounterIdent`] to expose the expected acii identifier per counter,
/// - [`InstanceTraitDocs`] to generate trait implementation guidance and
///   user-facing documentation.
/// - [`SumAttrRemoval`] removes the internal `#[sum]` marker from associated
/// types, keeping the expansion-only attribute out of the generated
/// documentation.
///
/// This allows generated documentation to present the semantic trait
/// contract without exposing the internal reflection, validation,
/// collection, affiliate, and typenum metadata used by the compiled
/// representation.
#[derive(Clone, Debug)]
pub(crate) struct InstanceTraitDocTarget;

impl Transformation<ItemTrait, Option<&InstanceArgs>> for InstanceTraitDocTarget {
    fn raw_transform(
        &self,
        transform: &mut ItemTrait,
        context: &Option<&InstanceArgs>,
    ) -> Result<(), TokenStream> {
        let counters = CounterParams::checked_extract(transform, context)?;
        let counters_slc = counters.as_slice();
        CountersTy::checked_extend(&CountersTy, transform, &counters_slc)?;
        InstanceTraitDocs::checked_transform(&InstanceTraitDocs, transform, &counters_slc)?;
        CounterIdent::checked_extend(&CounterIdent, transform, &counters_slc)?;
        SumAttrRemoval::checked_transform(&SumAttrRemoval, transform, &())?;
        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &ItemTrait,
        context: Option<&Option<&InstanceArgs>>,
    ) -> Result<(), TokenStream> {
        let context = match context {
            Some(Some(c)) => Some(*c),
            _ => None,
        };
        let counters = CounterParams::checked_extract(transform, &context)?;
        let counters_slc = counters.as_slice();
        CountersTy::validate_extend(&CountersTy, None, transform, Some(&counters_slc))?;
        InstanceTraitDocs::validate_transform(&InstanceTraitDocs, transform, Some(&counters_slc))?;
        CounterIdent::validate_extend(&CounterIdent, None, transform, Some(&counters_slc))?;
        SumAttrRemoval::validate_transform(&SumAttrRemoval, transform, None)?;
        Ok(())
    }
}
