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
// ``````````````````````````` INSTANCE IMPL UTILITIES ```````````````````````````
// ===============================================================================

//! Shared extraction utilities and validation helpers used throughout the
//! instance-impl proc-macro pipeline.

// ===============================================================================
// ``````````````````````````````````` IMPORTS ```````````````````````````````````
// ===============================================================================

// --- Std Crate ---
use std::{fmt::Debug, marker::PhantomData};

// --- Proc Macro Crates ---
use proc_macro::TokenStream;
use quote::format_ident;
use syn::{
    Expr, GenericArgument, Ident, ImplItem, ImplItemConst, ItemImpl, Path, PathArguments,
    PathSegment, Type, parse_quote, punctuated::Punctuated, token::Comma,
};

// --- Local Crates ---
use crate::{
    Extraction, Utilization,
    impls::{
        counters::*,
        errors::{UtilityBugs, UtilityErrors},
    },
    traits::{affiliates::*, meta::*},
};

// --- Proc-Suite ---
use proc_suite::misc::*;

// ===============================================================================
// ``````````````````````````````` IMPL UTILIZATION ``````````````````````````````
// ===============================================================================

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ````````````````````````` IMPL TRAIT PATH UTILIZATION `````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// Utilizable view of the implemented trait path.
///
/// Example:
/// ```ignore
/// impl crate_of::Example<u8, 4, T, 7> for MyType {}
/// //   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
/// ```
#[derive(Debug)]
pub(crate) struct ImplTraitPath<'a>(pub(crate) &'a Path);

impl<'a> Utilization<'a, ItemImpl> for ImplTraitPath<'a> {
    fn raw_utilize(from: &'a ItemImpl, _: &'a ()) -> Result<Self, proc_macro2::TokenStream> {
        let Some((neg, trait_path, _)) = &from.trait_ else {
            return Err(UtilityErrors::NotTraitImpl {
                impl_of: from.clone(),
            }
            .into());
        };
        if neg.is_some() {
            return Err(UtilityErrors::NegativeTraitsNotApplicable {
                trait_path: trait_path.clone(),
            }
            .into());
        }
        Ok(Self(trait_path))
    }

    fn validate_utilize(
        &self,
        _: &ItemImpl,
        _: Option<&()>,
    ) -> Result<(), proc_macro2::TokenStream> {
        Ok(())
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ``````````````````````` IMPL TRAIT SEGMENT UTILIZATION ````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// Utilizable view of the implemented trait's final path segment including
/// its arguments.
///
/// Example:
/// ```ignore
/// impl crate_of::Example<u8, 4, T, 7> for MyType {}
/// //             ^^^^^^^^^^^^^^^^^^^^
/// ```
#[derive(Debug)]
pub(crate) struct ImplTraitSegment<'a>(pub(crate) &'a PathSegment);

impl<'a> Utilization<'a, ItemImpl> for ImplTraitSegment<'a> {
    fn raw_utilize(from: &'a ItemImpl, context: &'a ()) -> Result<Self, proc_macro2::TokenStream> {
        let path = ImplTraitPath::checked_utilize(from, context)?.0;
        // Infallible: since we received a valid trait path
        // hence we know that a valid non-inherent impl block parsed
        // Although real Rust code always yields a nonempty path,
        // `syn` allows empty segment lists (for interoperability
        // with other similar segmented syntax nodes)
        let Some(last) = path.segments.last() else {
            return Err(UtilityErrors::TraitPathLastSegmentNotFound {
                trait_path: path.clone(),
            }
            .into());
        };
        Ok(Self(last))
    }

    fn validate_utilize(
        &self,
        _: &ItemImpl,
        _: Option<&()>,
    ) -> Result<(), proc_macro2::TokenStream> {
        Ok(())
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ````````````````````````` IMPL TRAIT IDENT UTILIZATION ````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// Utilizable view of the implemented trait identifier.
///
/// Example:
/// ```ignore
/// impl crate_of::Example<u8, 4, T, 7> for MyType {}
/// //             ^^^^^^^
/// ```
#[derive(Debug)]
pub(crate) struct ImplTraitIdent<'a>(pub(crate) &'a Ident);

