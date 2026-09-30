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
// ``````````````````````````` INSTANCE COUNTER PARAMS ```````````````````````````
// ===============================================================================

//! This module extracts and validates trait-side const-generic parameters
//! used as instance counters.
//!
//! Instance counters define the positional counter space used to define an
//! implementation's and its relatives instance relationships.
//!
//! ## Why this exists
//!
//! Trait implementations do not reference const generics by identifier.
//!
//! For example:
//!
//! ```ignore
//! trait Example<const A: u8, const B: u8> {}
//!
//! impl Example<4, 7> for MyType {}
//! ```
//!
//! the impl does not know the names `A` or `B`; it only supplies values by
//! position.
//!
//! Because of this, the instance system identifies counters exclusively by
//! their index inside the trait generic parameter list.
//!
//! ## What this module provides
//!
//! This module:
//!
//! - extracts instance-counter const generics from traits,
//! - validates supported counter types,
//! - normalizes counters into positional metadata,
//! - and preserves user-defined counter ordering.
//!
//! Extracted counters are represented as [`CounterParam`] values collected
//! into [`CounterParams`].
//!
//! ## Extraction Modes
//!
//! Counters can be selected in three ways:
//!
//! ### Identifier-based extraction
//!
//! ```ignore
//! #[trait_instance(A, B)]
//! ```
//!
//! Resolves counters by const-generic identifier.
//!
//! ### Index-based extraction
//!
//! ```ignore
//! #[trait_instance(0, 2)]
//! ```
//!
//! Resolves counters by generic parameter position.
//!
//! ### Default extraction
//!
//! When no explicit selection is provided, leading contiguous same-type
//! const generics are used automatically.
//!
//! ```ignore
//! trait Example<const A: u8, const B: u8, const C: u16, T> {}
//! ```
//!
//! extracts:
//!
//! ```ignore
//! A, B
//! ```
//!
//! ## Ordering Semantics
//!
//! Counter ordering is semantic and user-controlled.
//!
//! The extracted order does not need to match the textual order of the trait
//! generics.
//!
//! Example:
//!
//! ```ignore
//! trait Example<const A: u8, T, const B: u8> {}
//!
//! #[trait_instance(B, A)]
//! ```
//!
//! produces:
//!
//! ```ignore
//! [B, A]
//! ```
//!
//! even though `A` appears first in the trait definition.
//!
//! ## Supported Counter Types
//!
//! Only some primitive unsigned integer const generics are supported:
//!
//! - `u8`
//! - `u16`
//! - `u32`
//!
//! All extracted counters must use the same integer type.

// ===============================================================================
// ``````````````````````````````````` IMPORTS ```````````````````````````````````
// ===============================================================================

// --- Proc-Macro crates ---
use proc_macro2::TokenStream;
use quote::quote;
use syn::{ConstParam, GenericParam, ItemTrait, Type, TypePath};

// --- Local Crate ---
use crate::{
    Extraction, InstanceArgs,
    traits::errors::{CounterParamError, CounterParamExtractionError},
};

// --- Proc-suite ---
use proc_suite::{DuplicateCheck, IdentList, IntList, misc::*};

// ===============================================================================
// ```````````````````````````` INSTANCE COUNTER PARAM ```````````````````````````
// ===============================================================================

/// Describes a **trait-side** const-generic parameter used for instance counters.
///
/// When defining a trait, the author may name const-generic parameters, but
/// those names carry **no meaning** for instance-counter resolution on the
/// impl side. Instance counters represented here are const generics, not types,
/// hence, impls do not reference them by identifier.
///
/// For stable identification across the trait <-> impl boundary, the instance
/// system relies **exclusively on positional indexing** into the trait's
/// `<...>` generic parameter list.
///
/// This type records:
/// - the zero-based index of the const-generic parameter in the instance trait definition
/// - the original [`ConstParam`] syntax node for local inspection and reuse.
///
/// The stored `generic_index` is the **only communication mechanism** used to
/// associate trait-declared instance counters with impl-supplied values.
///
/// ## Example
///
/// ```rust
/// trait Example<T, const A: u8, U, const B: u8> {}
/// ```
///
/// In the trait above:
///
/// - `A` has `generic_index == 1`
/// - `B` has `generic_index == 3`
///
/// Even though the parameters are named, impls do **not** refer to them by
/// identifier:
///
/// ```ignore
/// impl<T, U> Example<T, 4, U, 7> for MyType {}
/// //                    ^     ^
/// //                    |     |
/// //                 index1 index3
/// ```
///
/// Renaming `A` or `B` in the trait definition has no effect on instance
/// resolution, as long as their positional indices remain unchanged.
#[derive(Clone, Debug)]
pub(crate) struct CounterParam {
    /// The zero-based position of this const-generic parameter in the trait's
    /// generic parameter list.
    ///
    /// This index uniquely identifies which instance counter this parameter
    /// represents and is preserved verbatim for matching against impl-side
    /// numeric constant arguments.
    pub(super) generic_index: usize,

