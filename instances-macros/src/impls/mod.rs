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
// ```````````````````````````````` INSTANCE IMPL ````````````````````````````````
// ===============================================================================

//! Instance-implementation expansion.
//!
//! This module implements the core transformations performed by the
//! `#[instance_impl]` and `#[last_instance]` proc macros.
//!
//! Whereas instance-trait expansion (see [`crate::traits`]) defines the
//! compile-time contracts for an instance hierarchy, this module
//! provides the concrete implementation of those contracts for each
//! individual instance.
//!
//! ## Mental Model
//!
//! An instance trait defines the coordinate system:
//!
//! ```text
//! (CATEGORY, ALGORITHM)
//! ```
//!
//! while each implementation represents exactly one coordinate:
//!
//! ```ignore
//! #[instance_impl(1, 2)]
//! impl Example<0, 1> for MyType { ... }
//! ```
//!
//! The purpose of implementation expansion is to enrich that single
//! implementation with all metadata required to participate in the
//! complete instance graph.
//!
//! ## Expansion Pipeline
//!
//! `#[instance_impl]` performs the common expansion shared by every
//! implementation:
//!
//! ```text
//! Counter Extraction
//!         +-- TypeNum replacement
//!         +-- Metadata
//!         +-- Identifier metadata
//!         +-- Affiliate metadata
//!         +-- Compile-time validation
//! ```
//!
//! These phases generate:
//!
//! - typenum counter projections (see [`typenum`]),
//! - reflected implementation metadata (see [`meta`]),
//! - semantic identifier metadata and collections (see [`idents`]),
//! - affiliate projections (see [`affiliates`]),
//! - compile-time validation,
//! - and the default cumulative checker.
//!
//! ## Boundary Specialization
//!
//! Most implementations follow the common expansion path above.
//!
//! However, instances that terminate one or more counter dimensions
//! require specialized affiliate semantics.
//!
//! Such implementations additionally participate in
//! `#[last_instance(...)]`, which replaces boundary-independent
//! projections with boundary-aware implementations.
//!
//! These replacements specialize:
//!
//! - terminal next affiliates,
//! - floor-affiliate boundaries,
//! - predecessor recovery,
//! - affiliate consistency assertions,
//! - and installation of the cumulative checker that forces validation
//!   across every reachable instance.
//!
//! ## Supplementary Expansion
//!
//! Additional helper implementations are emitted alongside the expanded
//! instance (see [`addon`]).
//!
//! These expose compile-time lookup facilities that recover an
//! instance's typenum counter tuple from its semantic identifier hashes
//! and implementing `Self` type.
//!
//! ## Documentation vs Compilation Targets
//!
//! Two expansion targets exist.
//!
//! [`InstanceImpl`] performs the complete implementation expansion used
//! during normal compilation.
//!
//! [`InstanceImplDocTarget`] produces a documentation-oriented view that
//! removes expansion-only attributes before documentation generation.
//!
//! ## Post Expansion
//!
//! After all transformations have completed, temporary expansion
//! metadata is removed by the post-expansion cleanup phase (see
//! [`post`]) so the final implementation contains only meaningful Rust
//! items.
//!

// ===============================================================================
// ``````````````````````````````````` MODULES ```````````````````````````````````
// ===============================================================================

mod addon;
mod affiliates;
mod counters;
mod errors;
mod idents;
mod last;
mod meta;
mod post;
mod typenum;
pub(crate) mod utils;

// ===============================================================================
// ``````````````````````````````````` IMPORTS ```````````````````````````````````
// ===============================================================================

// --- Std Crate ---
use std::fmt::Debug;

// --- Local-crate ---
use crate::{
    Extension, Extraction, Transformation,
    impls::{
        addon::*, affiliates::*, counters::*, idents::*, last::*, meta::*, post::*, typenum::*,
    },
};

// --- Proc-macro Utilties ---
use proc_macro2::TokenStream;
use syn::{File, ItemImpl};

// --- Proc Suite ---
use proc_suite::IntList;

// ===============================================================================
// ```````````````````````````````` INSTANCE IMPL ````````````````````````````````
// ===============================================================================

/// Expands the internal implementation representation.
///
/// See the module documentation ([`crate::impls`]) for the complete
/// semantic overview.
///
/// This transformation is the primary orchestration entry point for the
/// `#[instance_impl]` expansion. It extracts instance-counter metadata
/// and applies all compile-time transformations required to materialize
/// a complete implementation of an instance.
///
/// The expansion pipeline delegates to:
///
/// - [`InstanceImplTypeNumCounters`] for typenum-based counter reflection (see [`typenum`]),
/// - [`InstanceImplMeta`] for implementation metadata generation (see [`meta`]),
/// - [`InstanceImplIdents`] for semantic identifier metadata, collections and historical state (see [`idents`]),
/// - [`InstanceImplAffiliates`] for affiliate projections and consistency validation (see [`affiliates`]),
/// - [`ImplCumulatedConstChecker`] for installing the default cumulative compile-time checker (see [`meta`]).
///
/// The resulting implementation satisfies the contracts generated by the
/// corresponding expanded instance trait and serves as the foundation
/// for optional boundary specialization performed by
/// [`LastInstanceImpl`].
#[derive(Clone, Debug)]
pub(crate) struct InstanceImpl;

