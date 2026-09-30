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
// ``````````````````````````` INSTANCE SUBSCRIBER NODE ``````````````````````````
// ===============================================================================

//! Subscriber instance node transformations.
//!
//! Subscriber instance nodes define how an instance declaration is interpreted
//! as a set of instances. Each subscriber model represents a different
//! selection strategy:
//!
//! - [`TraitLeafSubscriber`] - Selects one or more exact instances.
//! - [`TraitBranchSubscriber`] - Selects an entire subtree rooted at a
//!   specified branch.
//! - [`TraitPruneSubscriber`] - Selects every instance from the root of the
//!   tree to a specified leaf.
//! - [`TraitTrimSubscriber`] - Selects every instance from the root of the
//!   tree to a specified branch.
//! - [`TraitExtendSubscriber`] - Selects every instance from a specified leaf
//!   to the terminal boundary of the tree.
//! - [`TraitSpreadSubscriber`] - Selects every instance from a specified
//!   branch to the terminal boundary of the tree.
//! - [`TraitTraverseSubscriber`] - Selects every instance between a specified
//!   branch and a specified leaf using the complementary traversal model.
//! - [`TraitDescendSubscriber`] - Selects every instance between a specified
//!   branch and a specified leaf.
//! - [`TraitRootSubscriber`] - Selects the entire instance tree.
//!
//! Every subscriber transformation operates on both the declaration and
//! implementation sides.
//!
//! - Declaration-side transformations construct the subscriber semantics by
//!   rewriting the original instance bounds and generating the required
//!   companion instance nodes.
//! - [`InstanceImplSubscriber`] resolves the corresponding implementation
//!   companion associated types, producing the concrete implementation for the
//!   generated declaration-side companion nodes.
//!
//! Together these transformations provide a declarative framework for
//! selecting, constraining, and resolving instance sets while preserving a
//! deterministic companion node structure across every subscriber model.

// ===============================================================================
// ``````````````````````````````````` IMPORTS ```````````````````````````````````
// ===============================================================================

// --- Proc Suite ---
use proc_suite::{BStringList, IntList, misc::*};

// --- Proc Macro Crates ---
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    GenericArgument, Ident, PathArguments, TraitBound, Type, TypeParamBound, TypePath, parse,
    parse_quote, punctuated::Punctuated, spanned::Spanned,
};

// --- Local Crate
use crate::{
    Extraction, Transformation,
    node::{access::*, args::*, errors::*, state::*},
    traits::sum::{GlobalTerminalAssoc, GlobalTerminalExact, SelfTerminalAssoc},
};

// ===============================================================================
// ``````````````````````````` INSTANCE SUBSCRIBER NODE ``````````````````````````
// ===============================================================================

impl<'a> Transformation<SubscriberNodeSpace<'a>, NodeArgs<Self>> for SubscriberNode {
    fn raw_transform(
        &self,
        transform: &mut SubscriberNodeSpace<'a>,
        context: &NodeArgs<Self>,
    ) -> Result<(), proc_macro2::TokenStream> {
        match transform {
            SubscriberNodeSpace::Trait(trait_space) => match context {
                NodeArgs::Unknown(token) => Err(KeysError::ExpectedKeys {
                    span: token.tokens.span(),
                }
                .into()),
                NodeArgs::Leaf(exact) => {
                    TraitLeafSubscriber::checked_transform(&TraitLeafSubscriber, trait_space, exact)
                }
                NodeArgs::Branch(block) => TraitBranchSubscriber::checked_transform(
                    &TraitBranchSubscriber,
                    trait_space,
                    block,
                ),
                NodeArgs::Traverse(traverse) => TraitTraverseSubscriber::checked_transform(
                    &TraitTraverseSubscriber,
                    trait_space,
                    traverse,
                ),
                NodeArgs::Extend(prefix) => TraitExtendSubscriber::checked_transform(
                    &TraitExtendSubscriber,
                    trait_space,
                    prefix,
                ),
                NodeArgs::Prune(limit) => TraitPruneSubscriber::checked_transform(
                    &TraitPruneSubscriber,
                    trait_space,
                    limit,
                ),
                NodeArgs::Trim(trim) => {
                    TraitTrimSubscriber::checked_transform(&TraitTrimSubscriber, trait_space, trim)
                }
                NodeArgs::Spread(spread) => TraitSpreadSubscriber::checked_transform(
                    &TraitSpreadSubscriber,
                    trait_space,
                    spread,
                ),
                NodeArgs::Descend(descend) => TraitDescendSubscriber::checked_transform(
                    &TraitDescendSubscriber,
                    trait_space,
                    descend,
                ),
                NodeArgs::Root(root) => {
                    TraitRootSubscriber::checked_transform(&TraitRootSubscriber, trait_space, root)
                }
            },
            SubscriberNodeSpace::Impl(impl_space) => {
                InstanceImplSubscriber::checked_transform(&InstanceImplSubscriber, impl_space, &())
            }
        }
    }

    fn validate_transform(
        &self,
        transform: &SubscriberNodeSpace,
        context: Option<&NodeArgs<Self>>,
    ) -> Result<(), proc_macro2::TokenStream> {
        match transform {
            SubscriberNodeSpace::Trait(TraitNodeSpace {
                trait_of,
                ty_idx,
                addons,
            }) => {
                let mut mut_trait_of = (**trait_of).clone();
                let mut mut_addons = (**addons).clone();
                let Some(context) = context else {
                    return Ok(());
                };

                let mut validate_space = TraitNodeSpace {
                    trait_of: &mut mut_trait_of,
                    ty_idx: *ty_idx,
                    addons: &mut mut_addons,
                };
                match context {
                    NodeArgs::Unknown(_) => Err(KeyBugs::ExpectedKeys {}.into()),
                    NodeArgs::Leaf(exact) => TraitLeafSubscriber::validate_transform(
                        &TraitLeafSubscriber,
                        &mut validate_space,
                        Some(exact),
                    ),
                    NodeArgs::Branch(block) => TraitBranchSubscriber::validate_transform(
                        &TraitBranchSubscriber,
                        &mut validate_space,
                        Some(block),
                    ),
                    NodeArgs::Traverse(traverse) => TraitTraverseSubscriber::validate_transform(
                        &TraitTraverseSubscriber,
                        &mut validate_space,
                        Some(traverse),
                    ),
                    NodeArgs::Extend(prefix) => TraitExtendSubscriber::validate_transform(
                        &TraitExtendSubscriber,
                        &mut validate_space,
                        Some(prefix),
                    ),
                    NodeArgs::Prune(limit) => TraitPruneSubscriber::validate_transform(
                        &TraitPruneSubscriber,
                        &mut validate_space,
                        Some(limit),
                    ),
                    NodeArgs::Trim(trim) => TraitTrimSubscriber::validate_transform(
                        &TraitTrimSubscriber,
                        &mut validate_space,
                        Some(trim),
                    ),
                    NodeArgs::Spread(spread) => TraitSpreadSubscriber::validate_transform(
                        &TraitSpreadSubscriber,
                        &mut validate_space,
                        Some(spread),
                    ),
                    NodeArgs::Descend(descend) => TraitDescendSubscriber::validate_transform(
                        &TraitDescendSubscriber,
                        &mut validate_space,
                        Some(descend),
                    ),
                    NodeArgs::Root(root) => TraitRootSubscriber::validate_transform(
                        &TraitRootSubscriber,
                        &mut validate_space,
                        Some(root),
                    ),
                }
            }
            SubscriberNodeSpace::Impl(ImplNodeSpace {
                impl_of,
                ty_idx,
                addons,
            }) => {
                let mut mut_impl_of = (**impl_of).clone();
                let mut mut_addons = (**addons).clone();
                let mut validate_space = ImplNodeSpace {
                    impl_of: &mut mut_impl_of,
                    ty_idx: *ty_idx,
                    addons: &mut mut_addons,
                };

                InstanceImplSubscriber::validate_transform(
                    &InstanceImplSubscriber,
                    &mut validate_space,
                    None,
                )
            }
        }
    }
}

// ===============================================================================
// ``````````````````````` INSTANCE TRAIT LEAF SUBSCRIBER ```````````````````````
// ===============================================================================

/// Subscriber transformation for the `leaf` instance node.
///
/// A leaf subscriber selects one or more exact instance identifier paths.
/// Each selected path is resolved into a corresponding [`CounterAccess`]
/// specialization using the supplied instance counter generic indexes.
///
/// This transformation:
///
/// - Replaces the original instance bound with one transformed
///   [`CounterAccess`] bound for each selected identifier path.
/// - Preserves the [`crate::traits::sum`] associated-type equalities in
///   the replaced bounds by binding,
///         - [`SelfTerminalAssoc`] to [`TERMINAL_TWIN_NODE`].
///         - [`GlobalTerminalAssoc`] to [`TERMINAL_GLOBAL_NODE`].
///         - [`GlobalTerminalExact`] to
///             `<TERMINAL_TWIN_NODE as TERMINAL_GLOBAL_NODE_TRAIT>::Terminal`.
/// - Contributes the same [`CounterAccess`] bounds to the generated global
///   companion node ([`GLOBAL_NODE`]), allowing the exact selections to be
///   accumulated independently of the original instance.
/// - Leaves the generated range companion nodes ([`INITIAL_NODE`] and
///   [`FINAL_NODE`]) unchanged, since a leaf represents an exact instance
///   rather than a range or subtree.
///
/// The resulting transformation ensures that every exact instance selected by
/// the leaf node is consistently represented by both the original instance
/// declaration and its generated global companion node.
#[derive(Debug, Clone)]
pub(crate) struct TraitLeafSubscriber;