    /// The original `ConstParam` node from the trait definition.
    ///
    /// This retains the declared syntax (attributes, bounds, defaults, etc.)
    /// but is not used for identification across the trait–impl boundary.
    pub(super) const_param: ConstParam,
}

/// A vector of [`CounterParam`] values describing all **instance-counter**
/// const-generic parameters declared on the instance trait.
///
/// Each [`CounterParam`] represents a single const-generic parameter from the
/// trait's generic parameter list and is identified exclusively by its
/// zero-based [`CounterParam::generic_index`].
///
/// ## Ordering semantics
///
/// The order of this vector is **intentional and user-defined**.  
/// It does **not** need to be ascending by `generic_index`, nor does it
/// necessarily mirror the textual order of the trait's generic parameters.
///
/// When instance-counter const generics are *mixed* with other generic
/// parameters in the trait definition, this vector preserves the order in
/// which the user intends those counters to participate in instance
/// resolution.
///
/// Expansion logic must therefore rely on `generic_index` for stable
/// trait–impl association and treat the vector order as semantically
/// meaningful.
///
/// ## Examples
///
/// ```rust
/// // Trait definition
/// trait MyTrait<const A: u8, T, const B: u8, const C: u8> {}
/// ```
///
/// If the user declares instance counters in the order `B, A`, the collected
/// parameters will be:
///
/// ```ignore
/// CounterParams [
///   CounterParam { generic_index: 2, const_param: B },
///   CounterParam { generic_index: 0, const_param: A },
/// ]
/// ```
///
/// Likewise, if the user interleaves counters with other generics:
///
/// ```rust
/// trait MyTrait<T, const X: u8, U, const Y: u8> {}
/// ```
///
/// And specifies the instance order `Y, X`, the resulting vector preserves
/// that intent:
///
/// ```ignore
/// CounterParams [
///   CounterParam { generic_index: 3, const_param: Y },
///   CounterParam { generic_index: 1, const_param: X },
/// ]
/// ```
pub(crate) type CounterParams = Vec<CounterParam>;

impl CounterParam {
    /// Ensures all extracted instance counters use the same integer type.
    /// Returns `Ok(())` when counters slice is empty.
    ///
    /// Example:
    ///
    /// valid:
    /// ```ignore
    /// const A: usize, const B: usize
    /// ```
    ///
    /// invalid:
    /// ```ignore
    /// const A: usize, const B: u32
    /// ```
    pub(crate) fn types_equality(params: &[CounterParam]) -> Result<(), TokenStream> {
        if params.is_empty() {
            return Ok(());
        }

        let first = &params[0].const_param.ty;
        let first_ty_ident = &params[0].const_param.ident;
        let expected = normalize_unsigned_type(first);

        /// Normalizes a type into a stable comparable string representation.
        ///
        /// Used internally for counter type equality checks by removing
        /// formatting differences from tokenized type output.
        fn normalize_unsigned_type(ty: &Type) -> String {
            let mut ts = TokenStream::new();
            use quote::ToTokens;
            ty.to_tokens(&mut ts);
            ts.to_string().replace(' ', "")
        }

        for param in params {
            let norm = normalize_unsigned_type(&param.const_param.ty);

            if norm != expected {
                let missed = &param.const_param.ty;
                return Err(CounterParamExtractionError::ExpConsistentCountersTy {
                    exp_ty: expected,
                    first_ty_ident: first_ty_ident.to_string(),
                    found: missed.clone(),
                }
                .into());
            }
        }

        Ok(())
    }
}