impl Transformation<ItemImpl, Option<&IntList>> for InstanceImpl {
    fn raw_transform(
        &self,
        transform: &mut ItemImpl,
        context: &Option<&IntList>,
    ) -> Result<(), TokenStream> {
        let counters = CounterArgs::checked_extract(transform, context)?;
        let counters_slc = counters.as_slice();
        InstanceImplTypeNumCounters::checked_transform(
            &InstanceImplTypeNumCounters,
            transform,
            &counters_slc,
        )?;
        InstanceImplMeta::checked_transform(&InstanceImplMeta, transform, &counters_slc)?;
        InstanceImplIdents::checked_transform(&InstanceImplIdents, transform, &counters_slc)?;
        InstanceImplAffiliates::checked_transform(
            &InstanceImplAffiliates,
            transform,
            &counters_slc,
        )?;
        ImplCumulatedConstChecker::checked_extend(&ImplCumulatedConstChecker, transform, &())?;
        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &ItemImpl,
        _: Option<&Option<&IntList>>,
    ) -> Result<(), TokenStream> {
        let counters = CounterArgsAsAssoc::checked_extract(transform, &())?.0;
        let counters_slc = counters.as_slice();
        InstanceImplTypeNumCounters::validate_transform(
            &InstanceImplTypeNumCounters,
            transform,
            Some(&counters_slc),
        )?;
        InstanceImplMeta::validate_transform(&InstanceImplMeta, transform, Some(&counters_slc))?;
        InstanceImplIdents::validate_transform(
            &InstanceImplIdents,
            transform,
            Some(&counters_slc),
        )?;
        InstanceImplAffiliates::validate_transform(
            &InstanceImplAffiliates,
            transform,
            Some(&counters_slc),
        )?;
        ImplCumulatedConstChecker::validate_extend(
            &ImplCumulatedConstChecker,
            None,
            transform,
            None,
        )?;
        Ok(())
    }
}

// ===============================================================================
// ````````````````````````````` INSTANCE IMPL (DOC) `````````````````````````````
// ===============================================================================

/// Produces the documentation-oriented implementation view.
///
/// This transformation is used during documentation generation
/// (`cfg(doc)`).
///
/// Unlike [`InstanceImpl`], which generates the complete internal
/// implementation metadata required for compilation, this transformation
/// performs only the cleanup necessary to present a user-facing
/// implementation.
///
/// The documentation pipeline delegates to:
///
/// - [`CounterIdentAttrRemoval`] to remove temporary counter-identification attributes,
/// - [`SelfBoundsAttrRemoval`] to remove temporary self-bound metadata,
/// - [`LastInstanceAttrRemoval`] to remove the `#[last_instance]` proc-macro attribute.
///
/// This produces a clean implementation signature suitable for generated
/// documentation while hiding expansion-only metadata and proc-macro
/// artifacts.
#[derive(Clone, Debug)]
pub(crate) struct InstanceImplDocTarget;

impl Transformation<ItemImpl> for InstanceImplDocTarget {
    fn raw_transform(&self, transform: &mut ItemImpl, context: &()) -> Result<(), TokenStream> {
        CounterIdentAttrRemovalStrict::checked_transform(
            &CounterIdentAttrRemovalStrict,
            transform,
            context,
        )?;
        SelfBoundsAttrRemovalStrict::checked_transform(
            &SelfBoundsAttrRemovalStrict,
            transform,
            context,
        )?;
        LastInstanceAttrRemoval::checked_transform(&LastInstanceAttrRemoval, transform, context)?;
        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &ItemImpl,
        context: Option<&()>,
    ) -> Result<(), TokenStream> {
        CounterIdentAttrRemovalStrict::validate_transform(
            &CounterIdentAttrRemovalStrict,
            transform,
            context,
        )?;
        SelfBoundsAttrRemovalStrict::validate_transform(
            &SelfBoundsAttrRemovalStrict,
            transform,
            context,
        )?;
        LastInstanceAttrRemoval::validate_transform(&LastInstanceAttrRemoval, transform, context)?;
        Ok(())
    }
}

// ===============================================================================
// ```````````````````````````` INSTANCE IMPL ADDONS `````````````````````````````
// ===============================================================================