impl<'a> Transformation<TraitNodeSpace<'a>, LeafArgs<SubscriberNode>> for TraitLeafSubscriber {
    fn raw_transform(
        &self,
        transform: &mut TraitNodeSpace<'a>,
        context: &LeafArgs<SubscriberNode>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let indexes = &context.indexes;
        Self::prepare(transform, context, |state| {
            let global_assoc = &state.get_global_assoc()?;
            let base_assoc = &state.get_base_assoc()?;
            let term_twin_assoc = &state.get_terminal_twin_assoc()?;
            let term_global_assoc = &state.get_terminal_global_assoc()?;
            let term_context = (base_assoc, context.get()?).into();
            let term_bound = TerminalAccess::checked_extract(&term_context, &())?.0;
            let term_projection = TerminalAccess::projection();
            let term_exact_assoc: Type =
                parse_quote!(<#term_global_assoc as #term_bound>::#term_projection);
            let self_term = gen_type_ident::<SelfTerminalAssoc>();
            let global_term = gen_type_ident::<GlobalTerminalAssoc>();
            let global_term_exact = gen_type_ident::<GlobalTerminalExact>();

            let _ = state.replace_instance_bounds(|trait_bounds| {
                if trait_bounds.len() > 1 {
                    return Err(SubscriberError::OneInstanceBoundAllowed {
                        bound: trait_bounds[1].clone(),
                    }
                    .into());
                }

                let trait_bound = trait_bounds[0];

                let mut bounds = Vec::new();
                for list in &context.idents {
                    let counter_context = (base_assoc, indexes, list).into();
                    let counter_access = CounterAccess::checked_extract(&counter_context, &())?;
                    let mut access_bound = trait_bound.clone();
                    let trait_path = access_bound.path.segments.last_mut().unwrap();
                    let transform_context = (global_assoc, counter_context, false).into();
                    CounterAccess::checked_transform(
                        &counter_access,
                        trait_path,
                        &transform_context,
                    )?;

                    let syn::PathArguments::AngleBracketed(angle) =
                        &mut access_bound.path.segments.last_mut().unwrap().arguments
                    else {
                        unreachable!()
                    };

                    let index = angle
                        .args
                        .iter()
                        .position(|arg| {
                            matches!(
                                arg,
                                syn::GenericArgument::AssocType(_) | syn::GenericArgument::Constraint(_)
                                    | syn::GenericArgument::AssocConst(_)
                            )
                        })
                        .unwrap_or(angle.args.len());

                    angle.args.insert(index, parse_quote!(#self_term = #term_twin_assoc));
                    angle.args.insert(index + 1, parse_quote!(#global_term = #term_global_assoc));
                    angle.args.insert(index + 2, parse_quote!(#global_term_exact = #term_exact_assoc));

                    bounds.push(access_bound);
                }
                Ok(bounds)
            })?;

            state.mutate_global(|global| {
                for list in &context.idents {
                    let counter_context = (base_assoc, indexes, list).into();
                    let counter_access = CounterAccess::checked_extract(&counter_context, &())?;
                    global.push(TypeParamBound::Trait(counter_access.0));
                }
                Ok(())
            })?;

            Ok(())
        })?;

        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &TraitNodeSpace<'a>,
        context: Option<&LeafArgs<SubscriberNode>>,
    ) -> Result<(), proc_macro2::TokenStream> {
        Self::validate(transform, context, |state| {
            let Some(context) = context else {
                return Ok(());
            };
            let indexes = &context.indexes;
            let idents = &context.idents;

            let base_assoc = &state.get_base_assoc()?;
            let global_assoc = &state.get_global_assoc()?;
            let term_twin_assoc = &state.get_terminal_twin_assoc()?;
            let term_global_assoc = &state.get_terminal_global_assoc()?;
            let term_context = (base_assoc, context.get()?).into();
            let term_bound = TerminalAccess::checked_extract(&term_context, &())?.0;
            let term_projection = TerminalAccess::projection();
            let term_exact_assoc: Type =
                parse_quote!(<#term_global_assoc as #term_bound>::#term_projection);
            let self_term = gen_type_ident::<SelfTerminalAssoc>();
            let global_term = gen_type_ident::<GlobalTerminalAssoc>();
            let global_term_exact = gen_type_ident::<GlobalTerminalExact>();

            // Validate replaced bounds (specifically their generic typenum arg)
            let bounds = state.get_instance_bounds()?;

            if bounds.is_empty() {
                return Err(ResolveBugs::ReplacedBoundsAreEmpty {}.into());
            }

            if bounds.len() != idents.len() {
                return Err(ResolveBugs::ReplacedBoundsUnavailableForEveryIdentList {}.into());
            }

            for (list, bound) in idents.iter().zip(bounds) {
                let counter_context = (base_assoc, indexes, list).into();
                let counter_access = CounterAccess::checked_extract(&counter_context, &())?;
                let Some(seg) = bound.path.segments.last() else {
                    return Err(ResolveBugs::ReplacedBoundLastPathSegNotFound {}.into());
                };
                let transform_context = (global_assoc, counter_context).into();
                CounterAccess::validate_transform(&counter_access, seg, Some(&transform_context))?;

                let PathArguments::AngleBracketed(angle) = &seg.arguments else {
                    unreachable!()
                };
                let mut equiv_bounds = angle.args.iter().filter_map(|item| {
                    let GenericArgument::AssocType(assoc) = item else {
                        return None;
                    };
                    Some(assoc)
                });
                let Some(equiv) = equiv_bounds.next() else {
                    return Err(ResolveBugs::TerminalEquivalenceBoundNotFound {}.into());
                };
                let exp = quote! {#self_term = #term_twin_assoc};

                if equiv.to_token_stream().to_string() != exp.to_string() {
                    return Err(ResolveBugs::TerminalEquivalenceBoundInvalid {}.into());
                }

                let Some(equiv) = equiv_bounds.next() else {
                    return Err(ResolveBugs::TerminalEquivalenceBoundNotFound {}.into());
                };
                let exp = quote! {#global_term = #term_global_assoc};

                if equiv.to_token_stream().to_string() != exp.to_string() {
                    return Err(ResolveBugs::TerminalEquivalenceBoundInvalid {}.into());
                }

                let Some(equiv) = equiv_bounds.next() else {
                    return Err(ResolveBugs::TerminalEquivalenceBoundNotFound {}.into());
                };
                let exp = quote! {#global_term_exact = #term_exact_assoc};

                if equiv.to_token_stream().to_string() != exp.to_string() {
                    return Err(ResolveBugs::TerminalEquivalenceBoundInvalid {}.into());
                }

            }

            // Validate Global
            let global = state.get_global()?;

            let bounds = global.bounds.iter().try_fold(
                Vec::new(),
                |mut bounds, item| -> Result<_, TokenStream> {
                    let TypeParamBound::Trait(t) = item else {
                        return Err(ResolveBugs::GlobalHaveNonTraitBound {}.into());
                    };

                    bounds.push(t);
                    Ok(bounds)
                },
            )?;

            let access_bounds_len = bounds.len();
            if access_bounds_len != idents.len() {
                return Err(ResolveBugs::GlobalAccessBoundsLenInconsistentWithIdentList {}.into());
            }

            if global.bounds.len() != access_bounds_len {
                return Err(ResolveBugs::GlobalAllBoundsLenInconsistentWithIdentList {}.into());
            }

            for (list, bound) in idents.iter().zip(bounds) {
                let counter_context = (base_assoc, indexes, list).into();
                CounterAccess::validate_extract(
                    &CounterAccess(bound.clone()),
                    &counter_context,
                    None,
                )?;
            }

            Ok(())
        })?;
        Ok(())
    }
}

// ===============================================================================
// ``````````````````````` INSTANCE TRAIT BRANCH SUBSCRIBER ``````````````````````
// ===============================================================================

/// Subscriber transformation for the `branch` instance node.
///
/// A branch subscriber selects a subtree rooted at a parent instance
/// identifier. The selected branch is resolved into both an
/// [`OnSetAccess`] specialization, marking the beginning of the selected
/// subtree, and a [`BoundaryAccess`] specialization, marking its terminal
/// boundary.
///
/// This transformation:
///
/// - Replaces the original instance bound with transformed
///   [`InitialAssocAccess`] of [`OnSetAccess`] bounds and
///   [`FinalAssocAccess`] of [`BoundaryAccess`] bounds.
/// - Preserves the [`crate::traits::sum`] associated-type equalities in
///   the replaced bounds by binding,
///         - [`SelfTerminalAssoc`] to [`TERMINAL_TWIN_NODE`].
///         - [`GlobalTerminalAssoc`] to [`TERMINAL_GLOBAL_NODE`].
///         - [`GlobalTerminalExact`] to
///             `<TERMINAL_TWIN_NODE as TERMINAL_GLOBAL_NODE_TRAIT>::Terminal`.
/// - Contributes the same bounds to the generated global companion node
///   ([`GLOBAL_NODE`]), allowing the selected subtree to be accumulated
///   independently of the original instance.
/// - Contributes an [`ExactAccess`] derived from the generated
///   [`OnSetAccess`] to the initial range companion node
///   ([`INITIAL_NODE`]), representing the exact beginning of the selected
///   subtree.
/// - Contributes an [`ExactAccess`] derived from the generated
///   [`BoundaryAccess`] to the final range companion node
///   ([`FINAL_NODE`]), representing the exact end of the selected subtree.
///
/// Unlike a leaf, a branch does not represent a complete exact instance.
/// Instead, it represents a subtree whose beginning is identified by
/// [`OnSetAccess`] and whose end is identified by [`BoundaryAccess`]. The
/// generated initial and final companion nodes preserve these exact
/// endpoints for subsequent range-based instance node transformations.
#[derive(Debug, Clone)]
pub(crate) struct TraitBranchSubscriber;

impl<'a> Transformation<TraitNodeSpace<'a>, BranchArgs<SubscriberNode>> for TraitBranchSubscriber {
    fn raw_transform(
        &self,
        transform: &mut TraitNodeSpace<'a>,
        context: &BranchArgs<SubscriberNode>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let indexes = &context.indexes;
        let list = &context.parent;
        Self::prepare(transform, context, |state| {
            let global_assoc = &state.get_global_assoc()?;
            let base_assoc = &state.get_base_assoc()?;
            let base = &state.get_base()?;
            let base_ident = base.ident.clone();
            let term_twin_assoc = &state.get_terminal_twin_assoc()?;
            let term_global_assoc = &state.get_terminal_global_assoc()?;
            let term_context = (base_assoc, context.get()?).into();
            let term_bound = TerminalAccess::checked_extract(&term_context, &())?.0;
            let term_projection = TerminalAccess::projection();
            let term_exact_assoc: Type =
                parse_quote!(<#term_global_assoc as #term_bound>::#term_projection);
            let self_term = gen_type_ident::<SelfTerminalAssoc>();
            let global_term = gen_type_ident::<GlobalTerminalAssoc>();
            let global_term_exact = gen_type_ident::<GlobalTerminalExact>();

            let context = (base_assoc, indexes, list).into();
            let o_access = OnSetAccess::checked_extract(&context, &())?;
            let b_access = BoundaryAccess::checked_extract(&context, &())?;

            state.mutate_global(|global| {
                global.push(TypeParamBound::Trait(b_access.0.clone()));
                global.push(TypeParamBound::Trait(o_access.0.clone()));
                Ok(())
            })?;

            state.mutate_initial(|initial| {
                let o_exact = ExactAccess::checked_extract(&o_access, global_assoc)?.bound;
                initial.push(TypeParamBound::Trait(o_exact));
                Ok(())
            })?;

            state.mutate_final(|finalz| {
                let b_exact = ExactAccess::checked_extract(&b_access, global_assoc)?.bound;
                finalz.push(TypeParamBound::Trait(b_exact));
                Ok(())
            })?;

            let _ = state.replace_instance_bounds(|trait_bounds| {
                let (mut o_access_bound, mut b_access_bound) =
                    provide_bounds(trait_bounds, &base_ident, indexes)?;
                let mut bounds = Vec::new();

                let syn::PathArguments::AngleBracketed(angle) =
                    &mut o_access_bound.path.segments.last_mut().unwrap().arguments
                else {
                    unreachable!()
                };
                let index = angle
                    .args
                    .iter()
                    .position(|arg| {
                        matches!(
                            arg,
                            syn::GenericArgument::AssocType(_) | syn::GenericArgument::Constraint(_)
                                | syn::GenericArgument::AssocConst(_)
                        )
                    })
                    .unwrap_or(angle.args.len());

                angle.args.insert(index, parse_quote!(#self_term = #term_twin_assoc));
                angle.args.insert(index + 1, parse_quote!(#global_term = #term_global_assoc));
                angle.args.insert(index + 2, parse_quote!(#global_term_exact = #term_exact_assoc));

                let syn::PathArguments::AngleBracketed(angle) =
                    &mut b_access_bound.path.segments.last_mut().unwrap().arguments
                else {
                    unreachable!()
                };
                let index = angle
                    .args
                    .iter()
                    .position(|arg| {
                        matches!(
                            arg,
                            syn::GenericArgument::AssocType(_) | syn::GenericArgument::Constraint(_)
                                | syn::GenericArgument::AssocConst(_)
                        )
                    })
                    .unwrap_or(angle.args.len());

                angle.args.insert(index, parse_quote!(#self_term = #term_twin_assoc));
                angle.args.insert(index + 1, parse_quote!(#global_term = #term_global_assoc));
                angle.args.insert(index + 2, parse_quote!(#global_term_exact = #term_exact_assoc));

                bounds.push(o_access_bound);
                bounds.push(b_access_bound);
                Ok(bounds)
            })?;

            Ok(())
        })?;

        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &TraitNodeSpace<'a>,
        context: Option<&BranchArgs<SubscriberNode>>,
    ) -> Result<(), proc_macro2::TokenStream> {
        Self::validate(transform, context, |state| {
            let Some(context) = context else {
                return Ok(());
            };
            let indexes = &context.indexes;
            let list = &context.parent;

            let base_assoc = &state.get_base_assoc()?;
            let global_assoc = &state.get_global_assoc()?;
            let base = state.get_base()?;
            let base_ident = base.ident.clone();
            let term_twin_assoc = &state.get_terminal_twin_assoc()?;
            let term_global_assoc = &state.get_terminal_global_assoc()?;
            let term_context = (base_assoc, context.get()?).into();
            let term_bound = TerminalAccess::checked_extract(&term_context, &())?.0;
            let term_projection = TerminalAccess::projection();
            let term_exact_assoc: Type =
                parse_quote!(<#term_global_assoc as #term_bound>::#term_projection);
            let self_term = gen_type_ident::<SelfTerminalAssoc>();
            let global_term = gen_type_ident::<GlobalTerminalAssoc>();
            let global_term_exact = gen_type_ident::<GlobalTerminalExact>();

            // Validate replaced bounds (specifically their generic typenum arg)
            let bounds = state.get_instance_bounds()?;

            if bounds.is_empty() {
                return Err(BranchBugs::ReplacedBoundsAreEmpty {}.into());
            }

            if bounds.len() != 2 {
                return Err(BranchBugs::ReplacedBoundsInconsistent {}.into());
            }

            let initial_ident =
                gen_type_ident_from_ident_with_suffix::<InstanceNode>(&base_ident, INITIAL_NODE)
                    .to_string();
            let final_ident =
                gen_type_ident_from_ident_with_suffix::<InstanceNode>(&base_ident, FINAL_NODE)
                    .to_string();

            for bound in bounds {
                let Some(seg) = bound.path.segments.last() else {
                    return Err(BranchBugs::ReplacedBoundLastPathSegNotFound {}.into());
                };
                let context = (&base_ident, indexes, true, None).into();
                let str = seg.arguments.to_token_stream().to_string();

                if str.contains(&initial_ident) {
                    InitialAssocAccess::validate_transform(
                        &InitialAssocAccess,
                        seg,
                        Some(&context),
                    )?;
                } else if str.contains(&final_ident) {
                    FinalAssocAccess::validate_transform(&FinalAssocAccess, seg, Some(&context))?;
                } else {
                    return Err(BranchBugs::ReplacedArgNeitherInitialNorFinal {}.into());
                }

                let PathArguments::AngleBracketed(angle) = &seg.arguments else {
                    unreachable!()
                };
                let mut equiv_bounds = angle.args.iter().filter_map(|item| {
                    let GenericArgument::AssocType(assoc) = item else {
                        return None;
                    };
                    Some(assoc)
                });
                let Some(equiv) = equiv_bounds.next() else {
                    return Err(BranchBugs::TerminalEquivalenceBoundNotFound {}.into());
                };
                let exp = quote! {#self_term = #term_twin_assoc};

                if equiv.to_token_stream().to_string() != exp.to_string() {
                    return Err(BranchBugs::TerminalEquivalenceBoundInvalid {}.into());
                }

                let Some(equiv) = equiv_bounds.next() else {
                    return Err(BranchBugs::TerminalEquivalenceBoundNotFound {}.into());
                };
                let exp = quote! {#global_term = #term_global_assoc};

                if equiv.to_token_stream().to_string() != exp.to_string() {
                    return Err(BranchBugs::TerminalEquivalenceBoundInvalid {}.into());
                }

                let Some(equiv) = equiv_bounds.next() else {
                    return Err(BranchBugs::TerminalEquivalenceBoundNotFound {}.into());
                };
                let exp = quote! {#global_term_exact = #term_exact_assoc};

                if equiv.to_token_stream().to_string() != exp.to_string() {
                    return Err(BranchBugs::TerminalEquivalenceBoundInvalid {}.into());
                }
            }

            // Validate Global
            let global = state.get_global()?;

            let bounds = global.bounds.iter().try_fold(
                Vec::new(),
                |mut bounds, item| -> Result<_, TokenStream> {
                    let TypeParamBound::Trait(t) = item else {
                        return Err(BranchBugs::GlobalHaveNonTraitBound {}.into());
                    };

                    bounds.push(t);
                    Ok(bounds)
                },
            )?;

            let access_bounds_len = bounds.len();
            if access_bounds_len != 2 || global.bounds.len() != access_bounds_len {
                return Err(BranchBugs::GlobalAccessBoundsInconsistent {}.into());
            }

            let context = (base_assoc, indexes, list).into();

            let initial = &state.get_initial()?.bounds;
            if initial.len() > 1 {
                return Err(BranchBugs::InitialBoundsInconsistent {}.into());
            };
            let initial = &initial[0];
            let TypeParamBound::Trait(initial) = initial else {
                return Err(BranchBugs::InitialBoundNotTraitBound {}.into());
            };

            let finalz = &state.get_final()?.bounds;
            if finalz.len() > 1 {
                return Err(BranchBugs::FinalBoundsInconsistent {}.into());
            };
            let finalz = &finalz[0];
            let TypeParamBound::Trait(finalz) = finalz else {
                return Err(BranchBugs::FinalBoundNotTraitBound {}.into());
            };

            for bound in bounds {
                let Some(seg) = bound.path.segments.last() else {
                    return Err(BranchBugs::ReplacedBoundLastPathSegNotFound {}.into());
                };
                let str = seg.ident.to_string();
                if str.contains("OnSetAccess") {
                    let o_bound = OnSetAccess(bound.clone());
                    OnSetAccess::validate_extract(&o_bound, &context, None)?;
                    ExactAccess::validate_extract(
                        &ExactAccess::new(initial),
                        &o_bound,
                        Some(global_assoc),
                    )?;
                } else if str.contains("BoundaryAccess") {
                    let b_bound = BoundaryAccess(bound.clone());
                    BoundaryAccess::validate_extract(
                        &BoundaryAccess(bound.clone()),
                        &context,
                        None,
                    )?;
                    ExactAccess::validate_extract(
                        &ExactAccess::new(finalz),
                        &b_bound,
                        Some(global_assoc),
                    )?;
                } else {
                    return Err(BranchBugs::ReplacedBoundNeitherOnSetNorBoundary {}.into());
                }
            }

            Ok(())
        })?;
        Ok(())
    }
}

fn provide_bounds(
    bounds: Vec<&TraitBound>,
    base: &Ident,
    indexes: &IntList,
) -> Result<(TraitBound, TraitBound), TokenStream> {
    let context = (base, indexes, false, None).into();
    if bounds.len() > 1 {
        return Err(SubscriberError::OneInstanceBoundAllowed {
            bound: bounds[1].clone(),
        }
        .into());
    }

    let trait_bound = bounds[0];

    let mut initial_bound = trait_bound.clone();
    let mut final_bound = trait_bound.clone();
    let initial_trait_path = initial_bound.path.segments.last_mut().unwrap();
    let final_trait_path = final_bound.path.segments.last_mut().unwrap();

    InitialAssocAccess::checked_transform(&InitialAssocAccess, initial_trait_path, &context)?;
    FinalAssocAccess::checked_transform(&FinalAssocAccess, final_trait_path, &context)?;

    Ok((initial_bound, final_bound))
}

// ===============================================================================
// ``````````````````````` INSTANCE TRAIT PRUNE SUBSCRIBER ```````````````````````
// ===============================================================================

/// Subscriber transformation for the `prune` instance node.
///
/// A prune subscriber selects every instance from the root of the instance
/// tree up to and including a selected exact instance identifier. The range
/// begins at the implicit root and terminates at the specified leaf.
///
/// This transformation:
///
/// - Replaces the original instance bound with transformed
///   [`InitialAssocAccess`] of [`OnSetAccess`] bounds and
///   [`FinalAssocAccess`] of [`CounterAccess`] bounds.
/// - Preserves the [`crate::traits::sum`] associated-type equalities in
///   the replaced bounds by binding,
///         - [`SelfTerminalAssoc`] to [`TERMINAL_TWIN_NODE`].
///         - [`GlobalTerminalAssoc`] to [`TERMINAL_GLOBAL_NODE`].
///         - [`GlobalTerminalExact`] to
///             `<TERMINAL_TWIN_NODE as TERMINAL_GLOBAL_NODE_TRAIT>::Terminal`.
/// - Uses [`OnSetAccess`] to represent the beginning of the selected range,
///   corresponding to the root of the instance tree.
/// - Uses [`CounterAccess`] to represent the end of the selected range,
///   corresponding to the selected exact instance.
/// - Contributes both bounds to the generated global companion node
///   ([`GLOBAL_NODE`]), allowing the selected range to be accumulated
///   independently of the original instance.
/// - Contributes an [`ExactAccess`] derived from the generated
///   [`OnSetAccess`] to the initial range companion node
///   ([`INITIAL_NODE`]), representing the exact beginning of the range.
/// - Contributes an [`ExactAccess`] derived from the generated
///   [`CounterAccess`] to the final range companion node
///   ([`FINAL_NODE`]), representing the exact end of the range.
///
/// Unlike a leaf or branch selection, a prune subscriber always begins at the
/// root of the instance tree and terminates at an exact instance. The
/// generated initial and final companion nodes preserve these exact range
/// endpoints.
#[derive(Debug, Clone)]
pub(crate) struct TraitPruneSubscriber;

impl<'a> Transformation<TraitNodeSpace<'a>, PruneArgs<SubscriberNode>> for TraitPruneSubscriber {
    fn raw_transform(
        &self,
        transform: &mut TraitNodeSpace<'a>,
        context: &PruneArgs<SubscriberNode>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let indexes = &context.indexes;
        let def = BStringList {
            bytes: Punctuated::new(),
        };
        let list = &context.until;
        Self::prepare(transform, context, |state| {
            let global_assoc = &state.get_global_assoc()?;
            let base_assoc = &state.get_base_assoc()?;
            let base = &state.get_base()?;
            let base_ident = base.ident.clone();
            let term_twin_assoc = &state.get_terminal_twin_assoc()?;
            let term_global_assoc = &state.get_terminal_global_assoc()?;
            let term_context = (base_assoc, context.get()?).into();
            let term_bound = TerminalAccess::checked_extract(&term_context, &())?.0;
            let term_projection = TerminalAccess::projection();
            let term_exact_assoc: Type =
                parse_quote!(<#term_global_assoc as #term_bound>::#term_projection);
            let self_term = gen_type_ident::<SelfTerminalAssoc>();
            let global_term = gen_type_ident::<GlobalTerminalAssoc>();
            let global_term_exact = gen_type_ident::<GlobalTerminalExact>();

            let o_context = (base_assoc, indexes, &def).into();
            let c_context = (base_assoc, indexes, list).into();
            let o_access = OnSetAccess::checked_extract(&o_context, &())?;
            let c_access = CounterAccess::checked_extract(&c_context, &())?;

            state.mutate_global(|global| {
                global.push(TypeParamBound::Trait(o_access.0.clone()));
                global.push(TypeParamBound::Trait(c_access.0.clone()));
                Ok(())
            })?;

            let o_exact = ExactAccess::checked_extract(&o_access, global_assoc)?.bound;
            let c_exact = ExactAccess::checked_extract(&c_access, global_assoc)?.bound;

            state.mutate_initial(|initial| {
                initial.push(TypeParamBound::Trait(o_exact));
                Ok(())
            })?;

            state.mutate_final(|finalz| {
                finalz.push(TypeParamBound::Trait(c_exact));
                Ok(())
            })?;

            let _ = state.replace_instance_bounds(|trait_bounds| {
                let (mut o_access_bound, mut c_access_bound) =
                    provide_bounds(trait_bounds, &base_ident, indexes)?;
                let mut bounds = Vec::new();

                let syn::PathArguments::AngleBracketed(angle) =
                    &mut o_access_bound.path.segments.last_mut().unwrap().arguments
                else {
                    unreachable!()
                };
                let index = angle
                    .args
                    .iter()
                    .position(|arg| {
                        matches!(
                            arg,
                            syn::GenericArgument::AssocType(_) | syn::GenericArgument::Constraint(_)
                                | syn::GenericArgument::AssocConst(_)
                        )
                    })
                    .unwrap_or(angle.args.len());

                angle.args.insert(index, parse_quote!(#self_term = #term_twin_assoc));
                angle.args.insert(index + 1, parse_quote!(#global_term = #term_global_assoc));
                angle.args.insert(index + 2, parse_quote!(#global_term_exact = #term_exact_assoc));

                let syn::PathArguments::AngleBracketed(angle) =
                    &mut c_access_bound.path.segments.last_mut().unwrap().arguments
                else {
                    unreachable!()
                };
                let index = angle
                    .args
                    .iter()
                    .position(|arg| {
                        matches!(
                            arg,
                            syn::GenericArgument::AssocType(_) | syn::GenericArgument::Constraint(_)
                                | syn::GenericArgument::AssocConst(_)
                        )
                    })
                    .unwrap_or(angle.args.len());

                angle.args.insert(index, parse_quote!(#self_term = #term_twin_assoc));
                angle.args.insert(index + 1, parse_quote!(#global_term = #term_global_assoc));
                angle.args.insert(index + 2, parse_quote!(#global_term_exact = #term_exact_assoc));

                bounds.push(o_access_bound);
                bounds.push(c_access_bound);
                Ok(bounds)
            })?;

            Ok(())
        })?;

        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &TraitNodeSpace<'a>,
        context: Option<&PruneArgs<SubscriberNode>>,
    ) -> Result<(), proc_macro2::TokenStream> {
        Self::validate(transform, context, |state| {
            let Some(context) = context else {
                return Ok(());
            };
            let indexes = &context.indexes;
            let def = BStringList {
                bytes: Punctuated::new(),
            };
            let list = &context.until;

            let base_assoc = &state.get_base_assoc()?;
            let global_assoc = &state.get_global_assoc()?;
            let base = state.get_base()?;
            let base_ident = base.ident.clone();
            let term_twin_assoc = &state.get_terminal_twin_assoc()?;
            let term_global_assoc = &state.get_terminal_global_assoc()?;
            let term_context = (base_assoc, context.get()?).into();
            let term_bound = TerminalAccess::checked_extract(&term_context, &())?.0;
            let term_projection = TerminalAccess::projection();
            let term_exact_assoc: Type =
                parse_quote!(<#term_global_assoc as #term_bound>::#term_projection);
            let self_term = gen_type_ident::<SelfTerminalAssoc>();
            let global_term = gen_type_ident::<GlobalTerminalAssoc>();
            let global_term_exact = gen_type_ident::<GlobalTerminalExact>();

            // Validate replaced bounds (specifically their generic typenum arg)
            let bounds = state.get_instance_bounds()?;

            if bounds.is_empty() {
                return Err(PruneBugs::ReplacedBoundsAreEmpty {}.into());
            }

            if bounds.len() != 2 {
                return Err(PruneBugs::ReplacedBoundsInconsistent {}.into());
            }

            let initial_ident =
                gen_type_ident_from_ident_with_suffix::<InstanceNode>(&base_ident, INITIAL_NODE)
                    .to_string();
            let final_ident =
                gen_type_ident_from_ident_with_suffix::<InstanceNode>(&base_ident, FINAL_NODE)
                    .to_string();

            for bound in bounds {
                let Some(seg) = bound.path.segments.last() else {
                    return Err(PruneBugs::ReplacedBoundLastPathSegNotFound {}.into());
                };
                let context = (&base_ident, indexes, true, None).into();
                let str = seg.arguments.to_token_stream().to_string();

                if str.contains(&initial_ident) {
                    InitialAssocAccess::validate_transform(
                        &InitialAssocAccess,
                        seg,
                        Some(&context),
                    )?;
                } else if str.contains(&final_ident) {
                    FinalAssocAccess::validate_transform(&FinalAssocAccess, seg, Some(&context))?;
                } else {
                    return Err(PruneBugs::ReplacedArgNeitherInitialNorFinal {}.into());
                }

                let PathArguments::AngleBracketed(angle) = &seg.arguments else {
                    unreachable!()
                };
                let mut equiv_bounds = angle.args.iter().filter_map(|item| {
                    let GenericArgument::AssocType(assoc) = item else {
                        return None;
                    };
                    Some(assoc)
                });
                let Some(equiv) = equiv_bounds.next() else {
                    return Err(PruneBugs::TerminalEquivalenceBoundNotFound {}.into());
                };
                let exp = quote! {#self_term = #term_twin_assoc};

                if equiv.to_token_stream().to_string() != exp.to_string() {
                    return Err(PruneBugs::TerminalEquivalenceBoundInvalid {}.into());
                }
                let Some(equiv) = equiv_bounds.next() else {
                    return Err(PruneBugs::TerminalEquivalenceBoundNotFound {}.into());
                };
                let exp = quote! {#global_term = #term_global_assoc};

                if equiv.to_token_stream().to_string() != exp.to_string() {
                    return Err(PruneBugs::TerminalEquivalenceBoundInvalid {}.into());
                }

                let Some(equiv) = equiv_bounds.next() else {
                    return Err(PruneBugs::TerminalEquivalenceBoundNotFound {}.into());
                };
                let exp = quote! {#global_term_exact = #term_exact_assoc};

                if equiv.to_token_stream().to_string() != exp.to_string() {
                    return Err(PruneBugs::TerminalEquivalenceBoundInvalid {}.into());
                }
            }

            // Validate Global
            let global = state.get_global()?;

            let bounds = global.bounds.iter().try_fold(
                Vec::new(),
                |mut bounds, item| -> Result<_, TokenStream> {
                    let TypeParamBound::Trait(t) = item else {
                        return Err(PruneBugs::GlobalHaveNonTraitBound {}.into());
                    };

                    bounds.push(t);
                    Ok(bounds)
                },
            )?;

            let access_bounds_len = bounds.len();
            if access_bounds_len != 2 || global.bounds.len() != access_bounds_len {
                return Err(PruneBugs::GlobalAccessBoundsInconsistent {}.into());
            }

            let initial = &state.get_initial()?.bounds;
            if initial.len() > 1 {
                return Err(PruneBugs::InitialBoundsInconsistent {}.into());
            };
            let initial = &initial[0];
            let TypeParamBound::Trait(initial) = initial else {
                return Err(PruneBugs::InitialBoundNotTraitBound {}.into());
            };

            let finalz = &state.get_final()?.bounds;
            if finalz.len() > 1 {
                return Err(PruneBugs::FinalBoundsInconsistent {}.into());
            };
            let finalz = &finalz[0];
            let TypeParamBound::Trait(finalz) = finalz else {
                return Err(PruneBugs::FinalBoundNotTraitBound {}.into());
            };

            let o_context = (base_assoc, indexes, &def).into();
            let c_context = (base_assoc, indexes, list).into();
            for bound in bounds {
                let Some(seg) = bound.path.segments.last() else {
                    return Err(PruneBugs::ReplacedBoundLastPathSegNotFound {}.into());
                };
                let str = seg.ident.to_string();
                if str.contains("OnSetAccess") {
                    let o_bound = OnSetAccess(bound.clone());
                    OnSetAccess::validate_extract(&OnSetAccess(bound.clone()), &o_context, None)?;
                    ExactAccess::validate_extract(
                        &ExactAccess::new(initial),
                        &o_bound,
                        Some(global_assoc),
                    )?;
                } else if str.contains("CounterAccess") {
                    let c_bound = CounterAccess(bound.clone());
                    CounterAccess::validate_extract(
                        &CounterAccess(bound.clone()),
                        &c_context,
                        None,
                    )?;
                    ExactAccess::validate_extract(
                        &ExactAccess::new(finalz),
                        &c_bound,
                        Some(global_assoc),
                    )?;
                } else {
                    return Err(PruneBugs::ReplacedBoundNeitherOnSetNorCounter {}.into());
                }
            }

            Ok(())
        })?;
        Ok(())
    }
}

// ===============================================================================
// ``````````````````````` INSTANCE TRAIT TRIM SUBSCRIBER ````````````````````````
// ===============================================================================

/// Subscriber transformation for the `trim` instance node.
///
/// A trim subscriber selects every instance from the root of the instance
/// tree up to and including a selected branch. The range begins at the
/// implicit root and terminates at the boundary of the selected subtree.
///
/// This transformation:
///
/// - Replaces the original instance bound with transformed
///   [`InitialAssocAccess`] of [`OnSetAccess`] bounds and
///   [`FinalAssocAccess`] of [`BoundaryAccess`] bounds.
/// - Preserves the [`crate::traits::sum`] associated-type equalities in
///   the replaced bounds by binding,
///         - [`SelfTerminalAssoc`] to [`TERMINAL_TWIN_NODE`].
///         - [`GlobalTerminalAssoc`] to [`TERMINAL_GLOBAL_NODE`].
///         - [`GlobalTerminalExact`] to
///             `<TERMINAL_TWIN_NODE as TERMINAL_GLOBAL_NODE_TRAIT>::Terminal`.
/// - Uses [`OnSetAccess`] to represent the beginning of the selected range,
///   corresponding to the root of the instance tree.
/// - Uses [`BoundaryAccess`] to represent the end of the selected range,
///   corresponding to the terminal boundary of the selected subtree.
/// - Contributes both bounds to the generated global companion node
///   ([`GLOBAL_NODE`]), allowing the selected range to be accumulated
///   independently of the original instance.
/// - Contributes an [`ExactAccess`] derived from the generated
///   [`OnSetAccess`] to the initial range companion node
///   ([`INITIAL_NODE`]), representing the exact beginning of the range.
/// - Contributes an [`ExactAccess`] derived from the generated
///   [`BoundaryAccess`] to the final range companion node
///   ([`FINAL_NODE`]), representing the exact end of the selected range.
///
/// Unlike a prune subscriber, which terminates at an exact instance, a trim
/// subscriber terminates at the boundary of a subtree. The generated initial
/// and final companion nodes preserve these exact range endpoints.
#[derive(Debug, Clone)]
pub(crate) struct TraitTrimSubscriber;

impl<'a> Transformation<TraitNodeSpace<'a>, TrimArgs<SubscriberNode>> for TraitTrimSubscriber {
    fn raw_transform(
        &self,
        transform: &mut TraitNodeSpace<'a>,
        context: &TrimArgs<SubscriberNode>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let indexes = &context.indexes;
        let def = BStringList {
            bytes: Punctuated::new(),
        };
        let list = &context.until;
        Self::prepare(transform, context, |state| {
            let global_assoc = &state.get_global_assoc()?;
            let base_assoc = &state.get_base_assoc()?;
            let base = &state.get_base()?;
            let base_ident = base.ident.clone();
            let term_twin_assoc = &state.get_terminal_twin_assoc()?;
            let term_global_assoc = &state.get_terminal_global_assoc()?;
            let term_context = (base_assoc, context.get()?).into();
            let term_bound = TerminalAccess::checked_extract(&term_context, &())?.0;
            let term_projection = TerminalAccess::projection();
            let term_exact_assoc: Type =
                parse_quote!(<#term_global_assoc as #term_bound>::#term_projection);
            let self_term = gen_type_ident::<SelfTerminalAssoc>();
            let global_term = gen_type_ident::<GlobalTerminalAssoc>();
            let global_term_exact = gen_type_ident::<GlobalTerminalExact>();

            let o_context = (base_assoc, indexes, &def).into();
            let o_access = OnSetAccess::checked_extract(&o_context, &())?;
            let b_context = (base_assoc, indexes, list).into();
            let b_access = BoundaryAccess::checked_extract(&b_context, &())?;

            state.mutate_global(|global| {
                global.push(TypeParamBound::Trait(o_access.0.clone()));
                global.push(TypeParamBound::Trait(b_access.0.clone()));
                Ok(())
            })?;

            let o_exact = ExactAccess::checked_extract(&o_access, global_assoc)?.bound;
            let b_exact = ExactAccess::checked_extract(&b_access, global_assoc)?.bound;

            state.mutate_initial(|initial| {
                initial.push(TypeParamBound::Trait(o_exact));
                Ok(())
            })?;

            state.mutate_final(|finalz| {
                finalz.push(TypeParamBound::Trait(b_exact));
                Ok(())
            })?;

            let _ = state.replace_instance_bounds(|trait_bounds| {
                let (mut o_access_bound, mut b_access_bound) =
                    provide_bounds(trait_bounds, &base_ident, indexes)?;
                let mut bounds = Vec::new();

                let syn::PathArguments::AngleBracketed(angle) =
                    &mut o_access_bound.path.segments.last_mut().unwrap().arguments
                else {
                    unreachable!()
                };
                 let index = angle
                    .args
                    .iter()
                    .position(|arg| {
                        matches!(
                            arg,
                            syn::GenericArgument::AssocType(_) | syn::GenericArgument::Constraint(_)
                                | syn::GenericArgument::AssocConst(_)
                        )
                    })
                    .unwrap_or(angle.args.len());

                angle.args.insert(index, parse_quote!(#self_term = #term_twin_assoc));
                angle.args.insert(index + 1, parse_quote!(#global_term = #term_global_assoc));
                angle.args.insert(index + 2, parse_quote!(#global_term_exact = #term_exact_assoc));

                let syn::PathArguments::AngleBracketed(angle) =
                    &mut b_access_bound.path.segments.last_mut().unwrap().arguments
                else {
                    unreachable!()
                };
                 let index = angle
                    .args
                    .iter()
                    .position(|arg| {
                        matches!(
                            arg,
                            syn::GenericArgument::AssocType(_) | syn::GenericArgument::Constraint(_)
                                | syn::GenericArgument::AssocConst(_)
                        )
                    })
                    .unwrap_or(angle.args.len());

                angle.args.insert(index, parse_quote!(#self_term = #term_twin_assoc));
                angle.args.insert(index + 1, parse_quote!(#global_term = #term_global_assoc));
                angle.args.insert(index + 2, parse_quote!(#global_term_exact = #term_exact_assoc));

                bounds.push(o_access_bound);
                bounds.push(b_access_bound);
                Ok(bounds)
            })?;

            Ok(())
        })?;

        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &TraitNodeSpace<'a>,
        context: Option<&TrimArgs<SubscriberNode>>,
    ) -> Result<(), proc_macro2::TokenStream> {
        Self::validate(transform, context, |state| {
            let Some(context) = context else {
                return Ok(());
            };
            let indexes = &context.indexes;
            let def = BStringList {
                bytes: Punctuated::new(),
            };
            let list = &context.until;

            let base_assoc = &state.get_base_assoc()?;
            let global_assoc = &state.get_global_assoc()?;
            let base = state.get_base()?;
            let base_ident = base.ident.clone();
            let term_twin_assoc = &state.get_terminal_twin_assoc()?;
            let term_global_assoc = &state.get_terminal_global_assoc()?;
            let term_context = (base_assoc, context.get()?).into();
            let term_bound = TerminalAccess::checked_extract(&term_context, &())?.0;
            let term_projection = TerminalAccess::projection();
            let term_exact_assoc: Type =
                parse_quote!(<#term_global_assoc as #term_bound>::#term_projection);
            let self_term = gen_type_ident::<SelfTerminalAssoc>();
            let global_term = gen_type_ident::<GlobalTerminalAssoc>();
            let global_term_exact = gen_type_ident::<GlobalTerminalExact>();

            // Validate replaced bounds (specifically their generic typenum arg)
            let bounds = state.get_instance_bounds()?;

            if bounds.is_empty() {
                return Err(TrimBugs::ReplacedBoundsAreEmpty {}.into());
            }

            if bounds.len() != 2 {
                return Err(TrimBugs::ReplacedBoundsInconsistent {}.into());
            }

            let initial_ident =
                gen_type_ident_from_ident_with_suffix::<InstanceNode>(&base_ident, INITIAL_NODE)
                    .to_string();
            let final_ident =
                gen_type_ident_from_ident_with_suffix::<InstanceNode>(&base_ident, FINAL_NODE)
                    .to_string();

            for bound in bounds {
                let Some(seg) = bound.path.segments.last() else {
                    return Err(TrimBugs::ReplacedBoundLastPathSegNotFound {}.into());
                };
                let context = (&base_ident, indexes, true, None).into();
                let str = seg.arguments.to_token_stream().to_string();

                if str.contains(&initial_ident) {
                    InitialAssocAccess::validate_transform(
                        &InitialAssocAccess,
                        seg,
                        Some(&context),
                    )?;
                } else if str.contains(&final_ident) {
                    FinalAssocAccess::validate_transform(&FinalAssocAccess, seg, Some(&context))?;
                } else {
                    return Err(TrimBugs::ReplacedArgNeitherInitialNorFinal {}.into());
                }

                let PathArguments::AngleBracketed(angle) = &seg.arguments else {
                    unreachable!()
                };
                let mut equiv_bounds = angle.args.iter().filter_map(|item| {
                    let GenericArgument::AssocType(assoc) = item else {
                        return None;
                    };
                    Some(assoc)
                });
                let Some(equiv) = equiv_bounds.next() else {
                    return Err(TrimBugs::TerminalEquivalenceBoundNotFound {}.into());
                };
                let exp = quote! {#self_term = #term_twin_assoc};

                if equiv.to_token_stream().to_string() != exp.to_string() {
                    return Err(TrimBugs::TerminalEquivalenceBoundInvalid {}.into());
                }
                let Some(equiv) = equiv_bounds.next() else {
                    return Err(TrimBugs::TerminalEquivalenceBoundNotFound {}.into());
                };
                let exp = quote! {#global_term = #term_global_assoc};

                if equiv.to_token_stream().to_string() != exp.to_string() {
                    return Err(TrimBugs::TerminalEquivalenceBoundInvalid {}.into());
                }

                let Some(equiv) = equiv_bounds.next() else {
                    return Err(TrimBugs::TerminalEquivalenceBoundNotFound {}.into());
                };
                let exp = quote! {#global_term_exact = #term_exact_assoc};

                if equiv.to_token_stream().to_string() != exp.to_string() {
                    return Err(TrimBugs::TerminalEquivalenceBoundInvalid {}.into());
                }
            }

            // Validate Global
            let global = state.get_global()?;

            let bounds = global.bounds.iter().try_fold(
                Vec::new(),
                |mut bounds, item| -> Result<_, TokenStream> {
                    let TypeParamBound::Trait(t) = item else {
                        return Err(TrimBugs::GlobalHaveNonTraitBound {}.into());
                    };

                    bounds.push(t);
                    Ok(bounds)
                },
            )?;

            let access_bounds_len = bounds.len();
            if access_bounds_len != 2 || global.bounds.len() != access_bounds_len {
                return Err(TrimBugs::GlobalAccessBoundsInconsistent {}.into());
            }

            let initial = &state.get_initial()?.bounds;
            if initial.len() > 1 {
                return Err(TrimBugs::InitialBoundsInconsistent {}.into());
            };
            let initial = &initial[0];
            let TypeParamBound::Trait(initial) = initial else {
                return Err(TrimBugs::InitialBoundNotTraitBound {}.into());
            };

            let finalz = &state.get_final()?.bounds;
            if finalz.len() > 1 {
                return Err(TrimBugs::FinalBoundsInconsistent {}.into());
            };
            let finalz = &finalz[0];
            let TypeParamBound::Trait(finalz) = finalz else {
                return Err(TrimBugs::FinalBoundNotTraitBound {}.into());
            };

            let o_context = (base_assoc, indexes, &def).into();
            let b_context = (base_assoc, indexes, list).into();
            for bound in bounds {
                let Some(seg) = bound.path.segments.last() else {
                    return Err(TrimBugs::ReplacedBoundLastPathSegNotFound {}.into());
                };
                let str = seg.ident.to_string();
                if str.contains("OnSetAccess") {
                    let o_bound = OnSetAccess(bound.clone());
                    OnSetAccess::validate_extract(&OnSetAccess(bound.clone()), &o_context, None)?;
                    ExactAccess::validate_extract(
                        &ExactAccess::new(initial),
                        &o_bound,
                        Some(global_assoc),
                    )?;
                } else if str.contains("BoundaryAccess") {
                    let b_bound = BoundaryAccess(bound.clone());
                    BoundaryAccess::validate_extract(
                        &BoundaryAccess(bound.clone()),
                        &b_context,
                        None,
                    )?;
                    ExactAccess::validate_extract(
                        &ExactAccess::new(finalz),
                        &b_bound,
                        Some(global_assoc),
                    )?;
                } else {
                    return Err(TrimBugs::ReplacedBoundNeitherOnSetNorBoundary {}.into());
                }
            }

            Ok(())
        })?;
        Ok(())
    }
}

// ===============================================================================
// ``````````````````````` INSTANCE TRAIT EXTEND SUBSCRIBER ``````````````````````
// ===============================================================================

/// Subscriber transformation for the `extend` instance node.
///
/// An extend subscriber selects every instance from a selected exact instance
/// identifier to the end of the instance tree. The range begins at the
/// specified leaf and terminates at the implicit terminal boundary.
///
/// This transformation:
///
/// - Replaces the original instance bound with transformed
///   [`InitialAssocAccess`] of [`CounterAccess`] bounds and
///   [`FinalAssocAccess`] of [`BoundaryAccess`] bounds.
/// - Preserves the [`crate::traits::sum`] associated-type equalities in
///   the replaced bounds by binding,
///         - [`SelfTerminalAssoc`] to [`TERMINAL_TWIN_NODE`].
///         - [`GlobalTerminalAssoc`] to [`TERMINAL_GLOBAL_NODE`].
///         - [`GlobalTerminalExact`] to
///             `<TERMINAL_TWIN_NODE as TERMINAL_GLOBAL_NODE_TRAIT>::Terminal`.
/// - Uses [`CounterAccess`] to represent the beginning of the selected range,
///   corresponding to the selected exact instance.
/// - Uses [`BoundaryAccess`] to represent the end of the selected range,
///   corresponding to the terminal boundary of the instance tree.
/// - Contributes both bounds to the generated global companion node
///   ([`GLOBAL_NODE`]), allowing the selected range to be accumulated
///   independently of the original instance.
/// - Contributes an [`ExactAccess`] derived from the generated
///   [`CounterAccess`] to the initial range companion node
///   ([`INITIAL_NODE`]), representing the exact beginning of the range.
/// - Contributes an [`ExactAccess`] derived from the generated
///   [`BoundaryAccess`] to the final range companion node
///   ([`FINAL_NODE`]), representing the exact end of the range.
///
/// Unlike a prune subscriber, which begins at the root of the instance tree,
/// an extend subscriber begins at an exact instance and extends to the
/// terminal boundary. The generated initial and final companion nodes
/// preserve these exact range endpoints.
#[derive(Debug, Clone)]
pub(crate) struct TraitExtendSubscriber;

impl<'a> Transformation<TraitNodeSpace<'a>, ExtendArgs<SubscriberNode>> for TraitExtendSubscriber {
    fn raw_transform(
        &self,
        transform: &mut TraitNodeSpace<'a>,
        context: &ExtendArgs<SubscriberNode>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let indexes = &context.indexes;
        let def = BStringList {
            bytes: Punctuated::new(),
        };
        let list = &context.from;
        Self::prepare(transform, context, |state| {
            let global_assoc = &state.get_global_assoc()?;
            let base_assoc = &state.get_base_assoc()?;
            let base = &state.get_base()?;
            let base_ident = base.ident.clone();
            let term_twin_assoc = &state.get_terminal_twin_assoc()?;
            let term_global_assoc = &state.get_terminal_global_assoc()?;
            let term_context = (base_assoc, context.get()?).into();
            let term_bound = TerminalAccess::checked_extract(&term_context, &())?.0;
            let term_projection = TerminalAccess::projection();
            let term_exact_assoc: Type =
                parse_quote!(<#term_global_assoc as #term_bound>::#term_projection);
            let self_term = gen_type_ident::<SelfTerminalAssoc>();
            let global_term = gen_type_ident::<GlobalTerminalAssoc>();
            let global_term_exact = gen_type_ident::<GlobalTerminalExact>();

            let c_context = (base_assoc, indexes, list).into();
            let c_access = CounterAccess::checked_extract(&c_context, &())?;
            let b_context = (base_assoc, indexes, &def).into();
            let b_access = BoundaryAccess::checked_extract(&b_context, &())?;

            state.mutate_global(|global| {
                global.push(TypeParamBound::Trait(c_access.0.clone()));
                global.push(TypeParamBound::Trait(b_access.0.clone()));
                Ok(())
            })?;

            let c_exact = ExactAccess::checked_extract(&c_access, global_assoc)?.bound;
            let b_exact = ExactAccess::checked_extract(&b_access, global_assoc)?.bound;

            state.mutate_initial(|initial| {
                initial.push(TypeParamBound::Trait(c_exact));
                Ok(())
            })?;

            state.mutate_final(|finalz| {
                finalz.push(TypeParamBound::Trait(b_exact));
                Ok(())
            })?;

            let _ = state.replace_instance_bounds(|trait_bounds| {
                let (mut c_access_bound, mut b_access_bound) =
                    provide_bounds(trait_bounds, &base_ident, indexes)?;
                let mut bounds = Vec::new();

                let syn::PathArguments::AngleBracketed(angle) =
                    &mut c_access_bound.path.segments.last_mut().unwrap().arguments
                else {
                    unreachable!()
                };
                 let index = angle
                    .args
                    .iter()
                    .position(|arg| {
                        matches!(
                            arg,
                            syn::GenericArgument::AssocType(_) | syn::GenericArgument::Constraint(_)
                                | syn::GenericArgument::AssocConst(_)
                        )
                    })
                    .unwrap_or(angle.args.len());

                angle.args.insert(index, parse_quote!(#self_term = #term_twin_assoc));
                angle.args.insert(index + 1, parse_quote!(#global_term = #term_global_assoc));
                angle.args.insert(index + 2, parse_quote!(#global_term_exact = #term_exact_assoc));

                let syn::PathArguments::AngleBracketed(angle) =
                    &mut b_access_bound.path.segments.last_mut().unwrap().arguments
                else {
                    unreachable!()
                };
                 let index = angle
                    .args
                    .iter()
                    .position(|arg| {
                        matches!(
                            arg,
                            syn::GenericArgument::AssocType(_) | syn::GenericArgument::Constraint(_)
                                | syn::GenericArgument::AssocConst(_)
                        )
                    })
                    .unwrap_or(angle.args.len());

                angle.args.insert(index, parse_quote!(#self_term = #term_twin_assoc));
                angle.args.insert(index + 1, parse_quote!(#global_term = #term_global_assoc));
                angle.args.insert(index + 2, parse_quote!(#global_term_exact = #term_exact_assoc));

                bounds.push(c_access_bound);
                bounds.push(b_access_bound);
                Ok(bounds)
            })?;

            Ok(())
        })?;

        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &TraitNodeSpace<'a>,
        context: Option<&ExtendArgs<SubscriberNode>>,
    ) -> Result<(), proc_macro2::TokenStream> {
        Self::validate(transform, context, |state| {
            let Some(context) = context else {
                return Ok(());
            };
            let indexes = &context.indexes;
            let def = BStringList {
                bytes: Punctuated::new(),
            };
            let list = &context.from;

            let base_assoc = &state.get_base_assoc()?;
            let global_assoc = &state.get_global_assoc()?;
            let base = state.get_base()?;
            let base_ident = base.ident.clone();
            let term_twin_assoc = &state.get_terminal_twin_assoc()?;
            let term_global_assoc = &state.get_terminal_global_assoc()?;
            let term_context = (base_assoc, context.get()?).into();
            let term_bound = TerminalAccess::checked_extract(&term_context, &())?.0;
            let term_projection = TerminalAccess::projection();
            let term_exact_assoc: Type =
                parse_quote!(<#term_global_assoc as #term_bound>::#term_projection);
            let self_term = gen_type_ident::<SelfTerminalAssoc>();
            let global_term = gen_type_ident::<GlobalTerminalAssoc>();
            let global_term_exact = gen_type_ident::<GlobalTerminalExact>();

            // Validate replaced bounds (specifically their generic typenum arg)
            let bounds = state.get_instance_bounds()?;

            if bounds.is_empty() {
                return Err(ExtendBugs::ReplacedBoundsAreEmpty {}.into());
            }

            if bounds.len() != 2 {
                return Err(ExtendBugs::ReplacedBoundsInconsistent {}.into());
            }

            let initial_ident =
                gen_type_ident_from_ident_with_suffix::<InstanceNode>(&base_ident, INITIAL_NODE)
                    .to_string();
            let final_ident =
                gen_type_ident_from_ident_with_suffix::<InstanceNode>(&base_ident, FINAL_NODE)
                    .to_string();

            for bound in bounds {
                let Some(seg) = bound.path.segments.last() else {
                    return Err(ExtendBugs::ReplacedBoundLastPathSegNotFound {}.into());
                };
                let context = (&base_ident, indexes, true, None).into();
                let str = seg.arguments.to_token_stream().to_string();

                if str.contains(&initial_ident) {
                    InitialAssocAccess::validate_transform(
                        &InitialAssocAccess,
                        seg,
                        Some(&context),
                    )?;
                } else if str.contains(&final_ident) {
                    FinalAssocAccess::validate_transform(&FinalAssocAccess, seg, Some(&context))?;
                } else {
                    return Err(ExtendBugs::ReplacedArgNeitherInitialNorFinal {}.into());
                }

                let PathArguments::AngleBracketed(angle) = &seg.arguments else {
                    unreachable!()
                };
                let mut equiv_bounds = angle.args.iter().filter_map(|item| {
                    let GenericArgument::AssocType(assoc) = item else {
                        return None;
                    };
                    Some(assoc)
                });
                let Some(equiv) = equiv_bounds.next() else {
                    return Err(ExtendBugs::TerminalEquivalenceBoundNotFound {}.into());
                };
                let exp = quote! {#self_term = #term_twin_assoc};

                if equiv.to_token_stream().to_string() != exp.to_string() {
                    return Err(ExtendBugs::TerminalEquivalenceBoundInvalid {}.into());
                }
                let Some(equiv) = equiv_bounds.next() else {
                    return Err(ExtendBugs::TerminalEquivalenceBoundNotFound {}.into());
                };
                let exp = quote! {#global_term = #term_global_assoc};

                if equiv.to_token_stream().to_string() != exp.to_string() {
                    return Err(ExtendBugs::TerminalEquivalenceBoundInvalid {}.into());
                }

                let Some(equiv) = equiv_bounds.next() else {
                    return Err(ExtendBugs::TerminalEquivalenceBoundNotFound {}.into());
                };
                let exp = quote! {#global_term_exact = #term_exact_assoc};

                if equiv.to_token_stream().to_string() != exp.to_string() {
                    return Err(ExtendBugs::TerminalEquivalenceBoundInvalid {}.into());
                }
            }

            // Validate Global
            let global = state.get_global()?;

            let bounds = global.bounds.iter().try_fold(
                Vec::new(),
                |mut bounds, item| -> Result<_, TokenStream> {
                    let TypeParamBound::Trait(t) = item else {
                        return Err(ExtendBugs::GlobalHaveNonTraitBound {}.into());
                    };

                    bounds.push(t);
                    Ok(bounds)
                },
            )?;

            let access_bounds_len = bounds.len();
            if access_bounds_len != 2 || global.bounds.len() != access_bounds_len {
                return Err(ExtendBugs::GlobalAccessBoundsInconsistent {}.into());
            }

            let initial = &state.get_initial()?.bounds;
            if initial.len() > 1 {
                return Err(ExtendBugs::InitialBoundsInconsistent {}.into());
            };
            let initial = &initial[0];
            let TypeParamBound::Trait(initial) = initial else {
                return Err(ExtendBugs::InitialBoundNotTraitBound {}.into());
            };

            let finalz = &state.get_final()?.bounds;
            if finalz.len() > 1 {
                return Err(ExtendBugs::FinalBoundsInconsistent {}.into());
            };
            let finalz = &finalz[0];
            let TypeParamBound::Trait(finalz) = finalz else {
                return Err(ExtendBugs::FinalBoundNotTraitBound {}.into());
            };

            let c_context = (base_assoc, indexes, list).into();
            let b_context = (base_assoc, indexes, &def).into();
            for bound in bounds {
                let Some(seg) = bound.path.segments.last() else {
                    return Err(ExtendBugs::ReplacedBoundLastPathSegNotFound {}.into());
                };
                let str = seg.ident.to_string();
                if str.contains("CounterAccess") {
                    let c_bound = CounterAccess(bound.clone());
                    CounterAccess::validate_extract(
                        &CounterAccess(bound.clone()),
                        &c_context,
                        None,
                    )?;
                    ExactAccess::validate_extract(
                        &ExactAccess::new(initial),
                        &c_bound,
                        Some(global_assoc),
                    )?;
                } else if str.contains("BoundaryAccess") {
                    let b_bound = BoundaryAccess(bound.clone());
                    BoundaryAccess::validate_extract(
                        &BoundaryAccess(bound.clone()),
                        &b_context,
                        None,
                    )?;
                    ExactAccess::validate_extract(
                        &ExactAccess::new(finalz),
                        &b_bound,
                        Some(global_assoc),
                    )?;
                } else {
                    return Err(ExtendBugs::ReplacedBoundNeitherCounterNorBoundary {}.into());
                }
            }

            Ok(())
        })?;
        Ok(())
    }
}

// ===============================================================================
// ``````````````````````` INSTANCE TRAIT SPREAD SUBSCRIBER ``````````````````````
// ===============================================================================

/// Subscriber transformation for the `spread` instance node.
///
/// A spread subscriber selects every instance from a selected branch to the
/// end of the instance tree. The range begins at the selected subtree and
/// terminates at the implicit terminal boundary.
///
/// This transformation:
///
/// - Replaces the original instance bound with transformed
///   [`InitialAssocAccess`] of [`OnSetAccess`] bounds and
///   [`FinalAssocAccess`] of [`BoundaryAccess`] bounds.
/// - Preserves the [`crate::traits::sum`] associated-type equalities in
///   the replaced bounds by binding,
///         - [`SelfTerminalAssoc`] to [`TERMINAL_TWIN_NODE`].
///         - [`GlobalTerminalAssoc`] to [`TERMINAL_GLOBAL_NODE`].
///         - [`GlobalTerminalExact`] to
///             `<TERMINAL_TWIN_NODE as TERMINAL_GLOBAL_NODE_TRAIT>::Terminal`.
/// - Uses [`OnSetAccess`] to represent the beginning of the selected range,
///   corresponding to the root of the selected subtree.
/// - Uses [`BoundaryAccess`] to represent the end of the selected range,
///   corresponding to the terminal boundary of the instance tree.
/// - Contributes both bounds to the generated global companion node
///   ([`GLOBAL_NODE`]), allowing the selected range to be accumulated
///   independently of the original instance.
/// - Contributes an [`ExactAccess`] derived from the generated
///   [`OnSetAccess`] to the initial range companion node
///   ([`INITIAL_NODE`]), representing the exact beginning of the range.
/// - Contributes an [`ExactAccess`] derived from the generated
///   [`BoundaryAccess`] to the final range companion node
///   ([`FINAL_NODE`]), representing the exact end of the range.
///
/// Unlike an extend subscriber, which begins at an exact instance, a spread
/// subscriber begins at the root of a subtree and extends to the terminal
/// boundary. The generated initial and final companion nodes preserve these
/// exact range endpoints.
#[derive(Debug, Clone)]
pub(crate) struct TraitSpreadSubscriber;

impl<'a> Transformation<TraitNodeSpace<'a>, SpreadArgs<SubscriberNode>> for TraitSpreadSubscriber {
    fn raw_transform(
        &self,
        transform: &mut TraitNodeSpace<'a>,
        context: &SpreadArgs<SubscriberNode>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let indexes = &context.indexes;
        let def = BStringList {
            bytes: Punctuated::new(),
        };
        let list = &context.from;
        Self::prepare(transform, context, |state| {
            let global_assoc = &state.get_global_assoc()?;
            let base_assoc = &state.get_base_assoc()?;
            let base = &state.get_base()?;
            let base_ident = base.ident.clone();
            let term_twin_assoc = &state.get_terminal_twin_assoc()?;
            let term_global_assoc = &state.get_terminal_global_assoc()?;
            let term_context = (base_assoc, context.get()?).into();
            let term_bound = TerminalAccess::checked_extract(&term_context, &())?.0;
            let term_projection = TerminalAccess::projection();
            let term_exact_assoc: Type =
                parse_quote!(<#term_global_assoc as #term_bound>::#term_projection);
            let self_term = gen_type_ident::<SelfTerminalAssoc>();
            let global_term = gen_type_ident::<GlobalTerminalAssoc>();
            let global_term_exact = gen_type_ident::<GlobalTerminalExact>();

            let o_context = (base_assoc, indexes, list).into();
            let o_access = OnSetAccess::checked_extract(&o_context, &())?;
            let b_context = (base_assoc, indexes, &def).into();
            let b_access = BoundaryAccess::checked_extract(&b_context, &())?;

            state.mutate_global(|global| {
                global.push(TypeParamBound::Trait(o_access.0.clone()));
                global.push(TypeParamBound::Trait(b_access.0.clone()));
                Ok(())
            })?;

            let o_exact = ExactAccess::checked_extract(&o_access, global_assoc)?.bound;
            let b_exact = ExactAccess::checked_extract(&b_access, global_assoc)?.bound;

            state.mutate_initial(|initial| {
                initial.push(TypeParamBound::Trait(o_exact));
                Ok(())
            })?;

            state.mutate_final(|finalz| {
                finalz.push(TypeParamBound::Trait(b_exact));
                Ok(())
            })?;

            let _ = state.replace_instance_bounds(|trait_bounds| {
                let (mut o_access_bound, mut b_access_bound) =
                    provide_bounds(trait_bounds, &base_ident, indexes)?;
                let mut bounds = Vec::new();

                let syn::PathArguments::AngleBracketed(angle) =
                    &mut o_access_bound.path.segments.last_mut().unwrap().arguments
                else {
                    unreachable!()
                };
                 let index = angle
                    .args
                    .iter()
                    .position(|arg| {
                        matches!(
                            arg,
                            syn::GenericArgument::AssocType(_) | syn::GenericArgument::Constraint(_)
                                | syn::GenericArgument::AssocConst(_)
                        )
                    })
                    .unwrap_or(angle.args.len());

                angle.args.insert(index, parse_quote!(#self_term = #term_twin_assoc));
                angle.args.insert(index + 1, parse_quote!(#global_term = #term_global_assoc));
                angle.args.insert(index + 2, parse_quote!(#global_term_exact = #term_exact_assoc));

                let syn::PathArguments::AngleBracketed(angle) =
                    &mut b_access_bound.path.segments.last_mut().unwrap().arguments
                else {
                    unreachable!()
                };
                 let index = angle
                    .args
                    .iter()
                    .position(|arg| {
                        matches!(
                            arg,
                            syn::GenericArgument::AssocType(_) | syn::GenericArgument::Constraint(_)
                                | syn::GenericArgument::AssocConst(_)
                        )
                    })
                    .unwrap_or(angle.args.len());

                angle.args.insert(index, parse_quote!(#self_term = #term_twin_assoc));
                angle.args.insert(index + 1, parse_quote!(#global_term = #term_global_assoc));
                angle.args.insert(index + 2, parse_quote!(#global_term_exact = #term_exact_assoc));

                bounds.push(o_access_bound);
                bounds.push(b_access_bound);
                Ok(bounds)
            })?;

            Ok(())
        })?;

        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &TraitNodeSpace<'a>,
        context: Option<&SpreadArgs<SubscriberNode>>,
    ) -> Result<(), proc_macro2::TokenStream> {
        Self::validate(transform, context, |state| {
            let Some(context) = context else {
                return Ok(());
            };
            let indexes = &context.indexes;
            let def = BStringList {
                bytes: Punctuated::new(),
            };
            let list = &context.from;

            let base_assoc = &state.get_base_assoc()?;
            let global_assoc = &state.get_global_assoc()?;
            let base = state.get_base()?;
            let base_ident = base.ident.clone();
            let term_twin_assoc = &state.get_terminal_twin_assoc()?;
            let term_global_assoc = &state.get_terminal_global_assoc()?;
            let term_context = (base_assoc, context.get()?).into();
            let term_bound = TerminalAccess::checked_extract(&term_context, &())?.0;
            let term_projection = TerminalAccess::projection();
            let term_exact_assoc: Type =
                parse_quote!(<#term_global_assoc as #term_bound>::#term_projection);
            let self_term = gen_type_ident::<SelfTerminalAssoc>();
            let global_term = gen_type_ident::<GlobalTerminalAssoc>();
            let global_term_exact = gen_type_ident::<GlobalTerminalExact>();

            // Validate replaced bounds (specifically their generic typenum arg)
            let bounds = state.get_instance_bounds()?;

            if bounds.is_empty() {
                return Err(SpreadBugs::ReplacedBoundsAreEmpty {}.into());
            }

            if bounds.len() != 2 {
                return Err(SpreadBugs::ReplacedBoundsInconsistent {}.into());
            }

            let initial_ident =
                gen_type_ident_from_ident_with_suffix::<InstanceNode>(&base_ident, INITIAL_NODE)
                    .to_string();
            let final_ident =
                gen_type_ident_from_ident_with_suffix::<InstanceNode>(&base_ident, FINAL_NODE)
                    .to_string();

            for bound in bounds {
                let Some(seg) = bound.path.segments.last() else {
                    return Err(SpreadBugs::ReplacedBoundLastPathSegNotFound {}.into());
                };
                let context = (&base_ident, indexes, true, None).into();
                let str = seg.arguments.to_token_stream().to_string();

                if str.contains(&initial_ident) {
                    InitialAssocAccess::validate_transform(
                        &InitialAssocAccess,
                        seg,
                        Some(&context),
                    )?;
                } else if str.contains(&final_ident) {
                    FinalAssocAccess::validate_transform(&FinalAssocAccess, seg, Some(&context))?;
                } else {
                    return Err(SpreadBugs::ReplacedArgNeitherInitialNorFinal {}.into());
                }

                let PathArguments::AngleBracketed(angle) = &seg.arguments else {
                    unreachable!()
                };
                let mut equiv_bounds = angle.args.iter().filter_map(|item| {
                    let GenericArgument::AssocType(assoc) = item else {
                        return None;
                    };
                    Some(assoc)
                });
                let Some(equiv) = equiv_bounds.next() else {
                    return Err(SpreadBugs::TerminalEquivalenceBoundNotFound {}.into());
                };
                let exp = quote! {#self_term = #term_twin_assoc};

                if equiv.to_token_stream().to_string() != exp.to_string() {
                    return Err(SpreadBugs::TerminalEquivalenceBoundInvalid {}.into());
                }
                let Some(equiv) = equiv_bounds.next() else {
                    return Err(SpreadBugs::TerminalEquivalenceBoundNotFound {}.into());
                };
                let exp = quote! {#global_term = #term_global_assoc};

                if equiv.to_token_stream().to_string() != exp.to_string() {
                    return Err(SpreadBugs::TerminalEquivalenceBoundInvalid {}.into());
                }

                let Some(equiv) = equiv_bounds.next() else {
                    return Err(SpreadBugs::TerminalEquivalenceBoundNotFound {}.into());
                };
                let exp = quote! {#global_term_exact = #term_exact_assoc};

                if equiv.to_token_stream().to_string() != exp.to_string() {
                    return Err(SpreadBugs::TerminalEquivalenceBoundInvalid {}.into());
                }
            }

            // Validate Global
            let global = state.get_global()?;

            let bounds = global.bounds.iter().try_fold(
                Vec::new(),
                |mut bounds, item| -> Result<_, TokenStream> {
                    let TypeParamBound::Trait(t) = item else {
                        return Err(SpreadBugs::GlobalHaveNonTraitBound {}.into());
                    };

                    bounds.push(t);
                    Ok(bounds)
                },
            )?;

            let access_bounds_len = bounds.len();
            if access_bounds_len != 2 || global.bounds.len() != access_bounds_len {
                return Err(SpreadBugs::GlobalAccessBoundsInconsistent {}.into());
            }

            let initial = &state.get_initial()?.bounds;
            if initial.len() > 1 {
                return Err(SpreadBugs::InitialBoundsInconsistent {}.into());
            };
            let initial = &initial[0];
            let TypeParamBound::Trait(initial) = initial else {
                return Err(SpreadBugs::InitialBoundNotTraitBound {}.into());
            };

            let finalz = &state.get_final()?.bounds;
            if finalz.len() > 1 {
                return Err(SpreadBugs::FinalBoundsInconsistent {}.into());
            };
            let finalz = &finalz[0];
            let TypeParamBound::Trait(finalz) = finalz else {
                return Err(SpreadBugs::FinalBoundNotTraitBound {}.into());
            };

            let o_context = (base_assoc, indexes, list).into();
            let b_context = (base_assoc, indexes, &def).into();
            for bound in bounds {
                let Some(seg) = bound.path.segments.last() else {
                    return Err(SpreadBugs::ReplacedBoundLastPathSegNotFound {}.into());
                };
                let str = seg.ident.to_string();
                if str.contains("OnSetAccess") {
                    let o_bound = OnSetAccess(bound.clone());
                    OnSetAccess::validate_extract(&OnSetAccess(bound.clone()), &o_context, None)?;
                    ExactAccess::validate_extract(
                        &ExactAccess::new(initial),
                        &o_bound,
                        Some(global_assoc),
                    )?;
                } else if str.contains("BoundaryAccess") {
                    let b_bound = BoundaryAccess(bound.clone());
                    BoundaryAccess::validate_extract(
                        &BoundaryAccess(bound.clone()),
                        &b_context,
                        None,
                    )?;
                    ExactAccess::validate_extract(
                        &ExactAccess::new(finalz),
                        &b_bound,
                        Some(global_assoc),
                    )?;
                } else {
                    return Err(SpreadBugs::ReplacedBoundNeitherOnSetNorBoundary {}.into());
                }
            }

            Ok(())
        })?;
        Ok(())
    }
}

// ===============================================================================
// `````````````````````` INSTANCE TRAIT TRAVERSE SUBSCRIBER `````````````````````
// ===============================================================================

/// Subscriber transformation for the `traverse` instance node.
///
/// A traverse subscriber selects every instance within a subtree from its
/// root to a selected exact instance identifier. The subtree is identified by
/// the `until` branch, while the traversal terminates at the `from` leaf.
///
/// This transformation:
///
/// - Replaces the original instance bound with transformed
///   [`InitialAssocAccess`] of [`OnSetAccess`] bounds and
///   [`FinalAssocAccess`] of [`CounterAccess`] bounds.
/// - Preserves the [`crate::traits::sum`] associated-type equalities in
///   the replaced bounds by binding,
///         - [`SelfTerminalAssoc`] to [`TERMINAL_TWIN_NODE`].
///         - [`GlobalTerminalAssoc`] to [`TERMINAL_GLOBAL_NODE`].
///         - [`GlobalTerminalExact`] to
///             `<TERMINAL_TWIN_NODE as TERMINAL_GLOBAL_NODE_TRAIT>::Terminal`.
/// - Uses [`OnSetAccess`] to represent the beginning of the selected range,
///   corresponding to the root of the selected subtree.
/// - Uses [`CounterAccess`] to represent the end of the selected range,
///   corresponding to the selected exact instance.
/// - Contributes both bounds to the generated global companion node
///   ([`GLOBAL_NODE`]).
/// - Contributes an [`ExactAccess`] derived from [`OnSetAccess`] to
///   [`INITIAL_NODE`], representing the exact beginning of the traversal.
/// - Contributes an [`ExactAccess`] derived from [`CounterAccess`] to
///   [`FINAL_NODE`], representing the exact terminating instance.
///
/// The generated initial and final companion nodes preserve the exact
/// traversal endpoints, allowing subsequent range-based instance node
/// transformations to recover the selected path.
#[derive(Debug, Clone)]
pub(crate) struct TraitTraverseSubscriber;

impl<'a> Transformation<TraitNodeSpace<'a>, TraverseArgs<SubscriberNode>>
    for TraitTraverseSubscriber
{
    fn raw_transform(
        &self,
        transform: &mut TraitNodeSpace<'a>,
        context: &TraverseArgs<SubscriberNode>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let indexes = &context.indexes;
        let leaf = &context.from;
        let branch = &context.until;

        let mut prefix = BStringList {
            bytes: Punctuated::new(),
        };
        let mut collected = false;

        for (branch, leaf) in branch.bytes.iter().zip(&leaf.bytes) {
            if !collected {
                if branch == leaf {
                    prefix.bytes.push(branch.clone());
                } else {
                    collected = true;
                }
            }
        }

        Self::prepare(transform, context, |state| {
            let global_assoc = &state.get_global_assoc()?;
            let base_assoc = &state.get_base_assoc()?;
            let base = &state.get_base()?;
            let base_ident = base.ident.clone();
            let term_twin_assoc = &state.get_terminal_twin_assoc()?;
            let term_global_assoc = &state.get_terminal_global_assoc()?;
            let term_context = (base_assoc, context.get()?).into();
            let term_bound = TerminalAccess::checked_extract(&term_context, &())?.0;
            let term_projection = TerminalAccess::projection();
            let term_exact_assoc: Type =
                parse_quote!(<#term_global_assoc as #term_bound>::#term_projection);
            let self_term = gen_type_ident::<SelfTerminalAssoc>();
            let global_term = gen_type_ident::<GlobalTerminalAssoc>();
            let global_term_exact = gen_type_ident::<GlobalTerminalExact>();

            // Selected subtree boundary.
            let o_context = (base_assoc, indexes, branch).into();
            let o_access = OnSetAccess::checked_extract(&o_context, &())?;

            // Common prefix boundary, when one exists.
            let p_access = if !prefix.bytes.is_empty() {
                let p_context = (base_assoc, indexes, &prefix).into();
                let p_access = OnSetAccess::checked_extract(&p_context, &())?;
                Some(p_access)
            } else {
                None
            };

            // Exact terminating instance.
            let c_context = (base_assoc, indexes, leaf).into();
            let c_access = CounterAccess::checked_extract(&c_context, &())?;

            state.mutate_global(|global| {
                global.push(TypeParamBound::Trait(o_access.0.clone()));

                if let Some(p_access) = &p_access {
                    global.push(TypeParamBound::Trait(p_access.0.clone()));
                }

                global.push(TypeParamBound::Trait(c_access.0.clone()));

                Ok(())
            })?;

            let o_exact = ExactAccess::checked_extract(&o_access, global_assoc)?.bound;

            let p_exact = if let Some(p_access) = p_access {
                let p_exact = ExactAccess::checked_extract(&p_access, global_assoc)?.bound;
                Some(p_exact)
            } else {
                None
            };

            let c_exact = ExactAccess::checked_extract(&c_access, global_assoc)?.bound;

            state.mutate_initial(|initial| {
                initial.push(TypeParamBound::Trait(o_exact));

                if let Some(p_exact) = p_exact {
                    initial.push(TypeParamBound::Trait(p_exact));
                }

                Ok(())
            })?;

            state.mutate_final(|finalz| {
                finalz.push(TypeParamBound::Trait(c_exact));
                Ok(())
            })?;

            let _ = state.replace_instance_bounds(|trait_bounds| {
                let (mut o_access_bound, mut c_access_bound) =
                    provide_bounds(trait_bounds, &base_ident, indexes)?;
                let mut bounds = Vec::new();

                let syn::PathArguments::AngleBracketed(angle) =
                    &mut o_access_bound.path.segments.last_mut().unwrap().arguments
                else {
                    unreachable!()
                };
                 let index = angle
                    .args
                    .iter()
                    .position(|arg| {
                        matches!(
                            arg,
                            syn::GenericArgument::AssocType(_) | syn::GenericArgument::Constraint(_)
                                | syn::GenericArgument::AssocConst(_)
                        )
                    })
                    .unwrap_or(angle.args.len());

                angle.args.insert(index, parse_quote!(#self_term = #term_twin_assoc));
                angle.args.insert(index + 1, parse_quote!(#global_term = #term_global_assoc));
                angle.args.insert(index + 2, parse_quote!(#global_term_exact = #term_exact_assoc));

                let syn::PathArguments::AngleBracketed(angle) =
                    &mut c_access_bound.path.segments.last_mut().unwrap().arguments
                else {
                    unreachable!()
                };
                 let index = angle
                    .args
                    .iter()
                    .position(|arg| {
                        matches!(
                            arg,
                            syn::GenericArgument::AssocType(_) | syn::GenericArgument::Constraint(_)
                                | syn::GenericArgument::AssocConst(_)
                        )
                    })
                    .unwrap_or(angle.args.len());

                angle.args.insert(index, parse_quote!(#self_term = #term_twin_assoc));
                angle.args.insert(index + 1, parse_quote!(#global_term = #term_global_assoc));
                angle.args.insert(index + 2, parse_quote!(#global_term_exact = #term_exact_assoc));

                bounds.push(o_access_bound);
                bounds.push(c_access_bound);

                Ok(bounds)
            })?;

            Ok(())
        })?;

        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &TraitNodeSpace<'a>,
        context: Option<&TraverseArgs<SubscriberNode>>,
    ) -> Result<(), proc_macro2::TokenStream> {
        Self::validate(transform, context, |state| {
            let Some(context) = context else {
                return Ok(());
            };

            let indexes = &context.indexes;
            let leaf = &context.from;
            let branch = &context.until;

            let mut prefix = BStringList {
                bytes: Punctuated::new(),
            };
            let mut collected = false;

            for (branch, leaf) in branch.bytes.iter().zip(&leaf.bytes) {
                if !collected {
                    if branch == leaf {
                        prefix.bytes.push(branch.clone());
                    } else {
                        collected = true;
                    }
                }
            }

            let base_assoc = &state.get_base_assoc()?;
            let global_assoc = &state.get_global_assoc()?;
            let base = state.get_base()?;
            let base_ident = base.ident.clone();
            let term_twin_assoc = &state.get_terminal_twin_assoc()?;
            let term_global_assoc = &state.get_terminal_global_assoc()?;
            let term_context = (base_assoc, context.get()?).into();
            let term_bound = TerminalAccess::checked_extract(&term_context, &())?.0;
            let term_projection = TerminalAccess::projection();
            let term_exact_assoc: Type =
                parse_quote!(<#term_global_assoc as #term_bound>::#term_projection);
            let self_term = gen_type_ident::<SelfTerminalAssoc>();
            let global_term = gen_type_ident::<GlobalTerminalAssoc>();
            let global_term_exact = gen_type_ident::<GlobalTerminalExact>();

            // Validate replaced bounds.
            let bounds = state.get_instance_bounds()?;

            if bounds.is_empty() {
                return Err(TraverseBugs::ReplacedBoundsAreEmpty {}.into());
            }

            if bounds.len() != 2 {
                return Err(TraverseBugs::ReplacedBoundsInconsistent {}.into());
            }

            let initial_ident =
                gen_type_ident_from_ident_with_suffix::<InstanceNode>(&base_ident, INITIAL_NODE)
                    .to_string();

            let final_ident =
                gen_type_ident_from_ident_with_suffix::<InstanceNode>(&base_ident, FINAL_NODE)
                    .to_string();

            for bound in bounds {
                let Some(seg) = bound.path.segments.last() else {
                    return Err(TraverseBugs::ReplacedBoundLastPathSegNotFound {}.into());
                };

                let context = (&base_ident, indexes, true, None).into();
                let str = seg.arguments.to_token_stream().to_string();

                if str.contains(&initial_ident) {
                    InitialAssocAccess::validate_transform(
                        &InitialAssocAccess,
                        seg,
                        Some(&context),
                    )?;
                } else if str.contains(&final_ident) {
                    FinalAssocAccess::validate_transform(&FinalAssocAccess, seg, Some(&context))?;
                } else {
                    return Err(TraverseBugs::ReplacedArgNeitherInitialNorFinal {}.into());
                }

                let PathArguments::AngleBracketed(angle) = &seg.arguments else {
                    unreachable!()
                };
                let mut equiv_bounds = angle.args.iter().filter_map(|item| {
                    let GenericArgument::AssocType(assoc) = item else {
                        return None;
                    };
                    Some(assoc)
                });
                let Some(equiv) = equiv_bounds.next() else {
                    return Err(TraverseBugs::TerminalEquivalenceBoundNotFound {}.into());
                };
                let exp = quote! {#self_term = #term_twin_assoc};

                if equiv.to_token_stream().to_string() != exp.to_string() {
                    return Err(TraverseBugs::TerminalEquivalenceBoundInvalid {}.into());
                }
                let Some(equiv) = equiv_bounds.next() else {
                    return Err(TraverseBugs::TerminalEquivalenceBoundNotFound {}.into());
                };
                let exp = quote! {#global_term = #term_global_assoc};

                if equiv.to_token_stream().to_string() != exp.to_string() {
                    return Err(TraverseBugs::TerminalEquivalenceBoundInvalid {}.into());
                }

                let Some(equiv) = equiv_bounds.next() else {
                    return Err(TraverseBugs::TerminalEquivalenceBoundNotFound {}.into());
                };
                let exp = quote! {#global_term_exact = #term_exact_assoc};

                if equiv.to_token_stream().to_string() != exp.to_string() {
                    return Err(TraverseBugs::TerminalEquivalenceBoundInvalid {}.into());
                }
            }

            // Validate Global.
            let global = state.get_global()?;

            let bounds = global.bounds.iter().try_fold(
                Vec::new(),
                |mut bounds, item| -> Result<_, TokenStream> {
                    let TypeParamBound::Trait(t) = item else {
                        return Err(TraverseBugs::GlobalHaveNonTraitBound {}.into());
                    };

                    bounds.push(t);
                    Ok(bounds)
                },
            )?;

            let access_bounds_len = bounds.len();
            let exp_access_bounds_len = if prefix.bytes.is_empty() { 2 } else { 3 };

            if access_bounds_len != exp_access_bounds_len
                || global.bounds.len() != access_bounds_len
            {
                return Err(TraverseBugs::GlobalAccessBoundsInconsistent {}.into());
            }

            // Validate Initial.
            let initial = &state.get_initial()?.bounds;
            let exp_initial_bounds_len = if prefix.bytes.is_empty() { 1 } else { 2 };

            if initial.len() != exp_initial_bounds_len {
                return Err(TraverseBugs::InitialBoundsInconsistent {}.into());
            }

            let initial_b = &initial[0];

            let TypeParamBound::Trait(initial_b) = initial_b else {
                return Err(TraverseBugs::InitialBoundNotTraitBound {}.into());
            };

            let prefix_initial = if !prefix.bytes.is_empty() {
                let prefix_initial = &initial[1];

                let TypeParamBound::Trait(prefix_initial) = prefix_initial else {
                    return Err(TraverseBugs::PrefixBoundNotTraitBound {}.into());
                };

                Some(prefix_initial)
            } else {
                None
            };

            // Validate Final.
            let finalz = &state.get_final()?.bounds;

            if finalz.len() != 1 {
                return Err(TraverseBugs::FinalBoundsInconsistent {}.into());
            }

            let finalz = &finalz[0];

            let TypeParamBound::Trait(finalz) = finalz else {
                return Err(TraverseBugs::FinalBoundNotTraitBound {}.into());
            };

            let o_context = (base_assoc, indexes, branch).into();
            let c_context = (base_assoc, indexes, leaf).into();

            for bound in bounds {
                let Some(seg) = bound.path.segments.last() else {
                    return Err(TraverseBugs::ReplacedBoundLastPathSegNotFound {}.into());
                };

                let str = seg.ident.to_string();

                if str.contains("OnSetAccess") {
                    let o_bound = OnSetAccess(bound.clone());

                    let first = (|| -> Result<(), TokenStream> {
                        OnSetAccess::validate_extract(
                            &OnSetAccess(bound.clone()),
                            &o_context,
                            None,
                        )?;

                        ExactAccess::validate_extract(
                            &ExactAccess::new(initial_b),
                            &o_bound,
                            Some(global_assoc),
                        )?;

                        Ok(())
                    })();

                    let result = if let Some(prefix_b) = prefix_initial {
                        let p_context = (base_assoc, indexes, &prefix).into();

                        let p_bound = OnSetAccess(bound.clone());

                        let second = (|| -> Result<(), TokenStream> {
                            OnSetAccess::validate_extract(
                                &OnSetAccess(bound.clone()),
                                &p_context,
                                None,
                            )?;

                            ExactAccess::validate_extract(
                                &ExactAccess::new(prefix_b),
                                &p_bound,
                                Some(global_assoc),
                            )?;

                            Ok(())
                        })();

                        first.or(second)
                    } else {
                        first
                    };

                    result?;
                } else if str.contains("CounterAccess") {
                    let c_bound = CounterAccess(bound.clone());

                    CounterAccess::validate_extract(
                        &CounterAccess(bound.clone()),
                        &c_context,
                        None,
                    )?;

                    ExactAccess::validate_extract(
                        &ExactAccess::new(finalz),
                        &c_bound,
                        Some(global_assoc),
                    )?;
                } else {
                    return Err(TraverseBugs::ReplacedBoundNeitherOnSetNorCounter {}.into());
                }
            }

            Ok(())
        })?;

        Ok(())
    }
}

// ===============================================================================
// `````````````````````` INSTANCE TRAIT DESCEND SUBSCRIBER ``````````````````````
// ===============================================================================

/// Subscriber transformation for the `descend` instance node.
///
/// A descend subscriber selects a range of instances from the root of a
/// selected subtree to a selected exact instance identifier. The range begins
/// at the selected branch and terminates at the specified leaf.
///
/// When the branch and leaf share a common path prefix, that prefix is
/// extracted as an intermediate range boundary. The resulting transformation
/// therefore represents up to three distinct positions:
///
/// - The selected branch, representing the root of the selected subtree.
/// - The common prefix, when non-empty, representing the shared intermediate
///   subtree boundary between the branch and leaf.
/// - The selected leaf, representing the exact terminating instance.
///
/// This transformation:
///
/// - Replaces the original instance bound with transformed
///   [`InitialAssocAccess`] of [`OnSetAccess`] bounds and
///   [`FinalAssocAccess`] of [`CounterAccess`] bounds.
/// - Preserves the [`crate::traits::sum`] associated-type equalities in
///   the replaced bounds by binding,
///         - [`SelfTerminalAssoc`] to [`TERMINAL_TWIN_NODE`].
///         - [`GlobalTerminalAssoc`] to [`TERMINAL_GLOBAL_NODE`].
///         - [`GlobalTerminalExact`] to
///             `<TERMINAL_TWIN_NODE as TERMINAL_GLOBAL_NODE_TRAIT>::Terminal`.
/// - Uses [`OnSetAccess`] to represent the beginning of the selected range,
///   corresponding to the root of the selected subtree.
/// - Uses an additional [`OnSetAccess`] for the common prefix when the branch
///   and leaf share one, representing the intermediate subtree boundary.
/// - Uses [`CounterAccess`] to represent the end of the selected range,
///   corresponding to the selected exact instance.
/// - Contributes the branch and leaf bounds to the generated global companion
///   node ([`GLOBAL_NODE`]), allowing the selected range to be accumulated
///   independently of the original instance.
/// - Contributes the prefix bound to [`GLOBAL_NODE`] when a common prefix
///   exists.
/// - Contributes an [`ExactAccess`] derived from the branch [`OnSetAccess`]
///   to the initial range companion node ([`INITIAL_NODE`]), representing the
///   exact beginning of the range.
/// - Contributes an [`ExactAccess`] derived from the prefix [`OnSetAccess`]
///   to [`INITIAL_NODE`] when a common prefix exists.
/// - Contributes an [`ExactAccess`] derived from the [`CounterAccess`] to the
///   final range companion node ([`FINAL_NODE`]), representing the exact end
///   of the range.
///
/// Unlike a traverse subscriber, which begins at an exact instance and
/// terminates at a subtree boundary, a descend subscriber begins at the root
/// of a selected subtree and terminates at an exact instance. When a common
/// prefix exists between those endpoints, the prefix preserves the
/// intermediate subtree boundary as an explicit access point.
#[derive(Debug, Clone)]
pub(crate) struct TraitDescendSubscriber;

impl<'a> Transformation<TraitNodeSpace<'a>, DescendArgs<SubscriberNode>>
    for TraitDescendSubscriber
{
    fn raw_transform(
        &self,
        transform: &mut TraitNodeSpace<'a>,
        context: &DescendArgs<SubscriberNode>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let indexes = &context.indexes;
        let branch = &context.from;
        let leaf = &context.until;

        let mut prefix = BStringList {
            bytes: Punctuated::new(),
        };
        let mut collected = false;
        for (branch, leaf) in branch.bytes.iter().zip(&leaf.bytes) {
            if !collected {
                if branch == leaf {
                    prefix.bytes.push(branch.clone());
                } else {
                    collected = true
                }
            }
        }

        Self::prepare(transform, context, |state| {
            let global_assoc = &state.get_global_assoc()?;
            let base_assoc = &state.get_base_assoc()?;
            let base = &state.get_base()?;
            let base_ident = base.ident.clone();
            let term_twin_assoc = &state.get_terminal_twin_assoc()?;
            let term_global_assoc = &state.get_terminal_global_assoc()?;
            let term_context = (base_assoc, context.get()?).into();
            let term_bound = TerminalAccess::checked_extract(&term_context, &())?.0;
            let term_projection = TerminalAccess::projection();
            let term_exact_assoc: Type =
                parse_quote!(<#term_global_assoc as #term_bound>::#term_projection);
            let self_term = gen_type_ident::<SelfTerminalAssoc>();
            let global_term = gen_type_ident::<GlobalTerminalAssoc>();
            let global_term_exact = gen_type_ident::<GlobalTerminalExact>();

            let o_context = (base_assoc, indexes, branch).into();
            let o_access = OnSetAccess::checked_extract(&o_context, &())?;

            let p_access = if !prefix.bytes.is_empty() {
                let p_context = (base_assoc, indexes, &prefix).into();
                let p_access = OnSetAccess::checked_extract(&p_context, &())?;
                Some(p_access)
            } else {
                None
            };

            let c_context = (base_assoc, indexes, leaf).into();
            let c_access = CounterAccess::checked_extract(&c_context, &())?;

            state.mutate_global(|global| {
                global.push(TypeParamBound::Trait(o_access.0.clone()));

                if let Some(p_access) = &p_access {
                    global.push(TypeParamBound::Trait(p_access.0.clone()));
                }

                global.push(TypeParamBound::Trait(c_access.0.clone()));
                Ok(())
            })?;

            let o_exact = ExactAccess::checked_extract(&o_access, global_assoc)?.bound;

            let p_exact = if let Some(p_access) = p_access {
                let p_exact = ExactAccess::checked_extract(&p_access, global_assoc)?.bound;
                Some(p_exact)
            } else {
                None
            };

            let c_exact = ExactAccess::checked_extract(&c_access, global_assoc)?.bound;

            state.mutate_initial(|initial| {
                initial.push(TypeParamBound::Trait(o_exact));

                if let Some(p_exact) = p_exact {
                    initial.push(TypeParamBound::Trait(p_exact));
                }

                Ok(())
            })?;

            state.mutate_final(|finalz| {
                finalz.push(TypeParamBound::Trait(c_exact));
                Ok(())
            })?;

            let _ = state.replace_instance_bounds(|trait_bounds| {
                let (mut o_p_access_bound, mut c_access_bound) =
                    provide_bounds(trait_bounds, &base_ident, indexes)?;
                let mut bounds = Vec::new();

                let syn::PathArguments::AngleBracketed(angle) =
                    &mut o_p_access_bound.path.segments.last_mut().unwrap().arguments
                else {
                    unreachable!()
                };
                 let index = angle
                    .args
                    .iter()
                    .position(|arg| {
                        matches!(
                            arg,
                            syn::GenericArgument::AssocType(_) | syn::GenericArgument::Constraint(_)
                                | syn::GenericArgument::AssocConst(_)
                        )
                    })
                    .unwrap_or(angle.args.len());

                angle.args.insert(index, parse_quote!(#self_term = #term_twin_assoc));
                angle.args.insert(index + 1, parse_quote!(#global_term = #term_global_assoc));
                angle.args.insert(index + 2, parse_quote!(#global_term_exact = #term_exact_assoc));

                let syn::PathArguments::AngleBracketed(angle) =
                    &mut c_access_bound.path.segments.last_mut().unwrap().arguments
                else {
                    unreachable!()
                };
                 let index = angle
                    .args
                    .iter()
                    .position(|arg| {
                        matches!(
                            arg,
                            syn::GenericArgument::AssocType(_) | syn::GenericArgument::Constraint(_)
                                | syn::GenericArgument::AssocConst(_)
                        )
                    })
                    .unwrap_or(angle.args.len());

                angle.args.insert(index, parse_quote!(#self_term = #term_twin_assoc));
                angle.args.insert(index + 1, parse_quote!(#global_term = #term_global_assoc));
                angle.args.insert(index + 2, parse_quote!(#global_term_exact = #term_exact_assoc));

                bounds.push(o_p_access_bound);
                bounds.push(c_access_bound);
                Ok(bounds)
            })?;

            Ok(())
        })?;

        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &TraitNodeSpace<'a>,
        context: Option<&DescendArgs<SubscriberNode>>,
    ) -> Result<(), proc_macro2::TokenStream> {
        Self::validate(transform, context, |state| {
            let Some(context) = context else {
                return Ok(());
            };
            let indexes = &context.indexes;
            let branch = &context.from;
            let leaf = &context.until;

            let mut prefix = BStringList {
                bytes: Punctuated::new(),
            };
            let mut collected = false;
            for (branch, leaf) in branch.bytes.iter().zip(&leaf.bytes) {
                if !collected {
                    if branch == leaf {
                        prefix.bytes.push(branch.clone());
                    } else {
                        collected = true
                    }
                }
            }

            let base_assoc = &state.get_base_assoc()?;
            let global_assoc = &state.get_global_assoc()?;
            let base = state.get_base()?;
            let base_ident = base.ident.clone();
            let term_twin_assoc = &state.get_terminal_twin_assoc()?;
            let term_global_assoc = &state.get_terminal_global_assoc()?;
            let term_context = (base_assoc, context.get()?).into();
            let term_bound = TerminalAccess::checked_extract(&term_context, &())?.0;
            let term_projection = TerminalAccess::projection();
            let term_exact_assoc: Type =
                parse_quote!(<#term_global_assoc as #term_bound>::#term_projection);
            let self_term = gen_type_ident::<SelfTerminalAssoc>();
            let global_term = gen_type_ident::<GlobalTerminalAssoc>();
            let global_term_exact = gen_type_ident::<GlobalTerminalExact>();

            // Validate replaced bounds (specifically their generic typenum arg)
            let bounds = state.get_instance_bounds()?;

            if bounds.is_empty() {
                return Err(DescendBugs::ReplacedBoundsAreEmpty {}.into());
            }

            if bounds.len() != 2 {
                return Err(DescendBugs::ReplacedBoundsInconsistent {}.into());
            }

            let initial_ident =
                gen_type_ident_from_ident_with_suffix::<InstanceNode>(&base_ident, INITIAL_NODE)
                    .to_string();
            let final_ident =
                gen_type_ident_from_ident_with_suffix::<InstanceNode>(&base_ident, FINAL_NODE)
                    .to_string();

            for bound in bounds {
                let Some(seg) = bound.path.segments.last() else {
                    return Err(DescendBugs::ReplacedBoundLastPathSegNotFound {}.into());
                };
                let context = (&base_ident, indexes, true, None).into();
                let str = seg.arguments.to_token_stream().to_string();

                if str.contains(&initial_ident) {
                    InitialAssocAccess::validate_transform(
                        &InitialAssocAccess,
                        seg,
                        Some(&context),
                    )?;
                } else if str.contains(&final_ident) {
                    FinalAssocAccess::validate_transform(&FinalAssocAccess, seg, Some(&context))?;
                } else {
                    return Err(DescendBugs::ReplacedArgNeitherInitialNorFinal {}.into());
                }

                let PathArguments::AngleBracketed(angle) = &seg.arguments else {
                    unreachable!()
                };
                let mut equiv_bounds = angle.args.iter().filter_map(|item| {
                    let GenericArgument::AssocType(assoc) = item else {
                        return None;
                    };
                    Some(assoc)
                });
                let Some(equiv) = equiv_bounds.next() else {
                    return Err(DescendBugs::TerminalEquivalenceBoundNotFound {}.into());
                };
                let exp = quote! {#self_term = #term_twin_assoc};

                if equiv.to_token_stream().to_string() != exp.to_string() {
                    return Err(DescendBugs::TerminalEquivalenceBoundInvalid {}.into());
                }
                let Some(equiv) = equiv_bounds.next() else {
                    return Err(DescendBugs::TerminalEquivalenceBoundNotFound {}.into());
                };
                let exp = quote! {#global_term = #term_global_assoc};

                if equiv.to_token_stream().to_string() != exp.to_string() {
                    return Err(DescendBugs::TerminalEquivalenceBoundInvalid {}.into());
                }

                let Some(equiv) = equiv_bounds.next() else {
                    return Err(DescendBugs::TerminalEquivalenceBoundNotFound {}.into());
                };
                let exp = quote! {#global_term_exact = #term_exact_assoc};

                if equiv.to_token_stream().to_string() != exp.to_string() {
                    return Err(DescendBugs::TerminalEquivalenceBoundInvalid {}.into());
                }
            }

            // Validate Global
            let global = state.get_global()?;

            let bounds = global.bounds.iter().try_fold(
                Vec::new(),
                |mut bounds, item| -> Result<_, TokenStream> {
                    let TypeParamBound::Trait(t) = item else {
                        return Err(DescendBugs::GlobalHaveNonTraitBound {}.into());
                    };

                    bounds.push(t);
                    Ok(bounds)
                },
            )?;

            let access_bounds_len = bounds.len();
            let exp_access_bounds_len = if prefix.bytes.is_empty() { 2 } else { 3 };

            if access_bounds_len != exp_access_bounds_len {
                return Err(DescendBugs::GlobalAccessBoundsInconsistent {}.into());
            }

            let initial = &state.get_initial()?.bounds;

            let exp_initial_bounds_len = if prefix.bytes.is_empty() { 1 } else { 2 };
            if initial.len() != exp_initial_bounds_len {
                return Err(DescendBugs::InitialBoundsInconsistent {}.into());
            };
            let initial_b = &initial[0];

            let TypeParamBound::Trait(initial_b) = initial_b else {
                return Err(DescendBugs::InitialBoundNotTraitBound {}.into());
            };
            let prefix_initial = if !prefix.bytes.is_empty() {
                let prefix_b = &initial[1];
                let TypeParamBound::Trait(prefix_b) = prefix_b else {
                    return Err(DescendBugs::PrefixBoundNotTraitBound {}.into());
                };
                Some(prefix_b)
            } else {
                None
            };

            let finalz = &state.get_final()?.bounds;
            if finalz.len() > 1 {
                return Err(DescendBugs::FinalBoundsInconsistent {}.into());
            };
            let finalz = &finalz[0];
            let TypeParamBound::Trait(finalz) = finalz else {
                return Err(DescendBugs::FinalBoundNotTraitBound {}.into());
            };

            for bound in bounds {
                let Some(seg) = bound.path.segments.last() else {
                    return Err(DescendBugs::ReplacedBoundLastPathSegNotFound {}.into());
                };
                let str = seg.ident.to_string();
                if str.contains("OnSetAccess") {
                    let o_context = (base_assoc, indexes, branch).into();
                    let o_bound = OnSetAccess(bound.clone());

                    let first = (|| -> Result<(), TokenStream> {
                        OnSetAccess::validate_extract(
                            &&OnSetAccess(bound.clone()),
                            &o_context,
                            None,
                        )?;

                        ExactAccess::validate_extract(
                            &ExactAccess::new(initial_b),
                            &o_bound,
                            Some(global_assoc),
                        )?;

                        Ok(())
                    })();

                    let result = if let Some(prefix_b) = prefix_initial {
                        let p_context = (base_assoc, indexes, branch).into();
                        let p_bound = OnSetAccess(bound.clone());

                        let second = (|| {
                            OnSetAccess::validate_extract(
                                &&OnSetAccess(bound.clone()),
                                &p_context,
                                None,
                            )?;

                            ExactAccess::validate_extract(
                                &ExactAccess::new(prefix_b),
                                &p_bound,
                                Some(global_assoc),
                            )?;

                            Ok(())
                        })();

                        first.or(second)
                    } else {
                        first
                    };

                    result?;
                } else if str.contains("CounterAccess") {
                    let c_context = (base_assoc, indexes, leaf).into();
                    let c_bound = CounterAccess(bound.clone());
                    CounterAccess::validate_extract(
                        &CounterAccess(bound.clone()),
                        &c_context,
                        None,
                    )?;
                    ExactAccess::validate_extract(
                        &ExactAccess::new(finalz),
                        &c_bound,
                        Some(global_assoc),
                    )?;
                } else {
                    return Err(DescendBugs::ReplacedBoundNeitherOnSetNorCounter {}.into());
                }
            }

            Ok(())
        })?;
        Ok(())
    }
}

// ===============================================================================
// ``````````````````````` INSTANCE TRAIT ROOT SUBSCRIBER ````````````````````````
// ===============================================================================

/// Subscriber transformation for the `root` instance node.
///
/// A root subscriber selects the entire instance tree. The range begins at
/// the implicit root of the tree and terminates at its implicit terminal
/// boundary.
///
/// This transformation:
///
/// - Replaces the original instance bound with transformed
///   [`InitialAssocAccess`] of [`OnSetAccess`] bounds and
///   [`FinalAssocAccess`] of [`BoundaryAccess`] bounds.
/// - Preserves the [`crate::traits::sum`] associated-type equalities in
///   the replaced bounds by binding,
///         - [`SelfTerminalAssoc`] to [`TERMINAL_TWIN_NODE`].
///         - [`GlobalTerminalAssoc`] to [`TERMINAL_GLOBAL_NODE`].
///         - [`GlobalTerminalExact`] to
///             `<TERMINAL_TWIN_NODE as TERMINAL_GLOBAL_NODE_TRAIT>::Terminal`.
/// - Uses [`OnSetAccess`] to represent the beginning of the selected range,
///   corresponding to the root of the instance tree.
/// - Uses [`BoundaryAccess`] to represent the end of the selected range,
///   corresponding to the terminal boundary of the instance tree.
/// - Contributes both bounds to the generated global companion node
///   ([`GLOBAL_NODE`]), allowing the complete instance tree to be accumulated
///   independently of the original instance.
/// - Contributes an [`ExactAccess`] derived from the generated
///   [`OnSetAccess`] to the initial range companion node
///   ([`INITIAL_NODE`]), representing the exact beginning of the range.
/// - Contributes an [`ExactAccess`] derived from the generated
///   [`BoundaryAccess`] to the final range companion node
///   ([`FINAL_NODE`]), representing the exact end of the range.
///
/// Unlike the other subscriber selection models, a root subscriber requires
/// no explicit instance identifiers. Instead, it spans the complete instance
/// tree, with the generated initial and final companion nodes preserving its
/// implicit range endpoints.
#[derive(Debug, Clone)]
pub(crate) struct TraitRootSubscriber;

impl<'a> Transformation<TraitNodeSpace<'a>, RootArgs<SubscriberNode>> for TraitRootSubscriber {
    fn raw_transform(
        &self,
        transform: &mut TraitNodeSpace<'a>,
        context: &RootArgs<SubscriberNode>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let indexes = &context.indexes;
        let def = BStringList {
            bytes: Punctuated::new(),
        };
        Self::prepare(transform, context, |state| {
            let global_assoc = &state.get_global_assoc()?;
            let base_assoc = &state.get_base_assoc()?;
            let base = &state.get_base()?;
            let base_ident = base.ident.clone();
            let term_twin_assoc = &state.get_terminal_twin_assoc()?;
            let term_global_assoc = &state.get_terminal_global_assoc()?;
            let term_context = (base_assoc, context.get()?).into();
            let term_bound = TerminalAccess::checked_extract(&term_context, &())?.0;
            let term_projection = TerminalAccess::projection();
            let term_exact_assoc: Type =
                parse_quote!(<#term_global_assoc as #term_bound>::#term_projection);
            let self_term = gen_type_ident::<SelfTerminalAssoc>();
            let global_term = gen_type_ident::<GlobalTerminalAssoc>();
            let global_term_exact = gen_type_ident::<GlobalTerminalExact>();

            let o_context = (base_assoc, indexes, &def).into();
            let o_access = OnSetAccess::checked_extract(&o_context, &())?;
            let b_context = (base_assoc, indexes, &def).into();
            let b_access = BoundaryAccess::checked_extract(&b_context, &())?;

            state.mutate_global(|global| {
                global.push(TypeParamBound::Trait(o_access.0.clone()));
                global.push(TypeParamBound::Trait(b_access.0.clone()));
                Ok(())
            })?;

            let o_exact = ExactAccess::checked_extract(&o_access, global_assoc)?.bound;
            let b_exact = ExactAccess::checked_extract(&b_access, global_assoc)?.bound;

            state.mutate_initial(|initial| {
                initial.push(TypeParamBound::Trait(o_exact));
                Ok(())
            })?;

            state.mutate_final(|finalz| {
                finalz.push(TypeParamBound::Trait(b_exact));
                Ok(())
            })?;

            let _ = state.replace_instance_bounds(|trait_bounds| {
                let (mut o_access_bound, mut b_access_bound) =
                    provide_bounds(trait_bounds, &base_ident, indexes)?;
                let mut bounds = Vec::new();

                let syn::PathArguments::AngleBracketed(angle) =
                    &mut o_access_bound.path.segments.last_mut().unwrap().arguments
                else {
                    unreachable!()
                };
                 let index = angle
                    .args
                    .iter()
                    .position(|arg| {
                        matches!(
                            arg,
                            syn::GenericArgument::AssocType(_) | syn::GenericArgument::Constraint(_)
                                | syn::GenericArgument::AssocConst(_)
                        )
                    })
                    .unwrap_or(angle.args.len());

                angle.args.insert(index, parse_quote!(#self_term = #term_twin_assoc));
                angle.args.insert(index + 1, parse_quote!(#global_term = #term_global_assoc));
                angle.args.insert(index + 2, parse_quote!(#global_term_exact = #term_exact_assoc));

                let syn::PathArguments::AngleBracketed(angle) =
                    &mut b_access_bound.path.segments.last_mut().unwrap().arguments
                else {
                    unreachable!()
                };
                 let index = angle
                    .args
                    .iter()
                    .position(|arg| {
                        matches!(
                            arg,
                            syn::GenericArgument::AssocType(_) | syn::GenericArgument::Constraint(_)
                                | syn::GenericArgument::AssocConst(_)
                        )
                    })
                    .unwrap_or(angle.args.len());

                angle.args.insert(index, parse_quote!(#self_term = #term_twin_assoc));
                angle.args.insert(index + 1, parse_quote!(#global_term = #term_global_assoc));
                angle.args.insert(index + 2, parse_quote!(#global_term_exact = #term_exact_assoc));

                bounds.push(o_access_bound);
                bounds.push(b_access_bound);
                Ok(bounds)
            })?;

            Ok(())
        })?;

        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &TraitNodeSpace<'a>,
        context: Option<&RootArgs<SubscriberNode>>,
    ) -> Result<(), proc_macro2::TokenStream> {
        Self::validate(transform, context, |state| {
            let Some(context) = context else {
                return Ok(());
            };
            let indexes = &context.indexes;
            let def = BStringList {
                bytes: Punctuated::new(),
            };

            let base_assoc = &state.get_base_assoc()?;
            let global_assoc = &state.get_global_assoc()?;
            let base = state.get_base()?;
            let base_ident = base.ident.clone();
            let term_twin_assoc = &state.get_terminal_twin_assoc()?;
            let term_global_assoc = &state.get_terminal_global_assoc()?;
            let term_context = (base_assoc, context.get()?).into();
            let term_bound = TerminalAccess::checked_extract(&term_context, &())?.0;
            let term_projection = TerminalAccess::projection();
            let term_exact_assoc: Type =
                parse_quote!(<#term_global_assoc as #term_bound>::#term_projection);
            let self_term = gen_type_ident::<SelfTerminalAssoc>();
            let global_term = gen_type_ident::<GlobalTerminalAssoc>();
            let global_term_exact = gen_type_ident::<GlobalTerminalExact>();

            // Validate replaced bounds (specifically their generic typenum arg)
            let bounds = state.get_instance_bounds()?;

            if bounds.is_empty() {
                return Err(RootBugs::ReplacedBoundsAreEmpty {}.into());
            }

            if bounds.len() != 2 {
                return Err(RootBugs::ReplacedBoundsInconsistent {}.into());
            }

            let initial_ident =
                gen_type_ident_from_ident_with_suffix::<InstanceNode>(&base_ident, INITIAL_NODE)
                    .to_string();
            let final_ident =
                gen_type_ident_from_ident_with_suffix::<InstanceNode>(&base_ident, FINAL_NODE)
                    .to_string();

            for bound in bounds {
                let Some(seg) = bound.path.segments.last() else {
                    return Err(RootBugs::ReplacedBoundLastPathSegNotFound {}.into());
                };
                let context = (&base_ident, indexes, true, None).into();
                let str = seg.arguments.to_token_stream().to_string();

                if str.contains(&initial_ident) {
                    InitialAssocAccess::validate_transform(
                        &InitialAssocAccess,
                        seg,
                        Some(&context),
                    )?;
                } else if str.contains(&final_ident) {
                    FinalAssocAccess::validate_transform(&FinalAssocAccess, seg, Some(&context))?;
                } else {
                    return Err(RootBugs::ReplacedArgNeitherInitialNorFinal {}.into());
                }

                let PathArguments::AngleBracketed(angle) = &seg.arguments else {
                    unreachable!()
                };
                let mut equiv_bounds = angle.args.iter().filter_map(|item| {
                    let GenericArgument::AssocType(assoc) = item else {
                        return None;
                    };
                    Some(assoc)
                });
                let Some(equiv) = equiv_bounds.next() else {
                    return Err(RootBugs::TerminalEquivalenceBoundNotFound {}.into());
                };
                let exp = quote! {#self_term = #term_twin_assoc};

                if equiv.to_token_stream().to_string() != exp.to_string() {
                    return Err(RootBugs::TerminalEquivalenceBoundInvalid {}.into());
                }
                let Some(equiv) = equiv_bounds.next() else {
                    return Err(RootBugs::TerminalEquivalenceBoundNotFound {}.into());
                };
                let exp = quote! {#global_term = #term_global_assoc};

                if equiv.to_token_stream().to_string() != exp.to_string() {
                    return Err(RootBugs::TerminalEquivalenceBoundInvalid {}.into());
                }

                let Some(equiv) = equiv_bounds.next() else {
                    return Err(RootBugs::TerminalEquivalenceBoundNotFound {}.into());
                };
                let exp = quote! {#global_term_exact = #term_exact_assoc};

                if equiv.to_token_stream().to_string() != exp.to_string() {
                    return Err(RootBugs::TerminalEquivalenceBoundInvalid {}.into());
                }
            }

            // Validate Global
            let global = state.get_global()?;

            let bounds = global.bounds.iter().try_fold(
                Vec::new(),
                |mut bounds, item| -> Result<_, TokenStream> {
                    let TypeParamBound::Trait(t) = item else {
                        return Err(RootBugs::GlobalHaveNonTraitBound {}.into());
                    };

                    bounds.push(t);
                    Ok(bounds)
                },
            )?;

            let access_bounds_len = bounds.len();
            if access_bounds_len != 2 || global.bounds.len() != access_bounds_len {
                return Err(RootBugs::GlobalAccessBoundsInconsistent {}.into());
            }

            let initial = &state.get_initial()?.bounds;
            if initial.len() > 1 {
                return Err(RootBugs::InitialBoundsInconsistent {}.into());
            };
            let initial = &initial[0];
            let TypeParamBound::Trait(initial) = initial else {
                return Err(RootBugs::InitialBoundNotTraitBound {}.into());
            };

            let finalz = &state.get_final()?.bounds;
            if finalz.len() > 1 {
                return Err(RootBugs::FinalBoundsInconsistent {}.into());
            };
            let finalz = &finalz[0];
            let TypeParamBound::Trait(finalz) = finalz else {
                return Err(RootBugs::FinalBoundNotTraitBound {}.into());
            };

            let o_context = (base_assoc, indexes, &def).into();
            let b_context = (base_assoc, indexes, &def).into();
            for bound in bounds {
                let Some(seg) = bound.path.segments.last() else {
                    return Err(RootBugs::ReplacedBoundLastPathSegNotFound {}.into());
                };
                let str = seg.ident.to_string();
                if str.contains("OnSetAccess") {
                    let o_bound = OnSetAccess(bound.clone());
                    OnSetAccess::validate_extract(&OnSetAccess(bound.clone()), &o_context, None)?;
                    ExactAccess::validate_extract(
                        &ExactAccess::new(initial),
                        &o_bound,
                        Some(global_assoc),
                    )?;
                } else if str.contains("BoundaryAccess") {
                    let b_bound = BoundaryAccess(bound.clone());
                    BoundaryAccess::validate_extract(
                        &BoundaryAccess(bound.clone()),
                        &b_context,
                        None,
                    )?;
                    ExactAccess::validate_extract(
                        &ExactAccess::new(finalz),
                        &b_bound,
                        Some(global_assoc),
                    )?;
                } else {
                    return Err(RootBugs::ReplacedBoundNeitherOnSetNorBoundary {}.into());
                }
            }

            Ok(())
        })?;
        Ok(())
    }
}

// ===============================================================================
// ```````````````````````` INSTANCE IMPL TYPE SUBSCRIBER ````````````````````````
// ===============================================================================

/// Shared implementation-side transformation for subscriber instance nodes.
///
/// This transformation materializes the generated companion associated types
/// for a subscriber implementation.
///
/// Unlike declaration-side subscriber transformations, which encode the
/// semantics of a particular subscriber selection model (`leaf`, `branch`,
/// `prune`, `trim`, `extend`, `spread`, `traverse`, `descend`, or `root`),
/// the implementation side is identical for every subscriber. Its sole
/// responsibility is to resolve the generated companion associated types for
/// an already resolved subscriber instance.
///
/// The original implementation instance associated type is therefore required
/// to be a qualified associated type (`TypePath` with a `QSelf`), for
/// example:
///
/// ```text
/// <T as Subscriber>::Account
/// ```
///
/// The qualified associated type uniquely identifies both the originating
/// subscriber trait and the resolved instance associated type. Using this
/// information, the transformation deterministically derives every generated
/// neighbouring companion associated type by preserving the qualified path
/// and replacing only the associated type identifier.
///
/// Example:
///
/// ```text
/// Base:
///     <T as Subscriber>::Account
///
/// Generated:
///     <T as Subscriber>::AccountGlobal
///     <T as Subscriber>::AccountTwin
///     <T as Subscriber>::AccountInitial
///     <T as Subscriber>::AccountFinal
/// ```
///
/// Specifically, this transformation:
///
/// - Requires the original subscriber instance associated type to be a
///   qualified associated type (`TypePath` with a `QSelf`).
/// - Resolves the generated neighbouring companion associated types
///   ([`GLOBAL_NODE`], [`TWIN_NODE`], [`INITIAL_NODE`], and
///   [`FINAL_NODE`]) by deterministically renaming the associated type
///   identifier while preserving the original qualified trait path.
/// - Resolves every generated companion associated type to the same concrete
///   implementation type as the original subscriber instance associated
///   type.
/// - Removes the generated checker companion constant, since subscriber
///   validation is performed entirely on the declaration side and therefore
///   no checker companion is required for implementation associated types.
///
/// Consequently, every subscriber implementation exposes the complete family
/// of generated neighbouring companion associated types, all derived
/// deterministically from the original qualified subscriber instance
/// associated type.
///
/// ## Example:
///
/// ```text
/// Trait:
///     trait Subscriber {
///         type Account;
///     }
///
/// Original implementation:
///     type Account =
///         <Runtime as Subscriber>::Account;
///
/// Generated implementation:
///     type AccountGlobal =
///         <Runtime as Subscriber>::AccountGlobal;
///
///     type AccountTwin =
///         <Runtime as Subscriber>::AccountTwin;
///
///     type AccountInitial =
///         <Runtime as Subscriber>::AccountInitial;
///
///     type AccountFinal =
///         <Runtime as Subscriber>::AccountFinal;
/// ```
///
/// Since the original implementation is a qualified associated type
/// (`<Runtime as Subscriber>::Account`), the originating trait
/// (`Subscriber`) and the original associated type (`Account`) are both
/// known. This allows every neighbouring companion associated type to be
/// derived deterministically by preserving the qualified path and replacing
/// only the associated type identifier.
#[derive(Debug, Clone)]
pub(crate) struct InstanceImplSubscriber;

impl<'a> Transformation<ImplNodeSpace<'a>> for InstanceImplSubscriber {
    fn raw_transform(
        &self,
        transform: &mut ImplNodeSpace<'a>,
        context: &(),
    ) -> Result<(), proc_macro2::TokenStream> {
        Self::prepare(transform, context, |state| {
            let base = state.get_base()?;

            let may_assoc_ty = &base.ty;
            let ty_path_tokens = quote! {#may_assoc_ty};
            let Ok(mut ty_path) = parse::<TypePath>(ty_path_tokens.into()) else {
                return Err(SubscriberError::ImplSubscriberQTypePath {
                    ty: base.ty.clone(),
                }
                .into());
            };
            let Some(q_self) = &ty_path.qself else {
                return Err(SubscriberError::ImplSubscriberTypePathQSelfNotFound {
                    ty: ty_path.clone(),
                }
                .into());
            };

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

            state.mutate_global(|global| {
                let seg = ty_path.path.segments.get_mut(pos).unwrap();
                seg.ident =
                    gen_type_ident_from_ident_with_suffix::<InstanceNode>(&prev_ident, GLOBAL_NODE);
                *global = Type::Path(ty_path.clone());
                Ok(())
            })?;

            state.mutate_twin(|twin| {
                let seg = ty_path.path.segments.get_mut(pos).unwrap();
                seg.ident =
                    gen_type_ident_from_ident_with_suffix::<InstanceNode>(&prev_ident, TWIN_NODE);
                *twin = Type::Path(ty_path.clone());
                Ok(())
            })?;

            state.mutate_initial(|initial| {
                let seg = ty_path.path.segments.get_mut(pos).unwrap();
                seg.ident = gen_type_ident_from_ident_with_suffix::<InstanceNode>(
                    &prev_ident,
                    INITIAL_NODE,
                );
                *initial = Type::Path(ty_path.clone());
                Ok(())
            })?;

            state.mutate_final(|finalz| {
                let seg = ty_path.path.segments.get_mut(pos).unwrap();
                seg.ident =
                    gen_type_ident_from_ident_with_suffix::<InstanceNode>(&prev_ident, FINAL_NODE);
                *finalz = Type::Path(ty_path.clone());
                Ok(())
            })?;

            state.remove_checker()?;

            Ok(())
        })?;

        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &ImplNodeSpace<'a>,
        context: Option<&()>,
    ) -> Result<(), proc_macro2::TokenStream> {
        Self::validate(transform, context, |state| {
            // No checker
            if state.get_checker()?.is_some() {
                return Err(SubscriberBugs::ImplSubCheckerAvailable {}.into());
            }

            // Base Requirements
            let base = state.get_base()?;
            let Type::Path(ty) = &base.ty else {
                return Err(SubscriberBugs::ImplSubBaseInstanceTypeNotTypePath {}.into());
            };
            let mut base_path = ty.path.clone();
            let Some(poped) = base_path.segments.pop() else {
                return Err(SubscriberBugs::ImplSubBaseInstanceTypeLastSegmentNotFound {}.into());
            };
            let base_ident = &poped.into_value().ident;
            base_path.segments.pop_punct();
            let Some(base_q) = &ty.qself else {
                return Err(SubscriberBugs::ImplSubBaseInstanceTypeQSelfNotFound {}.into());
            };
            let base_q_ty = &base_q.ty;

            // Validate Global
            let global = state.get_global()?;
            let Type::Path(ty) = &global.ty else {
                return Err(SubscriberBugs::ImplSubGlobalTyNotTypePath {}.into());
            };
            let Some(q_self) = &ty.qself else {
                return Err(SubscriberBugs::ImplSubGlobalTyQSelfNotFound {}.into());
            };
            let global_q_ty = &q_self.ty;
            if global_q_ty.to_token_stream().to_string() != base_q_ty.to_token_stream().to_string()
            {
                return Err(SubscriberBugs::ImplSubGlobalTyQSelfInvalid {}.into());
            }
            let exp_ident =
                gen_type_ident_from_ident_with_suffix::<InstanceNode>(&base_ident, GLOBAL_NODE);
            let mut global_path = ty.path.clone();
            let Some(poped) = global_path.segments.pop() else {
                return Err(SubscriberBugs::ImplSubGlobalTyLastPathSegNotFound {}.into());
            };
            let global_ident = &poped.into_value().ident;
            global_path.segments.pop_punct();
            if *global_ident != exp_ident {
                return Err(SubscriberBugs::ImplSubGlobalTyLastSegInvalid {}.into());
            }
            if base_path.to_token_stream().to_string() != global_path.to_token_stream().to_string()
            {
                return Err(SubscriberBugs::ImplSubGlobalTyQSelfBoundInvalid {}.into());
            }

            // Validate Twin
            let twin = state.get_twin()?;
            let Type::Path(ty) = &twin.ty else {
                return Err(SubscriberBugs::ImplSubTwinTyNotTypePath {}.into());
            };
            let Some(q_self) = &ty.qself else {
                return Err(SubscriberBugs::ImplSubTwinTyQSelfNotFound {}.into());
            };
            let twin_q_ty = &q_self.ty;
            if twin_q_ty.to_token_stream().to_string() != base_q_ty.to_token_stream().to_string() {
                return Err(SubscriberBugs::ImplSubTwinTyQSelfInvalid {}.into());
            }
            let exp_ident =
                gen_type_ident_from_ident_with_suffix::<InstanceNode>(&base_ident, TWIN_NODE);
            let mut twin_path = ty.path.clone();
            let Some(poped) = twin_path.segments.pop() else {
                return Err(SubscriberBugs::ImplSubTwinTyLastSegNotFound {}.into());
            };
            let twin_ident = &poped.into_value().ident;
            twin_path.segments.pop_punct();
            if *twin_ident != exp_ident {
                return Err(SubscriberBugs::ImplSubTwinTyLastSegInvalid {}.into());
            }
            if base_path.to_token_stream().to_string() != twin_path.to_token_stream().to_string() {
                return Err(SubscriberBugs::ImplSubTwinTyLastQSelfBoundInvalid {}.into());
            }

            // Validate Initial
            let initial = state.get_initial()?;
            let Type::Path(ty) = &initial.ty else {
                return Err(SubscriberBugs::ImplSubInitialTyNotTypePath {}.into());
            };
            let Some(q_self) = &ty.qself else {
                return Err(SubscriberBugs::ImplSubInitialTyQSelfNotFound {}.into());
            };
            let initial_q_ty = &q_self.ty;
            if initial_q_ty.to_token_stream().to_string() != base_q_ty.to_token_stream().to_string()
            {
                return Err(SubscriberBugs::ImplSubInitialTyQSelfInvalid {}.into());
            }
            let exp_ident =
                gen_type_ident_from_ident_with_suffix::<InstanceNode>(&base_ident, INITIAL_NODE);
            let mut initial_path = ty.path.clone();
            let Some(poped) = initial_path.segments.pop() else {
                return Err(SubscriberBugs::ImplSubInitialTyLastSegNotFound {}.into());
            };
            let initial_ident = &poped.into_value().ident;
            initial_path.segments.pop_punct();
            if *initial_ident != exp_ident {
                return Err(SubscriberBugs::ImplSubInitialTyLastSegInvalid {}.into());
            }
            if base_path.to_token_stream().to_string() != initial_path.to_token_stream().to_string()
            {
                return Err(SubscriberBugs::ImplSubInitialTyLastQSelfBoundInvalid {}.into());
            }

            // Validate Final
            let finalz = state.get_final()?;
            let Type::Path(ty) = &finalz.ty else {
                return Err(SubscriberBugs::ImplSubFinalTyNotTypePath {}.into());
            };
            let Some(q_self) = &ty.qself else {
                return Err(SubscriberBugs::ImplSubFinalTyQSelfNotFound {}.into());
            };
            let finalz_q_ty = &q_self.ty;
            if finalz_q_ty.to_token_stream().to_string() != base_q_ty.to_token_stream().to_string()
            {
                return Err(SubscriberBugs::ImplSubFinalTyQSelfInvalid {}.into());
            }
            let exp_ident =
                gen_type_ident_from_ident_with_suffix::<InstanceNode>(&base_ident, FINAL_NODE);
            let mut finalz_path = ty.path.clone();
            let Some(poped) = finalz_path.segments.pop() else {
                return Err(SubscriberBugs::ImplSubFinalTyLastSegNotFound {}.into());
            };
            let finalz_ident = &poped.into_value().ident;
            finalz_path.segments.pop_punct();
            if *finalz_ident != exp_ident {
                return Err(SubscriberBugs::ImplSubFinalTyLastSegInvalid {}.into());
            }
            if base_path.to_token_stream().to_string() != finalz_path.to_token_stream().to_string()
            {
                return Err(SubscriberBugs::ImplSubFinalTyLastQSelfBoundInvalid {}.into());
            }

            Ok(())
        })?;
        Ok(())
    }
}