/// Borrowed slice view of extracted instance-counter [`CounterParam`] parameters.
pub(crate) type CounterParamsSlice<'a> = &'a [CounterParam];

// ===============================================================================
// `````````````````````````` COUNTERS PARAM EXTRACTION ``````````````````````````
// ===============================================================================

impl Extraction<ItemTrait, Option<&InstanceArgs>> for CounterParams {
    fn raw_extract(from: &ItemTrait, context: &Option<&InstanceArgs>) -> Result<Self, TokenStream> {
        match context {
            Some(InstanceArgs::Ident(idents)) => {
                <Self as Extraction<ItemTrait, IdentList>>::checked_extract(from, idents)
            }
            Some(InstanceArgs::Index(ints)) => {
                <Self as Extraction<ItemTrait, IntList>>::checked_extract(from, ints)
            }
            None => <Self as Extraction<ItemTrait>>::checked_extract(from, &()),
        }
    }

    fn validate_extract(
        &self,
        from: &ItemTrait,
        context: Option<&Option<&InstanceArgs>>,
    ) -> Result<(), TokenStream> {
        let Some(args) = context else {
            return <Self as Extraction<ItemTrait>>::validate_extract(&self, from, None);
        };
        match args {
            Some(InstanceArgs::Ident(idents)) => {
                <Self as Extraction<ItemTrait, IdentList>>::validate_extract(
                    &self,
                    from,
                    Some(idents),
                )
            }
            Some(InstanceArgs::Index(ints)) => {
                <Self as Extraction<ItemTrait, IntList>>::validate_extract(&self, from, Some(ints))
            }
            None => <Self as Extraction<ItemTrait>>::validate_extract(&self, from, None),
        }
    }
}

impl Extraction<ItemTrait, IdentList> for CounterParams {
    fn validate_context(context: &IdentList) -> Result<(), TokenStream> {
        Ok(context.duplicate_check(Some(CounterParamError::DuplicateRawCounterIdents.into()))?)
    }

    fn validate_from(from: &ItemTrait) -> Result<(), TokenStream> {
        <Self as Extraction<ItemTrait, IntList>>::validate_from(from)
    }

    fn raw_extract(from: &ItemTrait, context: &IdentList) -> Result<Self, TokenStream> {
        let params = &from.generics.params;
        let mut collect = Vec::new();

        for ident in &context.idents {
            let ident_str = ident.to_string();
            let mut not_found = Some(ident.clone());

            for (i, param) in params.iter().enumerate() {
                let GenericParam::Const(c) = param else {
                    continue;
                };

                if c.ident.to_string() == ident_str {
                    collect.push((i, param));
                    // not found == false (hence found)
                    not_found = None;
                    break;
                }
            }

            if let Some(ident) = not_found {
                let mut valid_idents = Vec::new();
                for p in params {
                    if let Ok(c) = counter_param(p) {
                        valid_idents.push(c.ident.to_string())
                    }
                }

                return Err(CounterParamExtractionError::IdentNotConstGeneric {
                    unknown_param_ident: ident,
                    available_params: valid_idents.join(", "),
                    trait_ident: from.ident.clone(),
                    trait_generics: from.generics.clone(),
                }
                .into());
            }
        }
        counter_params(&collect)
    }

    fn validate_extract(
        &self,
        from: &ItemTrait,
        context: Option<&IdentList>,
    ) -> Result<(), TokenStream> {
        let Some(context) = context else {
            return <Self as Extraction<ItemTrait>>::validate_extract(&self, from, None);
        };
        let idents = &context.idents;

        if self.len() != idents.len() {
            return Err(CounterParamExtractionError::IdentsReCheckLenDifference {
                idents: context.clone(),
            }
            .into());
        }

        for (a, b) in self.iter().zip(idents) {
            let a_ident = &a.const_param.ident;
            let exp = b.to_string();
            let found = a_ident.to_string();
            if a_ident.to_string() != exp {
                return Err(
                    CounterParamExtractionError::IdentsReCheckWrongIdent { exp, found }.into(),
                );
            }
        }

        <Self as Extraction<ItemTrait>>::validate_extract(&self, from, None)?;

        Ok(())
    }
}

impl Extraction<ItemTrait, IntList> for CounterParams {
    fn validate_context(context: &IntList) -> Result<(), TokenStream> {
        Ok(context.duplicate_check(Some(CounterParamError::DuplicateCounterIndexes.into()))?)
    }