/// Generates supplementary implementation items outside the instance
/// implementation itself.
///
/// Some generated structures are more naturally expressed as additional
/// crate-level items rather than associated items inside the expanded
/// implementation.
///
/// This transformation delegates to addon generators that append such
/// items to the surrounding source file.
///
/// Current addons include:
///
/// - [`CounterAccessAddon`], which generates a global compile-time
///   mapping from an instance's identifier hashes and implementing
///   `Self` type to its typenum counter tuple.
/// - [`OnSetAccessAddon`], which generates compile-time lookup
///   implementations that recover the onset (first child) instance of a
///   child hierarchy from a partially resolved parent identifier
///   sequence.
///
/// These addons complement the expanded implementation while remaining
/// external to it.
#[derive(Clone, Debug)]
pub(crate) struct InstanceImplAddons;

impl Transformation<File, ItemImpl> for InstanceImplAddons {
    fn raw_transform(&self, transform: &mut File, context: &ItemImpl) -> Result<(), TokenStream> {
        CounterAccessAddon::checked_extend(&CounterAccessAddon, transform, context)?;
        OnSetAccessAddon::checked_transform(&OnSetAccessAddon, transform, context)?;
        IdentHashLenAddon::checked_transform(&IdentHashLenAddon, transform, context)?;
        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &File,
        context: Option<&ItemImpl>,
    ) -> Result<(), TokenStream> {
        CounterAccessAddon::validate_extend(&CounterAccessAddon, None, transform, context)?;
        OnSetAccessAddon::validate_transform(&OnSetAccessAddon, transform, context)?;
        IdentHashLenAddon::validate_transform(&IdentHashLenAddon, transform, context)?;
        Ok(())
    }
}

// ===============================================================================
// ````````````````````````````` POST INSTANCE IMPL ``````````````````````````````
// ===============================================================================

/// Executes the post-expansion cleanup phase for implementation items.
///
/// This transformation runs after all implementation-side expansion,
/// validation and addon generation have completed.
///
/// It delegates to the post-processing cleanup passes in [`post`] to
/// remove temporary proc-macro metadata that was required only during
/// expansion.
///
/// The cleanup pipeline currently includes:
///
/// - [`CounterIdentAttrRemoval`] for removing consumed
///   `#[counter(...)]` metadata,
/// - [`SelfBoundsAttrRemoval`] for conditionally removing consumed
///   `#[self_bounds(...)]` metadata (if [`LastInstanceImpl`] does not exists).
///
/// The resulting implementation contains only the generated semantic
/// items required at compile time, without expansion-only helper
/// attributes.
#[derive(Clone, Debug)]
pub(crate) struct PostInstanceImpl;

impl<'a> Transformation<ItemImpl> for PostInstanceImpl {
    fn raw_transform(&self, transform: &mut ItemImpl, context: &()) -> Result<(), TokenStream> {
        CounterIdentAttrRemoval::checked_transform(&CounterIdentAttrRemoval, transform, context)?;
        SelfBoundsAttrRemoval::checked_transform(&SelfBoundsAttrRemoval, transform, context)?;
        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &ItemImpl,
        context: Option<&()>,
    ) -> Result<(), TokenStream> {
        CounterIdentAttrRemoval::validate_transform(&CounterIdentAttrRemoval, transform, context)?;
        SelfBoundsAttrRemoval::validate_transform(&SelfBoundsAttrRemoval, transform, context)?;
        Ok(())
    }
}

// ===============================================================================
// `````````````````````` LAST INSTANCE TRANFORMATION SPACE ``````````````````````
// ===============================================================================

pub(crate) struct LastInstanceSpace<'a> {
    pub(crate) impl_of: &'a mut ItemImpl,
    pub(crate) addons: &'a mut File,
}

impl<'a> From<(&'a mut ItemImpl, &'a mut File)> for LastInstanceSpace<'a> {
    fn from(value: (&'a mut ItemImpl, &'a mut File)) -> Self {
        LastInstanceSpace {
            impl_of: value.0,
            addons: value.1,
        }
    }
}

// ===============================================================================
// ````````````````````````````` LAST INSTANCE IMPL ``````````````````````````````
// ===============================================================================

