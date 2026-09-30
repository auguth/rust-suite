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
// ```````````````````````````` INSTANCE LEAF ACCESS `````````````````````````````
// ===============================================================================

//! Recursive rewriting of instance-trait leaf-config access expressions.
//!
//! User-written instance impl accesses may appear directly as qualified
//! paths or be nested inside higher-level Rust expressions such as
//! function calls, tuples, references, `return` expressions, and `?`
//! expressions.
//!
//! This module recursively traverses those expression forms until the
//! underlying qualified instance-trait path is reached.
//!
//! Once located, the qualified path is rewritten to remove the counter
//! const generics and supply the hidden typenum counter generic required
//! to uniquely identify an instance implementation. That rewriting is
//! performed by [`InstanceLeafAccessExprPath`], while the remaining
//! transformations simply recurse through enclosing expression nodes
//! until such a path is encountered.
//!
//! Only an instance trait implementing associated-type's node instance resolution
//! is supported, allowing accesses or associated instance types to be rewritten uniformly.

// ===============================================================================
// ``````````````````````````````````` IMPORTS ```````````````````````````````````
// ===============================================================================

// --- Proc Macro Crates ---
use quote::ToTokens;
use syn::{ExprPath, parse};

// --- Proc Suite ---
use proc_suite::{BStringList, DuplicateCheck, IntList};

// --- Local Crate ---
use crate::{
    Transformation,
    access::{
        args::AssocAccessorMut,
        direct::*,
        errors::{ResolutionSpace, ResolveBugs, ResolveErrors},
    },
};

// ===============================================================================
// ```````````````````````````` INSTANCE LEAF ACCESS `````````````````````````````
// ===============================================================================

/// Recursively transforms instance-trait accesses within supported expression
/// forms for leaf instance access.
///
/// The transformation traverses the supported expression wrappers, including
/// function calls, tuples, references, `return` expressions, and `?`
/// expressions, until the underlying qualified instance-trait path is reached.
/// The qualified path is then rewritten by [`InstanceLeafAccessExprPath`] to
/// resolve the associated item through the concrete leaf instance and its
/// hidden typenum counter.
///
/// The supplied byte-string identifiers and counter indexes identify the
/// requested instance position. Their lengths are required to match, and
/// duplicate counter indexes are rejected before transformation begins.
#[derive(Debug, Clone, Default)]
pub(crate) struct InstanceLeafAccess;

impl<'a> Transformation<AssocAccessorMut<'a>, (&BStringList, &IntList)> for InstanceLeafAccess {
    fn validate_context(
        &self,
        context: &(&BStringList, &IntList),
    ) -> Result<(), proc_macro2::TokenStream> {
        context
            .1
            .duplicate_check(Some(ResolutionSpace::DuplicateCounterIndexes.into()))?;
        Ok(())
    }