    fn validate_from(from: &ItemTrait) -> Result<(), TokenStream> {
        let params = &from.generics.params;
        if params.is_empty() {
            return Err(CounterParamExtractionError::TraitNeedsCounterGenerics {
                trait_generics: from.generics.clone(),
                trait_ident: from.ident.clone(),
            }
            .into());
        }
        let mut found = false;
        for param in params {
            if let GenericParam::Const(_) = param {
                found = true;
            };
        }
        if !found {
            return Err(CounterParamExtractionError::TraitNeedsCounterGenerics {
                trait_generics: from.generics.clone(),
                trait_ident: from.ident.clone(),
            }
            .into());
        }

        Ok(())
    }

    fn raw_extract(from: &ItemTrait, context: &IntList) -> Result<Self, TokenStream> {
        let params = &from.generics.params;
        let mut collect = Vec::new();
        for lit in &context.ints {
            let idx = parse_pos_usize(lit)?;
            let Some(param) = params.get(idx) else {
                let mut valid_idents = Vec::new();
                for (i, p) in params.iter().enumerate() {
                    if let Ok(_) = counter_param(p) {
                        valid_idents.push(i.to_string())
                    }
                }
                return Err(CounterParamExtractionError::OutOfBoundsGenericIndex {
                    gen_idx: idx,
                    given_idx: lit.clone(),
                    trait_generics: from.generics.clone(),
                    trait_ident: from.ident.clone(),
                    available_indexes: valid_idents.join(", "),
                }
                .into());
            };
            let GenericParam::Const(_) = param else {
                let mut valid_idents = Vec::new();
                for (i, p) in params.iter().enumerate() {
                    if let Ok(_) = counter_param(p) {
                        valid_idents.push(i.to_string())
                    }
                }
                return Err(CounterParamExtractionError::IndexNotConstGeneric {
                    unknown_index: idx,
                    given_lit: lit.clone(),
                    trait_generics: from.generics.clone(),
                    trait_ident: from.ident.clone(),
                    available_indexes: valid_idents.join(", "),
                }
                .into());
            };
            collect.push((idx.clone(), param));
        }
        counter_params(&collect)
    }

    fn validate_extract(
        &self,
        from: &ItemTrait,
        context: Option<&IntList>,
    ) -> Result<(), TokenStream> {
        let Some(context) = context else {
            return <Self as Extraction<ItemTrait>>::validate_extract(&self, from, None);
        };
        let ints = &context.ints;

        if self.len() != ints.len() {
            return Err(CounterParamExtractionError::IntsReCheckLenDifference {
                ints: context.clone(),
            }
            .into());
        }

        for (a, b) in self.iter().zip(ints) {
            let a_idx = a.generic_index.to_string();
            let exp = b.base10_digits().to_string();
            let found = a_idx.to_string();
            if found != exp {
                return Err(
                    CounterParamExtractionError::IntsReCheckWrongIndex { exp, found }.into(),
                );
            }
        }

        <Self as Extraction<ItemTrait>>::validate_extract(&self, from, None)?;
        Ok(())
    }
}

impl Extraction<ItemTrait> for CounterParams {
    fn validate_from(from: &ItemTrait) -> Result<(), TokenStream> {
        let params = &from.generics.params;
        let Some(GenericParam::Const(_)) = params.get(0) else {
            return Err(CounterParamExtractionError::TraitNeedsLeadConstGenerics {
                trait_generics: from.generics.clone(),
                trait_ident: from.ident.clone(),
            }
            .into());
        };
        Ok(())
    }

    fn raw_extract(from: &ItemTrait, _: &()) -> Result<Self, TokenStream> {
        let mut params = Vec::new();

        for (i, param) in from.generics.params.iter().enumerate() {
            let GenericParam::Const(c) = param else {
                break;
            };

            let Type::Path(TypePath { qself: None, path }) = &c.ty else {
                break;
            };

            let Some(seg) = path.segments.last() else {
                break;
            };

            let matched = matches!(seg.ident.to_string().as_str(), "u8" | "u16" | "u32");

            if !matched {
                break;
            }

            params.push((i, param));
        }

        if params.is_empty() {
            return Err(CounterParamExtractionError::TraitNeedsCounterGenerics {
                trait_generics: from.generics.clone(),
                trait_ident: from.ident.clone(),
            }
            .into());
        }

        counter_params(&params)
    }

