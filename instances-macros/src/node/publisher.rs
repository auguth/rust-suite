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
// ``````````````````````````` INSTANCE PUBLISHER NODE ```````````````````````````
// ===============================================================================

//! Implementation-side publisher transformations.
//!
//! This module contains the implementation counterparts publisher nodes. Unlike
//! subscriber nodes, publisher nodes exist only as implementation transformations
//! because publishing is the terminal resolution stage of instance generation.
//!
//! Subscriber node transformations are split across both declaration and
//! implementation modules, allowing qualified associated types to be carried
//! across phases. Publisher node transformations instead finalize those
//! generated companion nodes, resolve implementation-side range endpoints,
//! and force cumulative compile-time validation.
//!
//! Implemented transformations:
//!
//! - [`InstanceImplPublisher`]: applies publisher-specific implementation
//!   semantics, resolves generated range companion types, and forces
//!   evaluation of the terminal checker.

// ===============================================================================
// ``````````````````````````````````` IMPORTS ```````````````````````````````````
// ===============================================================================

// --- Proc Macro Crates ---
use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident};
use syn::{Expr, ImplItem, Item, ItemConst, ItemImpl, parse_quote, spanned::Spanned};

// --- Proc Suite ---
use proc_suite::{BStringList, misc::*};

// --- Local Crate
use crate::{
    Extraction, Transformation, Utilization,
    impls::utils::ImplTraitPath,
    node::{
        access::*,
        args::*,
        errors::{PublisherBugs, PublisherError},
        state::*,
    },
    utils::*,
};

// ===============================================================================
// ``````````````````````````` INSTANCE PUBLISHER NODE ```````````````````````````
// ===============================================================================

impl<'a> Transformation<ImplNodeSpace<'a>, NodeArgs<Self>> for PublisherNode {
    fn raw_transform(
        &self,
        transform: &mut ImplNodeSpace<'a>,
        context: &NodeArgs<Self>,
    ) -> Result<(), proc_macro2::TokenStream> {
        InstanceImplPublisher::checked_transform(&InstanceImplPublisher, transform, context)?;
        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &ImplNodeSpace,
        context: Option<&NodeArgs<Self>>,
    ) -> Result<(), proc_macro2::TokenStream> {
        InstanceImplPublisher::validate_transform(&InstanceImplPublisher, transform, context)?;
        Ok(())
    }
}

// ===============================================================================
// ```````````````````````` INSTANCE IMPL TYPE PUBLISHER `````````````````````````
// ===============================================================================

/// Implementation-side publisher transformation.
///
/// This transformation performs the publisher-specific implementation phase
/// after the common implementation companion nodes have been synthesized by
/// [`ImplNodeSegment::prepare`].
///
/// Specifically, this transformation:
///
/// - Resolves the generated [`INITIAL_NODE`] and [`FINAL_NODE`] companion
///   associated type to the implementation-side starting instance expected
///   by the declaration-side subscriber. The resolved type is obtained
///   through the corresponding [`ExactAccess`] projection using the same
///   subscriber arguments.
///
/// - Verifies that the publisher implementation and its published associated
///   types declare no generic parameters. A publisher represents the terminal
///   resolution stage; therefore every associated type must already be fully
///   resolved.
///
/// - Rejects implementations requiring further generic resolution. Such
///   implementations should instead use
///   [`super::subscriber::InstanceImplSubscriber`], allowing the generated
///   companion associated types to remain deferred through qualified
///   associated types.
///
/// - Generates a compile-time forcing constant for the generated
///   [`TERMINAL_CHECKER_NODE`]. Since publisher implementations are fully
///   resolved, the cumulative terminal checker can be evaluated immediately
///   during constant evaluation.
///
/// Example:
///
/// ```text
/// Trait publisher:
///     #[branch(parent = Accounts)]
///
/// Implementation publisher:
///     #[branch(parent = Accounts)]
///
/// Both receive:
///
///     BranchArgs {
///         parent: Accounts,
///         indexes: ...
///     }
///
/// Generated by ImplNodeSegment::prepare:
///
///     type AccountGlobal;
///     type AccountTwin;
///     type AccountInitial;
///     type AccountFinal;
///
/// Publisher transformation:
///
///     type AccountInitial =
///         <AccountGlobal as OnSetAccess<...>>::Exact;
///
///     type AccountFinal =
///         <AccountGlobal as BoundaryAccess<...>>::Exact;
///
/// Separate constant:
///
///     const AccountTerminalChecker: () = {
///         /* forces cumulative terminal validation */
///     };
/// ```
///
/// Consequently, the declaration and implementation publishers remain
/// synchronized under the same publisher model. The declaration side
/// determines the expected [`ExactAccess`] endpoints, while the implementation
/// side resolves those same endpoints using the identical publisher
/// arguments, ensuring both phases remain consistent.
#[derive(Debug, Clone)]
pub(crate) struct InstanceImplPublisher;

