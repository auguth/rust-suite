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
// ``````````````````````````` INSTANCE DIRECT ACCESS ````````````````````````````
// ===============================================================================

//! Direct access to instance-trait associated items through a resolved instance.
//!
//! User-written instance-trait accesses may appear directly as qualified paths
//! or be nested inside supported Rust expression forms such as function calls,
//! tuples, references, `return` expressions, and `?` expressions.
//!
//! This module recursively traverses those expression forms until the
//! underlying qualified instance-trait path is reached.
//!
//! Unlike leaf access ([`crate::access::leaf`]), direct access supports both
//! sides of the instance-node boundary. A qualified instance-trait path may
//! resolve through a subscriber node whose instance type is provided by an
//! associated type, or directly through a publisher node whose concrete
//! implementing type is already known.
//!
//! For subscriber nodes, the hidden global lookup associated with the instance
//! node is recovered and used to resolve the typenum counter for the requested
//! instance. For publisher nodes, the concrete implementing type is used
//! directly with the globally shared counter lookup to resolve the same
//! typenum counter.
//!
//! In both cases, the user-facing counter const generics are removed from the
//! instance-trait path and replaced with the hidden typenum counter associated
//! with the resolved instance. This allows instance-trait associated types and
//! associated functions to be accessed uniformly regardless of whether the
//! instance is reached through a publisher node or a subscriber node.
//!
//! The recursive expression transformations preserve the surrounding Rust
//! expression while delegating the actual instance resolution to
//! [`InstanceDirectAccessExprPath`].
//!
// ===============================================================================
// ``````````````````````````````````` IMPORTS ```````````````````````````````````
// ===============================================================================

use std::marker::PhantomData;

// --- Proc Macro Crates ---
use proc_macro2::TokenStream;
use syn::{
    Expr, ExprCall, ExprPath, ExprReference, ExprReturn, ExprTry, ExprTuple, PathSegment, Type,
    parse::{ParseStream, Parser},
    parse_quote,
};

// --- Proc Suite ---
use proc_suite::{BStringList, DuplicateCheck, IntList, SupportCrate, misc::*};

// --- Local Crate ---
use crate::{
    Extraction, Instance, Transformation,
    access::{
        args::{AssocAccessor, AssocAccessorMut, LeafAccess},
        errors::{ResolutionSpace, ResolveBugs, ResolveErrors},
    },
    node::{
        access::CounterAccess,
        state::{GLOBAL_NODE, InstanceNode},
    },
};

// ===============================================================================
// ```````````````````````` INSTANCE DIRECT LEAF ACCESS ``````````````````````````
// ===============================================================================

/// Performs direct instance-trait leaf access on an expression.
///
/// Direct access supports instance-trait associated items through both concrete
/// implementing types and associated instance types.
///
/// - A concrete implementing type acts as the publisher node, allowing its
///   instance counter to be recovered directly from the global counter lookup.
/// - An associated instance type acts as a subscriber node, requiring its
///   hidden global lookup to be recovered before the instance counter can be
///   resolved.
///
/// For example, both forms:
///
/// ```ignore
/// <InstanceType as InstanceTrait<..>>::VALUE
///
/// <<T as InstanceNode>::NodeAssoc as InstanceTrait<..>>::VALUE
/// ```
///
/// can be resolved into their corresponding instance-specific representations.
/// The first form resolves the counter from the concrete publisher type,
/// while the second recovers the hidden global lookup exposed by the subscriber
/// node and uses its associated instance type for the counter lookup.
///
/// The expression is first converted into an [`AssocAccessor`] so that the
/// instance-trait access can be resolved independently of its surrounding
/// expression structure. [`InstanceDirectAccessEntry`] then recursively
/// resolves the contained access using the indexes and identifiers supplied by
/// [`LeafAccess`], after which the transformed accessor is converted back into
/// an [`Expr`].
///
/// Validation performs the same resolution checks without modifying the
/// expression, ensuring that both concrete and associated instance accesses
/// accepted during validation can be resolved during transformation.
#[derive(Debug, Clone)]
pub(crate) struct InstanceDirectAccess;