    fn validate_extract(&self, from: &ItemTrait, _: Option<&()>) -> Result<(), TokenStream> {
        let params = &from.generics.params;
        let mut collect = Vec::new();
        for counter_p in self {
            let idx = counter_p.generic_index;
            let Some(param) = params.get(idx) else {
                return Err(CounterParamExtractionError::ParamsReCheckOutOfBounds {
                    idx,
                    trait_generics: from.generics.clone(),
                    trait_ident: from.ident.clone(),
                }
                .into());
            };
            let a_counter = &counter_p.const_param;
            let b_counter = counter_param(param)?;
            let exp = b_counter.ident.to_string();
            let found = a_counter.ident.to_string();
            if exp != found {
                return Err(CounterParamExtractionError::ParamsReCheckWrongIdent {
                    exp,
                    found,
                    trait_ident: from.ident.clone(),
                }
                .into());
            }
            let a_ty = &a_counter.ty;
            let b_ty = &b_counter.ty;
            let exp_ty = quote! {#a_ty}.to_string();
            let found_ty = quote! {#b_ty}.to_string();
            if exp_ty != quote! {#b_ty}.to_string() {
                return Err(CounterParamExtractionError::ParamsReCheckWrongType {
                    param_ident: a_counter.ident.clone(),
                    trait_ident: from.ident.clone(),
                    exp_ty,
                    found_ty,
                }
                .into());
            }
            collect.push((idx, param));
        }
        counter_params(&collect)?;
        Ok(())
    }
}

// ===============================================================================
// ```````````````````````` EXTRACTION PRIVATE UTILITIES `````````````````````````
// ===============================================================================

/// Builds normalized [`CounterParams`] from validated generic parameters.
///
/// Converts raw trait generic parameters into structured
/// [`CounterParam`] values and verifies:
/// - supported counter types
/// - consistent type usage across all counters
fn counter_params(params: &[(usize, &GenericParam)]) -> Result<CounterParams, TokenStream> {
    let mut counters = CounterParams::new();
    for (idx, param) in params {
        let counter = counter_param(*param)?;
        counters.push(CounterParam {
            generic_index: idx.clone(),
            const_param: counter,
        });
    }
    CounterParam::types_equality(&counters)?;
    Ok(counters)
}

/// Allowed primitive unsigned integer types for instance counters.
///
/// Only these const-generic types are accepted for trait instance
/// counters to ensure predictable indexing semantics.
const COUNTER_TYPES: &[&str] = &["u8", "u16", "u32"];

/// Validates that a generic parameter is a supported const-generic
/// instance counter.
///
/// Rejects:
/// - lifetime generics
/// - type generics
/// - unsupported const generic types
///
/// Only unsigned primitive integer const generics are accepted else returns
/// a compile error with diagnostic on the generic param to be as expected.
fn counter_param(param: &GenericParam) -> Result<ConstParam, TokenStream> {
    let expected = COUNTER_TYPES;
    let c = match param {
        GenericParam::Lifetime(l) => {
            return Err(CounterParamExtractionError::ExpConstGeneric {
                found: l.lifetime.ident.clone(),
            }
            .into());
        }
        GenericParam::Type(t) => {
            return Err(CounterParamExtractionError::ExpConstGeneric {
                found: t.ident.clone(),
            }
            .into());
        }
        GenericParam::Const(c) => c,
    };

    if c.default.is_some() {
        let default = c.default.clone().unwrap();
        return Err(CounterParamExtractionError::ExpNonDefaultConstGeneric {
            found: c.ident.clone(),
            default,
        }
        .into());
    }

    let ty_err = CounterParamExtractionError::ExpCountersTy {
        found: c.ty.clone(),
    }
    .into();

    let Type::Path(TypePath { path, .. }) = &c.ty else {
        return Err(ty_err);
    };

    let Some(seg) = path.segments.last() else {
        return Err(ty_err);
    };

    let mut correct = false;
    for expect in expected {
        if seg.ident.to_string().as_str() == *expect {
            correct = true
        }
    }

    if !correct {
        return Err(ty_err);
    }

    Ok(c.clone())
}
