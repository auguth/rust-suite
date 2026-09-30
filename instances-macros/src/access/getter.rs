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
// ```````````````````````````` INSTANCE GETTER ACCESS ```````````````````````````
// ===============================================================================

//! Provides instance-getter transformation for expressions containing
//! instance-trait associated item's inherent associated items access (assoc's assoc).
//!
//! Instance getter transformation converts qualified instance-trait paths into
//! standalone getter paths whose generic arguments contain the information
//! required to resolve the requested instance.
//!
//! For example, an getter access such as:
//!
//! ```ignore
//! <Provider as InstanceTrait<'b, 0, T, 1>>::Assoc::<J>::get::<'a, T>()
//! ```
//!
//! is transformed into a getter representation conceptually of the form:
//!
//! ```ignore
//! get::<'a, 'b, T, Provider, 0, T, 1, J>()
//! ```
//!
//! The qualified self type, and the path generic arguments are incorporated into
//! the generated getter arguments together with any existing getter arguments.
//! This allows the generated getter to carry the complete information required
//! for subsequent instance resolution.
//!
//! For function calls, the getter arguments are additionally
//! appended to the function call as trailing arguments. For example:
//!
//! ```ignore
//! <Provider as InstanceTrait<'b, 0, T, 1>>::Assoc::get::<'a, T>(arg)
//! ```
//!
//! becomes conceptually:
//!
//! ```ignore
//! get::<'a, 'b, T, Provider, 0, T, 1>(arg, ...getter_args)
//! ```
//!
//! These additional arguments are appended only when the associated item is
//! called as a function. Other associated items, such as associated constants
//! and associated types, do not receive appended function arguments because
//! they do not have a function argument list.
//!
//! An associated constant:
//!
//! ```ignore
//! <Provider as InstanceTrait<'b, 0, T, 1>>::Assoc::VALUE
//! ```
//!
//! is transformed only into its getter representation:
//!
//! ```ignore
//! VALUE::<'b, Provider, 0, T, 1>
//! ```
//!
//! and no additional function arguments are appended.
//!
//! Likewise, an associated type:
//!
//! ```ignore
//! <Provider as InstanceTrait<'b, 0, T, 1>>::Assoc<J>::Value<K>
//! ```
//!
//! is transformed only through its getter representation:
//!
//! ```ignore
//! Value::<'b, K, Provider, 0, T, 1, J>
//! ```
//!
//! and does not receive appended arguments because an associated type has no
//! function call on which such arguments could be appended.

// ===============================================================================
// ``````````````````````````````````` IMPORTS ```````````````````````````````````
// ===============================================================================

use std::marker::PhantomData;

// --- Proc Macro Crates ---
use syn::{
    AngleBracketedGenericArguments, Expr, ExprCall, ExprPath, ExprReference, ExprReturn, ExprTry,
    ExprTuple, GenericArgument, PathArguments, parse_quote, punctuated::Punctuated, token::Comma,
};

// --- Local Crate ---
use crate::{
    Transformation,
    access::{
        args::{AssocAccessor, AssocAccessorMut, BStrInputList},
        errors::{GetterBugs, GetterErrors},
    },
};

// ===============================================================================
// ``````````````````````````````` INSTANCE GETTER ```````````````````````````````
// ===============================================================================

/// Performs instance-getter access transformation on an expression.
///
/// The expression is first converted into an [`AssocAccessor`] so that
/// instance-getter access can be resolved independently of its surrounding
/// expression structure. [`InstanceGetterEntry`] then recursively resolves
/// the contained accessor using the supplied dynamic identifier context.
///
/// Direct qualified instance-trait paths are resolved by
/// [`InstanceGetterExprPath`]. Supported surrounding expression forms such as
/// function calls, tuples, references, `return` expressions, and `?`
/// expressions preserve their outer structure while delegating their contained
/// access to the corresponding expression transformation.
///
/// Validation performs the same recursive traversal without modifying the
/// original expression, ensuring that the complete expression can be resolved
/// using the supplied getter context.
#[derive(Debug, Clone)]
pub(crate) struct InstanceGetter;