impl Transformation<Expr, LeafAccess> for InstanceDirectAccess {
    fn raw_transform(
        &self,
        transform: &mut Expr,
        context: &LeafAccess,
    ) -> Result<(), proc_macro2::TokenStream> {
        let LeafAccess { indexes, idents } = context;

        let mut assoc = match AssocAccessor::try_from(transform.clone()) {
            Ok(a) => a,
            Err(e) => {
                return Err(e.into_compile_error());
            }
        };

        let mut mut_assoc = (&mut assoc).into();

        InstanceDirectAccessEntry::checked_transform(
            &InstanceDirectAccessEntry,
            &mut mut_assoc,
            &(idents, indexes),
        )?;

        let new_expr: Expr = parse_quote!(#mut_assoc);

        *transform = new_expr;

        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &Expr,
        context: Option<&LeafAccess>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let mut assoc = match AssocAccessor::try_from(transform.clone()) {
            Ok(a) => a,
            Err(e) => {
                return Err(e.into_compile_error());
            }
        };

        let mut mut_assoc = (&mut assoc).into();

        let Some(LeafAccess { indexes, idents }) = context else {
            return Ok(());
        };

        InstanceDirectAccessEntry::validate_transform(
            &InstanceDirectAccessEntry,
            &mut mut_assoc,
            Some(&(idents, indexes)),
        )?;

        Ok(())
    }
}

// ===============================================================================
// ```````````````````````````` INSTANCE LEAF ACCESS `````````````````````````````
// ===============================================================================

/// Recursively transforms and validates direct instance-trait leaf access.
///
/// The accessor may contain a qualified/concrete instance-trait path directly or be
/// wrapped by a supported expression form such as a function call, tuple,
/// reference, `return` expression, or `?` expression. Each form is delegated
/// to its corresponding transformation so that the underlying instance-trait
/// access is eventually resolved by [`InstanceDirectAccessExprPath`].
#[derive(Debug, Clone, Default)]
pub(crate) struct InstanceDirectAccessEntry;

impl<'a> Transformation<AssocAccessorMut<'a>, (&BStringList, &IntList)>
    for InstanceDirectAccessEntry
{
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
            AssocAccessorMut::Path(expr_path) => InstanceDirectAccessExprPath::checked_transform(
                &InstanceDirectAccessExprPath,
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
            AssocAccessorMut::Path(expr_path) => InstanceDirectAccessExprPath::validate_transform(
                &InstanceDirectAccessExprPath,
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
/// ## Publisher resolution
///
/// When the qualified path's `Self` type is a concrete type:
///
/// ```ignore
/// <InstanceType as InstanceTrait<0,1>>::VALUE
/// ```
///
/// the implementation itself acts as the **publisher node**. Since the
/// implementing type is already known, its typenum counter tuple can be
/// recovered directly through the globally shared lookup (from the support crate):
///
/// ```ignore
/// <support_crate::Global as CounterAccess3<
///     HASH_0,
///     HASH_1,
///     InstanceType,
/// >>::Counter
/// ```
///
/// yielding:
///
/// ```ignore
/// <InstanceType as InstanceTrait<
///     <Global as CounterAccess3<
///         HASH_0,
///         HASH_1,
///         InstanceType,
///     >>::Counter,
/// >>::VALUE
/// ```
///
/// the counter const-generics are removed and replaced with the
/// counter access associated type.
///
/// ## Subscriber resolution
///
/// Qualified paths may also reference associated types:
///
/// ```ignore
/// <<Provider as Subscriber>::GetInstance as InstanceTrait<0,1>>::VALUE
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
/// trait Subscriber {
///     type GetInstance: InstanceTrait<0,1>;
/// }
/// ```
///
/// Each such associated type is called an **subscriber node**.
///
/// Unlike a concrete implementing type, a subscriber node is not itself
/// the canonical owner of its typenum counter tuple. Instead, every
/// subscriber node exposes a hidden associated type providing access to
/// the global counter lookup for each published instance node.
///
/// Conceptually:
///
/// ```ignore
/// trait Subscriber {
///     type GetInstance: InstanceTrait<0,1>;
///
///     #[doc(hidden)]
///     type __Global:
///         CounterAccessN<HASH_0, HASH_1, Self::GetInstance>;
/// }
/// ```
///
/// The corresponding publisher implementation supplies both the concrete
/// instance type and the global lookup type:
///
/// ```ignore
/// impl Subscriber for MyPublisher {
///     type GetInstance = InstanceType;
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
///     <Provider as Subscriber>::GetInstance,
/// >>::Counter
/// ```
///
/// because an associated type is not the canonical source of the counter
/// information. Instead, the transformation first recovers the hidden
/// global lookup type exposed by the instance node:
///
/// ```ignore
/// <Provider as Subscriber>::__Global
/// ```
///
/// and then performs the counter lookup through that recovered global
/// type:
///
/// ```ignore
/// <<Provider as Subscriber>::__Global as CounterAccessN<
///     HASH_0,
///     HASH_1,
///     <Provider as Subscriber>::GetInstance,
/// >>::Counter
/// ```
///
/// Thus, publisher nodes provide the global lookup directly, whereas
/// subscriber nodes first expose the appropriate global lookup through a
/// hidden associated type before the counter tuple is recovered.
///
/// ## Result
///
/// Regardless of whether the qualified path refers to a publisher node
/// or an instance node, this transformation rewrites the outermost
/// qualified instance-trait path so that the hidden typenum counter
/// argument is supplied automatically.
///
/// This preserves a single canonical mechanism for resolving instance
/// implementations while allowing both concrete implementing types and
/// associated instance types to participate uniformly in instance
/// dispatch.
#[derive(Debug, Clone)]
pub(crate) struct InstanceDirectAccessExprPath;

impl<'a> Transformation<ExprPath, (&BStringList, &IntList)> for InstanceDirectAccessExprPath {
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

        let base_ty = &q_base.ty;

        let assoc_check = quote::quote! {#base_ty};

        let parser =
            |input: ParseStream| -> syn::Result<ExprPath> { Ok(input.parse::<ExprPath>()?) };

        let pos = q_base.position;
        let trait_p = &mut expr.path.segments.iter_mut().take(pos);

        let Some(trait_path) = trait_p.last() else {
            return Err(ResolveErrors::InstanceTraitPathNotFound {
                path: expr.path.clone(),
            }
            .into());
        };

        match parser.parse2(assoc_check) {
            Ok(assoc_path) => {
                if assoc_path.qself.is_some() {
                    subscriber_call(&assoc_path, trait_path, indexes, idents)?
                } else {
                    publisher_call(base_ty, trait_path, indexes, idents)?
                }
            }
            Err(_) => publisher_call(base_ty, trait_path, indexes, idents)?,
        };

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
        let assoc_check = quote::quote! {#base_ty};

        let parser =
            |input: ParseStream| -> syn::Result<ExprPath> { Ok(input.parse::<ExprPath>()?) };

        let pos = q_base.position;
        let trait_p = expr.path.segments.iter().take(pos);
        let Some(trait_path) = trait_p.last() else {
            return Err(ResolveBugs::InstanceTraitPathNotFound {}.into());
        };

        match parser.parse2(assoc_check) {
            Ok(assoc_path) => {
                if assoc_path.qself.is_some() {
                    validate_subscriber_call(&assoc_path, trait_path, idents, indexes)?
                } else {
                    validate_publisher_call(base_ty, trait_path, idents, indexes)?
                }
            }
            Err(_) => validate_publisher_call(base_ty, trait_path, idents, indexes)?,
        };

        Ok(())
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ``````````````````````````````` PRIVATE HELPERS ```````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// Resolves counter access for subscriber instance types.
///
/// Example:
///
/// ```ignore
/// <<Provider as Subscriber>::GetInstance as InstanceTrait<...>>::VALUE
/// ```
///
/// becomes:
///
/// ```ignore
/// <<Provider as Subscriber>::GetInstance as InstanceTrait<
///     ...,
///     <<Provider as Subscriber>::__Global as CounterAccessN<
///         HASHES...,
///         <Provider as Subscriber>::GetInstance,
///     >>::Counter,
/// >>::VALUE
/// ```
pub(crate) fn subscriber_call(
    assoc_ty: &ExprPath,
    trait_path: &mut PathSegment,
    indexes: &IntList,
    idents: &BStringList,
) -> Result<(), TokenStream> {
    let mut global_ty = assoc_ty.clone();
    let Some(assoc) = global_ty.path.segments.last_mut() else {
        return Err(ResolveErrors::AssocQSelfLastIdentUnavailable {
            path: global_ty.path.clone(),
        }
        .into());
    };

    let ident = &mut assoc.ident;

    *ident = gen_type_ident_from_ident_with_suffix::<InstanceNode>(ident, GLOBAL_NODE);

    let self_ty: Type = parse_quote!(#global_ty);
    let base_ty: Type = parse_quote!(#assoc_ty);

    let context = (&base_ty, indexes, idents).into();
    let bound = CounterAccess::checked_extract(&context, &())?;
    let t_context = (&self_ty, context, false).into();
    CounterAccess::checked_transform(&bound, trait_path, &t_context)?;

    Ok(())
}

/// Resolves counter access for publisher instance types.
///
/// Example:
///
/// ```ignore
/// <InstanceType as InstanceTrait<...>>::VALUE
/// ```
///
/// becomes:
///
/// ```ignore
/// <InstanceType as InstanceTrait<
///     <Global as CounterAccessN<
///         HASHES...,
///         InstanceType,
///     >>::Counter,
/// >>::VALUE
/// ```
pub(crate) fn publisher_call(
    base_ty: &Type,
    trait_path: &mut PathSegment,
    indexes: &IntList,
    idents: &BStringList,
) -> Result<(), TokenStream> {
    let crate_of = Instance::support_crate();
    let self_ty = parse_quote!(#crate_of::Global);

    let context = (base_ty, indexes, idents).into();
    let bound = CounterAccess::checked_extract(&context, &())?;
    let t_context = (&self_ty, context, false).into();
    CounterAccess::checked_transform(&bound, trait_path, &t_context)?;
    Ok(())
}

/// Verifies subscriber-node counter insertion.
///
/// Example:
///
/// ```ignore
/// <<Provider as Subscriber>::GetInstance as InstanceTrait<
///     <<Provider as Subscriber>::__Global as CounterAccessN<...>>::Counter,
/// >>
/// ```
///
/// is validated against the expected generated projection.
pub(crate) fn validate_subscriber_call(
    assoc_path: &ExprPath,
    trait_path: &PathSegment,
    idents: &BStringList,
    indexes: &IntList,
) -> Result<(), TokenStream> {
    let mut self_ty = assoc_path.clone();
    let Some(assoc) = self_ty.path.segments.last_mut() else {
        return Err(ResolveBugs::AssocQSelfLastIdentUnavailable {}.into());
    };
    let ident = &mut assoc.ident;
    *ident = gen_type_ident_from_ident_with_suffix::<InstanceNode>(ident, GLOBAL_NODE);
    let base_ty: Type = parse_quote!(#assoc_path);
    let self_ty: Type = parse_quote!(#self_ty);

    let context = (&base_ty, indexes, idents).into();
    let bound = CounterAccess::checked_extract(&context, &())?;
    let t_context = (&self_ty, context).into();
    CounterAccess::validate_transform(&bound, trait_path, Some(&t_context))?;
    Ok(())
}

/// Verifies publisher-node counter insertion.
///
/// Example:
///
/// ```ignore
/// <InstanceType as InstanceTrait<
///     <Global as CounterAccessN<...>>::Counter,
/// >>
/// ```
///
/// is validated against the expected generated projection.
pub(crate) fn validate_publisher_call(
    base_ty: &Type,
    trait_path: &PathSegment,
    idents: &BStringList,
    indexes: &IntList,
) -> Result<(), TokenStream> {
    let crate_of = Instance::support_crate();
    let self_ty = parse_quote!(#crate_of::Global);
    let context = (base_ty, indexes, idents).into();
    let bound = CounterAccess::checked_extract(&context, &())?;
    let t_context = (&self_ty, context).into();
    CounterAccess::validate_transform(&bound, trait_path, Some(&t_context))?;
    Ok(())
}

// ===============================================================================
// ````````````````````````` ALLOWED EXPR-PATH CONSUMERS `````````````````````````
// ===============================================================================

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// `````````````````````````` INSTANCE ACCESS EXPR-CALL ``````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// Recursively rewrites the instance-trait access contained within an
/// associated function call.
///
/// Example:
///
/// ```ignore
/// <T as InstanceTrait<...>>::value(...)
/// ```
///
/// The call expression itself is preserved. Its callee is converted into an
/// [`AssocAccessorMut`] and delegated to `F`, allowing the underlying
/// instance-trait access to be resolved according to the transformation
/// implemented by `F`.
///
/// This wrapper provides a general function-call expression adapter for
/// transformations represented by `F`.
#[derive(Debug, Clone, Default)]
pub(crate) struct InstanceDirectAccessExprCall<F>(PhantomData<F>);

impl<'a, F> Transformation<ExprCall, (&BStringList, &IntList)> for InstanceDirectAccessExprCall<F>
where
    F: for<'b, 'c, 'd> Transformation<AssocAccessorMut<'b>, (&'c BStringList, &'d IntList)>
        + Default,
{
    fn raw_transform(
        &self,
        transform: &mut ExprCall,
        context: &(&BStringList, &IntList),
    ) -> Result<(), proc_macro2::TokenStream> {
        let expr = transform;

        for arg in expr.args.iter_mut() {
            let arg_clone = arg.clone();
            *arg = parse_quote!(From::from(#arg_clone));
        }

        let fn_path = &mut *expr.func;

        let mut access_mut =
            AssocAccessorMut::try_from(fn_path).map_err(|e: syn::Error| e.to_compile_error())?;

        F::checked_transform(&F::default(), &mut access_mut, context)?;

        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &ExprCall,
        context: Option<&(&BStringList, &IntList)>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let expr = transform;

        let mut fn_path = *expr.func.clone();

        let mut access_mut = AssocAccessorMut::try_from(&mut fn_path)
            .map_err(|e: syn::Error| e.to_compile_error())?;

        F::validate_transform(&F::default(), &mut access_mut, context)?;

        Ok(())
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ````````````````````````` INSTANCE ACCESS EXPR-TUPLE ``````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// Recursively rewrites every instance-trait access contained within a
/// tuple expression.
///
/// Example:
///
/// ```ignore
/// (
///     <T as InstanceTrait<...>>::A,
///     <U as InstanceTrait<...>>::B,
/// )
/// ```
///
/// Each tuple element is converted into an [`AssocAccessorMut`] and delegated
/// to `F`, allowing every contained instance-trait access to be resolved
/// according to the transformation implemented by `F`.
///
/// This wrapper provides a general tuple-expression adapter for transformations
/// represented by `F`.
#[derive(Debug, Clone, Default)]
pub(crate) struct InstanceDirectAccessExprTuple<F>(PhantomData<F>);

impl<'a, F> Transformation<ExprTuple, (&BStringList, &IntList)> for InstanceDirectAccessExprTuple<F>
where
    F: for<'b, 'c, 'd> Transformation<AssocAccessorMut<'b>, (&'c BStringList, &'d IntList)>
        + Default,
{
    fn raw_transform(
        &self,
        transform: &mut ExprTuple,
        context: &(&BStringList, &IntList),
    ) -> Result<(), proc_macro2::TokenStream> {
        let expr = transform;

        let elems = &mut expr.elems;

        for elem in elems {
            let mut access_mut =
                AssocAccessorMut::try_from(elem).map_err(|e: syn::Error| e.to_compile_error())?;

            F::checked_transform(&F::default(), &mut access_mut, context)?;
        }

        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &ExprTuple,
        context: Option<&(&BStringList, &IntList)>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let expr = transform;

        let elems = expr.elems.clone();

        for mut elem in elems {
            let mut access_mut = AssocAccessorMut::try_from(&mut elem)
                .map_err(|e: syn::Error| e.to_compile_error())?;

            F::validate_transform(&F::default(), &mut access_mut, context)?;
        }
        Ok(())
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ```````````````````````` INSTANCE ACCESS EXPR-REFERENCE ```````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// Recursively rewrites the instance-trait access referenced by a
/// reference expression.
///
/// Example:
///
/// ```ignore
/// &<T as InstanceTrait<...>>::VALUE
/// ```
///
/// The referenced expression is converted into an [`AssocAccessorMut`] and
/// delegated to `F`, allowing the contained instance-trait access to be
/// resolved according to the transformation implemented by `F`.
///
/// This wrapper provides a general reference-expression adapter for
/// transformations represented by `F`.
#[derive(Debug, Clone, Default)]
pub(crate) struct InstanceDirectAccessExprRef<F>(PhantomData<F>);

impl<'a, F> Transformation<ExprReference, (&BStringList, &IntList)>
    for InstanceDirectAccessExprRef<F>
where
    F: for<'b, 'c, 'd> Transformation<AssocAccessorMut<'b>, (&'c BStringList, &'d IntList)>
        + Default,
{
    fn raw_transform(
        &self,
        transform: &mut ExprReference,
        context: &(&BStringList, &IntList),
    ) -> Result<(), proc_macro2::TokenStream> {
        let expr = transform;

        let expr = &mut *expr.expr;

        let mut access_mut =
            AssocAccessorMut::try_from(expr).map_err(|e: syn::Error| e.to_compile_error())?;

        F::checked_transform(&F::default(), &mut access_mut, context)?;

        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &ExprReference,
        context: Option<&(&BStringList, &IntList)>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let expr = transform;

        let mut expr = *expr.expr.clone();

        let mut access_mut =
            AssocAccessorMut::try_from(&mut expr).map_err(|e: syn::Error| e.to_compile_error())?;

        F::validate_transform(&F::default(), &mut access_mut, context)?;
        Ok(())
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ````````````````````````` INSTANCE ACCESS EXPR-RETURN `````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// Recursively rewrites the instance-trait access returned by a
/// `return` expression.
///
/// Example:
///
/// ```ignore
/// return <T as InstanceTrait<...>>::VALUE;
/// ```
///
/// The returned expression is converted into an [`AssocAccessorMut`] and
/// delegated to `F`, allowing the contained instance-trait access to be
/// resolved according to the transformation implemented by `F`.
///
/// This wrapper provides a general `return` expression adapter for
/// transformations represented by `F`.
#[derive(Debug, Clone, Default)]
pub(crate) struct InstanceDirectAccessExprRetn<F>(PhantomData<F>);

impl<'a, F> Transformation<ExprReturn, (&BStringList, &IntList)> for InstanceDirectAccessExprRetn<F>
where
    F: for<'b, 'c, 'd> Transformation<AssocAccessorMut<'b>, (&'c BStringList, &'d IntList)>
        + Default,
{
    fn raw_transform(
        &self,
        transform: &mut ExprReturn,
        context: &(&BStringList, &IntList),
    ) -> Result<(), proc_macro2::TokenStream> {
        let expr = transform;

        let Some(expr) = &mut expr.expr else {
            return Ok(());
        };

        let expr = &mut **expr;

        let mut access_mut =
            AssocAccessorMut::try_from(expr).map_err(|e: syn::Error| e.to_compile_error())?;

        F::checked_transform(&F::default(), &mut access_mut, context)?;

        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &ExprReturn,
        context: Option<&(&BStringList, &IntList)>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let expr = transform;

        let Some(expr) = &expr.expr else {
            return Ok(());
        };

        let mut expr = *expr.clone();

        let mut access_mut =
            AssocAccessorMut::try_from(&mut expr).map_err(|e: syn::Error| e.to_compile_error())?;

        F::validate_transform(&F::default(), &mut access_mut, context)?;
        Ok(())
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ``````````````````````````` INSTANCE ACCESS EXPR-TRY ``````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// Recursively rewrites the instance-trait access wrapped by the `?`
/// operator.
///
/// Example:
///
/// ```ignore
/// <T as InstanceTrait<...>>::value()?
/// ```
///
/// The operand of the try expression is converted into an [`AssocAccessorMut`]
/// and delegated to `F`, allowing the contained instance-trait access to be
/// resolved according to the transformation implemented by `F`.
///
/// This wrapper therefore provides a general try-expression entry point for
/// any transformation that can resolve the contained associated accessor.
#[derive(Debug, Clone, Default)]
pub(crate) struct InstanceDirectAccessExprTry<F>(PhantomData<F>);

impl<'a, F> Transformation<ExprTry, (&BStringList, &IntList)> for InstanceDirectAccessExprTry<F>
where
    F: for<'b, 'c, 'd> Transformation<AssocAccessorMut<'b>, (&'c BStringList, &'d IntList)>
        + Default,
{
    fn raw_transform(
        &self,
        transform: &mut ExprTry,
        context: &(&BStringList, &IntList),
    ) -> Result<(), proc_macro2::TokenStream> {
        let expr = transform;

        let expr = &mut *expr.expr;

        let mut access_mut =
            AssocAccessorMut::try_from(expr).map_err(|e: syn::Error| e.to_compile_error())?;

        F::checked_transform(&F::default(), &mut access_mut, context)?;

        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &ExprTry,
        context: Option<&(&BStringList, &IntList)>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let expr = transform;

        let mut expr = *expr.expr.clone();

        let mut access_mut =
            AssocAccessorMut::try_from(&mut expr).map_err(|e: syn::Error| e.to_compile_error())?;

        F::validate_transform(&F::default(), &mut access_mut, context)?;
        Ok(())
    }
}