/// Applies boundary-specific specialization for instances annotated by
/// the `#[last_instance(...)]` proc macro.
///
/// While [`InstanceImpl`] generates affiliate metadata assuming ordinary
/// successor/predecessor relationships, boundary instances require
/// several affiliate projections and validation nodes to be refined so
/// they correctly represent the end of one or more counter dimensions.
///
/// This transformation first resolves the designated boundary counter
/// from the `#[last_instance(...)]` arguments, then delegates to the
/// replacement pipeline in [`last`] to update the generated
/// implementation.
///
/// The specialization pipeline includes:
///
/// - [`NextAffiliateCounterReplacement`] for correcting terminal
///   successor projections,
/// - [`FloorAffiliatesCountersReplacement`] for rebuilding floor
///   affiliates from the boundary,
/// - [`BackAffiliatesCountersReplacement`] for correcting predecessor
///   recovery across rollover boundaries,
/// - [`AffiliateCountersCheckerReplacement`] for adapting affiliate
///   consistency validation for terminal instances,
/// - [`CumulatedConstCheckerReplacement`] for installing the recursive
///   cumulative compile-time checker on the final terminal instance.
/// - [`TerminalAccessAddon`] for generating the terminal-instance
///   typenum counter lookup addon.
/// - [`TerminalCheckerAddon`] for generating the cumulative
///   checker compile-time valuation constant,
/// - [`BoundaryAccessAddon`] for generating boundary-instance lookup
///   addons for the boundary counters (last instance declarations)
///
/// Together these replacements convert the generic implementation
/// produced by [`InstanceImpl`] into one that accurately models the
/// semantics of boundary and terminal instances.
#[derive(Clone, Debug)]
pub(crate) struct LastInstanceImpl;

impl<'a> Transformation<LastInstanceSpace<'a>, Option<&IntList>> for LastInstanceImpl {
    fn raw_transform(
        &self,
        transform: &mut LastInstanceSpace<'a>,
        context: &Option<&IntList>,
    ) -> Result<(), TokenStream> {
        let LastInstanceSpace {
            impl_of: transform,
            addons,
        } = transform;
        let index = &LastInstanceArg::checked_extract(transform, context)?.0;
        let counters = CounterArgsAsAssoc::checked_extract(transform, &())?.0;
        let counters_slc = counters.as_slice();
        let context = &(counters_slc, index);
        NextAffiliateCounterReplacement::checked_transform(
            &NextAffiliateCounterReplacement,
            transform,
            context,
        )?;
        FloorAffiliatesCountersReplacement::checked_transform(
            &FloorAffiliatesCountersReplacement,
            transform,
            context,
        )?;
        BackAffiliatesCountersReplacement::checked_transform(
            &BackAffiliatesCountersReplacement,
            transform,
            context,
        )?;
        AffiliateCountersCheckerReplacement::checked_transform(
            &AffiliateCountersCheckerReplacement,
            transform,
            context,
        )?;
        CumulatedConstCheckerReplacement::checked_transform(
            &CumulatedConstCheckerReplacement,
            transform,
            context,
        )?;
        TerminalAccessAddon::checked_transform(
            &TerminalAccessAddon,
            addons,
            &(transform, counters_slc, index),
        )?;
        TerminalCheckerAddon::checked_transform(
            &TerminalCheckerAddon,
            addons,
            &(transform, counters_slc, index),
        )?;
        BoundaryAccessAddon::checked_transform(
            &BoundaryAccessAddon,
            addons,
            &(transform, counters_slc, index),
        )?;
        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &LastInstanceSpace<'a>,
        context: Option<&Option<&IntList>>,
    ) -> Result<(), TokenStream> {
        let LastInstanceSpace {
            impl_of: transform,
            addons,
        } = transform;
        let context = match context {
            Some(Some(c)) => Some(*c),
            _ => None,
        };
        let index = &LastInstanceArg::checked_extract(transform, &context)?.0;
        let counters = CounterArgsAsAssoc::checked_extract(transform, &())?.0;
        let counters_slc = counters.as_slice();
        let context = Some(&(counters_slc, index));
        NextAffiliateCounterReplacement::validate_transform(
            &NextAffiliateCounterReplacement,
            transform,
            context,
        )?;
        FloorAffiliatesCountersReplacement::validate_transform(
            &FloorAffiliatesCountersReplacement,
            transform,
            context,
        )?;
        BackAffiliatesCountersReplacement::validate_transform(
            &BackAffiliatesCountersReplacement,
            transform,
            context,
        )?;
        AffiliateCountersCheckerReplacement::validate_transform(
            &AffiliateCountersCheckerReplacement,
            transform,
            context,
        )?;
        CumulatedConstCheckerReplacement::validate_transform(
            &CumulatedConstCheckerReplacement,
            transform,
            context,
        )?;
        TerminalAccessAddon::validate_transform(
            &TerminalAccessAddon,
            addons,
            Some(&(transform, counters_slc, index)),
        )?;
        TerminalCheckerAddon::validate_transform(
            &TerminalCheckerAddon,
            addons,
            Some(&(transform, counters_slc, index)),
        )?;
        BoundaryAccessAddon::validate_transform(
            &BoundaryAccessAddon,
            addons,
            Some(&(transform, counters_slc, index)),
        )?;
        Ok(())
    }
}
