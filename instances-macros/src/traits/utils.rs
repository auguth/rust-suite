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
// ``````````````````````````` INSTANCE TRAIT UTILITIES ``````````````````````````
// ===============================================================================

//! Shared extraction utilities and validation helpers used throughout the
//! instance-trait proc-macro pipeline.
//!
//! It also includes reusable validation macros for associated types,
//! associated constants, and generated `where` predicates.

// ===============================================================================
// ``````````````````````````````````` IMPORTS ```````````````````````````````````
// ===============================================================================

// --- Local crate ---
use crate::{
    Extraction,
    traits::{affiliates::*, counters::*, errors::CounterParamBugs},
};

// --- Proc-Macro crates ---
use proc_macro2::TokenStream;
use syn::{Ident, Type};

// --- Proc-Suite ---
use proc_suite::misc::*;

// ===============================================================================
// ```````````````````` COUNTER IDENTS COLLECTION EXTRACTION `````````````````````
// ===============================================================================

/// Extracts all counter identifiers as a comma-separated string in their
/// original declaration order.
pub(crate) type RawCounterIdentsJoined = String;

impl<'a> Extraction<CounterParamsSlice<'a>> for RawCounterIdentsJoined {
    fn validate_from(from: &CounterParamsSlice<'a>) -> Result<(), TokenStream> {
        <CounterType as Extraction<CounterParamsSlice<'a>>>::validate_from(from)
    }

    fn raw_extract(from: &CounterParamsSlice<'a>, _: &()) -> Result<Self, TokenStream> {
        let counter_names = from
            .iter()
            .map(|p| p.const_param.ident.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        Ok(counter_names)
    }

    fn validate_extract(
        &self,
        from: &CounterParamsSlice<'a>,
        _: Option<&()>,
    ) -> Result<(), TokenStream> {
        Self::validate_from(from)
    }
}

// ===============================================================================
// ``````````````````````````` COUNTER TYPE EXTRACTION ```````````````````````````
// ===============================================================================

/// Extracts the common unsigned integer type shared by all validated
/// instance counters.
pub(crate) type CounterType = Type;

impl<'a> Extraction<CounterParamsSlice<'a>> for CounterType {
    fn validate_from(from: &CounterParamsSlice<'a>) -> Result<(), TokenStream> {
        if CounterParam::types_equality(from).is_err() {
            return Err(CounterParamBugs::CountersNonSameTypesPassed {}.into());
        }

        if from.is_empty() {
            return Err(CounterParamBugs::NoValidCountersProvided {}.into());
        };
        Ok(())
    }

    fn raw_extract(from: &CounterParamsSlice<'a>, _: &()) -> Result<Self, TokenStream> {
        Ok(from[0].const_param.ty.clone())
    }

    fn validate_extract(
        &self,
        from: &CounterParamsSlice<'a>,
        _: Option<&()>,
    ) -> Result<(), TokenStream> {
        Self::validate_from(from)
    }
}

// ===============================================================================
// ``````````````````` ALL AFFILIATE COUNTER IDENTS EXTRACTION ```````````````````
// ===============================================================================

/// Extracts the generated associated-type identifiers representing every
/// affiliate counter family in [`affiliates`](crate::traits::affiliates).
///
/// /// Produces:
///
/// ```ignore
/// [
///     ReverseAffiliateCounters,
///     CeilAffiliateCounters,
///     NextAffiliateCounters,
///     BackAffiliateCounters,
///     FloorAffiliatesCounters, // -> for each counter's floor
/// ]
#[derive(Debug, Clone)]
pub(crate) struct AllAffiliateCounterIdents(pub(crate) Vec<Ident>);

impl<'a> Extraction<CounterParamsSlice<'a>> for AllAffiliateCounterIdents {
    fn raw_extract(from: &CounterParamsSlice<'a>, _: &()) -> Result<Self, TokenStream> {
        let mut collect_tys = Vec::new();

        collect_tys.push(gen_type_ident::<ReverseAffiliateCounters>());
        collect_tys.push(gen_type_ident::<CeilAffiliateCounters>());
        collect_tys.push(gen_type_ident::<NextAffiliateCounters>());
        collect_tys.push(gen_type_ident::<BackAffiliateCounters>());
        for c in from.iter() {
            collect_tys.push(gen_type_ident_with_suffix::<FloorAffiliatesCounters>(Some(
                c.generic_index.to_string().as_bytes(),
            )));
        }

        Ok(Self(collect_tys))
    }