    fn raw_transform(
        &self,
        transform: &mut AssocAccessorMut<'a>,
        context: &(&BStringList, &IntList),
    ) -> Result<(), proc_macro2::TokenStream> {
        match transform {
            AssocAccessorMut::Path(expr_path) => InstanceLeafAccessExprPath::checked_transform(
                &InstanceLeafAccessExprPath,
                expr_path,
                context,
            )?,
            AssocAccessorMut::Fn(expr_call) => {
                InstanceDirectAccessExprCall::<Self>::checked_transform(
                    &InstanceDirectAccessExprCall::<Self>::default(),
                    expr_call,
                    context,
                )?
            }
            AssocAccessorMut::Tuple(expr_tuple) => {
                InstanceDirectAccessExprTuple::<Self>::checked_transform(
                    &InstanceDirectAccessExprTuple::<Self>::default(),
                    expr_tuple,
                    context,
                )?
            }
            AssocAccessorMut::Ref(expr_reference) => {
                InstanceDirectAccessExprRef::<Self>::checked_transform(
                    &InstanceDirectAccessExprRef::<Self>::default(),
                    expr_reference,
                    context,
                )?
            }
            AssocAccessorMut::Return(expr_return) => {
                InstanceDirectAccessExprRetn::<Self>::checked_transform(
                    &InstanceDirectAccessExprRetn::<Self>::default(),
                    expr_return,
                    context,
                )?
            }
            AssocAccessorMut::Try(expr_try) => {
                InstanceDirectAccessExprTry::<Self>::checked_transform(
                    &InstanceDirectAccessExprTry::<Self>::default(),
                    expr_try,
                    context,
                )?
            }
        }

        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &AssocAccessorMut<'a>,
        context: Option<&(&BStringList, &IntList)>,
    ) -> Result<(), proc_macro2::TokenStream> {
        match transform {
            AssocAccessorMut::Path(expr_path) => InstanceLeafAccessExprPath::validate_transform(
                &InstanceLeafAccessExprPath,
                expr_path,
                context,
            )?,
            AssocAccessorMut::Fn(expr_call) => {
                InstanceDirectAccessExprCall::<Self>::validate_transform(
                    &InstanceDirectAccessExprCall::<Self>::default(),
                    expr_call,
                    context,
                )?
            }
            AssocAccessorMut::Tuple(expr_tuple) => {
                InstanceDirectAccessExprTuple::<Self>::validate_transform(
                    &InstanceDirectAccessExprTuple::<Self>::default(),
                    expr_tuple,
                    context,
                )?
            }
            AssocAccessorMut::Ref(expr_reference) => {
                InstanceDirectAccessExprRef::<Self>::validate_transform(
                    &InstanceDirectAccessExprRef::<Self>::default(),
                    expr_reference,
                    context,
                )?
            }
            AssocAccessorMut::Return(expr_return) => {
                InstanceDirectAccessExprRetn::<Self>::validate_transform(
                    &InstanceDirectAccessExprRetn::<Self>::default(),
                    expr_return,
                    context,
                )?
            }
            AssocAccessorMut::Try(expr_try) => {
                InstanceDirectAccessExprTry::<Self>::validate_transform(
                    &InstanceDirectAccessExprTry::<Self>::default(),
                    expr_try,
                    context,
                )?
            }
        }
        Ok(())
    }
}

// ===============================================================================
// `````````````````````````` INSTANCE ACCESS EXPR-PATH ``````````````````````````
// ===============================================================================

/// Rewrites qualified instance-trait path expressions to reference a
/// specific instance implementation's associate item.
///
/// Instance traits internally replace their counter const generics with
/// a hidden typenum counter type (both in [`traits`](crate::traits) and
/// [`impls`](crate::impls)). Consequently, every qualified
/// instance-trait path must be replaced with the corresponding typenum
/// counter in order to uniquely identify the intended implementation.
///
/// This transformation derives that counter const generics via the supplied:
///
/// - counter identifiers,
/// - counter-generic indexes,
/// - and the implementing `Self` type,
///
/// removes them and inserts the recovered counter typenum type
/// into the outermost qualified instance-trait path.
///
/// Qualified paths reference associated types:
///
/// ```ignore
/// <<T as InstanceNode>::NodeAssoc as InstanceTrait<0,1>>::VALUE
/// ```
///
/// Here the implementing `Self` type is **not** a concrete type but an
/// associated type produced by another trait.
///
/// A trait declaring associated types that themselves implement instance
/// traits is called a **instance node**.
///
/// For example:
///
/// ```ignore
/// trait InstanceNode {
///     type NodeAssoc: InstanceTrait<0,1>;
/// }
/// ```
///
/// Each such associated type is called an **subscriber node** (see [`crate::node`]).
///
/// Unlike a concrete implementing type, a subscriber node is not itself
/// the canonical owner of its typenum counter tuple. Instead, every
/// subscriber node exposes a hidden associated type providing access to
/// the global counter lookup for each published instance node.
///
/// Conceptually:
///
/// ```ignore
/// trait InstanceNode {
///     type NodeAssoc: InstanceTrait<0,1>;
///
///     #[doc(hidden)]
///     type __Global:
///         CounterAccessN<HASH_0, HASH_1, Self::NodeAssoc>;
/// }
/// ```
///
/// The corresponding publisher implementation supplies both the concrete
/// instance type and the global lookup type:
///
/// ```ignore
/// impl InstanceNode for ConcreteNode {
///     type NodeAssoc = InstanceType;
///
///     type __Global = Global;
/// }
/// ```
///
/// Consequently, subscriber resolution cannot directly construct:
///
/// ```ignore
/// <Global as CounterAccessN<
///     HASH_0,
///     HASH_1,
///     <T as InstanceNode>::NodeAssoc,
/// >>::Counter
/// ```
///
/// because an associated type is not the canonical source of the counter
/// information. Instead, the transformation first recovers the hidden
/// global lookup type exposed by the instance node:
///
/// ```ignore
/// <T as InstanceNode>::__Global
/// ```
///
/// and then performs the counter lookup through that recovered global
/// type:
///
/// ```ignore
/// <<T as InstanceNode>::__Global as CounterAccessN<
///     HASH_0,
///     HASH_1,
///     <T as InstanceNode>::NodeAssoc,
/// >>::Counter
/// ```
///
/// Thus, publisher nodes (concrete node structs) provide the global lookup directly, whereas
/// subscriber nodes first expose the appropriate global lookup through a
/// hidden associated type before the counter tuple is recovered.
///
/// ## Result
///
/// This transformation rewrites the outermost qualified instance-trait path
/// so that the hidden typenum counter argument is supplied automatically.
///
/// This preserves a single canonical mechanism for resolving instance
/// implementations of associated instance types to participate uniformly
/// in instance dispatch.
#[derive(Debug, Clone, Default)]
pub(crate) struct InstanceLeafAccessExprPath;