impl<'a> Utilization<'a, ItemImpl> for ImplTraitIdent<'a> {
    fn raw_utilize(from: &'a ItemImpl, context: &'a ()) -> Result<Self, proc_macro2::TokenStream> {
        let segment = ImplTraitSegment::checked_utilize(from, context)?.0;
        let ident = &segment.ident;
        Ok(Self(ident))
    }

    fn validate_utilize(
        &self,
        _: &ItemImpl,
        _: Option<&()>,
    ) -> Result<(), proc_macro2::TokenStream> {
        Ok(())
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// `````````````````````````` IMPL TRAIT ARGS UTILIZATION `````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// Utilizable view of the implemented trait generic arguments.
///
/// Example:
/// ```ignore
/// impl crate_of::Example<u8, 4, T, 7> for MyType {}
/// //                    ^^^^^^^^^^^^^
/// ```
#[derive(Debug)]
pub(crate) struct ImplTraitGenArgs<'a>(pub(crate) &'a Punctuated<GenericArgument, Comma>);

impl<'a> Utilization<'a, ItemImpl> for ImplTraitGenArgs<'a> {
    fn raw_utilize(from: &'a ItemImpl, context: &'a ()) -> Result<Self, proc_macro2::TokenStream> {
        let segment = ImplTraitSegment::checked_utilize(from, context)?.0;
        let PathArguments::AngleBracketed(angle_args) = &segment.arguments else {
            return Err(UtilityErrors::FunctionTraitsNotApplicable {
                segment: segment.clone(),
            }
            .into());
        };
        Ok(Self(&angle_args.args))
    }

    fn validate_utilize(
        &self,
        _: &ItemImpl,
        _: Option<&()>,
    ) -> Result<(), proc_macro2::TokenStream> {
        Ok(())
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ````````````````````````` IMPL CONST ITEM UTILIZATION `````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// Utilizable view of a const associated type of given ident.
///
/// Example:
/// ```ignore
/// impl crate_of::Example<u8, 4, T, 7> for MyType {
///     const GIVEN_IDENT: () = ();
/// //  ^^^^^^^^^^^^^^^^^^^^^^^^^^^
/// }
/// ```
#[derive(Debug)]
pub(crate) struct ImplConstItem<'a>(pub(crate) &'a ImplItemConst);