    fn validate_extract(
        &self,
        _: &CounterParamsSlice<'a>,
        _: Option<&()>,
    ) -> Result<(), TokenStream> {
        Ok(())
    }
}

// ===============================================================================
// `````````````````````````````` CONVENIENCE MACRO ``````````````````````````````
// ===============================================================================

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ``````````````````````````` TRAIT TYPE (VALIDATION) ```````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// Validates a single associated type against an expected identifier,
/// generic-parameter configuration, and optional trait-bound contract.
macro_rules! validate_trait_type {
    (
        item: $item:expr,
        towards: $towards:expr,

        ident: $expected_ident:expr,

        bounds: None => {
            not_found: $not_found:expr,
            wrong_ident: $wrong_ident:expr,
            has_bounds: $has_bounds:expr,
            has_generics: $has_generics:expr $(,)?
        } $(,)?
    ) => {{
        let validate = |item: &syn::TraitItemType| -> Result<(), TokenStream> {
            if item.ident != $expected_ident {
                return Err($wrong_ident.into());
            }

            if !item.generics.params.is_empty() {
                return Err($has_generics.into());
            }

            if !item.bounds.is_empty() {
                return Err($has_bounds.into());
            }

            Ok(())
        };

        if let Some(item) = $item {
            validate(item)?;
        }

        let mut found: Result<(), TokenStream> = Err($not_found.into());

        for i in &$towards.items {
            let syn::TraitItem::Type(item) = i else {
                continue;
            };

            if item.ident != $expected_ident {
                continue;
            }

            validate(item)?;
            found = Ok(());
            break;
        }

        found
    }};

    (
        item: $item:expr,
        towards: $towards:expr,

        ident: $expected_ident:expr,

        bounds: $expected_bounds:expr => {
            not_found: $not_found:expr,
            wrong_ident: $wrong_ident:expr,
            has_generics: $has_generics:expr,
            bound_not_found: $bound_not_found:expr $(,)?
        } $(,)?
    ) => {{
        let validate = |item: &syn::TraitItemType| -> Result<(), TokenStream> {
            if item.ident != $expected_ident {
                return Err($wrong_ident.into());
            }

            if !item.generics.params.is_empty() {
                return Err($has_generics.into());
            }

            let expected_bounds = $expected_bounds;

            for expected_bound in expected_bounds {
                let mut found = false;

                for bound in &item.bounds {
                    let syn::TypeParamBound::Trait(bound) = bound else {
                        continue;
                    };

                    if bound == expected_bound {
                        found = true;
                        break;
                    }
                }

                if !found {
                    return Err($bound_not_found.into());
                }
            }

            Ok(())
        };

        if let Some(item) = $item {
            validate(item)?;
        }

        let mut found: Result<(), TokenStream> = Err($not_found.into());

        for i in &$towards.items {
            let syn::TraitItem::Type(item) = i else {
                continue;
            };

            if item.ident != $expected_ident {
                continue;
            }

            validate(item)?;
            found = Ok(());
            break;
        }

        found
    }};
}

/// Validates a collection of associated types against an expected
/// associated-type contract.
///
/// Unlike [`validate_trait_type`], this helper operates on collections of
/// generated associated types before validating their presence on the trait.
macro_rules! validate_trait_types {
    (
        items: $items:expr,
        towards: $towards:expr,

        ident: $expected_ident:expr,

        bounds: None => {
            not_found: $not_found:expr,
            wrong_ident: $wrong_ident:expr,
            has_bounds: $has_bounds:expr,
            has_generics: $has_generics:expr $(,)?
        } $(,)?
    ) => {{
        let validate_type = |item: &syn::TraitItemType| -> Result<(), TokenStream> {
            if item.ident != $expected_ident {
                return Err($wrong_ident.into());
            }

            if !item.generics.params.is_empty() {
                return Err($has_generics.into());
            }

            if !item.bounds.is_empty() {
                return Err($has_bounds.into());
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
            let syn::TraitItem::Type(item) = i else {
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

    (
        items: $items:expr,
        towards: $towards:expr,

        ident: $expected_ident:expr,

        bounds: $expected_bounds:expr => {
            not_found: $not_found:expr,
            wrong_ident: $wrong_ident:expr,
            has_generics: $has_generics:expr,
            bound_not_found: $bound_not_found:expr $(,)?
        } $(,)?
    ) => {{
        let validate_type = |item: &syn::TraitItemType| -> Result<(), TokenStream> {
            if item.ident != $expected_ident {
                return Err($wrong_ident.into());
            }

            if !item.generics.params.is_empty() {
                return Err($has_generics.into());
            }

            let expected_bounds = $expected_bounds;

            for expected_bound in expected_bounds {
                let mut found = false;

                for bound in &item.bounds {
                    let syn::TypeParamBound::Trait(bound) = bound else {
                        continue;
                    };

                    if bound == expected_bound {
                        found = true;
                        break;
                    }
                }

                if !found {
                    return Err($bound_not_found.into());
                }
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
            let syn::TraitItem::Type(item) = i else {
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

pub(crate) use validate_trait_type;
pub(crate) use validate_trait_types;

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ``````````````````````````` TRAIT CONST (VALIDATION) ``````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// Validates a single associated constant against an expected identifier,
/// type, generic configuration, and optional default expression.
macro_rules! validate_trait_const {
    (
        item: $item:expr,
        towards: $towards:expr,

        ident: $ident:expr,
        ty: $expected_ty:expr,

        expr: None => {
            not_found: $not_found:expr,
            wrong_ident: $wrong_ident:expr,
            wrong_ty: $wrong_ty:expr,
            has_generics: $has_generics:expr,
            has_expr: $has_expr:expr $(,)?
        } $(,)?
    ) => {{
        let validate_const = |const_item: &syn::TraitItemConst| -> Result<(), TokenStream> {
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

            if const_item.default.is_some() {
                return Err($has_expr.into());
            }

            Ok(())
        };

        if let Some(item) = $item {
            validate_const(item)?;
        }

        let mut found: Result<(), TokenStream> = Err($not_found.into());

        for i in &$towards.items {
            if let syn::TraitItem::Const(c) = i {
                if c.ident == $ident {
                    validate_const(c)?;
                    found = Ok(());
                    break;
                }
            }
        }

        found
    }};

    (
        item: $item:expr,
        towards: $towards:expr,

        ident: $ident:expr,
        ty: $expected_ty:expr,

        expr: Some($expected_expr:expr) => {
            not_found: $not_found:expr,
            wrong_ident: $wrong_ident:expr,
            wrong_ty: $wrong_ty:expr,
            has_generics: $has_generics:expr,

            missing_expr: $missing_expr:expr,
            invalid_expr: $invalid_expr:expr $(,)?
        } $(,)?
    ) => {{
        let validate_const = |const_item: &syn::TraitItemConst| -> Result<(), TokenStream> {
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

            let Some((_, expr)) = const_item.default.as_ref() else {
                return Err($missing_expr.into());
            };

            if expr.to_token_stream().to_string() != $expected_expr.to_token_stream().to_string() {
                return Err($invalid_expr.into());
            }

            Ok(())
        };

        if let Some(item) = $item {
            validate_const(item)?;
        }

        let mut found: Result<(), TokenStream> = Err($not_found.into());

        for i in &$towards.items {
            if let syn::TraitItem::Const(c) = i {
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
/// associated-constant contract.
///
/// Unlike [`validate_trait_const`], this helper operates on collections of
/// generated constants before validating their presence on the trait.
macro_rules! validate_trait_consts {
    (
        items: $items:expr,
        towards: $towards:expr,

        ident: $ident:expr,
        ty: $expected_ty:expr,

        expr: None => {
            not_found: $not_found:expr,
            wrong_ident: $wrong_ident:expr,
            wrong_ty: $wrong_ty:expr,
            has_generics: $has_generics:expr,
            has_expr: $has_expr:expr $(,)?
        } $(,)?
    ) => {{
        let validate_const = |const_item: &syn::TraitItemConst| -> Result<(), TokenStream> {
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

            if const_item.default.is_some() {
                return Err($has_expr.into());
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
            if let syn::TraitItem::Const(c) = i {
                if c.ident == $ident {
                    validate_const(c)?;
                    trait_item_found = Ok(());
                    break;
                }
            }
        }

        trait_item_found?;
    }};

    (
        items: $items:expr,
        towards: $towards:expr,

        ident: $ident:expr,
        ty: $expected_ty:expr,

        expr: Some($expected_expr:expr) => {
            not_found: $not_found:expr,
            wrong_ident: $wrong_ident:expr,
            wrong_ty: $wrong_ty:expr,
            has_generics: $has_generics:expr,

            missing_expr: $missing_expr:expr,
            invalid_expr: $invalid_expr:expr $(,)?
        } $(,)?
    ) => {{
        let validate_const = |const_item: &syn::TraitItemConst| -> Result<(), TokenStream> {
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

            let Some((_, expr)) = const_item.default.as_ref() else {
                return Err($missing_expr.into());
            };

            if expr.to_token_stream().to_string() != $expected_expr.to_token_stream().to_string() {
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
            if let syn::TraitItem::Const(c) = i {
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

pub(crate) use validate_trait_const;
pub(crate) use validate_trait_consts;

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ```````````````````````` WHERE PREDICATE (VALIDATION) `````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// Validates a `where` predicate by matching a bounded type and verifying
/// that the expected trait bound is present.
#[allow(unused)]
macro_rules! validate_where_predicate {
    (
        predicates: $predicates:expr,

        bounded_ty: $expected_bounded_ty:expr,
        bound: $expected_bound:expr,

        errors: {
            not_found: $not_found:expr,

            has_lifetimes: $has_lifetimes:expr,
            bound_not_found: $bound_not_found:expr $(,)?
        } $(,)?
    ) => {{
        let mut matched_bounded_ty = false;

        let mut result: Result<(), TokenStream> = Err($not_found.into());

        for predicate in $predicates {
            let syn::WherePredicate::Type(predicate) = predicate else {
                continue;
            };

            if predicate.bounded_ty != $expected_bounded_ty {
                continue;
            }

            matched_bounded_ty = true;

            if predicate.lifetimes.is_some() {
                result = Err($has_lifetimes.into());
                continue;
            }

            let Some(bound) = predicate.bounds.first() else {
                result = Err($bound_not_found.into());
                continue;
            };

            let syn::TypeParamBound::Trait(bound) = bound else {
                result = Err($bound_not_found.into());
                continue;
            };

            if *bound != $expected_bound {
                result = Err($bound_not_found.into());
                continue;
            }

            result = Ok(());
            break;
        }

        if !matched_bounded_ty {
            Err($not_found.into())
        } else {
            result
        }
    }};
}

#[allow(unused)]
pub(crate) use validate_where_predicate;