impl<'a> Transformation<ExprPath, (&BStringList, &IntList)> for InstanceLeafAccessExprPath {
    fn raw_transform(
        &self,
        transform: &mut ExprPath,
        context: &(&BStringList, &IntList),
    ) -> Result<(), proc_macro2::TokenStream> {
        let expr = transform;
        let idents = context.0;
        let indexes = context.1;

        if idents.bytes.len() != indexes.ints.len() {
            return Err(ResolveErrors::IdentsAndIndexesNotSameLen {
                indexes: indexes.clone(),
                idents: idents.clone(),
            }
            .into());
        }

        let Some(q_base) = &mut expr.qself else {
            return Err(ResolveErrors::ExprPathRequiresQSelf {
                expr_path: expr.clone(),
            }
            .into());
        };

        let base_ty = &*q_base.ty;

        let pos = q_base.position;
        let trait_p = &mut expr.path.segments.iter_mut().take(pos);

        let Some(trait_path) = trait_p.last() else {
            return Err(ResolveErrors::InstanceTraitPathNotFound {
                path: expr.path.clone(),
            }
            .into());
        };

        let Ok(assoc_path) = parse::<ExprPath>(base_ty.to_token_stream().into()) else {
            return Err(ResolveErrors::QSelfRequiresExprPath {
                qself: base_ty.clone(),
            }
            .into());
        };

        subscriber_call(&assoc_path, trait_path, indexes, idents)?;

        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &ExprPath,
        context: Option<&(&BStringList, &IntList)>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let Some(context) = context else {
            return Ok(());
        };

        let expr = transform;
        let idents = context.0;
        let indexes = context.1;

        if idents.bytes.len() != indexes.ints.len() {
            return Err(ResolveBugs::IdentsAndIndexesNotSameLen {}.into());
        }

        let Some(q_base) = &expr.qself else {
            return Err(ResolveBugs::ExprPathRequiresQSelf {}.into());
        };

        let base_ty = &q_base.ty;

        let pos = q_base.position;
        let trait_p = expr.path.segments.iter().take(pos);
        let Some(trait_path) = trait_p.last() else {
            return Err(ResolveBugs::InstanceTraitPathNotFound {}.into());
        };

        let Ok(assoc_path) = parse::<ExprPath>(base_ty.to_token_stream().into()) else {
            return Err(ResolveBugs::QSelfRequiresExprPath {}.into());
        };

        validate_subscriber_call(&assoc_path, trait_path, idents, indexes)?;

        Ok(())
    }
}