impl<'a> Utilization<'a, ItemImpl, Ident> for ImplConstItem<'a> {
    fn raw_utilize(
        from: &'a ItemImpl,
        context: &'a Ident,
    ) -> Result<Self, proc_macro2::TokenStream> {
        for item in &from.items {
            if let ImplItem::Const(c) = item {
                if c.ident == *context {
                    return Ok(Self(c));
                }
            }
        }
        return Err(UtilityErrors::ExpectedConstAssocItemNotFound {
            exp: context.clone(),
        }
        .into());
    }

    fn validate_utilize(
        &self,
        _: &ItemImpl,
        _: Option<&Ident>,
    ) -> Result<(), proc_macro2::TokenStream> {
        Ok(())
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ````````````````````````````` UFCS TYPE PROJECTION ````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// Builds a nested UFCS (Universal Function Call Syntax) associated-type projection expression.
///
/// Example:
///
/// ```ignore
/// (Self, MyTrait<Counters>, AssocTy)
/// ```
///
/// produces:
///
/// ```ignore
/// <Self as MyTrait<Counters>>::AssocTy
/// ```
///
/// Example:
///
/// ```ignore
/// (Self, MyTrait<Counters>, AssocTy)
/// [(OtherTy, OtherTrait)]
/// ```
///
/// produces:
///
/// ```ignore
/// <<Self as MyTrait<Counters>>::OtherTy as OtherTrait>
///     ::AssocTy
/// ```
///
/// Example:
///
/// ```ignore
/// (Self, MyTrait<Counters>, AssocTy)
/// [
///     (Ty0, Trait0),
///     (Ty1, Trait1),
/// ]
/// ```
///
/// produces:
///
/// ```ignore
/// <<<Self as MyTrait<Counters>>::Ty0 as Trait0>
///     ::Ty1 as Trait1>
///     ::AssocTy
/// ```
///
/// Each `(associated_type, trait_path)` pair extends the current UFCS
/// projection by:
///
/// ```ignore
/// <current::associated_type as trait_path>
/// ```
#[derive(Debug, Clone)]
pub(crate) struct UFCSTypeExpr(pub(crate) Expr);

impl Extraction<(&Type, &Path, &Ident), &[(&Ident, &Path)]> for UFCSTypeExpr {
    fn raw_extract(
        from: &(&Type, &Path, &Ident),
        context: &&[(&Ident, &Path)],
    ) -> Result<Self, proc_macro2::TokenStream> {
        let self_ty = from.0;
        let base_trait_path = from.1;
        let final_ty = from.2;

        let mut ts = quote::quote! {
            <#self_ty as #base_trait_path>
        };

        for (assoc_ident, trait_path) in context.iter() {
            ts = quote::quote! {
                <#ts::#assoc_ident as #trait_path>
            };
        }

        ts = quote::quote!( #ts::#final_ty );

        // Parse once at the end
        let expr: Expr = match syn::parse2(ts) {
            Ok(e) => e,
            Err(e) => return Err(e.to_compile_error().into()),
        };

        Ok(Self(expr))
    }

    fn validate_extract(
        &self,
        _: &(&Type, &Path, &Ident),
        _: Option<&&[(&Ident, &Path)]>,
    ) -> Result<(), proc_macro2::TokenStream> {
        Ok(())
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// `````````````````````````` ASSOCIATED TYPE PROJECTION `````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// Builds a fully-qualified associated-type projection for the
/// implemented trait.
///
/// Example:
///
/// ```ignore
/// impl Example<Counters> for MyType {}
/// ```
///
/// extracting:
///
/// ```ignore
/// AssocTy
/// ```
///
/// produces:
///
/// ```ignore
/// <MyType as Example<Counters>>::AssocTy
/// ```
///
/// Fully-qualified syntax is required because the implementation type
/// may implement the same trait multiple times (instances) with different counter
/// tuples, making unqualified associated-type access `Self::*` ambiguous.
#[derive(Debug, Clone)]
pub(crate) struct AssocTyExpr(pub(crate) Expr);

impl Extraction<(&ItemImpl, &Ident)> for AssocTyExpr {
    fn raw_extract(from: &(&ItemImpl, &Ident), _: &()) -> Result<Self, proc_macro2::TokenStream> {
        let impl_of = from.0;
        let ident = from.1;

        let self_ty = impl_of.self_ty.clone();
        let base_trait_path = ImplTraitPath::checked_utilize(impl_of, &())?.0;

        let empty: &[(&Ident, &Path)] = &[];
        let expr = UFCSTypeExpr::checked_extract(&(&*self_ty, base_trait_path, ident), &empty)?.0;

        Ok(Self(expr))
    }

    fn validate_extract(
        &self,
        _: &(&ItemImpl, &Ident),
        _: Option<&()>,
    ) -> Result<(), proc_macro2::TokenStream> {
        Ok(())
    }
}

// ===============================================================================
// ```````````````````` ASSOCIATE OF AFFILIATE TYPE PROJECTION ```````````````````
// ===============================================================================

/// Builds a fully-qualified associated-type projection from an
/// affiliated trait instance.
///
/// Example:
///
/// ```ignore
/// impl Example<(U0, U0, U5)> for MyType {}
/// ```
///
/// extracting:
///
/// ```ignore
/// AssocTy
/// ```
///
/// with:
///
/// ```ignore
/// Instance = ReverseAffiliateInstance
/// Counter  = ReverseAffiliateCounters
/// ```
///
/// produces:
///
/// ```ignore
/// <
///     <MyType as Example<(U0, U0, U5)>>::ReverseAffiliateInstance
///         as Example<
///             <MyType as Example<(U0, U0, U5)>>::ReverseAffiliateCounters
///         >
/// >::AssocTy
/// ```
///
/// This allows associated items to be projected from affiliated trait
/// instances while preserving fully-qualified syntax.
///
/// ```ignore
/// AffiliateInstance::AssocTy
/// ```
///
/// but without ambiguity when multiple trait instantiations exist for
/// the same implementation type.
///
/// The implemented trait's typenum counter parameter is replaced with
/// the affiliate counter projection before constructing the final UFCS
/// expression.
///
/// Only valid after instance-counter generics have been transformed
/// into typenum-based counters.
#[derive(Debug, Clone)]
pub(crate) struct AssocOfAffiliateTyExpr<
    Instance: 'static + Clone + Debug,
    Counter: 'static + Clone + Debug,
>(pub(crate) Expr, PhantomData<(Instance, Counter)>);

impl<'a, Instance: 'static + Clone + Debug, Counter: 'static + Clone + Debug>
    Extraction<(&ItemImpl, &Ident), CounterArgsSlice<'a>>
    for AssocOfAffiliateTyExpr<Instance, Counter>
{
    fn raw_extract(
        from: &(&ItemImpl, &Ident),
        context: &CounterArgsSlice<'a>,
    ) -> Result<Self, proc_macro2::TokenStream> {
        let impl_of = from.0;
        let ident = from.1;

        let instance_ident = gen_type_ident::<Instance>();
        let instance_expr = AssocTyExpr::checked_extract(&(impl_of, &instance_ident), &())?.0;
        let instance_ty: Type = parse_quote!(#instance_expr);

        let counters_ident = gen_type_ident::<Counter>();
        let counter_expr = AssocTyExpr::checked_extract(&(impl_of, &counters_ident), &())?.0;
        let counter_ty: Type = parse_quote!(#counter_expr);

        let mut trait_path = ImplTraitPath::checked_utilize(impl_of, &())?.0.clone();

        let Some(path_last) = trait_path.segments.last_mut() else {
            return Err(UtilityErrors::TraitPathLastSegmentNotFound { trait_path }.into());
        };

        let PathArguments::AngleBracketed(angle) = &mut path_last.arguments else {
            return Err(UtilityErrors::FunctionTraitsNotApplicable {
                segment: path_last.clone(),
            }
            .into());
        };

        let Some(first_counter) = context.first() else {
            return Err(UtilityBugs::CounterArgsEmptyForFirstCounterAccess {}.into());
        };

        let Some(typenum_gen) = angle.args.get_mut(first_counter.generic_index) else {
            return Err(UtilityBugs::TypeNumGenericCountersNotReplacedYet {}.into());
        };

        *typenum_gen = parse_quote!(#counter_ty);

        let expr = parse_quote!(
            <#instance_ty as #trait_path>::#ident
        );

        Ok(Self(expr, PhantomData))
    }

    fn validate_extract(
        &self,
        _: &(&ItemImpl, &Ident),
        _: Option<&CounterArgsSlice<'a>>,
    ) -> Result<(), proc_macro2::TokenStream> {
        Ok(())
    }
}

// ===============================================================================
// ````````````````````````` CURRENT COUNTER UTILIZATION `````````````````````````
// ===============================================================================

/// Utilizable view of the current typenum counter tuple of the
/// implemented trait instance.
///
/// Example:
///
/// ```ignore
/// impl Example<(U0, U1, U5), Other> for MyType {}
/// //           ^^^^^^^^^^^^
/// ```
#[derive(Debug)]
pub(crate) struct ImplCountersTypeNum<'a>(pub(crate) &'a Type);

impl<'a> Utilization<'a, ItemImpl, CounterArgsSlice<'a>> for ImplCountersTypeNum<'a> {
    fn raw_utilize(
        from: &'a ItemImpl,
        context: &'a CounterArgsSlice<'a>,
    ) -> Result<Self, proc_macro2::TokenStream> {
        let args = ImplTraitGenArgs::checked_utilize(from, &())?.0;
        let Some(first_counter) = context.first() else {
            return Err(UtilityBugs::CounterArgsEmptyForFirstCounterAccess {}.into());
        };
        let Some(arg) = args.get(first_counter.generic_index) else {
            return Err(UtilityBugs::TypeNumGenericCountersNotReplacedYet {}.into());
        };
        let GenericArgument::Type(ty) = arg else {
            return Err(UtilityBugs::CountersGenericIsNotATypeGeneric {}.into());
        };
        Ok(Self(ty))
    }

    fn validate_utilize(
        &self,
        _: &ItemImpl,
        _: Option<&CounterArgsSlice<'a>>,
    ) -> Result<(), proc_macro2::TokenStream> {
        Ok(())
    }
}

// ===============================================================================
// ``````````````````````````` COUNTER TYPE UTILIZATION ``````````````````````````
// ===============================================================================

/// Utilizable view of the implementation's declared counter type.
///
/// Example:
///
/// ```ignore
/// impl Example<(U0, U1, U5)> for MyType {
///     type CounterTy = u8;
/// //                   ^^
/// }
/// ```
#[derive(Debug)]
pub(crate) struct ImplCountersUTy<'a>(pub(crate) &'a Type);

impl<'a> Utilization<'a, ItemImpl> for ImplCountersUTy<'a> {
    fn raw_utilize(from: &'a ItemImpl, _: &'a ()) -> Result<Self, proc_macro2::TokenStream> {
        let ident = format_ident!("{INSTANCE_COUNTER_TY}");
        for item in &from.items {
            let ImplItem::Type(t) = item else {
                continue;
            };

            if ident == t.ident {
                return Ok(Self(&t.ty));
            }
        }
        let trait_ident = ImplTraitIdent::checked_utilize(from, &())?.0.clone();
        return Err(UtilityErrors::CountersTyNotFound {
            exp: ident.clone(),
            trait_ident,
            impl_of: from.clone(),
        }
        .into());
    }

    fn validate_utilize(
        &self,
        _: &ItemImpl,
        _: Option<&()>,
    ) -> Result<(), proc_macro2::TokenStream> {
        Ok(())
    }
}

// ===============================================================================
// ``````````````````` ALL AFFILIATE INSTANCE IDENTS EXTRACTION ``````````````````
// ===============================================================================

/// Extracts all generated affiliate-instance associated-type
/// identifiers (deterministically random).
///
/// Produces:
///
/// ```ignore
/// [
///     ReverseAffiliateInstance,
///     CeilAffiliateInstance,
///     NextAffiliateInstance,
///     BackAffiliateInstance,
///     FloorAffiliatesInstances, // -> for each counter's floor
/// ]
/// ```
///
/// These identifiers correspond to the affiliate-instance projections
/// generated by trait-side affiliate expansion.
#[derive(Debug, Clone)]
pub(crate) struct AllAffiliateInstanceIdents(pub(crate) Vec<Ident>);

impl<'a> Extraction<CounterArgsSlice<'a>> for AllAffiliateInstanceIdents {
    fn raw_extract(from: &CounterArgsSlice<'a>, _: &()) -> Result<Self, proc_macro2::TokenStream> {
        let mut collect_tys = Vec::new();

        collect_tys.push(gen_type_ident::<ReverseAffiliateInstance>());
        collect_tys.push(gen_type_ident::<CeilAffiliateInstance>());
        collect_tys.push(gen_type_ident::<NextAffiliateInstance>());
        collect_tys.push(gen_type_ident::<BackAffiliateInstance>());
        for c in from.iter() {
            collect_tys.push(gen_type_ident_with_suffix::<FloorAffiliatesInstances>(
                Some(c.generic_index.to_string().as_bytes()),
            ));
        }

        Ok(Self(collect_tys))
    }

    fn validate_extract(
        &self,
        _: &CounterArgsSlice<'a>,
        _: Option<&()>,
    ) -> Result<(), proc_macro2::TokenStream> {
        Ok(())
    }
}

// ===============================================================================
// `````````````````````````````` UTILITY FUNCTIONS ``````````````````````````````
// ===============================================================================

/// Checks if the given counters arguments literals are all zero, `false` otherwise.
///
/// Idicates the very first instance implementation.
///
/// Also known as
///     - global minimum
///     - zeroth instance
pub(crate) fn is_zeroth_instance<'a>(counters: CounterArgsSlice<'a>) -> Result<bool, TokenStream> {
    for c in counters.iter() {
        if parse_pos_usize(&c.const_lit)? != 0 {
            return Ok(false);
        }
    }
    Ok(true)
}

// ===============================================================================
// `````````````````````````````` CONVINIENCE MACROS `````````````````````````````
// ===============================================================================

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ``````````````````````````` IMPL CONST (VALIDATION) ```````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// Validates a single associated constant against an expected identifier,
/// type, generic configuration, and expected expression.
macro_rules! validate_impl_const {
    (
        item: $item:expr,
        towards: $towards:expr,

        ident: $ident:expr,
        ty: $expected_ty:expr,

        expr: $expected_expr:expr,

        errors: {
            not_found: $not_found:expr,
            wrong_ident: $wrong_ident:expr,
            wrong_ty: $wrong_ty:expr,
            has_generics: $has_generics:expr,
            invalid_expr: $invalid_expr:expr $(,)?
        } $(,)?
    ) => {{
        let validate_const = |const_item: &syn::ImplItemConst| -> Result<(), TokenStream> {
            if const_item.ident != $ident {
                return Err($wrong_ident.into());
            }

            if const_item.ty.to_token_stream().to_string()
                != $expected_ty.to_token_stream().to_string()
            {
                return Err($wrong_ty.into());
            }

            if !const_item.generics.params.is_empty() {
                return Err($has_generics.into());
            }

            if const_item.expr.to_token_stream().to_string()
                != $expected_expr.to_token_stream().to_string()
            {
                return Err($invalid_expr.into());
            }

            Ok(())
        };

        if let Some(item) = $item {
            validate_const(item)?;
        }

        let mut found: Result<(), TokenStream> = Err($not_found.into());

        for i in &$towards.items {
            if let syn::ImplItem::Const(c) = i {
                if c.ident == $ident {
                    validate_const(c)?;
                    found = Ok(());
                    break;
                }
            }
        }

        found
    }};
}

/// Validates a collection of associated constants against an expected
/// associated-constant.
///
/// Unlike [`validate_impl_const`], this helper operates on collections of
/// generated constants before validating their presence on the impl.
macro_rules! validate_impl_consts {
    (
        items: $items:expr,
        towards: $towards:expr,

        ident: $ident:expr,
        ty: $expected_ty:expr,

        expr: $expected_expr:expr,

        errors: {
            not_found: $not_found:expr,
            wrong_ident: $wrong_ident:expr,
            wrong_ty: $wrong_ty:expr,
            has_generics: $has_generics:expr,
            invalid_expr: $invalid_expr:expr $(,)?
        } $(,)?
    ) => {{
        let validate_const = |const_item: &syn::ImplItemConst| -> Result<(), TokenStream> {
            if const_item.ident != $ident {
                return Err($wrong_ident.into());
            }

            if const_item.ty.to_token_stream().to_string()
                != $expected_ty.to_token_stream().to_string()
            {
                return Err($wrong_ty.into());
            }

            if !const_item.generics.params.is_empty() {
                return Err($has_generics.into());
            }

            if const_item.expr.to_token_stream().to_string()
                != $expected_expr.to_token_stream().to_string()
            {
                return Err($invalid_expr.into());
            }

            Ok(())
        };

        if let Some(vec) = $items {
            let mut item_found: Result<(), TokenStream> = Err($not_found.into());
            for item in vec {
                if item.ident == $ident {
                    validate_const(item)?;
                    item_found = Ok(());
                    break;
                }
            }
            item_found?;
        }

        let mut trait_item_found: Result<(), TokenStream> = Err($not_found.into());

        for i in &$towards.items {
            if let syn::ImplItem::Const(c) = i {
                if c.ident == $ident {
                    validate_const(c)?;
                    trait_item_found = Ok(());
                    break;
                }
            }
        }

        trait_item_found?;
    }};
}

pub(crate) use validate_impl_const;
pub(crate) use validate_impl_consts;

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ```````````````````````````` IMPL TYPE (VALIDATION) ```````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// Validates a single associated type against an expected identifier,
/// generic-parameter configuration, and expected resolved type.
macro_rules! validate_impl_type {
    (
        item: $item:expr,
        towards: $towards:expr,

        ident: $expected_ident:expr,

        ty: $expected_ty:expr,

        errors: {
            not_found: $not_found:expr,
            wrong_ident: $wrong_ident:expr,
            wrong_ty: $wrong_ty:expr,
            has_generics: $has_generics:expr,
        } $(,)?
    ) => {{
        let validate_type = |item: &syn::ImplItemType| -> Result<(), TokenStream> {
            if item.ident != $expected_ident {
                return Err($wrong_ident.into());
            }

            if !item.generics.params.is_empty() {
                return Err($has_generics.into());
            }

            if item.ty != $expected_ty {
                return Err($wrong_ty.into());
            }

            Ok(())
        };

        if let Some(item) = $item {
            validate_type(item)?;
        }

        let mut found: Result<(), TokenStream> = Err($not_found.into());

        for i in &$towards.items {
            let syn::ImplItem::Type(item) = i else {
                continue;
            };

            if item.ident != $expected_ident {
                continue;
            }

            validate_type(item)?;
            found = Ok(());
            break;
        }

        found
    }};
}

/// Validates a collection of associated types against an expected
/// associated-type.
///
/// Unlike [`validate_impl_type`], this helper operates on collections of
/// generated associated types before validating their presence on the impl.
macro_rules! validate_impl_types {
    (
        items: $items:expr,
        towards: $towards:expr,

        ident: $expected_ident:expr,

        ty: $expected_ty:expr,

        errors: {
            not_found: $not_found:expr,
            wrong_ident: $wrong_ident:expr,
            wrong_ty: $wrong_ty:expr,
            has_generics: $has_generics:expr $(,)?
        } $(,)?
    ) => {{
        let validate_type = |item: &syn::ImplItemType| -> Result<(), TokenStream> {
            if item.ident != $expected_ident {
                return Err($wrong_ident.into());
            }

            if !item.generics.params.is_empty() {
                return Err($has_generics.into());
            }

            if item.ty != $expected_ty {
                return Err($wrong_ty.into());
            }

            Ok(())
        };

        if let Some(vec) = $items {
            let mut item_found: Result<(), TokenStream> = Err($not_found.into());
            for item in vec {
                if item.ident == $expected_ident {
                    validate_type(item)?;
                    item_found = Ok(());
                    break;
                }
            }
            item_found?;
        }

        let mut trait_item_found: Result<(), TokenStream> = Err($not_found.into());

        for i in &$towards.items {
            let syn::ImplItem::Type(item) = i else {
                continue;
            };

            if item.ident != $expected_ident {
                continue;
            }

            validate_type(item)?;
            trait_item_found = Ok(());
            break;
        }

        trait_item_found?;
    }};
}

pub(crate) use validate_impl_type;
pub(crate) use validate_impl_types;