impl Transformation<Expr, BStrInputList> for InstanceGetter {
    fn raw_transform(
        &self,
        transform: &mut Expr,
        context: &BStrInputList,
    ) -> Result<(), proc_macro2::TokenStream> {
        let mut assoc = match AssocAccessor::try_from(transform.clone()) {
            Ok(a) => a,
            Err(e) => {
                return Err(e.into_compile_error());
            }
        };

        if matches!(assoc, AssocAccessor::Path(_)) && !context.exprs.is_empty() {
            return Err(GetterErrors::RemoveBStrInputs {
                list: context.clone(),
            }
            .into());
        }

        let mut mut_assoc = (&mut assoc).into();

        InstanceGetterEntry::checked_transform(&InstanceGetterEntry, &mut mut_assoc, context)?;

        let new_expr: Expr = parse_quote!(#mut_assoc);

        *transform = new_expr;

        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &Expr,
        context: Option<&BStrInputList>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let mut assoc = match AssocAccessor::try_from(transform.clone()) {
            Ok(a) => a,
            Err(e) => {
                return Err(e.into_compile_error());
            }
        };

        if let Some(context) = context {
            if matches!(assoc, AssocAccessor::Path(_)) && !context.exprs.is_empty() {
                return Err(GetterBugs::NotRemovedBStrInputs {}.into());
            }
        }

        let mut mut_assoc = (&mut assoc).into();

        InstanceGetterEntry::validate_transform(&InstanceGetterEntry, &mut mut_assoc, context)?;

        Ok(())
    }
}

// ===============================================================================
// ````````````````````````` INSTANCE GETTER ENTRY-POINT `````````````````````````
// ===============================================================================

/// Dispatches instance-getter transformation according to the structure of an
/// [`AssocAccessorMut`].
///
/// A qualified instance-trait path is resolved directly by
/// [`InstanceGetterExprPath`]. Function calls, tuples, references, `return`
/// expressions, and `?` expressions are delegated to their corresponding
/// expression adapters, each of which recursively resolves the contained
/// accessor into `F` using the supplied getter context.
///
/// This entry point separates accessor dispatch from the individual
/// expression transformations, allowing the same recursive resolution
/// mechanism to be reused by every supported expression form.
#[derive(Debug, Clone, Default)]
pub(crate) struct InstanceGetterEntry;

impl<'a> Transformation<AssocAccessorMut<'a>, BStrInputList> for InstanceGetterEntry {
    fn raw_transform(
        &self,
        transform: &mut AssocAccessorMut<'a>,
        context: &BStrInputList,
    ) -> Result<(), proc_macro2::TokenStream> {
        match transform {
            AssocAccessorMut::Path(expr_path) => {
                InstanceGetterExprPath::checked_transform(&InstanceGetterExprPath, expr_path, &())?
            }
            AssocAccessorMut::Fn(expr_call) => InstanceGetterExprCall::<Self>::checked_transform(
                &InstanceGetterExprCall::<Self>::default(),
                expr_call,
                context,
            )?,
            AssocAccessorMut::Tuple(expr_tuple) => {
                InstanceGetterExprTuple::<Self>::checked_transform(
                    &InstanceGetterExprTuple::<Self>::default(),
                    expr_tuple,
                    context,
                )?
            }
            AssocAccessorMut::Ref(expr_reference) => {
                InstanceGetterExprRef::<Self>::checked_transform(
                    &InstanceGetterExprRef::<Self>::default(),
                    expr_reference,
                    context,
                )?
            }
            AssocAccessorMut::Return(expr_return) => {
                InstanceGetterExprRetn::<Self>::checked_transform(
                    &InstanceGetterExprRetn::<Self>::default(),
                    expr_return,
                    context,
                )?
            }
            AssocAccessorMut::Try(expr_try) => InstanceGetterExprTry::<Self>::checked_transform(
                &InstanceGetterExprTry::<Self>::default(),
                expr_try,
                context,
            )?,
        }

        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &AssocAccessorMut<'a>,
        context: Option<&BStrInputList>,
    ) -> Result<(), proc_macro2::TokenStream> {
        match transform {
            AssocAccessorMut::Path(expr_path) => InstanceGetterExprPath::validate_transform(
                &InstanceGetterExprPath,
                expr_path,
                None,
            )?,
            AssocAccessorMut::Fn(expr_call) => InstanceGetterExprCall::<Self>::validate_transform(
                &InstanceGetterExprCall::<Self>::default(),
                expr_call,
                context,
            )?,
            AssocAccessorMut::Tuple(expr_tuple) => {
                InstanceGetterExprTuple::<Self>::validate_transform(
                    &InstanceGetterExprTuple::<Self>::default(),
                    expr_tuple,
                    context,
                )?
            }
            AssocAccessorMut::Ref(expr_reference) => {
                InstanceGetterExprRef::<Self>::validate_transform(
                    &InstanceGetterExprRef::<Self>::default(),
                    expr_reference,
                    context,
                )?
            }
            AssocAccessorMut::Return(expr_return) => {
                InstanceGetterExprRetn::<Self>::validate_transform(
                    &InstanceGetterExprRetn::<Self>::default(),
                    expr_return,
                    context,
                )?
            }
            AssocAccessorMut::Try(expr_try) => InstanceGetterExprTry::<Self>::validate_transform(
                &InstanceGetterExprTry::<Self>::default(),
                expr_try,
                context,
            )?,
        }
        Ok(())
    }
}

// ===============================================================================
// `````````````````````````` INSTANCE GETTER EXPR-PATH ``````````````````````````
// ===============================================================================

/// Resolves a qualified getter path into its generated getter representation.
///
/// Example:
///
/// ```ignore
/// // pre
/// <T as InstanceTrait<'a, W>>::Assoc<J>::get::<'b, X>()
///
/// // post
/// get::<'b, 'a, X, T, W, J>()
///
/// ```
///
/// The associated getter or the last-segment of the expr-path (existing)
/// is separated from the preceding trait path, and the path generic arguments
/// and qualified-self type are recovered from the qualified path. The getter's
/// generic arguments are then rebuilt in the required order:
///
/// ```text
/// existing lifetimes,
/// path lifetimes,
/// existing type and const arguments,
/// qualified-self type,
/// path type and const arguments,
/// existing associated arguments,
/// path associated arguments
/// ```
///
/// The qualified path is finally replaced by the generated getter segment,
/// removing the qualified self and retaining the getter as a standalone path.
///
/// Validation confirms that the resulting getter path no longer contains
/// qualified self, contains an associated segment, and uses only supported
/// generic argument syntax.
#[derive(Debug, Clone)]
pub(crate) struct InstanceGetterExprPath;

impl Transformation<ExprPath> for InstanceGetterExprPath {
    fn raw_transform(
        &self,
        expr_path: &mut ExprPath,
        _: &(),
    ) -> Result<(), proc_macro2::TokenStream> {
        let Some(qself) = &expr_path.qself else {
            return Err(GetterErrors::PathRequiresQSelf {
                expr_path: expr_path.clone(),
            }
            .into());
        };

        let mut trait_path = expr_path.path.clone();

        let Some(_) = trait_path.segments.pop() else {
            return Err(GetterErrors::PathRequiresQSelf {
                expr_path: expr_path.clone(),
            }
            .into());
        };

        let Some(_) = trait_path.segments.last() else {
            return Err(GetterErrors::PathRequiresQSelfTrait { path: trait_path }.into());
        };

        let mut trait_args: Punctuated<&GenericArgument, Comma> = Punctuated::new();
        for seg in &trait_path.segments {
            match &seg.arguments {
                PathArguments::None => {}
                PathArguments::AngleBracketed(args) => {
                    for arg in &args.args {
                        trait_args.push(arg);
                    }
                }
                PathArguments::Parenthesized(_) => {
                    return Err(GetterErrors::TraitHasParenArgs {
                        args: seg.arguments.clone(),
                    }
                    .into());
                }
            };
        }

        let qself_ty = (*qself.ty).clone();

        let Some(last_segment) = expr_path.path.segments.last() else {
            return Err(GetterErrors::PathRequiresAssoc { path: trait_path }.into());
        };

        let mut last_segment = last_segment.clone();

        let existing = match &last_segment.arguments {
            PathArguments::None => Punctuated::new(),

            PathArguments::AngleBracketed(args) => args.args.clone(),

            PathArguments::Parenthesized(_) => {
                return Err(GetterErrors::AssocHasParenArgs {
                    args: last_segment.arguments.clone(),
                }
                .into());
            }
        };

        let mut merged = syn::punctuated::Punctuated::new();

        // Existing getter lifetimes.
        for arg in &existing {
            if matches!(arg, GenericArgument::Lifetime(_)) {
                merged.push(arg.clone());
            }
        }

        // Trait lifetimes.
        for &arg in trait_args.iter() {
            if matches!(arg, GenericArgument::Lifetime(_)) {
                merged.push(arg.clone());
            }
        }

        // Existing getter non-lifetime arguments.
        for arg in &existing {
            if matches!(arg, GenericArgument::Type(_) | GenericArgument::Const(_)) {
                merged.push(arg.clone());
            }
        }

        // QSelf as first non-lifetime arguments.
        merged.push(GenericArgument::Type(qself_ty));

        // Trait non-lifetime arguments.
        for &arg in trait_args.iter() {
            if matches!(arg, GenericArgument::Type(_) | GenericArgument::Const(_)) {
                merged.push(arg.clone());
            }
        }

        // Existing getter Assoc arguments.
        for arg in &existing {
            if matches!(
                arg,
                GenericArgument::AssocConst(_)
                    | GenericArgument::AssocType(_)
                    | GenericArgument::Constraint(_)
            ) {
                merged.push(arg.clone());
            }
        }

        // Trait Assoc arguments.
        for &arg in trait_args.iter() {
            if matches!(
                arg,
                GenericArgument::AssocConst(_)
                    | GenericArgument::AssocType(_)
                    | GenericArgument::Constraint(_)
            ) {
                merged.push(arg.clone());
            }
        }

        last_segment.arguments = PathArguments::AngleBracketed(AngleBracketedGenericArguments {
            colon2_token: None,
            lt_token: Default::default(),
            args: merged,
            gt_token: Default::default(),
        });

        *expr_path = ExprPath {
            attrs: Default::default(),
            qself: None,
            path: parse_quote!(#last_segment),
        };

        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &ExprPath,
        _: Option<&()>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let expr_path = transform;

        if expr_path.qself.is_some() {
            return Err(GetterBugs::PathHasQSelf {}.into());
        };

        let Some(last_segment) = expr_path.path.segments.last() else {
            return Err(GetterBugs::PathHasNoAssoc {}.into());
        };

        match &last_segment.arguments {
            PathArguments::AngleBracketed(_) | PathArguments::None => {}
            PathArguments::Parenthesized(_) => {
                return Err(GetterBugs::PathHasParenArgs {}.into());
            }
        };

        Ok(())
    }
}

// ===============================================================================
// `````````````````````````` INSTANCE GETTER EXPR-CALL ``````````````````````````
// ===============================================================================

/// Recursively resolves instance-getter access contained within a function
/// call into `F` using the supplied getter context.
///
/// Example:
///
/// ```ignore
/// // context: [expr1, ident2]
/// // result:
/// <T as InstanceTrait<...>>::get(...expr1, ident2)
/// ```
///
/// The function call itself is preserved. Its callee is converted into an
/// [`AssocAccessorMut`] and delegated to `F`, allowing the underlying
/// instance-getter access to be resolved according to the transformation
/// implemented by `F`.
///
/// After the callee has been resolved, the getter context is appended to the
/// call arguments. This allows dynamic getter identifiers to be passed through
/// the generated function call while preserving the original call arguments.
#[derive(Debug, Clone, Default)]
pub(crate) struct InstanceGetterExprCall<F>(PhantomData<F>);

impl<'a, F> Transformation<ExprCall, BStrInputList> for InstanceGetterExprCall<F>
where
    F: for<'b> Transformation<AssocAccessorMut<'b>, BStrInputList> + Default,
{
    fn raw_transform(
        &self,
        transform: &mut ExprCall,
        context: &BStrInputList,
    ) -> Result<(), proc_macro2::TokenStream> {
        let expr = transform;

        let fn_path = &mut *expr.func;

        let mut access_mut =
            AssocAccessorMut::try_from(fn_path).map_err(|e: syn::Error| e.to_compile_error())?;

        F::checked_transform(&F::default(), &mut access_mut, context)?;

        let args = &mut expr.args;

        for inp in &context.exprs {
            args.push(parse_quote!(#inp));
        }

        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &ExprCall,
        context: Option<&BStrInputList>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let expr = transform;

        let mut fn_path = *expr.func.clone();

        let mut access_mut = AssocAccessorMut::try_from(&mut fn_path)
            .map_err(|e: syn::Error| e.to_compile_error())?;

        F::validate_transform(&F::default(), &mut access_mut, context)?;

        let Some(context) = context else {
            return Ok(());
        };

        let mut collect = Vec::<Expr>::new();

        for inp in &context.exprs {
            collect.push(parse_quote!(#inp))
        }

        let args = expr.args.iter().rev();

        for (exp, found) in collect.iter().rev().zip(args) {
            if exp != found {
                return Err(GetterBugs::PathHasParenArgs {}.into());
            }
        }

        Ok(())
    }
}

// ===============================================================================
// ````````````````````````` ALLOWED EXPR-PATH CONSUMERS `````````````````````````
// ===============================================================================

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ````````````````````````` INSTANCE GETTER EXPR-TUPLE ``````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// Recursively resolves every instance-getter access contained within a tuple
/// expression into `F` using the supplied getter context.
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
/// Each tuple element is independently converted into an [`AssocAccessorMut`]
/// and delegated to `F`, allowing every contained instance-getter access to be
/// resolved while preserving the tuple expression itself.
///
/// This wrapper provides a general tuple-expression adapter for transformations
/// represented by `F` with the getter context supplied to each element.
///
/// Validation performs the same traversal over each tuple element without
/// modifying the original expression.
#[derive(Debug, Clone, Default)]
pub(crate) struct InstanceGetterExprTuple<F>(PhantomData<F>);

impl<'a, F> Transformation<ExprTuple, BStrInputList> for InstanceGetterExprTuple<F>
where
    F: for<'b> Transformation<AssocAccessorMut<'b>, BStrInputList> + Default,
{
    fn raw_transform(
        &self,
        transform: &mut ExprTuple,
        context: &BStrInputList,
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
        context: Option<&BStrInputList>,
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
// ```````````````````````` INSTANCE GETTER EXPR-REFERENCE ```````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// Recursively resolves the instance-getter access contained within a
/// reference expression into `F` using the supplied getter context.
///
/// Example:
///
/// ```ignore
/// &<T as InstanceTrait<...>>::VALUE
/// ```
///
/// The referenced expression is converted into an [`AssocAccessorMut`] and
/// delegated to `F`, allowing the contained instance-getter access to be
/// resolved according to the transformation implemented by `F`.
///
/// The reference expression itself is preserved while only its referenced
/// accessor is transformed.
#[derive(Debug, Clone, Default)]
pub(crate) struct InstanceGetterExprRef<F>(PhantomData<F>);

impl<'a, F> Transformation<ExprReference, BStrInputList> for InstanceGetterExprRef<F>
where
    F: for<'b> Transformation<AssocAccessorMut<'b>, BStrInputList> + Default,
{
    fn raw_transform(
        &self,
        transform: &mut ExprReference,
        context: &BStrInputList,
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
        context: Option<&BStrInputList>,
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
// ````````````````````````` INSTANCE GETTER EXPR-RETURN `````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// Recursively resolves the instance-getter access returned by a `return`
/// expression into `F` using the supplied getter context.
///
/// Example:
///
/// ```ignore
/// return <T as InstanceTrait<...>>::VALUE;
/// ```
///
/// When a return expression contains a value, that value is converted into an
/// [`AssocAccessorMut`] and delegated to `F`, allowing the contained
/// instance-getter access to be resolved while preserving the surrounding
/// `return` expression.
///
/// A `return` expression without a value requires no transformation.
#[derive(Debug, Clone, Default)]
pub(crate) struct InstanceGetterExprRetn<F>(PhantomData<F>);

impl<'a, F> Transformation<ExprReturn, BStrInputList> for InstanceGetterExprRetn<F>
where
    F: for<'b> Transformation<AssocAccessorMut<'b>, BStrInputList> + Default,
{
    fn raw_transform(
        &self,
        transform: &mut ExprReturn,
        context: &BStrInputList,
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
        context: Option<&BStrInputList>,
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
// ``````````````````````````` INSTANCE GETTER EXPR-TRY ``````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// Recursively resolves the instance-getter access wrapped by the `?`
/// operator into `F` using the supplied getter context.
///
/// Example:
///
/// ```ignore
/// <T as InstanceTrait<...>>::get()?
/// ```
///
/// The operand of the try expression is converted into an [`AssocAccessorMut`]
/// and delegated to `F`, allowing the contained instance-getter access to be
/// resolved according to the transformation implemented by `F`.
///
/// The try expression itself is preserved while its operand is recursively
/// resolved.
#[derive(Debug, Clone, Default)]
pub(crate) struct InstanceGetterExprTry<F>(PhantomData<F>);

impl<'a, F> Transformation<ExprTry, BStrInputList> for InstanceGetterExprTry<F>
where
    F: for<'b> Transformation<AssocAccessorMut<'b>, BStrInputList> + Default,
{
    fn raw_transform(
        &self,
        transform: &mut ExprTry,
        context: &BStrInputList,
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
        context: Option<&BStrInputList>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let expr = transform;

        let mut expr = *expr.expr.clone();

        let mut access_mut =
            AssocAccessorMut::try_from(&mut expr).map_err(|e: syn::Error| e.to_compile_error())?;

        F::validate_transform(&F::default(), &mut access_mut, context)?;
        Ok(())
    }
}
