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
// ``````````````````````` INSTANCE NODE DECLARATIVE UTILS ```````````````````````
// ===============================================================================

//! Shared declarative infrastructure for generated instance node
//! transformations.
//!
//! This module declares the common transformation framework shared by every
//! specialized instance node on both the declaration (trait) and
//! implementation (impl) sides.
//!
//! It provides:
//!
//! - Shared preparation and validation pipelines
//!   ([`TraitNodeSegment`], [`ImplNodeSegment`]).
//! - Mutable transformation state and helper APIs for generated companion
//!   nodes ([`TraitNodeState`], [`ImplNodeState`]).
//!
//! Rather than implementing the semantics of individual instance nodes, this
//! module declares the common lifecycle, generated companion node model, and
//! mutation interfaces used by all specialized node transformations.
//!
// ===============================================================================
// ``````````````````````````````````` IMPORTS ```````````````````````````````````
// ===============================================================================

// --- Std Crate ---
use std::marker::PhantomData;

// --- Proc Macro Crates ---
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    Expr, GenericArgument, ImplItem, ImplItemConst, ImplItemType, PathArguments, Token, TraitBound, TraitBoundModifier, TraitItem, TraitItemConst, TraitItemType, Type, TypeParamBound, TypePath, parse, parse_quote, punctuated::Punctuated, token::{Comma, Plus},
};

// --- Local Crate
use crate::{
    Extraction, Instance, Transformation, Utilization,
    impls::utils::ImplTraitPath,
    node::{
        access::TerminalAccess,
        args::*,
        errors::{StateBugs, StateError},
    },
    traits::meta::CumulatedConstChecker,
};

// --- Proc Suite ---
use proc_suite::{SupportCrate, misc::*};

// ===============================================================================
// `````````````````````````````````` CONSTANTS ``````````````````````````````````
// ===============================================================================

/// Deterministic suffix for the generated global marker associated type.
///
/// Every instance declaration generates a neighbouring global companion type.
/// Rather than contributing supplementary or instance addon bounds to the
/// original instance associated type itself, specialized instance nodes
/// contribute them to this global marker.
///
/// The global marker serves as the common aggregation point for addon traits
/// implemented by multiple instance nodes, while the original instance
/// associated type remains the publisher-facing declaration. See
/// [`crate::node::access`] for the available addon traits.
///
/// Example:
/// ```text
/// Instance:
///     type Account;
///
/// Generated:
///     type AccountGlobal;
///
/// Node contributions:
///     Leaf      -> AccountGlobal: CounterAccess
///     Branch    -> AccountGlobal: TerminalAccess
///     Traverse  -> AccountGlobal: OnSetAccess
/// ```
///
/// The publisher implementation supplies the concrete instance type, while
/// specialized instance nodes contribute supplementary bounds to the generated
/// global marker. When different bounds are required from those of the
/// original instance associated type, they may instead be expressed through
/// the generated twin companion type ([`TWIN_NODE`]).
pub(crate) const GLOBAL_NODE: Option<&[u8]> = Some(b"global");

/// Deterministic suffix for the generated twin companion associated type.
///
/// Every instance declaration generates a neighbouring twin companion type.
/// The publisher implementation resolves both the original instance and its
/// twin to the same concrete type, while allowing each to expose independent
/// bounds.
///
/// The twin primarily exists so companion types such as the generated global
/// marker ([`GLOBAL_NODE`]) may depend on supplementary bounds without
/// requiring those bounds on the original instance associated type.
///
/// Example:
/// ```text
/// Publisher:
///     type Account = RuntimeAccount;
///
/// Generated:
///     type AccountTwin = RuntimeAccount;
///
/// Bounds:
///     Account      : PublisherBounds
///     AccountTwin  : CounterAccess
///                    OnSetAccess
///
///     AccountGlobal:
///         CounterAccess
///         OnSetAccess
/// ```
///
/// Although both associated types resolve to the same concrete type, the twin
/// provides an independent bound surface that specialized instance nodes may
/// constrain without coupling those constraints to the original instance
/// declaration.
pub(crate) const TWIN_NODE: Option<&[u8]> = Some(b"twin");

/// Deterministic suffix for the generated checker companion associated
/// constant.
///
/// Every instance declaration generates a neighbouring checker companion
/// constant used for compile-time validation. The generated declaration-side
/// checker defaults to `()`, taking advantage of stable Rust support for
/// associated constant defaults.
///
/// Specialized instance nodes and publisher implementations may override this
/// constant to perform additional compile-time validation of the generated
/// companion types (such as [`GLOBAL_NODE`] and [`TWIN_NODE`]) or the original
/// instance declaration.
///
/// Example:
/// ```text
/// Trait:
///     type Account;
///     type AccountGlobal;
///     type AccountTwin;
///
///     const AccountChecker: () = ();
///
/// Leaf node:
///     const AccountChecker: () = {
///         assert_leaf::<
///             Self::AccountGlobal,
///             Self::AccountTwin,
///         >();
///     };
///
/// Publisher:
///     const AccountChecker: () = {
///         assert_instance::<Self::Account>();
///     };
/// ```
pub(crate) const CHECKER_NODE: Option<&[u8]> = Some(b"checker");

/// Deterministic suffix for the generated terminal global companion
/// associated type.
///
/// The terminal instance generates its own neighbouring global companion
/// type. Unlike ordinary instance globals ([`GLOBAL_NODE`]), the terminal
/// global is reserved exclusively for the terminal instance and is not
/// extended directly by specialized instance nodes.
///
/// Instead, it represents the completed terminal instance after every
/// specialized instance node has contributed to the terminal companion
/// associated types.
///
/// Example:
/// ```text
/// Terminal Instance:
///     type Terminal;
///
/// Generated:
///     type TerminalGlobal;
///
/// TerminalGlobal:
///     TerminalAccess
/// ```
///
/// See [`crate::node::access::TerminalAccess`] for the terminal addon trait
/// exposed by the terminal global marker.
///
/// The terminal global serves as the system-defined aggregation point for the
/// completed terminal instance and is intended to be consumed by terminal
/// validation and companion types such as [`TERMINAL_TWIN_NODE`] and
/// [`TERMINAL_CHECKER_NODE`].
pub(crate) const TERMINAL_GLOBAL_NODE: Option<&[u8]> = Some(b"term_global");

/// Deterministic suffix for the generated terminal twin companion associated
/// type.
///
/// The terminal instance generates a neighbouring terminal twin companion
/// type. The publisher implementation resolves both the terminal instance and
/// its twin to the same concrete type, while allowing the terminal twin to
/// expose independent supplementary bounds.
///
/// The terminal twin primarily exists so the generated terminal global
/// ([`TERMINAL_GLOBAL_NODE`]) may depend on supplementary bounds without
/// requiring those bounds on the terminal instance associated type itself.
///
/// Example:
/// ```text
/// Publisher:
///     type Instance = RuntimeInstance;
///
/// Generated:
///     type TerminalTwin = RuntimeInstance;
///
/// Bounds:
///     Terminal      : PublisherBounds
///     TerminalGlobal: TerminalAccess
///     TerminalTwin  : OtherBounds
/// ```
///
/// Although both associated types resolve to the same concrete type, the
/// terminal twin provides an independent bound surface for the completed
/// terminal instance without coupling those constraints to the terminal
/// instance declaration.
pub(crate) const TERMINAL_TWIN_NODE: Option<&[u8]> = Some(b"term_twin");

/// Deterministic suffix for the generated terminal checker companion
/// associated constant.
///
/// Every terminal instance generates a neighbouring terminal checker companion
/// constant used for cumulative compile-time validation. The generated
/// declaration-side checker defaults to `()`, taking advantage of stable Rust
/// support for associated constant defaults.
///
/// The default expression validates the completed terminal companion types
/// through the generated terminal global ([`TERMINAL_GLOBAL_NODE`]) and
/// terminal twin ([`TERMINAL_TWIN_NODE`]), thereby validating the fully
/// constructed terminal instance.
///
/// Example:
/// ```text
/// Trait:
///     type Terminal;
///     type TerminalGlobal;
///     type TerminalTwin;
///
///     const TerminalChecker: () = {
///         assert_terminal::<
///             Self::TerminalGlobal,
///             Self::TerminalTwin,
///         >();
///     };
///
/// Publisher:
///     type Terminal = RuntimeTerminal;
///     type TerminalTwin = RuntimeTerminal;
/// ```
pub(crate) const TERMINAL_CHECKER_NODE: Option<&[u8]> = Some(b"term_checker");

/// Deterministic suffix for the generated initial range companion associated
/// type.
///
/// Range-based instance nodes generate a neighbouring initial companion type
/// representing the beginning of the selected instance range.
///
/// Unlike the original instance associated type, the initial companion may
/// depend on other generated companion types such as the instance global
/// ([`GLOBAL_NODE`]) and twin ([`TWIN_NODE`]). This allows specialized
/// instance nodes to contribute supplementary bounds without coupling them to
/// the original instance declaration.
///
/// The initial companion exposes the beginning of the selected range through
/// an associated type, allowing the original instance associated type to
/// recover the initial instance path.
///
/// Example:
/// ```text
/// Traverse:
///     Leaf ---> Branch
///
/// Instance:
///     type Account;
///
/// Generated:
///     type AccountInitial: CounterAccess
///
///     Account:
///         type Initial =
///             <Self::AccountInitial as InitialAssocAccess>::Initial;
/// ```
///
/// Together with [`FINAL_NODE`], the initial companion encodes the bounds of
/// a selected instance range while the publisher implementation supplies the
/// concrete instance associated type.
pub(crate) const INITIAL_NODE: Option<&[u8]> = Some(b"initial");

/// Deterministic suffix for the generated final range companion associated
/// type.
///
/// Range-based instance nodes generate a neighbouring final companion type
/// representing the end of the selected instance range.
///
/// Unlike the original instance associated type, the final companion may
/// depend on other generated companion types such as the instance global
/// ([`GLOBAL_NODE`]) and twin ([`TWIN_NODE`]). This allows specialized
/// instance nodes to contribute supplementary bounds without coupling them to
/// the original instance declaration.
///
/// The final companion exposes the end of the selected range through an
/// associated type, allowing the original instance associated type to recover
/// the final instance path.
///
/// Example:
/// ```text
/// Descend:
///     Branch -----> Leaf
///
/// Instance:
///     type Account;
///
/// Generated:
///     type AccountFinal;
///
///     AccountFinal: BoundaryAccess
///
///     Account:
///         type Final =
///             <Self::AccountFinal as FinalAccess>::Final;
/// ```
///
/// Together with [`INITIAL_NODE`], the final companion encodes the bounds of
/// a selected instance range while the publisher implementation supplies the
/// concrete instance associated type.
pub(crate) const FINAL_NODE: Option<&[u8]> = Some(b"final");

// ===============================================================================
// ``````````````````````````````````` MARKERS ```````````````````````````````````
// ===============================================================================

/// Marker type used for compile-time separation of ordinary instance node
/// transformations. Primarily serves as a distinct type identifier in generic
/// contexts.
pub(crate) struct InstanceNode;

/// Marker type used for compile-time separation of terminal instance node
/// transformations. Primarily serves as a distinct type identifier in generic
/// contexts.
pub(crate) struct InstanceTerminalNode;

// ===============================================================================
// `````````````````````` INSTANCE TRAIT NODE SEGMENT UTILS ``````````````````````
// ===============================================================================

/// Mutable transformation state for a generated trait instance node.
///
/// This state provides mutable access to the trait currently being
/// transformed while carrying a compile-time marker identifying the active
/// instance node segment.
///
/// The marker type `K` distinguishes specialized instance node segments (for
/// example, leaf, branch, prune, trim, extend, spread, traverse, descend,
/// and root), allowing helper methods and generated companion nodes to be
/// specialized without introducing runtime state.
///
/// A [`TraitNodeState`] is created only after the common preparation phase
/// has completed. At this point the standard companion associated items have
/// already been synthesized, including:
///
/// - Global companion type ([`GLOBAL_NODE`])
/// - Twin companion type ([`TWIN_NODE`])
/// - Checker companion constant ([`CHECKER_NODE`])
/// - Initial range companion type ([`INITIAL_NODE`])
/// - Final range companion type ([`FINAL_NODE`])
/// - Terminal global companion type ([`TERMINAL_GLOBAL_NODE`])
/// - Terminal twin companion type ([`TERMINAL_TWIN_NODE`])
/// - Terminal checker companion constant ([`TERMINAL_CHECKER_NODE`])
///
/// Specialized instance node segments receive this state to inspect, mutate,
/// and extend the generated companion associated items (neighbour nodes)
/// associated with the current instance declaration.
#[derive(Debug)]
pub(crate) struct TraitNodeState<'a, 'b, K>(&'b mut TraitNodeSpace<'a>, PhantomData<K>);