impl<'a> Transformation<ImplNodeSpace<'a>, NodeArgs<PublisherNode>> for InstanceImplPublisher {
    fn raw_transform(
        &self,
        transform: &mut ImplNodeSpace<'a>,
        context: &NodeArgs<PublisherNode>,
    ) -> Result<(), proc_macro2::TokenStream> {
        Self::prepare(transform, context, |state| {
            let global = &state.get_global()?.ty;
            let base = &state.get_base()?.ty;
            let def = &BStringList::default();
            let (initial, finalz) = match context {
                NodeArgs::Leaf(_) => (None, None),
                NodeArgs::Branch(branch) => {
                    let BranchArgs {
                        indexes, parent, ..
                    } = branch;
                    let context = (base, indexes, parent).into();
                    let initial_bound = OnSetAccess::checked_extract(&context, &())?.0;
                    let final_bound = BoundaryAccess::checked_extract(&context, &())?.0;
                    let initial_project = OnSetAccess::projection();
                    let initial = parse_quote!(<#global as #initial_bound>::#initial_project);
                    let final_project = BoundaryAccess::projection();
                    let finalz = parse_quote!(<#global as #final_bound>::#final_project);
                    (Some(initial), Some(finalz))
                }
                NodeArgs::Prune(prune) => {
                    let PruneArgs { indexes, until, .. } = prune;
                    let i_context = (base, indexes, def).into();
                    let initial_bound = OnSetAccess::checked_extract(&i_context, &())?.0;
                    let f_context = (base, indexes, until).into();
                    let final_bound = CounterAccess::checked_extract(&f_context, &())?.0;
                    let initial_project = OnSetAccess::projection();
                    let initial = parse_quote!(<#global as #initial_bound>::#initial_project);
                    let final_project = CounterAccess::projection();
                    let finalz = parse_quote!(<#global as #final_bound>::#final_project);
                    (Some(initial), Some(finalz))
                }
                NodeArgs::Trim(trim) => {
                    let TrimArgs { indexes, until, .. } = trim;
                    let i_context = (base, indexes, def).into();
                    let initial_bound = OnSetAccess::checked_extract(&i_context, &())?.0;
                    let f_context = (base, indexes, until).into();
                    let final_bound = BoundaryAccess::checked_extract(&f_context, &())?.0;
                    let initial_project = OnSetAccess::projection();
                    let initial = parse_quote!(<#global as #initial_bound>::#initial_project);
                    let final_project = BoundaryAccess::projection();
                    let finalz = parse_quote!(<#global as #final_bound>::#final_project);
                    (Some(initial), Some(finalz))
                }
                NodeArgs::Extend(extend) => {
                    let ExtendArgs { indexes, from, .. } = extend;
                    let i_context = (base, indexes, from).into();
                    let initial_bound = CounterAccess::checked_extract(&i_context, &())?.0;
                    let f_context = (base, indexes, def).into();
                    let final_bound = BoundaryAccess::checked_extract(&f_context, &())?.0;
                    let initial_project = CounterAccess::projection();
                    let initial = parse_quote!(<#global as #initial_bound>::#initial_project);
                    let final_project = BoundaryAccess::projection();
                    let finalz = parse_quote!(<#global as #final_bound>::#final_project);
                    (Some(initial), Some(finalz))
                }
                NodeArgs::Spread(spread) => {
                    let SpreadArgs { indexes, from, .. } = spread;
                    let i_context = (base, indexes, from).into();
                    let initial_bound = OnSetAccess::checked_extract(&i_context, &())?.0;
                    let f_context = (base, indexes, def).into();
                    let final_bound = BoundaryAccess::checked_extract(&f_context, &())?.0;
                    let initial_project = OnSetAccess::projection();
                    let initial = parse_quote!(<#global as #initial_bound>::#initial_project);
                    let final_project = BoundaryAccess::projection();
                    let finalz = parse_quote!(<#global as #final_bound>::#final_project);
                    (Some(initial), Some(finalz))
                }
                NodeArgs::Traverse(traverse) => {
                    let TraverseArgs {
                        indexes,
                        from,
                        until,
                        ..
                    } = traverse;
                    let leaf = from;
                    let branch = until;
                    let i_context = (base, indexes, branch).into();
                    let initial_bound = OnSetAccess::checked_extract(&i_context, &())?.0;
                    let f_context = (base, indexes, leaf).into();
                    let final_bound = CounterAccess::checked_extract(&f_context, &())?.0;
                    let initial_project = OnSetAccess::projection();
                    let initial = parse_quote!(<#global as #initial_bound>::#initial_project);
                    let final_project = CounterAccess::projection();
                    let finalz = parse_quote!(<#global as #final_bound>::#final_project);
                    (Some(initial), Some(finalz))
                }
                NodeArgs::Descend(descend) => {
                    let DescendArgs {
                        indexes,
                        from,
                        until,
                        ..
                    } = descend;
                    let leaf = until;
                    let branch = from;
                    let i_context = (base, indexes, branch).into();
                    let initial_bound = OnSetAccess::checked_extract(&i_context, &())?.0;
                    let f_context = (base, indexes, leaf).into();
                    let final_bound = CounterAccess::checked_extract(&f_context, &())?.0;
                    let initial_project = OnSetAccess::projection();
                    let initial = parse_quote!(<#global as #initial_bound>::#initial_project);
                    let final_project = CounterAccess::projection();
                    let finalz = parse_quote!(<#global as #final_bound>::#final_project);
                    (Some(initial), Some(finalz))
                }
                NodeArgs::Root(root) => {
                    let RootArgs { indexes, .. } = root;
                    let i_context = (base, indexes, def).into();
                    let initial_bound = OnSetAccess::checked_extract(&i_context, &())?.0;
                    let f_context = (base, indexes, def).into();
                    let final_bound = BoundaryAccess::checked_extract(&f_context, &())?.0;
                    let initial_project = OnSetAccess::projection();
                    let initial = parse_quote!(<#global as #initial_bound>::#initial_project);
                    let final_project = BoundaryAccess::projection();
                    let finalz = parse_quote!(<#global as #final_bound>::#final_project);
                    (Some(initial), Some(finalz))
                }
                NodeArgs::Unknown(token) => {
                    return Err(PublisherError::ExpectedKeys {
                        span: token.tokens.span(),
                    }
                    .into());
                }
            };

            if let Some(initial) = initial {
                state.mutate_initial(|ty| {
                    *ty = initial;
                    Ok(())
                })?;
            }

            if let Some(finalz) = finalz {
                state.mutate_final(|ty| {
                    *ty = finalz;
                    Ok(())
                })?;
            }
            Ok(())
        })?;

        let ImplNodeSpace {
            impl_of,
            ty_idx,
            addons,
        } = transform;
        let Some(impl_item) = impl_of.items.get(*ty_idx) else {
            return Err(PublisherBugs::ImplItemIdxFetchFailed {}.into());
        };
        let ImplItem::Type(assoc_ty) = impl_item else {
            return Err(PublisherBugs::FetchedImplItemNotTypeAssoc {}.into());
        };
        let ident = &assoc_ty.ident;
        let generics = assoc_ty.generics.clone();

        let self_ty = &impl_of.self_ty;

        let trait_path = ImplTraitPath::checked_utilize(impl_of, &())?.0;

        let merged = merge_generics(&generics, &impl_of.generics)?;

        if !merged.params.is_empty() {
            return Err(PublisherError::ImplOrAssocMayHaveGenerics {
                generic: merged.params.first().unwrap().clone(),
            }
            .into());
        }

        let checker_ident = gen_const_ident_from_ident_with_suffix::<InstanceTerminalNode>(
            ident,
            TERMINAL_CHECKER_NODE,
        );
        let checker = ItemConst {
            attrs: proc_suite::internal_code(),
            vis: syn::Visibility::Inherited,
            ident: format_ident!("_"),
            ty: parse_quote!(()),
            expr: parse_quote!(<#self_ty as #trait_path>::#checker_ident),
            generics: Default::default(),
            const_token: Default::default(),
            colon_token: Default::default(),
            eq_token: Default::default(),
            semi_token: Default::default(),
        };

        addons.items.push(syn::Item::Const(checker));

        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &ImplNodeSpace<'a>,
        context: Option<&NodeArgs<PublisherNode>>,
    ) -> Result<(), proc_macro2::TokenStream> {
        Self::validate(transform, context, |_| Ok(()))?;
        let ImplNodeSpace {
            impl_of,
            ty_idx,
            addons,
        } = transform;
        let Some(impl_item) = impl_of.items.get(*ty_idx) else {
            return Err(PublisherBugs::ImplItemIdxFetchFailed {}.into());
        };
        let ImplItem::Type(assoc_ty) = impl_item else {
            return Err(PublisherBugs::FetchedImplItemNotTypeAssoc {}.into());
        };

        let orig_ident = &assoc_ty.ident;
        let checker_ident = gen_const_ident_from_ident_with_suffix::<InstanceTerminalNode>(
            orig_ident,
            TERMINAL_CHECKER_NODE,
        );

        let validate_const = |c: &ItemConst, context: &ItemImpl| -> Result<(), TokenStream> {
            if c.ident != format_ident!("_") {
                return Err(PublisherBugs::PublisherCheckerConstInvalidIdent {}.into());
            };

            let expr = &*c.expr;

            let self_ty = &context.self_ty;
            let generics = &context.generics;

            if !generics.params.is_empty() {
                return Err(PublisherBugs::PublisherCheckerConstIsGeneric {}.into());
            }

            let trait_path = ImplTraitPath::checked_utilize(context, &())?.0;

            let exp_expr: Expr = parse_quote!(<#self_ty as #trait_path>::#checker_ident);

            if expr.to_token_stream().to_string() != exp_expr.to_token_stream().to_string() {
                return Err(PublisherBugs::PublisherCheckerConstInvalidExpr {}.into());
            }

            Ok(())
        };

        let mut found_const = false;
        for item in &addons.items {
            let Item::Const(c) = item else {
                continue;
            };

            if c.ident != format_ident!("_") {
                continue;
            }

            let Expr::Path(path) = &*c.expr else {
                continue;
            };

            if path
                .path
                .segments
                .last()
                .is_some_and(|seg| seg.ident == checker_ident)
            {
                validate_const(c, impl_of)?;
                found_const = true;
                break;
            }
        }

        if !found_const {
            return Err(PublisherBugs::PublisherCheckerConstNotFound {}.into());
        };

        Ok(())
    }
}