/// Common transformation interface for generated trait instance node
/// segments.
///
/// A trait instance node segment represents one stage of the instance node
/// transformation pipeline. Every specialized node segment shares the same
/// preparation and validation logic.
///
/// Before a specialized segment executes, the default preparation phase:
///
/// - Validates the source instance associated type.
/// - Generates the standard companion associated items.
/// - Generates the terminal companion associated items.
/// - Initializes default checker expressions.
/// - Constructs terminal global and twin relationships.
/// - Creates the mutable [`TraitNodeState`] passed to the specialized
///   transformation.
///
/// During validation, the segment verifies that every generated companion
/// associated item still satisfies the structural invariants expected by the
/// instance node system, including generated identifiers, generic
/// parameters, companion relationships, terminal bindings, and default
/// checker expressions.
///
/// This allows every specialized instance node to focus solely on its own
/// transformation while sharing a common generation and validation
/// infrastructure.
pub(crate) trait TraitNodeSegment<'a, 'b, 'c, C: GetCounterGenericIndexes<'b>>:
    Transformation<TraitNodeSpace<'a>, C>
{
    /// Performs the common preparation phase for a trait instance node
    /// transformation.
    ///
    /// Before the specialized instance node executes, this method validates the
    /// source instance associated type and synthesizes every standard companion
    /// node required by the instance node system.
    ///
    /// The preparation phase:
    ///
    /// - Validates the structural requirements of the source instance
    ///   associated type.
    /// - Generates the standard companion nodes:
    ///   - [`GLOBAL_NODE`]
    ///   - [`TWIN_NODE`]
    ///   - [`CHECKER_NODE`]
    ///   - [`INITIAL_NODE`]
    ///   - [`FINAL_NODE`]
    /// - Generates the terminal companion nodes:
    ///   - [`TERMINAL_GLOBAL_NODE`]
    ///   - [`TERMINAL_TWIN_NODE`]
    ///   - [`TERMINAL_CHECKER_NODE`]
    /// - Initializes the default terminal relationships and checker
    ///   expressions.
    /// - Constructs a [`TraitNodeState`] exposing the generated companion nodes.
    ///
    /// Once preparation is complete, the supplied callback receives the mutable
    /// transformation state, allowing the specialized instance node segment to
    /// inspect, mutate, or extend the generated companion nodes.
    fn prepare<A>(
        from: &'c mut TraitNodeSpace<'a>,
        context: &'b C,
        access: A,
    ) -> Result<(), TokenStream>
    where
        A: FnOnce(&mut TraitNodeState<'a, 'c, Self>) -> Result<(), TokenStream>,
    {
        let TraitNodeSpace {
            trait_of, ty_idx, ..
        } = from;
        let Some(trait_item) = trait_of.items.get_mut(*ty_idx) else {
            return Err(StateBugs::TraitItemIdxFetchFailed {}.into());
        };
        let TraitItem::Type(assoc_ty) = trait_item else {
            return Err(StateBugs::FetchedTraitItemNotTypeAssoc {}.into());
        };
        let assoc_clone = assoc_ty.clone();
        let indexes = context.get()?;

        let Some(bound) = assoc_ty.bounds.first_mut() else {
            return Err(StateError::FoundNoBoundForInstance {
                assoc: assoc_clone.clone(),
            }
            .into());
        };

        let TypeParamBound::Trait(trait_bound) = bound else {
            return Err(StateError::RequiresBoundAsInstanceTraitBound {
                assoc: assoc_clone,
                bound: bound.clone(),
            }
            .into());
        };
        let trait_bound_clone = trait_bound.clone();

        let TraitBoundModifier::None = trait_bound.modifier else {
            return Err(StateError::ExpectedInstanceTraitBoundInvalid {
                assoc: assoc_clone,
                trait_bound: trait_bound_clone,
            }
            .into());
        };

        if trait_bound.lifetimes.is_some() {
            return Err(StateError::HrtbInstanceTraitsUnavailable {
                assoc: assoc_clone,
                trait_bound: trait_bound_clone,
            }
            .into());
        }

        let Some(_) = trait_bound.path.segments.last_mut() else {
            return Err(StateError::InstanceTraitBoundLastSegUnavailable {
                assoc: assoc_clone,
                trait_bound: trait_bound_clone,
            }
            .into());
        };

        let cloned_trait_bound = trait_bound.clone();
        let generics = &assoc_clone.generics;
        let base_ident = &assoc_clone.ident;

        let global = TraitItemType {
            attrs: proc_suite::internal_code(),
            ident: gen_type_ident_from_ident_with_suffix::<InstanceNode>(base_ident, GLOBAL_NODE),
            default: None,
            type_token: Default::default(),
            generics: generics.clone(),
            colon_token: Default::default(),
            bounds: Default::default(),
            semi_token: Default::default(),
        };

        let twin = TraitItemType {
            attrs: proc_suite::internal_code(),
            ident: gen_type_ident_from_ident_with_suffix::<InstanceNode>(base_ident, TWIN_NODE),
            default: None,
            type_token: Default::default(),
            generics: generics.clone(),
            colon_token: Default::default(),
            bounds: Default::default(),
            semi_token: Default::default(),
        };

        let checker = TraitItemConst {
            attrs: proc_suite::internal_code(),
            ident: gen_const_ident_from_ident_with_suffix::<InstanceNode>(base_ident, CHECKER_NODE),
            default: Some((Default::default(), parse_quote!(()))),
            ty: parse_quote!(()),
            generics: generics.clone(),
            colon_token: Default::default(),
            semi_token: Default::default(),
            const_token: Default::default(),
        };

        let initial = TraitItemType {
            attrs: proc_suite::internal_code(),
            ident: gen_type_ident_from_ident_with_suffix::<InstanceNode>(base_ident, INITIAL_NODE),
            default: None,
            type_token: Default::default(),
            generics: generics.clone(),
            colon_token: Default::default(),
            bounds: Default::default(),
            semi_token: Default::default(),
        };

        let finalz = TraitItemType {
            attrs: proc_suite::internal_code(),
            ident: gen_type_ident_from_ident_with_suffix::<InstanceNode>(base_ident, FINAL_NODE),
            default: None,
            type_token: Default::default(),
            generics: generics.clone(),
            colon_token: Default::default(),
            bounds: Default::default(),
            semi_token: Default::default(),
        };

        trait_of.items.push(TraitItem::Type(global));
        trait_of.items.push(TraitItem::Type(twin));
        trait_of.items.push(TraitItem::Const(checker));
        trait_of.items.push(TraitItem::Type(initial));
        trait_of.items.push(TraitItem::Type(finalz));

        // Defaults

        let (_, ty_gen, _) = generics.split_for_impl();

        let terminal_global = gen_type_ident_from_ident_with_suffix::<InstanceTerminalNode>(
            base_ident,
            TERMINAL_GLOBAL_NODE,
        );
        let terminal_global_assoc: Type = parse_quote!(Self:: #terminal_global #ty_gen);

        let terminal_twin = gen_type_ident_from_ident_with_suffix::<InstanceTerminalNode>(
            base_ident,
            TERMINAL_TWIN_NODE,
        );
        let terminal_twin_assoc: Type = parse_quote!(Self:: #terminal_twin #ty_gen);
        let base_assoc: Type = parse_quote!(Self:: #base_ident #ty_gen);

        let term_context = (&base_assoc, indexes).into();
        let term_access = TerminalAccess::checked_extract(&term_context, &())?;

        let mut terminal_global_bounds = Punctuated::<TypeParamBound, Plus>::new();
        terminal_global_bounds.push(TypeParamBound::Trait(term_access.0.clone()));
        let terminal_global = TraitItemType {
            attrs: proc_suite::internal_code(),
            ident: terminal_global,
            default: None,
            type_token: Default::default(),
            generics: generics.clone(),
            colon_token: Default::default(),
            bounds: terminal_global_bounds,
            semi_token: Default::default(),
        };

        trait_of.items.push(TraitItem::Type(terminal_global));

        let mut terminal_twin_bounds = Punctuated::<TypeParamBound, Plus>::new();
        let mut terminal_twin_bound = cloned_trait_bound.clone();
        let trait_path = terminal_twin_bound.path.segments.last_mut().unwrap();
        let transform_context = (&terminal_global_assoc, term_context, false).into();
        TerminalAccess::checked_transform(&term_access, trait_path, &transform_context)?;
        terminal_twin_bounds.push(TypeParamBound::Trait(terminal_twin_bound.clone()));

        let terminal_twin = TraitItemType {
            attrs: proc_suite::internal_code(),
            ident: terminal_twin,
            default: None,
            type_token: Default::default(),
            generics: generics.clone(),
            colon_token: Default::default(),
            bounds: terminal_twin_bounds,
            semi_token: Default::default(),
        };

        trait_of.items.push(TraitItem::Type(terminal_twin));

        if !assoc_clone.generics.params.is_empty() {
            return Err(StateError::GATInstanceAssocTyUnavailable { assoc: assoc_clone }.into());
        }

        let terminal_checker = gen_const_ident_from_ident_with_suffix::<InstanceTerminalNode>(
            base_ident,
            TERMINAL_CHECKER_NODE,
        );
        let _terminal_checker_assoc: Type = parse_quote!(Self:: #terminal_checker #ty_gen);

        // since const-items doesn't support assoc equality nor constraints
        let mut twin_bound_stripped = terminal_twin_bound.clone();
        let PathArguments::AngleBracketed(angle) = &mut twin_bound_stripped.path.segments.last_mut().unwrap().arguments else {unreachable!()};
        let mut build: Punctuated<GenericArgument, Comma> = Punctuated::new();
        for arg in &angle.args {
            match arg {
                GenericArgument::AssocType(_) 
                | GenericArgument::AssocConst(_) 
                | GenericArgument::Constraint(_) => {},
                arg => build.push(arg.clone()),
            }
        }
        angle.args = build;

        let twin_bound_path = &twin_bound_stripped.path;
        let cumulated_check = gen_const_ident::<CumulatedConstChecker>();
        let expr: Expr = parse_quote!(<#terminal_twin_assoc as #twin_bound_path>::#cumulated_check);
        let terminal_checker = TraitItemConst {
            attrs: proc_suite::internal_code(),
            ident: terminal_checker,
            default: Some((Default::default(), expr)),
            ty: parse_quote!(()),
            generics: generics.clone(),
            colon_token: Default::default(),
            semi_token: Default::default(),
            const_token: Default::default(),
        };

        trait_of.items.push(TraitItem::Const(terminal_checker));

        access(&mut TraitNodeState(from, PhantomData))?;

        Ok(())
    }

    /// Validates the generated companion nodes after a trait instance node
    /// transformation has completed.
    ///
    /// Validation reconstructs the expected companion node layout and verifies
    /// that every generated declaration still satisfies the structural
    /// invariants required by the instance node system.
    ///
    /// The validation phase verifies:
    ///
    /// - The original instance associated type.
    /// - Generated companion node identifiers.
    /// - Generated generic parameter lists.
    /// - Terminal companion relationships.
    /// - Terminal checker expressions.
    /// - Generated access transformations.
    /// - Generated terminal access transformations.
    ///
    /// A cloned transformation state is then provided to the supplied callback,
    /// allowing specialized instance node segments to perform additional
    /// validation of their generated companion nodes without mutating the
    /// original trait.
    ///
    /// Successful validation guarantees that the transformed trait preserves the
    /// expected companion node structure established during the preparation
    /// phase.
    fn validate<A>(
        from: &'c TraitNodeSpace<'a>,
        context: Option<&'b C>,
        access: A,
    ) -> Result<(), TokenStream>
    where
        A: FnOnce(&TraitNodeState<'_, '_, Self>) -> Result<(), TokenStream>,
    {
        let TraitNodeSpace {
            trait_of,
            ty_idx,
            addons,
        } = from;

        let mut trait_of = (**trait_of).clone();

        let Some(trait_item) = &trait_of.items.get(*ty_idx) else {
            return Err(StateBugs::TraitItemIdxFetchFailed {}.into());
        };
        let TraitItem::Type(assoc_ty) = trait_item else {
            return Err(StateBugs::FetchedTraitItemNotTypeAssoc {}.into());
        };

        let Some(bound) = assoc_ty.bounds.first() else {
            return Err(StateBugs::FoundNoBoundForInstance {}.into());
        };

        let TypeParamBound::Trait(trait_bound) = bound else {
            return Err(StateBugs::RequiresBoundAsInstanceTraitBound {}.into());
        };

        let TraitBoundModifier::None = trait_bound.modifier else {
            return Err(StateBugs::ExpectedInstanceTraitBoundInvalid {}.into());
        };

        if trait_bound.lifetimes.is_some() {
            return Err(StateBugs::HrtbInstanceTraitsUnavailable {}.into());
        }

        let Some(segment) = &trait_bound.path.segments.last() else {
            return Err(StateBugs::InstanceTraitBoundLastSegUnavailable {}.into());
        };

        if let Some(context) = context {
            let indexes = context.get()?;
            let Some(first) = indexes.ints.first() else {
                return Err(StateBugs::ValidateContextFirstOfIndexesUnavailable {}.into());
            };
            let PathArguments::AngleBracketed(angle) = &segment.arguments else {
                return Err(StateBugs::InstanceTraitBoundNotAngleArgs {}.into());
            };
            let Some(arg) = angle.args.get(parse_pos_usize(first)?) else {
                return Err(StateBugs::InstanceTraitBoundInstanceTypeNumArgUnavailable {}.into());
            };
            let GenericArgument::Type(_) = arg else {
                return Err(StateBugs::InstanceTraitBoundInstanceTypeNumArgNotType {}.into());
            };
        }

        let generics = &assoc_ty.generics;
        let base_ident = &assoc_ty.ident;

        let global = gen_type_ident_from_ident_with_suffix::<InstanceNode>(base_ident, GLOBAL_NODE);
        match trait_of.items.iter().find(|item| {
            matches!(item, TraitItem::Type(ty)
                if ty.ident == global)
        }) {
            Some(TraitItem::Type(ty)) => {
                if ty.generics.to_token_stream().to_string()
                    != generics.to_token_stream().to_string()
                {
                    return Err(StateBugs::GlobalNodeInvalidGenerics {}.into());
                }
            }
            _ => return Err(StateBugs::GlobalNodeNotFound {}.into()),
        }

        let twin = gen_type_ident_from_ident_with_suffix::<InstanceNode>(base_ident, TWIN_NODE);
        match trait_of.items.iter().find(|item| {
            matches!(item, TraitItem::Type(ty)
                if ty.ident == twin)
        }) {
            Some(TraitItem::Type(ty)) => {
                if ty.generics.to_token_stream().to_string()
                    != generics.to_token_stream().to_string()
                {
                    return Err(StateBugs::TwinNodeInvalidGenerics {}.into());
                }
            }
            _ => return Err(StateBugs::TwinNodeNotFound {}.into()),
        }

        let initial =
            gen_type_ident_from_ident_with_suffix::<InstanceNode>(base_ident, INITIAL_NODE);
        match trait_of.items.iter().find(|item| {
            matches!(item, TraitItem::Type(ty)
                if ty.ident == initial)
        }) {
            Some(TraitItem::Type(ty)) => {
                if ty.generics.to_token_stream().to_string()
                    != generics.to_token_stream().to_string()
                {
                    return Err(StateBugs::InitialNodeInvalidGenerics {}.into());
                }
            }
            _ => return Err(StateBugs::InitialNodeNotFound {}.into()),
        }

        let finalz = gen_type_ident_from_ident_with_suffix::<InstanceNode>(base_ident, FINAL_NODE);
        match trait_of.items.iter().find(|item| {
            matches!(item, TraitItem::Type(ty)
                if ty.ident == finalz)
        }) {
            Some(TraitItem::Type(ty)) => {
                if ty.generics.to_token_stream().to_string()
                    != generics.to_token_stream().to_string()
                {
                    return Err(StateBugs::FinalNodeInvalidGenerics {}.into());
                }
            }
            _ => return Err(StateBugs::FinalNodeNotFound {}.into()),
        }

        let checker =
            gen_const_ident_from_ident_with_suffix::<InstanceNode>(base_ident, CHECKER_NODE);
        match trait_of.items.iter().find(|item| {
            matches!(item, TraitItem::Const(c)
                if c.ident == checker)
        }) {
            Some(TraitItem::Const(c)) => {
                if c.generics.to_token_stream().to_string()
                    != generics.to_token_stream().to_string()
                {
                    return Err(StateBugs::CheckerNodeInvalidGenerics {}.into());
                }
            }
            _ => return Err(StateBugs::CheckerNodeNotFound {}.into()),
        }

        let terminal_twin_ident = gen_type_ident_from_ident_with_suffix::<InstanceTerminalNode>(
            base_ident,
            TERMINAL_TWIN_NODE,
        );

        let terminal_twin = match trait_of.items.iter().find(|item| {
            matches!(item, TraitItem::Type(ty)
                if ty.ident == terminal_twin_ident)
        }) {
            Some(TraitItem::Type(ty)) => {
                if ty.generics.to_token_stream().to_string()
                    != generics.to_token_stream().to_string()
                {
                    Err(<StateBugs as Into<TokenStream>>::into(
                        StateBugs::TerminalTwinNodeInvalidGenerics {},
                    ))
                } else {
                    Ok(ty)
                }
            }
            _ => Err(<StateBugs as Into<TokenStream>>::into(
                StateBugs::TerminalTwinNodeNotFound {},
            )),
        }?;

        let terminal_global_ident = gen_type_ident_from_ident_with_suffix::<InstanceTerminalNode>(
            base_ident,
            TERMINAL_GLOBAL_NODE,
        );

        let terminal_global = match trait_of.items.iter().find(|item| {
            matches!(item, TraitItem::Type(ty)
                if ty.ident == terminal_global_ident)
        }) {
            Some(TraitItem::Type(ty)) => {
                if ty.generics.to_token_stream().to_string()
                    != generics.to_token_stream().to_string()
                {
                    Err(<StateBugs as Into<TokenStream>>::into(
                        StateBugs::TerminalGlobalNodeInvalidGenerics {},
                    ))
                } else {
                    Ok(ty)
                }
            }
            _ => Err(<StateBugs as Into<TokenStream>>::into(
                StateBugs::TerminalGlobalNodeNotFound {},
            )),
        }?;

        let terminal_checker_ident = gen_const_ident_from_ident_with_suffix::<InstanceTerminalNode>(
            base_ident,
            TERMINAL_CHECKER_NODE,
        );

        let terminal_checker = match trait_of.items.iter().find(|item| {
            matches!(item, TraitItem::Const(c)
                if c.ident == terminal_checker_ident)
        }) {
            Some(TraitItem::Const(c)) => {
                if c.generics.to_token_stream().to_string()
                    != generics.to_token_stream().to_string()
                {
                    Err(<StateBugs as Into<TokenStream>>::into(
                        StateBugs::TerminalCheckerNodeInvalidGenerics {},
                    ))
                } else {
                    Ok(c)
                }
            }
            _ => Err(<StateBugs as Into<TokenStream>>::into(
                StateBugs::TerminalCheckerNodeNotFound {},
            )),
        }?;

        let (_, ty_gen, _) = generics.split_for_impl();

        let base_assoc: Type = parse_quote!(Self:: #base_ident #ty_gen);
        let terminal_twin_assoc: Type = parse_quote!(Self:: #terminal_twin_ident #ty_gen);
        let terminal_global_assoc: Type = parse_quote!(Self:: #terminal_global_ident #ty_gen);
        let _terminal_checker_assoc: Type = parse_quote!(Self:: #terminal_checker_ident #ty_gen);

        // Validate Global's Terminal Bound
        if terminal_global.bounds.len() > 1 {
            return Err(StateBugs::TerminalGlobalBoundsMoreThanSingleTerminalBound {}.into());
        }
        let bound = &terminal_global.bounds[0];
        let TypeParamBound::Trait(trait_bound) = bound else {
            return Err(StateBugs::TerminalTwinBoundNotTraitBound {}.into());
        };
        let indexes = match context {
            Some(c) => c.get()?,
            None => &Default::default(),
        };
        let term_context = (&base_assoc, indexes).into();
        let terminal_global_bound = TerminalAccess(trait_bound.clone());
        TerminalAccess::validate_extract(&terminal_global_bound, &term_context, None)?;

        // Validate Terminal Twin's Bound
        if terminal_twin.bounds.len() > 1 {
            return Err(StateBugs::TerminalTwinBoundsMoreThanSingleTerminalBound {}.into());
        }
        let bound = &terminal_twin.bounds[0];
        let TypeParamBound::Trait(trait_bound) = bound else {
            return Err(StateBugs::TerminalTwinBoundNotTraitBound {}.into());
        };

        let Some(inst_bound_seg) = trait_bound.path.segments.last() else {
            return Err(StateBugs::NonReplacedInstanceBoundLastPathSegNotFound {}.into());
        };
        let transform_context = (&terminal_global_assoc, term_context, true).into();
        TerminalAccess::validate_transform(
            &terminal_global_bound,
            inst_bound_seg,
            Some(&transform_context),
        )?;

        // Validate Terminal Checker
        let cumulated_check = gen_const_ident::<CumulatedConstChecker>();
        let Some((_, expr)) = &terminal_checker.default else {
            return Err(StateBugs::TerminalCheckerDefaultExprNotFound {}.into());
        };
        let Expr::Path(p) = expr else {
            return Err(StateBugs::TerminalCheckerDefaultNotExprPath {}.into());
        };
        let Some(q_self) = &p.qself else {
            return Err(StateBugs::TerminalCheckerDefaultExprPathQSelfNotFound {}.into());
        };
        if q_self.ty.to_token_stream().to_string()
            != terminal_twin_assoc.to_token_stream().to_string()
        {
            return Err(StateBugs::TerminalCheckerDefaultExprPathQSelfInvalid {}.into());
        }
        let pos = q_self.position;
        let mut path_cloned = p.path.clone();
        let Some(last) = path_cloned.segments.get(pos) else {
            return Err(StateBugs::TerminalCheckerDefaultExprPathLastSegNotFound {}.into());
        };
        if last.ident != cumulated_check {
            return Err(StateBugs::TerminalCheckerDefaultExprPathLastSegInvalid {}.into());
        }
        path_cloned.segments.pop();
        path_cloned.segments.pop_punct();

        // since const-items doesn't support assoc equality nor constraints
        let mut twin_bound_stripped = trait_bound.clone();
        let PathArguments::AngleBracketed(angle) = &mut twin_bound_stripped.path.segments.last_mut().unwrap().arguments else {unreachable!()};
        let mut build: Punctuated<GenericArgument, Comma> = Punctuated::new();
        for arg in &angle.args {
            match arg {
                GenericArgument::AssocType(_) 
                | GenericArgument::AssocConst(_) 
                | GenericArgument::Constraint(_) => {},
                arg => build.push(arg.clone()),
            }
        }
        angle.args = build;

        if path_cloned.to_token_stream().to_string() != twin_bound_stripped.to_token_stream().to_string() {
            return Err(StateBugs::TerminalCheckerDefaultExprPathQSelfTraitBoundInvalid {}.into());
        }

        let mut mut_addons = (**addons).clone();
        let mut dup = TraitNodeSpace {
            trait_of: &mut trait_of,
            ty_idx: *ty_idx,
            addons: &mut mut_addons,
        };

        access(&TraitNodeState(&mut dup, PhantomData))?;

        Ok(())
    }
}

impl<'a, 'b, 'c, C, T> TraitNodeSegment<'a, 'b, 'c, C> for T
where
    C: GetCounterGenericIndexes<'b>,
    T: Transformation<TraitNodeSpace<'a>, C>,
{
}

/// Convenience accessors and mutation utilities for generated companion
/// associated items within a instance node trait transformation.
///
/// These helpers provide a uniform interface for locating, mutating, and
/// referencing the generated companion nodes associated with the current
/// instance declaration. Rather than repeatedly traversing the trait, each
/// helper deterministically resolves the companion item generated during the
/// preparation phase.
///
/// The API is organized into three categories:
///
/// - **Base instance**
///   - `get_base`
///   - `get_base_assoc`
///   - `get_instance_bounds`
///   - `replace_instance_bounds`
///
/// - **Generated companion nodes**
///   - Global ([`GLOBAL_NODE`])
///   - Twin ([`TWIN_NODE`])
///   - Checker ([`CHECKER_NODE`])
///   - Initial ([`INITIAL_NODE`])
///   - Final ([`FINAL_NODE`])
///   - Terminal Global ([`TERMINAL_GLOBAL_NODE`])
///   - Terminal Twin ([`TERMINAL_TWIN_NODE`])
///   - Terminal Checker ([`TERMINAL_CHECKER_NODE`])
///
/// For each generated companion node, the state exposes the operations
/// supported by that node:
///
/// - `get_*` - retrieves the generated associated item.
/// - `get_*_assoc` - constructs a `Self::Assoc<...>` type referencing the
///   generated associated item.
/// - `mutate_*` - available only for companion nodes whose generated
///   declarations are intended to be customized, providing controlled mutation
///   while preserving structural invariants.
///
/// These helpers are intended exclusively for specialized instance node
/// segments implementing [`TraitNodeSegment`], allowing transformations to
/// focus on semantic changes rather than repeatedly searching the trait for
/// generated companion items.
#[allow(unused)]
impl<'a, 'c, K> TraitNodeState<'a, 'c, K> {
    /// Replaces the contiguous sequence of bounds belonging to the original
    /// instance associated type.
    ///
    /// The provided closure receives the existing instance bounds and returns the
    /// replacement sequence. Structural validation ensures every replacement
    /// bound still refers to the original instance associated type.
    ///
    /// Expected and assumes the first bound is an instance bound unless
    /// validated externally and confirmed.
    ///
    /// Returns the previously associated instance bounds.
    pub fn replace_instance_bounds<M>(&mut self, mutate: M) -> Result<Vec<TraitBound>, TokenStream>
    where
        M: FnOnce(Vec<&TraitBound>) -> Result<Vec<TraitBound>, TokenStream>,
    {
        let exists = Self::get_instance_bounds(&self)?;
        let index = exists.len().saturating_sub(1);
        let bounds = mutate(exists)?;

        let TraitNodeSpace {
            trait_of, ty_idx, ..
        } = self.0;
        let TraitItem::Type(item) = trait_of.items.get_mut(*ty_idx).unwrap() else {
            return Err(StateBugs::StateInstanceNodeUnavailable {}.into());
        };
        let TypeParamBound::Trait(bound) = item.bounds.first().unwrap() else {
            return Err(StateBugs::StateInstanceNodeBoundUnavailable {}.into());
        };

        let mut new_bounds = Punctuated::<TypeParamBound, Plus>::new();
        let orig_ident = &bound.path.segments.last().unwrap().ident;
        for bound in bounds {
            let Some(seg) = bound.path.segments.last() else {
                return Err(
                    StateBugs::StateInstanceNodeReplacementBoundLastSegUnavailable {}.into(),
                );
            };

            if seg.ident != *orig_ident {
                return Err(StateBugs::StateInstanceNodeReplacementBoundIdentInvalid {}.into());
            };

            match seg.arguments {
                PathArguments::AngleBracketed(_) => {}
                _ => {
                    return Err(StateBugs::StateInstanceNodeReplacementBoundNotAngleArgs {}.into());
                }
            };

            new_bounds.push(TypeParamBound::Trait(bound));
        }

        let mut old_bounds = Vec::new();
        for (i, bound) in item.bounds.iter().enumerate() {
            if i <= index {
                let TypeParamBound::Trait(bound) = bound else {
                    return Err(StateBugs::StateInstanceOldNodeBoundNotTraitBound {}.into());
                };
                old_bounds.push(bound.clone());
                continue;
            }
            new_bounds.push(bound.clone())
        }
        item.bounds = new_bounds;
        Ok(old_bounds)
    }

    /// Returns the contiguous sequence of bounds belonging to the original
    /// instance associated type.
    ///
    /// Collection stops once a bound no longer refers to the original instance
    /// (starting from 1st bound), leaving generated companion node bounds untouched.
    ///
    /// Assumes the first bound is an instance bound unless validated externally and
    /// confirmed.
    pub fn get_instance_bounds(&self) -> Result<Vec<&TraitBound>, TokenStream> {
        let TraitNodeState(
            TraitNodeSpace {
                trait_of, ty_idx, ..
            },
            _,
        ) = self;
        let TraitItem::Type(item) = trait_of.items.get(*ty_idx).unwrap() else {
            return Err(StateBugs::StateInstanceNodeUnavailable {}.into());
        };
        let mut bound_iter = item.bounds.iter();
        let TypeParamBound::Trait(bound) = bound_iter.next().unwrap() else {
            return Err(StateBugs::StateInstanceNodeBoundUnavailable {}.into());
        };
        let instance_bound_ident = &bound.path.segments.last().unwrap().ident;

        let mut collect = Vec::new();
        collect.push(bound);

        for bound in bound_iter {
            let TypeParamBound::Trait(t) = bound else {
                break;
            };
            let Some(seg) = t.path.segments.last() else {
                break;
            };
            if seg.ident != *instance_bound_ident {
                break;
            }
            collect.push(t);
        }
        Ok(collect)
    }

    /// Returns the original instance associated type declaration currently being
    /// transformed.
    pub fn get_base<'b, C>(&self) -> Result<&TraitItemType, TokenStream>
    where
        K: TraitNodeSegment<'a, 'b, 'c, C>,
        C: GetCounterGenericIndexes<'b>,
    {
        let TraitNodeState(
            TraitNodeSpace {
                trait_of, ty_idx, ..
            },
            _,
        ) = self;
        let TraitItem::Type(item) = trait_of.items.get(*ty_idx).unwrap() else {
            return Err(StateBugs::StateInstanceNodeUnavailable {}.into());
        };
        Ok(item)
    }

    /// Returns a `Self::...` type referencing the original instance associated
    /// type.
    pub fn get_base_assoc<'b, C>(&self) -> Result<Type, TokenStream>
    where
        K: TraitNodeSegment<'a, 'b, 'c, C>,
        C: GetCounterGenericIndexes<'b>,
    {
        let base = self.get_base()?;
        let (_, ty_gen, _) = base.generics.split_for_impl();
        let ident = &base.ident;
        Ok(parse_quote!(Self:: #ident #ty_gen))
    }

    /// Mutates the bounds of the generated global companion associated type
    /// ([`GLOBAL_NODE`]).
    pub fn mutate_global<'b, M, C>(&mut self, mutate: M) -> Result<(), TokenStream>
    where
        M: FnOnce(&mut Punctuated<TypeParamBound, Plus>) -> Result<(), TokenStream>,
        K: TraitNodeSegment<'a, 'b, 'c, C>,
        C: GetCounterGenericIndexes<'b>,
    {
        let TraitNodeSpace {
            trait_of, ty_idx, ..
        } = self.0;
        let TraitItem::Type(item) = trait_of.items.get(*ty_idx).unwrap() else {
            return Err(StateBugs::StateInstanceNodeUnavailable {}.into());
        };
        let ident = &item.ident;
        let exp_ident = gen_type_ident_from_ident_with_suffix::<InstanceNode>(ident, GLOBAL_NODE);
        let Some(mutable) = trait_of.items.iter_mut().find_map(|item| {
            let TraitItem::Type(ty) = item else {
                return None;
            };
            if ty.ident == exp_ident {
                return Some(ty);
            }
            None
        }) else {
            return Err(StateBugs::GlobalNodeNotFound {}.into());
        };
        mutate(&mut mutable.bounds)?;
        Ok(())
    }

    /// Returns the generated global companion associated type
    /// ([`GLOBAL_NODE`]).
    pub fn get_global<'b, C>(&self) -> Result<&TraitItemType, TokenStream>
    where
        K: TraitNodeSegment<'a, 'b, 'c, C>,
        C: GetCounterGenericIndexes<'b>,
    {
        let TraitNodeState(
            TraitNodeSpace {
                trait_of, ty_idx, ..
            },
            _,
        ) = self;
        let TraitItem::Type(item) = trait_of.items.get(*ty_idx).unwrap() else {
            return Err(StateBugs::StateInstanceNodeUnavailable {}.into());
        };
        let ident = &item.ident;
        let exp_ident = gen_type_ident_from_ident_with_suffix::<InstanceNode>(ident, GLOBAL_NODE);
        let Some(global) = trait_of.items.iter().find_map(|item| {
            let TraitItem::Type(ty) = item else {
                return None;
            };
            if ty.ident == exp_ident {
                return Some(ty);
            }
            None
        }) else {
            return Err(StateBugs::GlobalNodeNotFound {}.into());
        };
        Ok(global)
    }

    /// Returns a `Self::...` type referencing the generated global companion
    /// associated type ([`GLOBAL_NODE`]).
    pub fn get_global_assoc<'b, C>(&self) -> Result<Type, TokenStream>
    where
        K: TraitNodeSegment<'a, 'b, 'c, C>,
        C: GetCounterGenericIndexes<'b>,
    {
        let global = self.get_global()?;
        let (_, ty_gen, _) = global.generics.split_for_impl();
        let ident = &global.ident;
        Ok(parse_quote!(Self:: #ident #ty_gen))
    }

    /// Returns the generated terminal global companion associated type
    /// ([`TERMINAL_GLOBAL_NODE`]).
    pub fn get_terminal_global<'b, C>(&self) -> Result<&TraitItemType, TokenStream>
    where
        K: TraitNodeSegment<'a, 'b, 'c, C>,
        C: GetCounterGenericIndexes<'b>,
    {
        let TraitNodeState(
            TraitNodeSpace {
                trait_of, ty_idx, ..
            },
            _,
        ) = self;
        let TraitItem::Type(item) = trait_of.items.get(*ty_idx).unwrap() else {
            return Err(StateBugs::StateInstanceNodeUnavailable {}.into());
        };
        let ident = &item.ident;
        let exp_ident = gen_type_ident_from_ident_with_suffix::<InstanceTerminalNode>(
            ident,
            TERMINAL_GLOBAL_NODE,
        );
        let Some(global) = trait_of.items.iter().find_map(|item| {
            let TraitItem::Type(ty) = item else {
                return None;
            };
            if ty.ident == exp_ident {
                return Some(ty);
            }
            None
        }) else {
            return Err(StateBugs::TerminalGlobalNodeNotFound {}.into());
        };
        Ok(global)
    }

    /// Returns a `Self::...` type referencing the generated terminal global
    /// companion associated type ([`TERMINAL_GLOBAL_NODE`]).
    pub fn get_terminal_global_assoc<'b, C>(&self) -> Result<Type, TokenStream>
    where
        K: TraitNodeSegment<'a, 'b, 'c, C>,
        C: GetCounterGenericIndexes<'b>,
    {
        let global = self.get_terminal_global()?;
        let (_, ty_gen, _) = global.generics.split_for_impl();
        let ident = &global.ident;
        Ok(parse_quote!(Self:: #ident #ty_gen))
    }

    /// Mutates the bounds of the generated twin companion associated type
    /// ([`TWIN_NODE`]).
    pub fn mutate_twin<'b, M, C>(&mut self, mutate: M) -> Result<(), TokenStream>
    where
        M: FnOnce(&mut Punctuated<TypeParamBound, Plus>) -> Result<(), TokenStream>,
        K: TraitNodeSegment<'a, 'b, 'c, C>,
        C: GetCounterGenericIndexes<'b>,
    {
        let TraitNodeSpace {
            trait_of, ty_idx, ..
        } = self.0;
        let TraitItem::Type(item) = trait_of.items.get(*ty_idx).unwrap() else {
            return Err(StateBugs::StateInstanceNodeUnavailable {}.into());
        };
        let ident = &item.ident;
        let exp_ident = gen_type_ident_from_ident_with_suffix::<InstanceNode>(ident, TWIN_NODE);
        let Some(mutable) = trait_of.items.iter_mut().find_map(|item| {
            let TraitItem::Type(ty) = item else {
                return None;
            };
            if ty.ident == exp_ident {
                return Some(ty);
            }
            None
        }) else {
            return Err(StateBugs::TwinNodeNotFound {}.into());
        };
        mutate(&mut mutable.bounds)?;
        Ok(())
    }

    /// Returns the generated twin companion associated type
    /// ([`TWIN_NODE`]).
    pub fn get_twin<'b, C>(&self) -> Result<&TraitItemType, TokenStream>
    where
        K: TraitNodeSegment<'a, 'b, 'c, C>,
        C: GetCounterGenericIndexes<'b>,
    {
        let TraitNodeState(
            TraitNodeSpace {
                trait_of, ty_idx, ..
            },
            _,
        ) = self;
        let TraitItem::Type(item) = trait_of.items.get(*ty_idx).unwrap() else {
            return Err(StateBugs::StateInstanceNodeUnavailable {}.into());
        };
        let ident = &item.ident;
        let exp_ident = gen_type_ident_from_ident_with_suffix::<InstanceNode>(ident, TWIN_NODE);
        let Some(twin) = trait_of.items.iter().find_map(|item| {
            let TraitItem::Type(ty) = item else {
                return None;
            };
            if ty.ident == exp_ident {
                return Some(ty);
            }
            None
        }) else {
            return Err(StateBugs::TwinNodeNotFound {}.into());
        };
        Ok(twin)
    }

    /// Returns a `Self::...` type referencing the generated twin companion
    /// associated type ([`TWIN_NODE`]).
    pub fn get_twin_assoc<'b, C>(&self) -> Result<Type, TokenStream>
    where
        K: TraitNodeSegment<'a, 'b, 'c, C>,
        C: GetCounterGenericIndexes<'b>,
    {
        let twin = self.get_twin()?;
        let (_, ty_gen, _) = twin.generics.split_for_impl();
        let ident = &twin.ident;
        Ok(parse_quote!(Self:: #ident #ty_gen))
    }

    /// Returns the generated terminal twin companion associated type
    /// ([`TERMINAL_TWIN_NODE`]).
    pub fn get_terminal_twin<'b, C>(&self) -> Result<&TraitItemType, TokenStream>
    where
        K: TraitNodeSegment<'a, 'b, 'c, C>,
        C: GetCounterGenericIndexes<'b>,
    {
        let TraitNodeState(
            TraitNodeSpace {
                trait_of, ty_idx, ..
            },
            _,
        ) = self;
        let TraitItem::Type(item) = trait_of.items.get(*ty_idx).unwrap() else {
            return Err(StateBugs::StateInstanceNodeUnavailable {}.into());
        };
        let ident = &item.ident;
        let exp_ident = gen_type_ident_from_ident_with_suffix::<InstanceTerminalNode>(
            ident,
            TERMINAL_TWIN_NODE,
        );
        let Some(twin) = trait_of.items.iter().find_map(|item| {
            let TraitItem::Type(ty) = item else {
                return None;
            };
            if ty.ident == exp_ident {
                return Some(ty);
            }
            None
        }) else {
            return Err(StateBugs::TerminalTwinNodeNotFound {}.into());
        };
        Ok(twin)
    }

    /// Returns a `Self::...` type referencing the generated terminal twin
    /// companion associated type ([`TERMINAL_TWIN_NODE`]).
    pub fn get_terminal_twin_assoc<'b, C>(&self) -> Result<Type, TokenStream>
    where
        K: TraitNodeSegment<'a, 'b, 'c, C>,
        C: GetCounterGenericIndexes<'b>,
    {
        let twin = self.get_terminal_twin()?;
        let (_, ty_gen, _) = twin.generics.split_for_impl();
        let ident = &twin.ident;
        Ok(parse_quote!(Self:: #ident #ty_gen))
    }

    /// Mutates the default expression of the generated checker companion
    /// associated constant ([`CHECKER_NODE`]).
    pub fn mutate_checker<'b, M, C>(&mut self, mutate: M) -> Result<(), TokenStream>
    where
        M: FnOnce(&mut Option<(Token![=], Expr)>) -> Result<(), TokenStream>,
        K: TraitNodeSegment<'a, 'b, 'c, C>,
        C: GetCounterGenericIndexes<'b>,
    {
        let TraitNodeSpace {
            trait_of, ty_idx, ..
        } = self.0;
        let TraitItem::Type(item) = trait_of.items.get(*ty_idx).unwrap() else {
            return Err(StateBugs::StateInstanceNodeUnavailable {}.into());
        };
        let ident = &item.ident;
        let exp_ident = gen_const_ident_from_ident_with_suffix::<InstanceNode>(ident, CHECKER_NODE);
        let Some(mutable) = trait_of.items.iter_mut().find_map(|item| {
            let TraitItem::Const(c) = item else {
                return None;
            };
            if c.ident == exp_ident {
                return Some(c);
            }
            None
        }) else {
            return Err(StateBugs::CheckerNodeNotFound {}.into());
        };
        mutate(&mut mutable.default)?;
        Ok(())
    }

    /// Returns the generated checker companion associated constant
    /// ([`CHECKER_NODE`]).
    pub fn get_checker<'b, C>(&self) -> Result<&TraitItemConst, TokenStream>
    where
        K: TraitNodeSegment<'a, 'b, 'c, C>,
        C: GetCounterGenericIndexes<'b>,
    {
        let TraitNodeState(
            TraitNodeSpace {
                trait_of, ty_idx, ..
            },
            _,
        ) = self;
        let TraitItem::Type(item) = trait_of.items.get(*ty_idx).unwrap() else {
            return Err(StateBugs::StateInstanceNodeUnavailable {}.into());
        };
        let ident = &item.ident;
        let exp_ident = gen_const_ident_from_ident_with_suffix::<InstanceNode>(ident, CHECKER_NODE);
        let Some(checker) = trait_of.items.iter().find_map(|item| {
            let TraitItem::Const(c) = item else {
                return None;
            };
            if c.ident == exp_ident {
                return Some(c);
            }
            None
        }) else {
            return Err(StateBugs::CheckerNodeNotFound {}.into());
        };
        Ok(checker)
    }

    /// Returns a `Self::...` path referencing the generated checker companion
    /// associated constant ([`CHECKER_NODE`]).
    pub fn get_checker_assoc<'b, C>(&self) -> Result<Type, TokenStream>
    where
        K: TraitNodeSegment<'a, 'b, 'c, C>,
        C: GetCounterGenericIndexes<'b>,
    {
        let checker = self.get_checker()?;
        let (_, ty_gen, _) = checker.generics.split_for_impl();
        let ident = &checker.ident;
        Ok(parse_quote!(Self:: #ident #ty_gen))
    }

    /// Returns the generated terminal checker companion associated constant
    /// ([`TERMINAL_CHECKER_NODE`]).
    pub fn get_terminal_checker<'b, C>(&self) -> Result<&TraitItemConst, TokenStream>
    where
        K: TraitNodeSegment<'a, 'b, 'c, C>,
        C: GetCounterGenericIndexes<'b>,
    {
        let TraitNodeState(
            TraitNodeSpace {
                trait_of, ty_idx, ..
            },
            _,
        ) = self;
        let TraitItem::Type(item) = trait_of.items.get(*ty_idx).unwrap() else {
            return Err(StateBugs::StateInstanceNodeUnavailable {}.into());
        };
        let ident = &item.ident;
        let exp_ident = gen_const_ident_from_ident_with_suffix::<InstanceTerminalNode>(
            ident,
            TERMINAL_CHECKER_NODE,
        );
        let Some(checker) = trait_of.items.iter().find_map(|item| {
            let TraitItem::Const(c) = item else {
                return None;
            };
            if c.ident == exp_ident {
                return Some(c);
            }
            None
        }) else {
            return Err(StateBugs::TerminalCheckerNodeNotFound {}.into());
        };
        Ok(checker)
    }

    /// Returns a `Self::...` path referencing the generated terminal checker
    /// companion associated constant ([`TERMINAL_CHECKER_NODE`]).
    pub fn get_terminal_checker_assoc<'b, C>(&self) -> Result<Type, TokenStream>
    where
        K: TraitNodeSegment<'a, 'b, 'c, C>,
        C: GetCounterGenericIndexes<'b>,
    {
        let checker = self.get_terminal_checker()?;
        let (_, ty_gen, _) = checker.generics.split_for_impl();
        let ident = &checker.ident;
        Ok(parse_quote!(Self:: #ident #ty_gen))
    }

    /// Mutates the bounds of the generated initial range companion associated
    /// type ([`INITIAL_NODE`]).
    pub fn mutate_initial<'b, M, C>(&mut self, mutate: M) -> Result<(), TokenStream>
    where
        M: FnOnce(&mut Punctuated<TypeParamBound, Plus>) -> Result<(), TokenStream>,
        K: TraitNodeSegment<'a, 'b, 'c, C>,
        C: GetCounterGenericIndexes<'b>,
    {
        let TraitNodeSpace {
            trait_of, ty_idx, ..
        } = self.0;
        let TraitItem::Type(item) = trait_of.items.get(*ty_idx).unwrap() else {
            return Err(StateBugs::StateInstanceNodeUnavailable {}.into());
        };
        let ident = &item.ident;
        let exp_ident = gen_type_ident_from_ident_with_suffix::<InstanceNode>(ident, INITIAL_NODE);
        let Some(mutable) = trait_of.items.iter_mut().find_map(|item| {
            let TraitItem::Type(ty) = item else {
                return None;
            };
            if ty.ident == exp_ident {
                return Some(ty);
            }
            None
        }) else {
            return Err(StateBugs::InitialNodeNotFound {}.into());
        };
        mutate(&mut mutable.bounds)?;
        Ok(())
    }

    /// Returns the generated initial range companion associated type
    /// ([`INITIAL_NODE`]).
    pub fn get_initial<'b, C>(&self) -> Result<&TraitItemType, TokenStream>
    where
        K: TraitNodeSegment<'a, 'b, 'c, C>,
        C: GetCounterGenericIndexes<'b>,
    {
        let TraitNodeState(
            TraitNodeSpace {
                trait_of, ty_idx, ..
            },
            _,
        ) = self;
        let TraitItem::Type(item) = trait_of.items.get(*ty_idx).unwrap() else {
            return Err(StateBugs::StateInstanceNodeUnavailable {}.into());
        };
        let ident = &item.ident;
        let exp_ident = gen_type_ident_from_ident_with_suffix::<InstanceNode>(ident, INITIAL_NODE);
        let Some(initial) = trait_of.items.iter().find_map(|item| {
            let TraitItem::Type(ty) = item else {
                return None;
            };
            if ty.ident == exp_ident {
                return Some(ty);
            }
            None
        }) else {
            return Err(StateBugs::InitialNodeNotFound {}.into());
        };
        Ok(initial)
    }

    /// Returns a `Self::...` type referencing the generated initial range
    /// companion associated type ([`INITIAL_NODE`]).
    pub fn get_initial_assoc<'b, C>(&self) -> Result<Type, TokenStream>
    where
        K: TraitNodeSegment<'a, 'b, 'c, C>,
        C: GetCounterGenericIndexes<'b>,
    {
        let initial = self.get_initial()?;
        let (_, ty_gen, _) = initial.generics.split_for_impl();
        let ident = &initial.ident;
        Ok(parse_quote!(Self:: #ident #ty_gen))
    }

    /// Mutates the bounds of the generated final range companion associated type
    /// ([`FINAL_NODE`]).
    pub fn mutate_final<'b, M, C>(&mut self, mutate: M) -> Result<(), TokenStream>
    where
        M: FnOnce(&mut Punctuated<TypeParamBound, Plus>) -> Result<(), TokenStream>,
        K: TraitNodeSegment<'a, 'b, 'c, C>,
        C: GetCounterGenericIndexes<'b>,
    {
        let TraitNodeSpace {
            trait_of, ty_idx, ..
        } = self.0;
        let TraitItem::Type(item) = trait_of.items.get(*ty_idx).unwrap() else {
            return Err(StateBugs::StateInstanceNodeUnavailable {}.into());
        };
        let ident = &item.ident;
        let exp_ident = gen_type_ident_from_ident_with_suffix::<InstanceNode>(ident, FINAL_NODE);
        let Some(mutable) = trait_of.items.iter_mut().find_map(|item| {
            let TraitItem::Type(ty) = item else {
                return None;
            };
            if ty.ident == exp_ident {
                return Some(ty);
            }
            None
        }) else {
            return Err(StateBugs::FinalNodeNotFound {}.into());
        };
        mutate(&mut mutable.bounds)?;
        Ok(())
    }

    /// Returns the generated final range companion associated type
    /// ([`FINAL_NODE`]).
    pub fn get_final<'b, C>(&self) -> Result<&TraitItemType, TokenStream>
    where
        K: TraitNodeSegment<'a, 'b, 'c, C>,
        C: GetCounterGenericIndexes<'b>,
    {
        let TraitNodeState(
            TraitNodeSpace {
                trait_of, ty_idx, ..
            },
            _,
        ) = self;
        let TraitItem::Type(item) = trait_of.items.get(*ty_idx).unwrap() else {
            return Err(StateBugs::StateInstanceNodeUnavailable {}.into());
        };
        let ident = &item.ident;
        let exp_ident = gen_type_ident_from_ident_with_suffix::<InstanceNode>(ident, FINAL_NODE);
        let Some(finalz) = trait_of.items.iter().find_map(|item| {
            let TraitItem::Type(ty) = item else {
                return None;
            };
            if ty.ident == exp_ident {
                return Some(ty);
            }
            None
        }) else {
            return Err(StateBugs::FinalNodeNotFound {}.into());
        };
        Ok(finalz)
    }

    /// Returns a `Self::...` type referencing the generated final range companion
    /// associated type ([`FINAL_NODE`]).
    pub fn get_final_assoc<'b, C>(&self) -> Result<Type, TokenStream>
    where
        K: TraitNodeSegment<'a, 'b, 'c, C>,
        C: GetCounterGenericIndexes<'b>,
    {
        let finalz = self.get_final()?;
        let (_, ty_gen, _) = finalz.generics.split_for_impl();
        let ident = &finalz.ident;
        Ok(parse_quote!(Self:: #ident #ty_gen))
    }
}

// ===============================================================================
// ``````````````````````` INSTANCE IMPL NODE SEGMENT UTILS ``````````````````````
// ===============================================================================

/// Mutable transformation state for a generated implementation instance node.
///
/// This state provides mutable access to the implementation currently being
/// transformed while carrying a compile-time marker identifying the active
/// instance node segment.
///
/// The marker type `K` distinguishes specialized instance node segments (for
/// example, leaf, branch, prune, trim, extend, spread, traverse, descend,
/// and root), allowing helper methods and generated companion nodes to be
/// specialized without introducing runtime state.
///
/// An [`ImplNodeState`] is created only after the common preparation phase
/// has completed. At this point the standard companion associated items have
/// already been synthesized, including:
///
/// - Global companion type ([`GLOBAL_NODE`])
/// - Twin companion type ([`TWIN_NODE`])
/// - Initial range companion type ([`INITIAL_NODE`])
/// - Final range companion type ([`FINAL_NODE`])
/// - Terminal global companion type ([`TERMINAL_GLOBAL_NODE`])
/// - Terminal twin companion type ([`TERMINAL_TWIN_NODE`])
///
/// Depending on the publisher implementation, an instance checker companion
/// constant ([`CHECKER_NODE`]) may also be synthesized, while the terminal
/// checker is derived from the generated terminal companions.
///
/// Specialized instance node segments receive this state to inspect, mutate,
/// and extend the generated companion nodes associated with the current
/// implementation, primarily by assigning concrete types, replacing delegated
/// implementations, or customizing generated checker expressions.
#[derive(Debug)]
pub(crate) struct ImplNodeState<'a, 'b, K>(&'b mut ImplNodeSpace<'a>, PhantomData<K>);

/// Common transformation interface for generated implementation instance node
/// segments.
///
/// An implementation instance node segment represents a specialized
/// transformation stage within the instance node implementation pipeline.
/// Every specialized instance node shares the same preparation and validation
/// lifecycle.
///
/// Before a specialized segment executes, the default preparation phase:
///
/// - Validates the source implementation associated type.
/// - Generates the standard companion associated types.
/// - Generates the terminal companion associated types.
/// - Resolves generated companion implementations for delegated instances.
/// - Constructs the mutable [`ImplNodeState`] passed to the specialized
///   transformation.
///
/// Unlike trait node generation, implementation node generation assigns the
/// concrete implementations of generated companion nodes rather than their
/// declarations, bounds, or default checker expressions.
///
/// During validation, the generated companion nodes are verified to ensure
/// they still satisfy the structural invariants expected by the instance node
/// system, including generated identifiers, generic parameters, delegated
/// instance mappings, terminal companion relationships, and generated
/// implementation types.
///
/// This allows specialized instance nodes to focus exclusively on their
/// semantic transformations while reusing a common generation, lookup, and
/// validation infrastructure.
pub(crate) trait ImplNodeSegment<'a, 'b, 'c, C>:
    Transformation<ImplNodeSpace<'a>, C>
{
    /// Performs the common preparation phase for an implementation instance node
    /// transformation.
    ///
    /// Before the specialized instance node executes, this method validates the
    /// source implementation associated type and synthesizes every standard
    /// companion node required by the instance node system.
    ///
    /// The preparation phase:
    ///
    /// - Validates the structural requirements of the source implementation
    ///   associated type.
    /// - Generates the standard companion nodes:
    ///   - [`GLOBAL_NODE`]
    ///   - [`TWIN_NODE`]
    ///   - [`INITIAL_NODE`]
    ///   - [`FINAL_NODE`]
    /// - Generates the terminal companion nodes:
    ///   - [`TERMINAL_GLOBAL_NODE`]
    ///   - [`TERMINAL_TWIN_NODE`]
    /// - Resolves default implementations for every generated companion node.
    /// - Detects delegated instance implementations and constructs the terminal
    ///   companion relationships accordingly.
    /// - Constructs an [`ImplNodeState`] exposing the generated companion nodes.
    ///
    /// Once preparation is complete, the supplied callback receives the mutable
    /// transformation state, allowing the specialized instance node segment to
    /// inspect, mutate, or extend the generated companion nodes.
    fn prepare<A>(from: &'c mut ImplNodeSpace<'a>, _: &'b C, access: A) -> Result<(), TokenStream>
    where
        A: FnOnce(&mut ImplNodeState<'a, 'c, Self>) -> Result<(), TokenStream>,
    {
        let ImplNodeSpace {
            impl_of, ty_idx, ..
        } = from;
        let Some(impl_item) = impl_of.items.get(*ty_idx) else {
            return Err(StateBugs::ImplItemIdxFetchFailed {}.into());
        };

        let self_ty_path = &impl_of.self_ty;
        let self_trait_path = ImplTraitPath::checked_utilize(impl_of, &())?.0;

        let ImplItem::Type(assoc_ty) = impl_item else {
            return Err(StateBugs::FetchedImplItemNotTypeAssoc {}.into());
        };

        let generics = &assoc_ty.generics;
        let (_, ty_gen, _) = generics.split_for_impl();

        let crate_of = Instance::support_crate();
        let assoc_ident = &assoc_ty.ident;

        let global = ImplItemType {
            attrs: proc_suite::internal_code(),
            ident: gen_type_ident_from_ident_with_suffix::<InstanceNode>(assoc_ident, GLOBAL_NODE),
            ty: parse_quote!(#crate_of::Global),
            vis: syn::Visibility::Inherited,
            defaultness: None,
            type_token: Default::default(),
            generics: generics.clone(),
            eq_token: Default::default(),
            semi_token: Default::default(),
        };

        let twin = ImplItemType {
            attrs: proc_suite::internal_code(),
            ident: gen_type_ident_from_ident_with_suffix::<InstanceNode>(assoc_ident, TWIN_NODE),
            ty: parse_quote!(<#self_ty_path as #self_trait_path>::#assoc_ident #ty_gen),
            vis: syn::Visibility::Inherited,
            defaultness: None,
            type_token: Default::default(),
            generics: generics.clone(),
            eq_token: Default::default(),
            semi_token: Default::default(),
        };

        let initial = ImplItemType {
            attrs: proc_suite::internal_code(),
            ident: gen_type_ident_from_ident_with_suffix::<InstanceNode>(assoc_ident, INITIAL_NODE),
            ty: parse_quote!(()),
            vis: syn::Visibility::Inherited,
            defaultness: None,
            type_token: Default::default(),
            generics: generics.clone(),
            eq_token: Default::default(),
            semi_token: Default::default(),
        };

        let finalz = ImplItemType {
            attrs: proc_suite::internal_code(),
            ident: gen_type_ident_from_ident_with_suffix::<InstanceNode>(assoc_ident, FINAL_NODE),
            ty: parse_quote!(()),
            vis: syn::Visibility::Inherited,
            defaultness: None,
            type_token: Default::default(),
            generics: generics.clone(),
            eq_token: Default::default(),
            semi_token: Default::default(),
        };

        let may_assoc_ty = &assoc_ty.ty;
        let ty_path_tokens = quote! {#may_assoc_ty};

        let Some(mut ty_path) = (|| {
            let ty_path = parse::<TypePath>(ty_path_tokens.into()).ok()?;
            if ty_path.qself.is_none() {
                return None;
            }
            Some(ty_path)
        })() else {
            let terminal_global = ImplItemType {
                attrs: proc_suite::internal_code(),
                ident: gen_type_ident_from_ident_with_suffix::<InstanceTerminalNode>(
                    assoc_ident,
                    TERMINAL_GLOBAL_NODE,
                ),
                ty: parse_quote!(#crate_of::Global),
                vis: syn::Visibility::Inherited,
                defaultness: None,
                type_token: Default::default(),
                generics: generics.clone(),
                eq_token: Default::default(),
                semi_token: Default::default(),
            };

            let terminal_twin = ImplItemType {
                attrs: proc_suite::internal_code(),
                ident: gen_type_ident_from_ident_with_suffix::<InstanceTerminalNode>(
                    assoc_ident,
                    TERMINAL_TWIN_NODE,
                ),
                ty: parse_quote!(<#self_ty_path as #self_trait_path>::#assoc_ident #ty_gen),
                vis: syn::Visibility::Inherited,
                defaultness: None,
                type_token: Default::default(),
                generics: generics.clone(),
                eq_token: Default::default(),
                semi_token: Default::default(),
            };

            impl_of.items.push(ImplItem::Type(terminal_global));
            impl_of.items.push(ImplItem::Type(terminal_twin));
            impl_of.items.push(ImplItem::Type(global));
            impl_of.items.push(ImplItem::Type(twin));
            impl_of.items.push(ImplItem::Type(initial));
            impl_of.items.push(ImplItem::Type(finalz));

            access(&mut ImplNodeState(from, PhantomData))?;

            return Ok(());
        };

        let q_self = ty_path.qself.as_ref().unwrap();

        let pos = q_self.position;
        let prev_ident = {
            let Some(seg) = ty_path.path.segments.get(pos) else {
                return Err(StateError::DelegateAssocPathSegNotFound {
                    ty: ty_path.clone(),
                }
                .into());
            };
            seg.ident.clone()
        };

        let seg = ty_path.path.segments.get_mut(pos).unwrap();
        seg.ident = gen_type_ident_from_ident_with_suffix::<InstanceTerminalNode>(
            &prev_ident,
            TERMINAL_GLOBAL_NODE,
        );

        let terminal_global = ImplItemType {
            attrs: proc_suite::internal_code(),
            ident: gen_type_ident_from_ident_with_suffix::<InstanceTerminalNode>(
                assoc_ident,
                TERMINAL_GLOBAL_NODE,
            ),
            ty: Type::Path(ty_path.clone()),
            vis: syn::Visibility::Inherited,
            defaultness: None,
            type_token: Default::default(),
            generics: generics.clone(),
            eq_token: Default::default(),
            semi_token: Default::default(),
        };

        let seg = ty_path.path.segments.get_mut(pos).unwrap();
        seg.ident = gen_type_ident_from_ident_with_suffix::<InstanceTerminalNode>(
            &prev_ident,
            TERMINAL_TWIN_NODE,
        );

        let terminal_twin = ImplItemType {
            attrs: proc_suite::internal_code(),
            ident: gen_type_ident_from_ident_with_suffix::<InstanceTerminalNode>(
                assoc_ident,
                TERMINAL_TWIN_NODE,
            ),
            ty: Type::Path(ty_path),
            vis: syn::Visibility::Inherited,
            defaultness: None,
            type_token: Default::default(),
            generics: generics.clone(),
            eq_token: Default::default(),
            semi_token: Default::default(),
        };

        impl_of.items.push(ImplItem::Type(terminal_global));
        impl_of.items.push(ImplItem::Type(terminal_twin));
        impl_of.items.push(ImplItem::Type(global));
        impl_of.items.push(ImplItem::Type(twin));
        impl_of.items.push(ImplItem::Type(initial));
        impl_of.items.push(ImplItem::Type(finalz));

        access(&mut ImplNodeState(from, PhantomData))?;

        Ok(())
    }

    /// Validates the generated companion nodes after an implementation instance
    /// node transformation has completed.
    ///
    /// Validation reconstructs the expected companion node layout and verifies
    /// that every generated implementation still satisfies the structural
    /// invariants required by the instance node system.
    ///
    /// The validation phase verifies:
    ///
    /// - The original implementation associated type.
    /// - Generated companion node identifiers.
    /// - Generated generic parameter lists.
    /// - Default and delegated companion implementations.
    /// - Terminal companion relationships.
    /// - Generated implementation types.
    ///
    /// A cloned transformation state is then provided to the supplied callback,
    /// allowing specialized instance node segments to perform additional
    /// validation of their generated companion nodes without mutating the
    /// original implementation.
    ///
    /// Successful validation guarantees that the transformed implementation
    /// preserves the expected companion node structure established during the
    /// preparation phase.
    fn validate<A>(
        from: &'c ImplNodeSpace<'a>,
        _: Option<&'b C>,
        access: A,
    ) -> Result<(), TokenStream>
    where
        A: FnOnce(&ImplNodeState<'_, '_, Self>) -> Result<(), TokenStream>,
    {
        let ImplNodeSpace {
            impl_of,
            ty_idx,
            addons,
        } = from;

        let mut impl_of = (**impl_of).clone();

        let self_ty_path = &impl_of.self_ty;
        let self_trait_path = ImplTraitPath::checked_utilize(&impl_of, &())?.0;

        let Some(impl_item) = impl_of.items.get(*ty_idx) else {
            return Err(StateBugs::ImplItemIdxFetchFailed {}.into());
        };
        let ImplItem::Type(assoc_ty) = impl_item else {
            return Err(StateBugs::FetchedImplItemNotTypeAssoc {}.into());
        };

        let Some((base_ty, base_q)) = (|| {
            let Type::Path(base_ty) = &assoc_ty.ty else {
                return None;
            };
            let base_q = base_ty.qself.as_ref()?;
            Some((base_ty, base_q))
        })() else {
            let base_ident = &assoc_ty.ident;
            let generics = &assoc_ty.generics;

            let terminal_global = gen_type_ident_from_ident_with_suffix::<InstanceTerminalNode>(
                base_ident,
                TERMINAL_GLOBAL_NODE,
            );
            match impl_of.items.iter().find(|item| {
                matches!(item, ImplItem::Type(ty)
                    if ty.ident == terminal_global)
            }) {
                Some(ImplItem::Type(ty)) => {
                    if ty.generics.to_token_stream().to_string()
                        != generics.to_token_stream().to_string()
                    {
                        return Err(StateBugs::TerminalGlobalNodeInvalidGenerics {}.into());
                    }

                    let crate_of = Instance::support_crate();
                    let exp_ty: Type = parse_quote!(#crate_of::Global);
                    if ty.ty.to_token_stream().to_string() != exp_ty.to_token_stream().to_string() {
                        return Err(StateBugs::TerminalGlobalNodeInvalidType {}.into());
                    }
                }
                _ => return Err(StateBugs::TerminalGlobalNodeNotFound {}.into()),
            }

            let terminal_twin = gen_type_ident_from_ident_with_suffix::<InstanceTerminalNode>(
                base_ident,
                TERMINAL_TWIN_NODE,
            );
            match impl_of.items.iter().find(|item| {
                matches!(item, ImplItem::Type(ty)
                    if ty.ident == terminal_twin)
            }) {
                Some(ImplItem::Type(ty)) => {
                    if ty.generics.to_token_stream().to_string()
                        != generics.to_token_stream().to_string()
                    {
                        return Err(StateBugs::TerminalTwinNodeInvalidGenerics {}.into());
                    }
                    let (_, ty_gen, _) = generics.split_for_impl();
                    let exp_ty: Type =
                        parse_quote!(<#self_ty_path as #self_trait_path>::#base_ident #ty_gen);
                    if ty.ty.to_token_stream().to_string() != exp_ty.to_token_stream().to_string() {
                        return Err(StateBugs::TerminalTwinNodeInvalidType {}.into());
                    }
                }
                _ => return Err(StateBugs::TerminalTwinNodeNotFound {}.into()),
            }

            let initial =
                gen_type_ident_from_ident_with_suffix::<InstanceNode>(base_ident, INITIAL_NODE);
            match impl_of.items.iter().find(|item| {
                matches!(item, ImplItem::Type(ty)
                    if ty.ident == initial)
            }) {
                Some(ImplItem::Type(ty)) => {
                    if ty.generics.to_token_stream().to_string()
                        != generics.to_token_stream().to_string()
                    {
                        return Err(StateBugs::InitialNodeInvalidGenerics {}.into());
                    }
                }
                _ => return Err(StateBugs::InitialNodeNotFound {}.into()),
            }

            let finalz =
                gen_type_ident_from_ident_with_suffix::<InstanceNode>(base_ident, FINAL_NODE);
            match impl_of.items.iter().find(|item| {
                matches!(item, ImplItem::Type(ty)
                    if ty.ident == finalz)
            }) {
                Some(ImplItem::Type(ty)) => {
                    if ty.generics.to_token_stream().to_string()
                        != generics.to_token_stream().to_string()
                    {
                        return Err(StateBugs::FinalNodeInvalidGenerics {}.into());
                    }
                }
                _ => return Err(StateBugs::FinalNodeNotFound {}.into()),
            }

            let mut mut_addons = (**addons).clone();
            let mut dup = ImplNodeSpace {
                impl_of: &mut impl_of,
                ty_idx: *ty_idx,
                addons: &mut mut_addons,
            };

            access(&ImplNodeState(&mut dup, PhantomData))?;

            return Ok(());
        };

        let base_q_ty = &base_q.ty;

        let generics = &assoc_ty.generics;
        let base_ident = &assoc_ty.ident;

        let global = gen_type_ident_from_ident_with_suffix::<InstanceNode>(base_ident, GLOBAL_NODE);
        match impl_of.items.iter().find(|item| {
            matches!(item, ImplItem::Type(ty)
                if ty.ident == global)
        }) {
            Some(ImplItem::Type(ty)) => {
                if ty.generics.to_token_stream().to_string()
                    != generics.to_token_stream().to_string()
                {
                    return Err(StateBugs::GlobalNodeInvalidGenerics {}.into());
                }
            }
            _ => return Err(StateBugs::GlobalNodeNotFound {}.into()),
        }

        let twin = gen_type_ident_from_ident_with_suffix::<InstanceNode>(base_ident, TWIN_NODE);
        match impl_of.items.iter().find(|item| {
            matches!(item, ImplItem::Type(ty)
                if ty.ident == twin)
        }) {
            Some(ImplItem::Type(ty)) => {
                if ty.generics.to_token_stream().to_string()
                    != generics.to_token_stream().to_string()
                {
                    return Err(StateBugs::TwinNodeInvalidGenerics {}.into());
                }
            }
            _ => return Err(StateBugs::TwinNodeNotFound {}.into()),
        }

        let terminal_global_ident = gen_type_ident_from_ident_with_suffix::<InstanceTerminalNode>(
            base_ident,
            TERMINAL_GLOBAL_NODE,
        );
        let terminal_global = match impl_of.items.iter().find(|item| {
            matches!(item, ImplItem::Type(ty)
                if ty.ident == terminal_global_ident)
        }) {
            Some(ImplItem::Type(ty)) => {
                if ty.generics.to_token_stream().to_string()
                    != generics.to_token_stream().to_string()
                {
                    Err(<StateBugs as Into<TokenStream>>::into(
                        StateBugs::TerminalGlobalNodeInvalidGenerics {},
                    ))
                } else {
                    Ok(ty)
                }
            }
            _ => Err(<StateBugs as Into<TokenStream>>::into(
                StateBugs::TerminalGlobalNodeNotFound {},
            )),
        }?;

        let terminal_twin_ident = gen_type_ident_from_ident_with_suffix::<InstanceTerminalNode>(
            base_ident,
            TERMINAL_TWIN_NODE,
        );
        let terminal_twin = match impl_of.items.iter().find(|item| {
            matches!(item, ImplItem::Type(ty)
                if ty.ident == terminal_twin_ident)
        }) {
            Some(ImplItem::Type(ty)) => {
                if ty.generics.to_token_stream().to_string()
                    != generics.to_token_stream().to_string()
                {
                    Err(<StateBugs as Into<TokenStream>>::into(
                        StateBugs::TerminalTwinNodeInvalidGenerics {},
                    ))
                } else {
                    Ok(ty)
                }
            }
            _ => Err(<StateBugs as Into<TokenStream>>::into(
                StateBugs::TerminalTwinNodeNotFound {},
            )),
        }?;

        let mut try_path = base_ty.path.clone();
        let Some(prev) = try_path.segments.pop() else {
            return Err(StateBugs::ImplInstanceTyLastSegNotFound {}.into());
        };
        let prev_ident = prev.into_value().ident;
        try_path.segments.pop_punct();

        // Validate Global
        let Type::Path(terminal_global_ty) = &terminal_global.ty else {
            return Err(StateBugs::ImplTerminalGlobalTyNotTypePath {}.into());
        };
        let Some(q_self) = &terminal_global_ty.qself else {
            return Err(StateBugs::ImplTerminalGlobalTyQSelfNotFound {}.into());
        };
        let global_q_ty = &q_self.ty;
        if global_q_ty.to_token_stream().to_string() != base_q_ty.to_token_stream().to_string() {
            return Err(StateBugs::ImplTerminalGlobalTyQSelfInvalid {}.into());
        }

        let exp_ident = gen_type_ident_from_ident_with_suffix::<InstanceTerminalNode>(
            &prev_ident,
            TERMINAL_GLOBAL_NODE,
        );

        let mut global_path = terminal_global_ty.path.clone();
        let Some(poped) = global_path.segments.pop() else {
            return Err(StateBugs::ImplTerminalGlobalTyLastSegNotFound {}.into());
        };
        let global_ident = &poped.into_value().ident;
        global_path.segments.pop_punct();
        if *global_ident != exp_ident {
            return Err(StateBugs::ImplTerminalGlobalTyLastSegInvalid {}.into());
        }
        if try_path.to_token_stream().to_string() != global_path.to_token_stream().to_string() {
            return Err(StateBugs::ImplTerminalGlobalTyLastQSelfBoundInvalid {}.into());
        }

        // Validate Twin
        let Type::Path(terminal_twin_ty) = &terminal_twin.ty else {
            return Err(StateBugs::ImplTerminalTwinTyNotTypePath {}.into());
        };
        let Some(q_self) = &terminal_twin_ty.qself else {
            return Err(StateBugs::ImplTerminalTwinTyQSelfNotFound {}.into());
        };
        let twin_q_ty = &q_self.ty;
        if twin_q_ty.to_token_stream().to_string() != base_q_ty.to_token_stream().to_string() {
            return Err(StateBugs::ImplTerminalTwinTyQSelfInvalid {}.into());
        }

        let exp_ident = gen_type_ident_from_ident_with_suffix::<InstanceTerminalNode>(
            &prev_ident,
            TERMINAL_TWIN_NODE,
        );

        let mut twin_path = terminal_twin_ty.path.clone();
        let Some(poped) = twin_path.segments.pop() else {
            return Err(StateBugs::ImplTerminalTwinTyLastSegNotFound {}.into());
        };
        let twin_ident = &poped.into_value().ident;
        twin_path.segments.pop_punct();
        if *twin_ident != exp_ident {
            return Err(StateBugs::ImplTerminalTwinTyLastSegInvalid {}.into());
        }

        if try_path.to_token_stream().to_string() != twin_path.to_token_stream().to_string() {
            return Err(StateBugs::ImplTerminalTwinTyLastQSelfBoundInvalid {}.into());
        }

        let initial =
            gen_type_ident_from_ident_with_suffix::<InstanceNode>(base_ident, INITIAL_NODE);
        match impl_of.items.iter().find(|item| {
            matches!(item, ImplItem::Type(ty)
                if ty.ident == initial)
        }) {
            Some(ImplItem::Type(ty)) => {
                if ty.generics.to_token_stream().to_string()
                    != generics.to_token_stream().to_string()
                {
                    return Err(StateBugs::InitialNodeInvalidGenerics {}.into());
                }
            }
            _ => return Err(StateBugs::InitialNodeNotFound {}.into()),
        }

        let finalz = gen_type_ident_from_ident_with_suffix::<InstanceNode>(base_ident, FINAL_NODE);
        match impl_of.items.iter().find(|item| {
            matches!(item, ImplItem::Type(ty)
                if ty.ident == finalz)
        }) {
            Some(ImplItem::Type(ty)) => {
                if ty.generics.to_token_stream().to_string()
                    != generics.to_token_stream().to_string()
                {
                    return Err(StateBugs::FinalNodeInvalidGenerics {}.into());
                }
            }
            _ => return Err(StateBugs::FinalNodeNotFound {}.into()),
        }

        let mut mut_addons = (**addons).clone();
        let mut dup = ImplNodeSpace {
            impl_of: &mut impl_of,
            ty_idx: *ty_idx,
            addons: &mut mut_addons,
        };

        access(&ImplNodeState(&mut dup, PhantomData))?;

        Ok(())
    }
}

impl<'a, 'b, 'c, C, T> ImplNodeSegment<'a, 'b, 'c, C> for T where
    T: Transformation<ImplNodeSpace<'a>, C>
{
}

/// Convenience accessors and mutation utilities for generated companion
/// nodes within an implementation instance transformation.
///
/// These helpers provide a uniform interface for locating, mutating, and
/// referencing the generated companion nodes associated with the current
/// implementation instance. Rather than repeatedly traversing the
/// implementation, each helper deterministically resolves the companion item
/// generated during the preparation phase.
///
/// The API is organized into three categories:
///
/// - **Base implementation**
///   - `get_base`
///   - `get_base_assoc`
///
/// - **Generated companion nodes**
///   - Global ([`GLOBAL_NODE`])
///   - Twin ([`TWIN_NODE`])
///   - Checker ([`CHECKER_NODE`])
///   - Initial ([`INITIAL_NODE`])
///   - Final ([`FINAL_NODE`])
///   - Terminal Global ([`TERMINAL_GLOBAL_NODE`])
///   - Terminal Twin ([`TERMINAL_TWIN_NODE`])
///   - Terminal Checker ([`TERMINAL_CHECKER_NODE`])
///
/// For each generated companion node, the state exposes the operations
/// supported by that node:
///
/// - `get_*` - retrieves the generated associated item.
/// - `get_*_assoc` - constructs a `Self::Assoc<...>` type referencing the
///   generated associated item.
/// - `mutate_*` - where supported, provides controlled mutation of the
///   generated implementation while preserving structural invariants.
/// - `add_*` / `remove_*` - where supported, creates or removes optional
///   generated companion declarations.
///
/// Unlike the trait transformation helpers, these utilities operate on the
/// concrete implementation of generated companion nodes, allowing specialized
/// instance node segments to assign resolved types, customize generated
/// checker expressions, and manipulate optional generated companion
/// declarations without repeatedly traversing the implementation.
#[allow(unused)]
impl<'a, 'c, K> ImplNodeState<'a, 'c, K> {
    /// Returns the original implementation associated type currently being
    /// transformed.
    pub fn get_base<'b, C>(&self) -> Result<&ImplItemType, TokenStream>
    where
        K: ImplNodeSegment<'a, 'b, 'c, C>,
    {
        let ImplNodeState(
            ImplNodeSpace {
                impl_of, ty_idx, ..
            },
            _,
        ) = self;
        let ImplItem::Type(item) = impl_of.items.get(*ty_idx).unwrap() else {
            return Err(StateBugs::StateInstanceNodeUnavailable {}.into());
        };
        Ok(item)
    }

    /// Returns a `Self::...` type referencing the original implementation
    /// associated type.
    pub fn get_base_assoc<'b, C>(&self) -> Result<Type, TokenStream>
    where
        K: ImplNodeSegment<'a, 'b, 'c, C>,
    {
        let self_ty_path = &self.0.impl_of.self_ty;
        let self_trait_path = ImplTraitPath::checked_utilize(&self.0.impl_of, &())?.0;

        let base = self.get_base()?;
        let (_, ty_gen, _) = base.generics.split_for_impl();
        let ident = &base.ident;
        Ok(parse_quote!(<#self_ty_path as #self_trait_path>:: #ident #ty_gen))
    }

    /// Mutates the resolved implementation type of the generated global companion
    /// associated type ([`GLOBAL_NODE`]).
    pub fn mutate_global<'b, M, C>(&mut self, mutate: M) -> Result<(), TokenStream>
    where
        M: FnOnce(&mut Type) -> Result<(), TokenStream>,
        K: ImplNodeSegment<'a, 'b, 'c, C>,
    {
        let ImplNodeSpace {
            impl_of, ty_idx, ..
        } = self.0;
        let ImplItem::Type(item) = impl_of.items.get(*ty_idx).unwrap() else {
            return Err(StateBugs::StateInstanceNodeUnavailable {}.into());
        };
        let ident = &item.ident;
        let exp_ident = gen_type_ident_from_ident_with_suffix::<InstanceNode>(ident, GLOBAL_NODE);
        let Some(mutable) = impl_of.items.iter_mut().find_map(|item| {
            let ImplItem::Type(ty) = item else {
                return None;
            };
            if ty.ident == exp_ident {
                return Some(ty);
            }
            None
        }) else {
            return Err(StateBugs::GlobalNodeNotFound {}.into());
        };
        mutate(&mut mutable.ty)?;
        Ok(())
    }

    /// Returns the generated global companion associated type
    /// ([`GLOBAL_NODE`]).
    pub fn get_global<'b, C>(&self) -> Result<&ImplItemType, TokenStream>
    where
        K: ImplNodeSegment<'a, 'b, 'c, C>,
    {
        let ImplNodeState(
            ImplNodeSpace {
                impl_of, ty_idx, ..
            },
            _,
        ) = self;
        let ImplItem::Type(item) = impl_of.items.get(*ty_idx).unwrap() else {
            return Err(StateBugs::StateInstanceNodeUnavailable {}.into());
        };
        let ident = &item.ident;
        let exp_ident = gen_type_ident_from_ident_with_suffix::<InstanceNode>(ident, GLOBAL_NODE);
        let Some(global) = impl_of.items.iter().find_map(|item| {
            let ImplItem::Type(ty) = item else {
                return None;
            };
            if ty.ident == exp_ident {
                return Some(ty);
            }
            None
        }) else {
            return Err(StateBugs::GlobalNodeNotFound {}.into());
        };
        Ok(global)
    }

    /// Returns a `Self::...` type referencing the generated global companion
    /// associated type ([`GLOBAL_NODE`]).
    pub fn get_global_assoc<'b, C>(&self) -> Result<Type, TokenStream>
    where
        K: ImplNodeSegment<'a, 'b, 'c, C>,
    {
        let self_ty_path = &self.0.impl_of.self_ty;
        let self_trait_path = ImplTraitPath::checked_utilize(&self.0.impl_of, &())?.0;

        let global = self.get_global()?;
        let (_, ty_gen, _) = global.generics.split_for_impl();
        let ident = &global.ident;
        Ok(parse_quote!(<#self_ty_path as #self_trait_path>:: #ident #ty_gen))
    }

    /// Returns the generated terminal global companion associated type
    /// ([`TERMINAL_GLOBAL_NODE`]).
    pub fn get_terminal_global<'b, C>(&self) -> Result<&ImplItemType, TokenStream>
    where
        K: ImplNodeSegment<'a, 'b, 'c, C>,
    {
        let ImplNodeState(
            ImplNodeSpace {
                impl_of, ty_idx, ..
            },
            _,
        ) = self;
        let ImplItem::Type(item) = impl_of.items.get(*ty_idx).unwrap() else {
            return Err(StateBugs::StateInstanceNodeUnavailable {}.into());
        };
        let ident = &item.ident;
        let exp_ident = gen_type_ident_from_ident_with_suffix::<InstanceTerminalNode>(
            ident,
            TERMINAL_GLOBAL_NODE,
        );
        let Some(global) = impl_of.items.iter().find_map(|item| {
            let ImplItem::Type(ty) = item else {
                return None;
            };
            if ty.ident == exp_ident {
                return Some(ty);
            }
            None
        }) else {
            return Err(StateBugs::TerminalGlobalNodeNotFound {}.into());
        };
        Ok(global)
    }

    /// Returns a `Self::...` type referencing the generated terminal global
    /// companion associated type ([`TERMINAL_GLOBAL_NODE`]).
    pub fn get_terminal_global_assoc<'b, C>(&self) -> Result<Type, TokenStream>
    where
        K: ImplNodeSegment<'a, 'b, 'c, C>,
    {
        let self_ty_path = &self.0.impl_of.self_ty;
        let self_trait_path = ImplTraitPath::checked_utilize(&self.0.impl_of, &())?.0;

        let global = self.get_terminal_global()?;
        let (_, ty_gen, _) = global.generics.split_for_impl();
        let ident = &global.ident;
        Ok(parse_quote!(<#self_ty_path as #self_trait_path>:: #ident #ty_gen))
    }

    /// Mutates the resolved implementation type of the generated twin companion
    /// associated type ([`TWIN_NODE`]).
    pub fn mutate_twin<'b, M, C>(&mut self, mutate: M) -> Result<(), TokenStream>
    where
        M: FnOnce(&mut Type) -> Result<(), TokenStream>,
        K: ImplNodeSegment<'a, 'b, 'c, C>,
    {
        let ImplNodeSpace {
            impl_of, ty_idx, ..
        } = self.0;
        let ImplItem::Type(item) = impl_of.items.get(*ty_idx).unwrap() else {
            return Err(StateBugs::StateInstanceNodeUnavailable {}.into());
        };
        let ident = &item.ident;
        let exp_ident = gen_type_ident_from_ident_with_suffix::<InstanceNode>(ident, TWIN_NODE);
        let Some(mutable) = impl_of.items.iter_mut().find_map(|item| {
            let ImplItem::Type(ty) = item else {
                return None;
            };
            if ty.ident == exp_ident {
                return Some(ty);
            }
            None
        }) else {
            return Err(StateBugs::TwinNodeNotFound {}.into());
        };
        mutate(&mut mutable.ty)?;
        Ok(())
    }

    /// Returns the generated twin companion associated type
    /// ([`TWIN_NODE`]).
    pub fn get_twin<'b, C>(&self) -> Result<&ImplItemType, TokenStream>
    where
        K: ImplNodeSegment<'a, 'b, 'c, C>,
    {
        let ImplNodeState(
            ImplNodeSpace {
                impl_of, ty_idx, ..
            },
            _,
        ) = self;
        let ImplItem::Type(item) = impl_of.items.get(*ty_idx).unwrap() else {
            return Err(StateBugs::StateInstanceNodeUnavailable {}.into());
        };
        let ident = &item.ident;
        let exp_ident = gen_type_ident_from_ident_with_suffix::<InstanceNode>(ident, TWIN_NODE);
        let Some(twin) = impl_of.items.iter().find_map(|item| {
            let ImplItem::Type(ty) = item else {
                return None;
            };
            if ty.ident == exp_ident {
                return Some(ty);
            }
            None
        }) else {
            return Err(StateBugs::TwinNodeNotFound {}.into());
        };
        Ok(twin)
    }

    /// Returns a `Self::...` type referencing the generated twin companion
    /// associated type ([`TWIN_NODE`]).
    pub fn get_twin_assoc<'b, C>(&self) -> Result<Type, TokenStream>
    where
        K: ImplNodeSegment<'a, 'b, 'c, C>,
    {
        let self_ty_path = &self.0.impl_of.self_ty;
        let self_trait_path = ImplTraitPath::checked_utilize(&self.0.impl_of, &())?.0;

        let twin = self.get_twin()?;
        let (_, ty_gen, _) = twin.generics.split_for_impl();
        let ident = &twin.ident;
        Ok(parse_quote!(<#self_ty_path as #self_trait_path>:: #ident #ty_gen))
    }

    /// Returns the generated terminal twin companion associated type
    /// ([`TERMINAL_TWIN_NODE`]).
    pub fn get_terminal_twin<'b, C>(&self) -> Result<&ImplItemType, TokenStream>
    where
        K: ImplNodeSegment<'a, 'b, 'c, C>,
    {
        let ImplNodeState(
            ImplNodeSpace {
                impl_of, ty_idx, ..
            },
            _,
        ) = self;
        let ImplItem::Type(item) = impl_of.items.get(*ty_idx).unwrap() else {
            return Err(StateBugs::StateInstanceNodeUnavailable {}.into());
        };
        let ident = &item.ident;
        let exp_ident = gen_type_ident_from_ident_with_suffix::<InstanceTerminalNode>(
            ident,
            TERMINAL_TWIN_NODE,
        );
        let Some(twin) = impl_of.items.iter().find_map(|item| {
            let ImplItem::Type(ty) = item else {
                return None;
            };
            if ty.ident == exp_ident {
                return Some(ty);
            }
            None
        }) else {
            return Err(StateBugs::TerminalTwinNodeNotFound {}.into());
        };
        Ok(twin)
    }

    /// Returns a `Self::...` type referencing the generated terminal twin
    /// companion associated type ([`TERMINAL_TWIN_NODE`]).
    pub fn get_terminal_twin_assoc<'b, C>(&self) -> Result<Type, TokenStream>
    where
        K: ImplNodeSegment<'a, 'b, 'c, C>,
    {
        let self_ty_path = &self.0.impl_of.self_ty;
        let self_trait_path = ImplTraitPath::checked_utilize(&self.0.impl_of, &())?.0;

        let twin = self.get_terminal_twin()?;
        let (_, ty_gen, _) = twin.generics.split_for_impl();
        let ident = &twin.ident;
        Ok(parse_quote!(<#self_ty_path as #self_trait_path>:: #ident #ty_gen))
    }

    /// Mutates the default expression of the generated checker companion
    /// associated constant ([`CHECKER_NODE`]) when present.
    pub fn mutate_checker<'b, M, C>(&mut self, mutate: M) -> Result<(), TokenStream>
    where
        M: FnOnce(Option<&mut Expr>) -> Result<(), TokenStream>,
        K: ImplNodeSegment<'a, 'b, 'c, C>,
    {
        let ImplNodeSpace {
            impl_of, ty_idx, ..
        } = self.0;
        let ImplItem::Type(item) = impl_of.items.get(*ty_idx).unwrap() else {
            return Err(StateBugs::StateInstanceNodeUnavailable {}.into());
        };
        let ident = &item.ident;
        let exp_ident = gen_const_ident_from_ident_with_suffix::<InstanceNode>(ident, CHECKER_NODE);
        let mutable = impl_of.items.iter_mut().find_map(|item| {
            let ImplItem::Const(c) = item else {
                return None;
            };
            if c.ident == exp_ident {
                return Some(c);
            }
            None
        });
        let mutable = match mutable {
            Some(c) => Some(&mut c.expr),
            None => None,
        };
        let res = mutate(mutable)?;
        Ok(())
    }

    /// Returns the generated checker companion associated constant
    /// ([`CHECKER_NODE`]), if present.
    pub fn get_checker<'b, C>(&self) -> Result<Option<&ImplItemConst>, TokenStream>
    where
        K: ImplNodeSegment<'a, 'b, 'c, C>,
    {
        let ImplNodeState(
            ImplNodeSpace {
                impl_of, ty_idx, ..
            },
            _,
        ) = self;
        let ImplItem::Type(item) = impl_of.items.get(*ty_idx).unwrap() else {
            return Err(StateBugs::StateInstanceNodeUnavailable {}.into());
        };
        let ident = &item.ident;
        let exp_ident = gen_const_ident_from_ident_with_suffix::<InstanceNode>(ident, CHECKER_NODE);
        let checker = impl_of.items.iter().find_map(|item| {
            let ImplItem::Const(c) = item else {
                return None;
            };
            if c.ident == exp_ident {
                return Some(c);
            }
            None
        });
        Ok(checker)
    }

    /// Returns a `Self::...` path referencing the generated checker companion
    /// associated constant ([`CHECKER_NODE`]), if present.
    pub fn get_checker_assoc<'b, C>(&self) -> Result<Option<Type>, TokenStream>
    where
        K: ImplNodeSegment<'a, 'b, 'c, C>,
    {
        let self_ty_path = &self.0.impl_of.self_ty;
        let self_trait_path = ImplTraitPath::checked_utilize(&self.0.impl_of, &())?.0;

        let Some(checker) = self.get_checker()? else {
            return Ok(None);
        };
        let (_, ty_gen, _) = &checker.generics.split_for_impl();
        let ident = &checker.ident;
        Ok(Some(
            parse_quote!(<#self_ty_path as #self_trait_path>:: #ident #ty_gen),
        ))
    }

    /// Returns the generated terminal checker companion associated constant
    /// ([`TERMINAL_CHECKER_NODE`]).
    pub fn get_terminal_checker<'b, C>(&self) -> Result<&ImplItemConst, TokenStream>
    where
        K: ImplNodeSegment<'a, 'b, 'c, C>,
    {
        let ImplNodeState(
            ImplNodeSpace {
                impl_of, ty_idx, ..
            },
            _,
        ) = self;
        let ImplItem::Type(item) = impl_of.items.get(*ty_idx).unwrap() else {
            return Err(StateBugs::StateInstanceNodeUnavailable {}.into());
        };
        let ident = &item.ident;
        let exp_ident = gen_const_ident_from_ident_with_suffix::<InstanceTerminalNode>(
            ident,
            TERMINAL_CHECKER_NODE,
        );
        let Some(checker) = impl_of.items.iter().find_map(|item| {
            let ImplItem::Const(c) = item else {
                return None;
            };
            if c.ident == exp_ident {
                return Some(c);
            }
            None
        }) else {
            return Err(StateBugs::TerminalCheckerNodeNotFound {}.into());
        };
        Ok(checker)
    }

    /// Returns a `Self::...` path referencing the generated terminal checker
    /// companion associated constant ([`TERMINAL_CHECKER_NODE`]).
    pub fn get_terminal_checker_assoc<'b, C>(&self) -> Result<Type, TokenStream>
    where
        K: ImplNodeSegment<'a, 'b, 'c, C>,
    {
        let self_ty_path = &self.0.impl_of.self_ty;
        let self_trait_path = ImplTraitPath::checked_utilize(&self.0.impl_of, &())?.0;

        let checker = self.get_terminal_checker()?;
        let (_, ty_gen, _) = &checker.generics.split_for_impl();
        let ident = &checker.ident;
        Ok(parse_quote!(<#self_ty_path as #self_trait_path>:: #ident #ty_gen))
    }

    /// Removes the generated checker companion associated constant
    /// ([`CHECKER_NODE`]) from the current implementation if present.
    pub fn remove_checker<'b, C>(&mut self) -> Result<(), TokenStream>
    where
        K: ImplNodeSegment<'a, 'b, 'c, C>,
    {
        let ImplNodeSpace {
            impl_of, ty_idx, ..
        } = self.0;
        let ImplItem::Type(item) = impl_of.items.get(*ty_idx).unwrap() else {
            return Err(StateBugs::StateInstanceNodeUnavailable {}.into());
        };
        let ident = &item.ident;
        let exp_ident = gen_const_ident_from_ident_with_suffix::<InstanceNode>(ident, CHECKER_NODE);
        let index = impl_of.items.iter_mut().enumerate().find_map(|(i, item)| {
            let ImplItem::Const(c) = item else {
                return None;
            };
            if c.ident == exp_ident {
                return Some(i);
            }
            None
        });
        if let Some(index) = index {
            impl_of.items.remove(index);
        }
        Ok(())
    }

    /// Generates or replaces the checker companion associated constant
    /// ([`CHECKER_NODE`]) using the supplied default expression.
    pub fn add_checker<'b, C>(&mut self, expr: &Expr) -> Result<(), TokenStream>
    where
        K: ImplNodeSegment<'a, 'b, 'c, C>,
    {
        if Self::get_checker(&self)?.is_some() {
            Self::remove_checker(self);
        }

        let ImplNodeSpace {
            impl_of, ty_idx, ..
        } = self.0;

        let ImplItem::Type(item) = impl_of.items.get(*ty_idx).unwrap() else {
            return Err(StateBugs::StateInstanceNodeUnavailable {}.into());
        };
        let assoc_ident = &item.ident;
        let generics = &item.generics;

        let checker = ImplItemConst {
            attrs: proc_suite::internal_code(),
            ident: gen_const_ident_from_ident_with_suffix::<InstanceNode>(
                assoc_ident,
                CHECKER_NODE,
            ),
            ty: parse_quote!(()),
            expr: expr.clone(),
            vis: syn::Visibility::Inherited,
            defaultness: None,
            generics: generics.clone(),
            eq_token: Default::default(),
            semi_token: Default::default(),
            const_token: Default::default(),
            colon_token: Default::default(),
        };

        impl_of.items.push(ImplItem::Const(checker));
        Ok(())
    }

    /// Mutates the resolved implementation type of the generated initial range
    /// companion associated type ([`INITIAL_NODE`]).
    pub fn mutate_initial<'b, M, C>(&mut self, mutate: M) -> Result<(), TokenStream>
    where
        M: FnOnce(&mut Type) -> Result<(), TokenStream>,
        K: ImplNodeSegment<'a, 'b, 'c, C>,
    {
        let ImplNodeSpace {
            impl_of, ty_idx, ..
        } = self.0;
        let ImplItem::Type(item) = impl_of.items.get(*ty_idx).unwrap() else {
            return Err(StateBugs::StateInstanceNodeUnavailable {}.into());
        };
        let ident = &item.ident;
        let exp_ident = gen_type_ident_from_ident_with_suffix::<InstanceNode>(ident, INITIAL_NODE);
        let Some(mutable) = impl_of.items.iter_mut().find_map(|item| {
            let ImplItem::Type(ty) = item else {
                return None;
            };
            if ty.ident == exp_ident {
                return Some(ty);
            }
            None
        }) else {
            return Err(StateBugs::InitialNodeNotFound {}.into());
        };
        mutate(&mut mutable.ty)?;
        Ok(())
    }

    /// Returns the generated initial range companion associated type
    /// ([`INITIAL_NODE`]).
    pub fn get_initial<'b, C>(&self) -> Result<&ImplItemType, TokenStream>
    where
        K: ImplNodeSegment<'a, 'b, 'c, C>,
    {
        let ImplNodeState(
            ImplNodeSpace {
                impl_of, ty_idx, ..
            },
            _,
        ) = self;
        let ImplItem::Type(item) = impl_of.items.get(*ty_idx).unwrap() else {
            return Err(StateBugs::StateInstanceNodeUnavailable {}.into());
        };
        let ident = &item.ident;
        let exp_ident = gen_type_ident_from_ident_with_suffix::<InstanceNode>(ident, INITIAL_NODE);
        let Some(initial) = impl_of.items.iter().find_map(|item| {
            let ImplItem::Type(ty) = item else {
                return None;
            };
            if ty.ident == exp_ident {
                return Some(ty);
            }
            None
        }) else {
            return Err(StateBugs::InitialNodeNotFound {}.into());
        };
        Ok(initial)
    }

    /// Returns a `Self::...` type referencing the generated initial range
    /// companion associated type ([`INITIAL_NODE`]).
    pub fn get_initial_assoc<'b, C>(&self) -> Result<Type, TokenStream>
    where
        K: ImplNodeSegment<'a, 'b, 'c, C>,
    {
        let self_ty_path = &self.0.impl_of.self_ty;
        let self_trait_path = ImplTraitPath::checked_utilize(&self.0.impl_of, &())?.0;

        let initial = self.get_initial()?;
        let (_, ty_gen, _) = initial.generics.split_for_impl();
        let ident = &initial.ident;
        Ok(parse_quote!(<#self_ty_path as #self_trait_path>:: #ident #ty_gen))
    }

    /// Mutates the resolved implementation type of the generated final range
    /// companion associated type ([`FINAL_NODE`]).
    pub fn mutate_final<'b, M, C>(&mut self, mutate: M) -> Result<(), TokenStream>
    where
        M: FnOnce(&mut Type) -> Result<(), TokenStream>,
        K: ImplNodeSegment<'a, 'b, 'c, C>,
    {
        let ImplNodeSpace {
            impl_of, ty_idx, ..
        } = self.0;
        let ImplItem::Type(item) = impl_of.items.get(*ty_idx).unwrap() else {
            return Err(StateBugs::StateInstanceNodeUnavailable {}.into());
        };
        let ident = &item.ident;
        let exp_ident = gen_type_ident_from_ident_with_suffix::<InstanceNode>(ident, FINAL_NODE);
        let Some(mutable) = impl_of.items.iter_mut().find_map(|item| {
            let ImplItem::Type(ty) = item else {
                return None;
            };
            if ty.ident == exp_ident {
                return Some(ty);
            }
            None
        }) else {
            return Err(StateBugs::FinalNodeNotFound {}.into());
        };
        mutate(&mut mutable.ty)?;
        Ok(())
    }

    /// Returns the generated final range companion associated type
    /// ([`FINAL_NODE`]).
    pub fn get_final<'b, C>(&self) -> Result<&ImplItemType, TokenStream>
    where
        K: ImplNodeSegment<'a, 'b, 'c, C>,
    {
        let ImplNodeState(
            ImplNodeSpace {
                impl_of, ty_idx, ..
            },
            _,
        ) = self;
        let ImplItem::Type(item) = impl_of.items.get(*ty_idx).unwrap() else {
            return Err(StateBugs::StateInstanceNodeUnavailable {}.into());
        };
        let ident = &item.ident;
        let exp_ident = gen_type_ident_from_ident_with_suffix::<InstanceNode>(ident, FINAL_NODE);
        let Some(finalz) = impl_of.items.iter().find_map(|item| {
            let ImplItem::Type(ty) = item else {
                return None;
            };
            if ty.ident == exp_ident {
                return Some(ty);
            }
            None
        }) else {
            return Err(StateBugs::InitialNodeNotFound {}.into());
        };
        Ok(finalz)
    }

    /// Returns a `Self::...` type referencing the generated final range companion
    /// associated type ([`FINAL_NODE`]).
    pub fn get_final_assoc<'b, C>(&self) -> Result<Type, TokenStream>
    where
        K: ImplNodeSegment<'a, 'b, 'c, C>,
    {
        let self_ty_path = &self.0.impl_of.self_ty;
        let self_trait_path = ImplTraitPath::checked_utilize(&self.0.impl_of, &())?.0;

        let finalz = self.get_final()?;
        let (_, ty_gen, _) = finalz.generics.split_for_impl();
        let ident = &finalz.ident;
        Ok(parse_quote!(<#self_ty_path as #self_trait_path>:: #ident #ty_gen))
    }
}
