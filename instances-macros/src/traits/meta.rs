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
// `````````````````````` INSTANCE TRAIT META (REFLECTION) ```````````````````````
// ===============================================================================

//! This module generates hidden trait-side metadata used by the
//! instance-trait proc-macro pipeline.
//!
//! The generated metadata acts as a reflection and synchronization layer
//! between:
//!
//! - trait-side expansion,
//! - and impl-side expansion.
//!
//! ## Why this exists
//!
//! Rust proc macros expanding on an `impl` block cannot reliably recover:
//!
//! - which generic parameters are instance counters,
//! - what counter type the trait uses,
//! - how counters were ordered,
//! - or what structural contracts the trait originally declared.
//!
//! This module solves that limitation by reflecting the required metadata
//! directly into the trait itself through hidden associated items.
//!
//! Impl-side macros later read and validate those generated items to ensure
//! both sides of the expansion pipeline remain synchronized.
//!
//! ## Example
//!
//! Given:
//!
//! ```ignore
//! #[trait_instance(A, C)]
//! trait Example<
//!     const A: u8,
//!     const B: usize,
//!     const C: u8,
//! > {}
//! ```
//!
//! After
//! [`InstanceTraitTypeNumCounters`](crate::traits::typenum::InstanceTraitTypeNumCounters)
//! transformation:
//!
//! ```ignore
//! trait Example<__TypeNumCounters, const B: usize> {}
//! ```
//!
//! this module generates:
//!
//! ```ignore
//! trait Example<__TypeNumCounters, const B: usize> {
//!     type InstanceCounter;
//!
//!     #[doc(hidden)]
//!     const __COUNTERS_GENERICS_META:
//!         [(usize, &'static str); 2] = [
//!             (0, "A"),
//!             (2, "C"),
//!         ];
//!
//!     #[doc(hidden)]
//!     const __COUNTERS_GENERIC_INDEXES_META:
//!         &'static [usize];
//!
//!     #[doc(hidden)]
//!     const __COUNTERS_LEN_META: usize;
//!
//!     #[doc(hidden)]
//!     const __COUNTERS_GENERICS_CHECKER: ();
//!
//!     #[doc(hidden)]
//!     const __COUNTER_INDEX_0: u8;
//!
//!     #[doc(hidden)]
//!     const __COUNTER_INDEX_2: u8;
//! }
//! ```
//!
//! where:
//!
//! - [`CountersTy`] exposes the expected implementation-side counter
//!   type contract.
//!
//! - [`CountersTyMeta`] exposes the counter's type information as string literal.
//!
//! - [`CountersGenericsMeta`] reflects counter indexes and original
//!   identifiers.
//!
//! - [`CountersGenericsIndexesMeta`] exposes the expected impl-side
//!   counter-index contract.
//!
//! - [`CountersLenMeta`] exposes the expected impl-side counter count.
//!
//! - [`CountersGenericsChecker`] defines a synchronization checkpoint
//!   for generic metadata validation.
//!
//! - [`OriginalCounterConst`] reflects original counters as hidden
//!   associated constants after typenum transformation.
//!
//! - [`InstanceTraitDocs`] appends implementation guidance when explicit
//!   counter-index mapping is required.
//!
//! - [`MarkerSuperBound`] Adds the `Self: Marker` superbound to the
//!   generated instance trait (**not utilized currently**)
//!
//! Together, these generated items provide the reflection metadata that
//! impl-side expansion later consumes.
//!
//! ## Reflection Model
//!
//! Generated metadata is intentionally:
//!
//! - deterministic,
//! - hidden from public APIs,
//! - proc-macro-readable,
//! - and structurally verifiable.
//!
//! Most generated identifiers are deterministic names derived from
//! proc-macro crate local types.
//!
//! This prevents external crates from forging or accessing metadata
//! while allowing implementation-side proc macros from this crate to
//! safely reconstruct trait-side instance semantics.

// ===============================================================================
// ``````````````````````````````````` IMPORTS ```````````````````````````````````
// ===============================================================================

// --- Proc-Macro crates ---
use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident};
use syn::{
    ConstParam, Expr, GenericParam, ItemTrait, Lit, TraitItem, TraitItemConst, TraitItemType, Type,
    TypeParamBound, TypePath, WherePredicate, parse_quote,
};

// --- Local Crate ---
use crate::{
    Extension, Extraction, Insertion, Instance, InstanceArgs, TRAIT_IMPL_MACRO_NAME,
    Transformation,
    traits::{counters::*, errors::MetaTraitItemsBugs, utils::*},
};

// --- Proc-suite ---
use proc_suite::{
    SupportCrate,
    docs::{DocAttr, InsertDocs},
    misc::*,
};

// ===============================================================================
// ```````````````````````````````` META EXPANSION ```````````````````````````````
// ===============================================================================

/// Aggregates and executes all trait-side metadata expansion phases used
/// by the instance-trait proc-macro pipeline.
///
/// This phase acts as the orchestration layer responsible for applying:
///
/// - impl-side counter type expectations via [`CountersTy`],
/// - counter type reflection via [`CountersTyMeta`],
/// - counter generic reflection via [`CountersGenericsMeta`],
/// - impl-side expected generic indexes via
///   [`CountersGenericsIndexesMeta`],
/// - impl-side expected counter length via [`CountersLenMeta`],
/// - generic metadata synchronization checkpoints via
///   [`CountersGenericsChecker`],
///
/// Conceptually, this phase converts extracted instance-counter metadata
/// into:
///
/// - reflected trait-side metadata,
/// - impl-side validation contracts,
/// - synchronization/checkpoint nodes,
/// - and implementation guidance documentation.
///
/// This effectively forms the complete trait-side metadata/reflection
/// expansion stage of the proc-macro pipeline.
///
/// Each phase remains independently composable and validatable through
/// the proc-macro pipeline trait system.
#[derive(Clone, Debug)]
pub(crate) struct InstanceTraitMeta;

impl<'a> Transformation<ItemTrait, CounterParamsSlice<'a>> for InstanceTraitMeta {
    fn raw_transform(
        &self,
        transform: &mut ItemTrait,
        context: &CounterParamsSlice<'a>,
    ) -> Result<(), TokenStream> {
        CountersTy::checked_extend(&CountersTy, transform, context)?;
        CountersTyMeta::checked_extend(&CountersTyMeta, transform, context)?;
        CountersGenericsMeta::checked_extend(&CountersGenericsMeta, transform, context)?;
        CountersGenericsIndexesMeta::checked_extend(&CountersGenericsIndexesMeta, transform, &())?;
        CountersLenMeta::checked_extend(&CountersLenMeta, transform, &())?;
        CountersGenericsChecker::checked_extend(&CountersGenericsChecker, transform, &())?;
        OriginalCounterConst::checked_extend(&OriginalCounterConst, transform, context)?;
        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &ItemTrait,
        context: Option<&CounterParamsSlice<'a>>,
    ) -> Result<(), TokenStream> {
        CountersTy::validate_extend(&CountersTy, None, transform, context)?;
        CountersTyMeta::validate_extend(&CountersTyMeta, None, transform, context)?;
        CountersGenericsMeta::validate_extend(&CountersGenericsMeta, None, transform, context)?;
        CountersGenericsIndexesMeta::validate_extend(
            &CountersGenericsIndexesMeta,
            None,
            transform,
            None,
        )?;
        CountersLenMeta::validate_extend(&CountersLenMeta, None, transform, None)?;
        CountersGenericsChecker::validate_extend(&CountersGenericsChecker, None, transform, None)?;
        OriginalCounterConst::validate_extend(&OriginalCounterConst, None, transform, context)?;
        Ok(())
    }
}

// ===============================================================================
// ``````````````````````````` COUNTER TYPE REFLECTION ```````````````````````````
// ===============================================================================

/// Extends an instance trait with hidden metadata describing the
/// unsigned integer type used by its instance counters.
///
/// Rust does not allow procedural macros expanding on an `impl` block to
/// directly recover the original const-generic counter type declared by
/// the trait, because that type information is hidden behind the trait's
/// generic definition and is not reflectable from the impl-side syntax.
///
/// This metadata acts as a trait-side reflection mechanism by generating
/// a hidden constant that stores the counter type as a string literal.
///
/// It is also useful after [`typenum`](crate::traits::typenum) transformation
/// which will loose the original counters const generic's information.
///
/// ## Example
///
/// Given:
///
/// ```ignore
/// #[trait_instance]
/// trait Example<const A: u8> {}
/// ```
///
/// this extension generates:
///
/// ```ignore
/// trait Example<const A: u8> {
///     #[doc(hidden)]
///     const __COUNTER_TY_META: &'static str = "u8";
/// }
/// ```
#[derive(Clone, Debug)]
pub(crate) struct CountersTyMeta;

impl<'a> Insertion<ItemTrait, CounterParamsSlice<'a>> for TraitItemConst {
    fn raw_insert(
        &self,
        to: &mut ItemTrait,
        _: &CounterParamsSlice<'a>,
    ) -> Result<(), TokenStream> {
        to.items.push(TraitItem::Const(self.clone()));
        Ok(())
    }

    fn validate_inserted(
        _: Option<&Self>,
        _: &ItemTrait,
        _: Option<&CounterParamsSlice<'a>>,
    ) -> Result<(), TokenStream> {
        // We don't want to test syn internals
        Ok(())
    }
}

impl<'a> Extension<TraitItemConst, ItemTrait, CounterParamsSlice<'a>> for CountersTyMeta {
    fn raw_extend(
        &self,
        _: &ItemTrait,
        context: &CounterParamsSlice<'a>,
    ) -> Result<TraitItemConst, TokenStream> {
        let ident = gen_const_ident::<Self>();
        let counter_ty = CounterType::checked_extract(context, &())?;
        let ty_str = counter_ty.to_token_stream().to_string();
        // Store the type as a string literal for const-eval comparison later
        let expr: Expr = parse_quote!(#ty_str);
        let ty: Type = parse_quote!(&'static str);
        let item = TraitItemConst {
            attrs: proc_suite::internal_code(),
            ident,
            default: Some((Default::default(), expr)),
            ty,
            const_token: Default::default(),
            generics: Default::default(),
            colon_token: Default::default(),
            semi_token: Default::default(),
        };
        Ok(item)
    }

    fn validate_extend(
        &self,
        item: Option<&TraitItemConst>,
        towards: &ItemTrait,
        context: Option<&CounterParamsSlice<'a>>,
    ) -> Result<(), TokenStream> {
        let ident = gen_const_ident::<Self>();

        let extracted;
        let counters = match context {
            Some(c) => c,
            None => {
                extracted = CounterParams::unchecked_extract(towards, &None)?;
                extracted.as_slice().as_ref().as_ref()
            }
        };

        let counter_ty = CounterType::checked_extract(&counters, &())?;

        let expected_ty: Type = parse_quote!(&'static str);

        let expected_expr = {
            let ty_str = counter_ty.to_token_stream().to_string();
            let expr: Expr = parse_quote!(#ty_str);
            expr
        };

        validate_trait_const! {
            item: item,
            towards: towards,
            ident: ident,
            ty: expected_ty,
            expr: Some(expected_expr) => {
                not_found: MetaTraitItemsBugs::CountersTyMetaNotFound {},
                wrong_ident: MetaTraitItemsBugs::CountersTyMetaWrongIdent {},
                wrong_ty: MetaTraitItemsBugs::CountersTyMetaInvalidType {},
                has_generics: MetaTraitItemsBugs::CountersTyMetaHasGenerics {},
                missing_expr: MetaTraitItemsBugs::CountersTyMetaExprNotFound {},
                invalid_expr: MetaTraitItemsBugs::CountersTyMetaInvalidExpr {},
            }
        }
    }
}

// ===============================================================================
// `````````````````````````` COUNTER TYPE (FROM IMPL) ```````````````````````````
// ===============================================================================

/// Extends an instance trait with an associated type representing the
/// expected instance-counter type provided by implementations.
///
/// Rust does not expose the original const-generic counter type from the
/// trait when expanding on the impl-side syntax.
///
/// To work around this limitation, this phase restates the counter type
/// contract as an associated type:
///
/// ```ignore
/// type InstanceCounter;
/// ```
///
/// The implementation-side macro later captures this associated type from
/// the impl syntax and utilizes its [`OriginalCounterConst`] impl.
///
/// Since the extraction phase ([`CounterParams`]) already guarantees that
/// all instance counters share the same unsigned integer type, expecting
/// a single associated type is sufficient.
///
/// And sufficiently its self validating since trait side [`OriginalCounterConst`]
/// expects a counter type which should be provided via [`CountersTy`] from the
/// impl side. Mismatches will result in expected type errors.
///
/// ## Example
///
/// Given:
///
/// ```ignore
/// #[trait_instance]
/// trait Example<const A: u8> {}
/// ```
///
/// this extension generates:
///
/// ```ignore
/// trait Example<const A: u8> {
///     type InstanceCounter;
/// }
/// ```
///
/// Later:
///
/// ```ignore
/// #[trait_instance_impl]
/// impl Example<1> for MyType {
///     type InstanceCounter = u8;
/// }
/// ```
///
/// the impl-side macro validates:
///
/// - the impl-provided associated type (`u8`)
/// - against the reflected trait metadata generated by [`CountersTyMeta`]
///
/// ensuring both sides agree on the counter type.
#[derive(Clone, Debug)]
pub(crate) struct CountersTy;

/// Associated type identifier used to restate the expected
/// instance-counter type on the trait side (see [`CountersTy`]).
///
/// This type is later implemented and validated from the impl side
/// against the reflected metadata generated by [`CountersTyMeta`].
pub(crate) const INSTANCE_COUNTER_TY: &'static str = "InstanceCounter";

impl<'a> Insertion<ItemTrait, CounterParamsSlice<'a>> for TraitItemType {
    fn raw_insert(
        &self,
        to: &mut ItemTrait,
        _: &CounterParamsSlice<'a>,
    ) -> Result<(), TokenStream> {
        to.items.push(TraitItem::Type(self.clone()));
        Ok(())
    }

    fn validate_inserted(
        _: Option<&Self>,
        _: &ItemTrait,
        _: Option<&CounterParamsSlice<'a>>,
    ) -> Result<(), TokenStream> {
        // We don't want to test syn internals
        Ok(())
    }
}

impl<'a> Extension<TraitItemType, ItemTrait, CounterParamsSlice<'a>> for CountersTy {
    fn raw_extend(
        &self,
        towards: &ItemTrait,
        context: &CounterParamsSlice<'a>,
    ) -> Result<TraitItemType, TokenStream> {
        let ident = format_ident!("{}", INSTANCE_COUNTER_TY);

        let trait_name = &towards.ident;

        let expected_ty = CounterType::checked_extract(&context, &())?;

        let counter_names = RawCounterIdentsJoined::checked_extract(context, &())?;

        let mut item = TraitItemType {
            attrs: Default::default(),
            ident,
            default: None,
            type_token: Default::default(),
            generics: Default::default(),
            colon_token: Default::default(),
            bounds: Default::default(),
            semi_token: Default::default(),
        };

        let _ = item.insert_docs(vec![
            DocAttr::Raw("Associated instance counter type for instance trait ".to_string()),
            DocAttr::Ref(format!("{trait_name}")),
            DocAttr::LineBreak,
            DocAttr::Bullet {
                level: 1usize,
                content: vec![
                    DocAttr::Raw(
                        "Must be the same unsigned type used by all instance counters: "
                            .to_string(),
                    ),
                    DocAttr::Inline(format!("{counter_names}")),
                ],
            },
            DocAttr::Bullet {
                level: 1usize,
                content: vec![
                    DocAttr::Raw("Expected type: ".to_string()),
                    DocAttr::Inline(format!("{}", expected_ty.to_token_stream())),
                ],
            },
        ]);

        Ok(item)
    }

    fn validate_extend(
        &self,
        item: Option<&TraitItemType>,
        towards: &ItemTrait,
        _: Option<&CounterParamsSlice<'a>>,
    ) -> Result<(), TokenStream> {
        let ident = format_ident!("{}", INSTANCE_COUNTER_TY);

        let validate_item = |ty_item: &TraitItemType| -> Result<(), TokenStream> {
            if ty_item.ident != ident {
                return Err(MetaTraitItemsBugs::CountersTyWrongIdent {}.into());
            }

            if !ty_item.generics.params.is_empty() {
                return Err(MetaTraitItemsBugs::CountersTyHasGenerics {}.into());
            }

            if !ty_item.bounds.is_empty() {
                return Err(MetaTraitItemsBugs::CountersTyHasBounds {}.into());
            }

            Ok(())
        };

        // Validate the produced item itself if any
        if let Some(item) = item {
            validate_item(item)?;
        }

        let mut found: Result<(), TokenStream> =
            Err(MetaTraitItemsBugs::CountersTyNotFound {}.into());

        // Validate already existing matching item inside the trait if present
        for i in &towards.items {
            if let TraitItem::Type(existing) = i {
                if existing.ident == ident {
                    validate_item(existing)?;
                    found = Ok(());
                    break;
                }
            }
        }

        found
    }
}

// ===============================================================================
// ````````````````````````` COUNTERS GENERIC REFLECTION `````````````````````````
// ===============================================================================

/// Extends an instance trait with hidden reflection metadata describing
/// the positions and identifiers of all extracted instance counters.
///
/// The generated metadata stores:
///
/// - the generic index of each counter parameter,
/// - and the original const-generic identifier name.
///
/// ## Example
///
/// Given:
///
/// ```ignore
/// #[trait_instance(0, 2)]
/// trait Example<const A: u8, const B: u8, const C: u8> {}
/// ```
///
/// where extraction determines:
///
/// ```text
/// A -> generic index 0
/// C -> generic index 2
/// ```
///
/// this extension generates:
///
/// ```ignore
/// #[trait_instance(0, 2)]
/// trait Example<const A: u8, const B: u8, const C: u8> {
///     #[doc(hidden)]
///     const __COUNTERS_GENERICS_META:
///         [(usize, &'static str); 2] = [
///             (0, "A"),
///             (2, "C"),
///         ];
/// }
/// ```
///
/// This metadata acts as a trait-side reflection mechanism allowing
/// impl-side proc-macro expansion to later recover:
///
/// - which generics are instance counters,
/// - where they exist in the generic parameter list,
/// - and what their original identifiers were.
///
/// The generated constant identifier is deterministically derived from
/// this phase type itself, making the metadata stable while remaining
/// hidden from external crate APIs.
#[derive(Clone, Debug)]
pub(crate) struct CountersGenericsMeta;

impl<'a> Extension<TraitItemConst, ItemTrait, CounterParamsSlice<'a>> for CountersGenericsMeta {
    fn validate_context(&self, context: &CounterParamsSlice<'a>) -> Result<(), TokenStream> {
        Type::validate_from(&context)
    }

    fn raw_extend(
        &self,
        _: &ItemTrait,
        context: &CounterParamsSlice<'a>,
    ) -> Result<TraitItemConst, TokenStream> {
        let ident = gen_const_ident::<Self>();

        let mut elems = Vec::<Expr>::new();
        for p in context.iter() {
            let idx = p.generic_index;
            let name = p.const_param.ident.to_string();
            elems.push(parse_quote!((#idx, #name)));
        }

        let expr = parse_quote!([ #( #elems ),* ]);
        let len = context.len();
        let ty: Type = parse_quote!([(usize, &'static str); #len]);

        let item = TraitItemConst {
            attrs: proc_suite::internal_code(),
            ident,
            default: Some((Default::default(), expr)),
            ty,
            const_token: Default::default(),
            generics: Default::default(),
            colon_token: Default::default(),
            semi_token: Default::default(),
        };

        Ok(item)
    }

    fn validate_extend(
        &self,
        item: Option<&TraitItemConst>,
        towards: &ItemTrait,
        context: Option<&CounterParamsSlice<'a>>,
    ) -> Result<(), TokenStream> {
        let ident = gen_const_ident::<Self>();

        let extracted;
        let counters = match context {
            Some(c) => c,
            None => {
                extracted = CounterParams::checked_extract(towards, &None)?;
                extracted.as_slice().as_ref().as_ref()
            }
        };

        Self::validate_context(&Self, &counters)?;

        let mut elems = Vec::<Expr>::new();

        for p in counters.iter() {
            let idx = p.generic_index;
            let name = p.const_param.ident.to_string();
            elems.push(parse_quote!((#idx, #name)));
        }

        let exp_expr: Expr = parse_quote!([ #( #elems ),* ]);
        let len = counters.len();
        let exp_ty: Type = parse_quote!([(usize, &'static str); #len]);

        validate_trait_const! {
            item: item,
            towards: towards,
            ident: ident,
            ty: exp_ty,
            expr: Some(exp_expr) => {
                not_found: MetaTraitItemsBugs::CountersGenericsMetaNotFound {},
                wrong_ident: MetaTraitItemsBugs::CountersGenericsMetaWrongIdent {},
                wrong_ty: MetaTraitItemsBugs::CountersGenericsMetaInvalidType {},
                has_generics: MetaTraitItemsBugs::CountersGenericsMetaHasGenerics {},
                missing_expr: MetaTraitItemsBugs::CountersGenericsMetaExprNotFound {},
                invalid_expr: MetaTraitItemsBugs::CountersGenericsMetaInvalidExpr {},
            }
        }
    }
}

// ===============================================================================
// ``````````````````` GENERIC INDEXES OF COUNTERS (FROM IMPL) ```````````````````
// ===============================================================================

/// Extends an instance trait with a hidden associated constant used to
/// restate impl-side instance-counter generic indexes.
///
/// Although [`CountersGenericsMeta`] reflects this information into the
/// trait using hidden metadata, impl-side expansion cannot directly
/// project that reflected metadata back into syntax-level generic layout.
///
/// To bridge this limitation, this phase generates a hidden associated
/// constant that impl-side macros must later define with their own view
/// of the instance-counter generic indexes.
///
/// ## Example
///
/// Trait-side expansion:
///
/// ```ignore
/// #[trait_instance(0, 2)]
/// trait Example<const A: u8, const B: u8, const C: u8> {
///     #[doc(hidden)]
///     const __COUNTERS_GENERIC_INDEXES_META: &'static [usize];
/// }
/// ```
///
/// Impl-side expansion later provides:
///
/// ```ignore
/// #[impl_instance(0, 2)]
/// impl Example<1, 2, 3> for MyType {
///     const __COUNTERS_GENERIC_INDEXES_META: &'static [usize] = &[0, 2];
/// }
/// ```
///
/// The proc-macro pipeline can then compare:
///
/// - the reflected trait-side metadata from [`CountersGenericsMeta`],
/// - against the impl-side provided indexes.
///
/// This enables validation that:
///
/// - the impl identified the correct instance counters,
/// - the generic ordering matches,
/// - and the impl-side expansion participates in the expected pipeline.
///
/// The generated identifier is deterministically derived from this phase
/// type itself, making it stable while remaining hidden from external
/// crate APIs.
#[derive(Clone, Debug)]
pub(crate) struct CountersGenericsIndexesMeta;

impl<'a> Insertion<ItemTrait> for TraitItemConst {
    fn raw_insert(&self, to: &mut ItemTrait, _: &()) -> Result<(), proc_macro2::TokenStream> {
        to.items.push(TraitItem::Const(self.clone()));
        Ok(())
    }

    fn validate_inserted(
        _of: Option<&Self>,
        _to: &ItemTrait,
        __: Option<&()>,
    ) -> Result<(), proc_macro2::TokenStream> {
        Ok(())
    }
}

impl Extension<TraitItemConst, ItemTrait> for CountersGenericsIndexesMeta {
    fn raw_extend(&self, _: &ItemTrait, _: &()) -> Result<TraitItemConst, TokenStream> {
        let ident = gen_const_ident::<Self>();

        let ty: Type = parse_quote!(&'static [usize]);

        let item = TraitItemConst {
            attrs: proc_suite::internal_code(),
            ident,
            default: None,
            ty,
            const_token: Default::default(),
            generics: Default::default(),
            colon_token: Default::default(),
            semi_token: Default::default(),
        };
        Ok(item)
    }

    fn validate_extend(
        &self,
        item: Option<&TraitItemConst>,
        towards: &ItemTrait,
        _: Option<&()>,
    ) -> Result<(), TokenStream> {
        let ident = gen_const_ident::<Self>();
        let exp_ty: Type = parse_quote!(&'static [usize]);

        validate_trait_const! {
            item: item,
            towards: towards,
            ident: ident,
            ty: exp_ty,
            expr: None => {
                not_found: MetaTraitItemsBugs::CountersGenericsIndexesMetaNotFound {},
                wrong_ident: MetaTraitItemsBugs::CountersGenericsIndexesMetaWrongIdent {},
                wrong_ty: MetaTraitItemsBugs::CountersGenericsIndexesMetaInvalidType {},
                has_generics: MetaTraitItemsBugs::CountersGenericsIndexesMetaHasGenerics {},
                has_expr: MetaTraitItemsBugs::CountersGenericsIndexesMetaHasExpr {},
            }
        }
    }
}

// ===============================================================================
// ````````````````````````` COUNTERS LENGTH (FROM IMPL) `````````````````````````
// ===============================================================================

/// Extends an instance trait with hidden reflection metadata representing
/// the expected total number of instance counters provided by impl-side
/// expansion.
///
/// Since [`CountersGenericsIndexesMeta`] provides all impl-side
/// instance-counter generic indexes as a slice, this phase provides the
/// expected slice length so later validation phases can ensure both
/// values remain structurally consistent.
///
/// In particular, [`CountersGenericsChecker`] uses this
/// metadata together with [`CountersGenericsMeta`] to validate:
///
/// - that the impl provided the correct number of instance counters,
/// - that the provided generic indexes match the reflected trait metadata,
/// - and that impl-side expansion participates in the expected pipeline.
///
/// ## Example
///
/// Given:
///
/// ```ignore
/// #[trait_instance(0, 2)]
/// trait Example<const A: u8, const B: usize, const C: u8> {}
/// ```
///
/// this extension generates:
///
/// ```ignore
/// trait Example<const A: u8, const B: usize, const C: u8> {
///     #[doc(hidden)]
///     const __COUNTERS_LEN_META: usize;
/// }
/// ```
///
/// Later:
///
/// ```ignore
/// #[impl_instance(0, 2)]
/// impl Example<1, 2, 3> for MyType {
///     const __COUNTERS_LEN_META: usize = 2;
/// }
/// ```
///
/// This metadata acts as a reflection mechanism allowing impl-side
/// validation phases to enforce instance-counter structural consistency
/// and provide meaningful diagnostics.
///
/// The generated identifier is deterministically derived from this phase
/// type itself, making it stable while remaining hidden from external
/// crate APIs.
#[derive(Clone, Debug)]
pub(crate) struct CountersLenMeta;

impl Extension<TraitItemConst, ItemTrait> for CountersLenMeta {
    fn raw_extend(&self, _: &ItemTrait, _: &()) -> Result<TraitItemConst, TokenStream> {
        let ident = gen_const_ident::<Self>();

        let ty: Type = parse_quote!(usize);

        let item = TraitItemConst {
            attrs: proc_suite::internal_code(),
            ident,
            default: None,
            ty,
            const_token: Default::default(),
            generics: Default::default(),
            colon_token: Default::default(),
            semi_token: Default::default(),
        };
        Ok(item)
    }

    fn validate_extend(
        &self,
        item: Option<&TraitItemConst>,
        towards: &ItemTrait,
        _: Option<&()>,
    ) -> Result<(), TokenStream> {
        let ident = gen_const_ident::<Self>();
        let exp_ty: Type = parse_quote!(usize);

        validate_trait_const! {
            item: item,
            towards: towards,
            ident: ident,
            ty: exp_ty,
            expr: None => {
                not_found: MetaTraitItemsBugs::CountersLenMetaNotFound {},
                wrong_ident: MetaTraitItemsBugs::CountersLenMetaWrongIdent {},
                wrong_ty: MetaTraitItemsBugs::CountersLenMetaInvalidType {},
                has_generics: MetaTraitItemsBugs::CountersLenMetaHasGenerics {},
                has_expr: MetaTraitItemsBugs::CountersLenMetaHasExpr {},
            }
        }
    }
}

// ===============================================================================
// ```````````````````` COUNTERS GENERICS CHECKER (FROM IMPL) ````````````````````
// ===============================================================================

/// Extends an instance trait with a hidden associated constant used as a
/// structural validation checkpoint for impl-side instance-counter
/// generic metadata.
///
/// This phase acts as a synchronization/checkpoint node between:
///
/// - [`CountersGenericsMeta`],
/// - [`CountersGenericsIndexesMeta`],
/// - [`CountersLenMeta`],
/// - and impl-side expansion.
///
/// The generated constant is later required on the impl side, allowing
/// the proc-macro pipeline to verify that:
///
/// - impl-side generic indexes were provided,
/// - the provided index count matches the expected length,
/// - the indexes match reflected trait-side metadata,
/// - and the impl participated in the expected expansion pipeline.
///
/// The constant uses unit type (`()`) because it exists purely as a
/// structural validation marker and carries no semantic value.
///
/// ## Example
///
/// Trait-side expansion:
///
/// ```ignore
/// trait Example<const A: u8, const B: usize, const C: u8> {
///     #[doc(hidden)]
///     const __COUNTERS_GENERICS_CHECKER: ();
/// }
/// ```
///
/// Impl-side expansion later provides:
///
/// ```ignore
/// #[impl_instance(0, 2)]
/// impl Example<1, 2, 3> for MyType {
///     const __COUNTERS_GENERICS_CHECKER: () = {
///         // assertions
///         ()
///     };
/// }
/// ```
///
/// The generated identifier is deterministically derived from this phase
/// type itself, making it stable while remaining hidden from external
/// crate APIs.
#[derive(Clone, Debug)]
pub(crate) struct CountersGenericsChecker;

impl Extension<TraitItemConst, ItemTrait> for CountersGenericsChecker {
    fn raw_extend(&self, _: &ItemTrait, _: &()) -> Result<TraitItemConst, TokenStream> {
        let ident = gen_const_ident::<Self>();
        let ty: Type = parse_quote!(());
        let item = TraitItemConst {
            attrs: proc_suite::internal_code(),
            ident,
            default: None,
            ty,
            const_token: Default::default(),
            generics: Default::default(),
            colon_token: Default::default(),
            semi_token: Default::default(),
        };
        Ok(item)
    }

    fn validate_extend(
        &self,
        item: Option<&TraitItemConst>,
        towards: &ItemTrait,
        _: Option<&()>,
    ) -> Result<(), TokenStream> {
        let ident = gen_const_ident::<Self>();
        let expected_ty: Type = parse_quote!(());

        validate_trait_const! {
            item: item,
            towards: towards,
            ident: ident,
            ty: expected_ty,
            expr: None => {
                not_found: MetaTraitItemsBugs::CountersGenericsCheckerNotFound {},
                wrong_ident: MetaTraitItemsBugs::CountersGenericsCheckerWrongIdent {},
                wrong_ty: MetaTraitItemsBugs::CountersGenericsCheckerNotUnitType {},
                has_generics: MetaTraitItemsBugs::CountersGenericsCheckerHasGenerics {},
                has_expr: MetaTraitItemsBugs::CountersGenericsCheckerHasExpr {},
            }
        }
    }
}

// ===============================================================================
// ```````````````````````` INSTANCE TRAIT DOCS (PREPEND) ````````````````````````
// ===============================================================================

/// Appends user-facing documentation describing how instance-counter
/// generic indexes may be specified when implementing the trait.
///
/// Instance-counter generics normally follow their declaration order and
/// can therefore be inferred automatically from the trait definition.
///
/// For example:
///
/// ```ignore
/// #[instance]
/// trait Example<const A: u8, const B: u8> {}
/// ```
///
/// maps to:
///
/// ```text
/// A -> generic index 0
/// B -> generic index 1
/// ```
///
/// Or in generic traits:
///
/// ```ignore
/// /// Counters A, B, since the next generic is a type generic and the
/// /// impl-side syntax remains unambiguous.
/// #[instance]
/// trait Example<const A: u8, const B: u8, T> {}
///
/// /// Counter A only, since the next generic is a type generic and the
/// /// impl-side syntax remains unambiguous.
/// #[instance]
/// trait Example<const A: u8, T, const B: u8, U> {}
/// ```
///
/// In these cases, no additional implementation guidance is necessary,
/// because the instance counters can be derived directly from their
/// position within the generic parameter list.
///
/// However, there are cases where the extracted instance counters do not
/// correspond to their natural ordering within the generic parameter
/// list, or where the impl-side syntax alone is insufficient to
/// determine which const generics represent instance counters.
///
/// In such situations, implementations must explicitly specify the
/// generic indexes that correspond to instance counters.
///
/// This phase appends documentation explaining the required
/// `#[instance_impl(...)]` syntax so that trait implementers can
/// correctly map instance counters to their corresponding generic
/// parameters.
///
/// ## Example
///
/// Given:
///
/// ```ignore
/// /// Some Module Comment
/// #[trait_instance(0, 2)]
/// trait Example<const A: u8, const X: usize, const B: u8> {}
/// ```
///
/// where extraction determines:
///
/// ```text
/// A -> generic index 0
/// B -> generic index 2
/// ```
///
/// the instance-counter mapping no longer follows the natural sequence
/// of extracted generics. In this case, implementations must explicitly
/// provide the counter indexes.
///
/// This phase appends documentation similar to:
///
/// ```ignore
/// /// Some Module Comment
/// ///
/// /// Quick Note:
/// /// When implementing this trait, use:
/// /// `#[instance_impl(0, 2)]`
/// trait Example<const A: u8, const X: usize, const B: u8> {}
/// ```
///
/// Edge cases also exist where the extracted counter set itself is
/// unambiguous from the trait definition, but the impl-side syntax is
/// not sufficient to determine which const generic corresponds to an
/// instance counter.
///
/// For example:
///
/// ```ignore
/// /// Only A is an instance counter, but on the impl side both A and X
/// /// may be represented using the same positive integer literal syntax,
/// /// making them syntactically indistinguishable without additional
/// /// guidance.
/// #[trait_instance]
/// trait Example<const A: u8, const X: usize, const B: u8> {}
///
/// /// Only A is an instance counter, but X may also be represented by a
/// /// positive integer literal on the impl side, resulting in the same
/// /// ambiguity.
/// #[trait_instance]
/// trait Example<const A: u8, const X: i8, const B: u8> {}
/// ```
///
/// In these cases, documentation is appended to guide the required
/// impl-side macro usage and ensure that instance-counter arguments are
/// interpreted correctly.
///
/// This phase only performs documentation transformation and mutation.
/// It does not generate reflection metadata or contribute to runtime
/// reflection output.
#[derive(Clone, Debug)]
pub(crate) struct InstanceTraitDocs;

impl InstanceTraitDocs {
    fn is_doc_required(counters: &[CounterParam], trait_of: &ItemTrait) -> bool {
        let params = &trait_of.generics.params;

        for (i, p) in counters.iter().enumerate() {
            if i != p.generic_index {
                return true;
            }
        }

        let neighbour = counters.len();

        let Some(GenericParam::Const(c)) = params.get(neighbour) else {
            return false;
        };

        let Type::Path(TypePath { qself: None, path }) = &c.ty else {
            return false;
        };

        let Some(seg) = path.segments.last() else {
            return false;
        };

        matches!(
            seg.ident.to_string().as_str(),
            "u8" | "u16" | "u32" | "u64" | "usize" | "i8" | "i16" | "i32" | "i64" | "isize"
        )
    }

    const INSTANCE_TRAIT_NOTE: &'static str = "**Quick Note:** Implementations of this trait may specify \
        instance-counter indexes using the instance-impl macro as ";

    fn macro_support(counters: &[CounterParam]) -> String {
        let mut counters_indexes = String::new();

        for (i, p) in counters.iter().enumerate() {
            if i > 0 {
                counters_indexes.push_str(", ");
            }
            counters_indexes.push_str(&p.generic_index.to_string());
        }
        format!("#[{}({})]", TRAIT_IMPL_MACRO_NAME, counters_indexes)
    }
}

impl<'a> Transformation<ItemTrait, CounterParamsSlice<'a>> for InstanceTraitDocs {
    fn validate_context(&self, context: &CounterParamsSlice<'a>) -> Result<(), TokenStream> {
        Type::validate_from(&context)
    }

    fn raw_transform(
        &self,
        transform: &mut ItemTrait,
        context: &CounterParamsSlice<'a>,
    ) -> Result<(), TokenStream> {
        if !Self::is_doc_required(context, transform) {
            return Ok(());
        }
        transform.append_docs(vec![
            DocAttr::Raw(Self::INSTANCE_TRAIT_NOTE.to_string()),
            DocAttr::Inline(Self::macro_support(context)),
        ]);
        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &ItemTrait,
        context: Option<&CounterParamsSlice<'a>>,
    ) -> Result<(), TokenStream> {
        let extracted;
        let counters = match context {
            Some(c) => c,
            None => {
                extracted = CounterParams::checked_extract(transform, &None)?;
                extracted.as_slice().as_ref().as_ref()
            }
        };

        Self::validate_context(&Self, &counters)?;

        if !Self::is_doc_required(counters, transform) {
            return Ok(());
        }

        let expected_raw = Self::INSTANCE_TRAIT_NOTE.to_string();
        let expected_macro = format!("`{}`", Self::macro_support(counters));

        let Some(raw) = transform.attrs.iter().rev().nth(1) else {
            return Err(MetaTraitItemsBugs::InstanceTraitDocsRawAttrMissing {}.into());
        };

        let Some(macro_) = transform.attrs.last() else {
            return Err(MetaTraitItemsBugs::InstanceTraitDocsMacroInlineAttrMissing {}.into());
        };

        for (index, (attr, expected)) in [(raw, expected_raw), (macro_, expected_macro)]
            .iter()
            .enumerate()
        {
            let Ok(meta) = attr.meta.require_name_value() else {
                return Err(MetaTraitItemsBugs::InstanceTraiDocsNotDocsAttr {}.into());
            };

            let syn::Expr::Lit(expr) = &meta.value else {
                return Err(MetaTraitItemsBugs::InstanceTraiDocsAttrNotLiteralExpr {}.into());
            };

            let syn::Lit::Str(value) = &expr.lit else {
                return Err(MetaTraitItemsBugs::InstanceTraiDocsAttrNotStrLiteral {}.into());
            };

            if value.value() != *expected {
                if index == 0 {
                    return Err(MetaTraitItemsBugs::InstanceTraitDocsInvalidRawAttr {}.into());
                }
                return Err(MetaTraitItemsBugs::InstanceTraitDocsInvalidMacroInlineAttr {}.into());
            }
        }

        Ok(())
    }
}

// ===============================================================================
// `````````````````````` ORIGINAL COUNTER CONST REFLECTION ``````````````````````
// ===============================================================================

/// Extends an instance trait with hidden associated constants representing
/// the original extracted counter const generics.
///
/// After [`InstanceTraitTypeNumCounters`](crate::traits::typenum::InstanceTraitTypeNumCounters)
/// transformation, instance-counter const generics are removed from the trait
/// signature and replaced by a synthesized type-level counter carrier.
///
/// Since later expansion stages or impl side may still need access to the
/// original counter values, this phase reflects each counter into a hidden
/// associated constant suffixed by its generic-index (since impls does not have
/// access to the original counter identifiers).
///
/// ## Example
///
/// Before transformation:
///
/// ```ignore
/// #[instance(0, 2)]
/// trait Example<const A: u8, T, const B: u8> {}
/// ```
///
/// After transformation and metadata expansion:
///
/// ```ignore
/// trait Example<__TypeNumCounters, T> {
///     #[doc(hidden)]
///     const __COUNTER_INDEX_0: u8;
///
///     #[doc(hidden)]
///     const __COUNTER_INDEX_2: u8;
/// }
/// ```
///
/// These generated constants provide a stable trait-associated representation
/// of the original counters that can be referenced by later expansion phases.
///
/// The generated identifier is deterministically derived from this phase
/// type itself, making it stable while remaining hidden from external
/// crate APIs.
#[derive(Debug, Clone)]
pub(crate) struct OriginalCounterConst;

impl<'a> Insertion<ItemTrait, CounterParamsSlice<'a>> for Vec<TraitItemConst> {
    fn raw_insert(
        &self,
        to: &mut ItemTrait,
        _: &CounterParamsSlice<'a>,
    ) -> Result<(), TokenStream> {
        for c in self {
            to.items.push(TraitItem::Const(c.clone()));
        }
        Ok(())
    }

    fn validate_inserted(
        _: Option<&Self>,
        _: &ItemTrait,
        _: Option<&CounterParamsSlice<'a>>,
    ) -> Result<(), TokenStream> {
        Ok(())
    }
}

impl<'a> Extension<Vec<TraitItemConst>, ItemTrait, CounterParamsSlice<'a>>
    for OriginalCounterConst
{
    fn raw_extend(
        &self,
        _towards: &ItemTrait,
        context: &CounterParamsSlice<'a>,
    ) -> Result<Vec<TraitItemConst>, proc_macro2::TokenStream> {
        let mut collect = Vec::new();
        for gen_idx in context.iter().map(|c| &c.generic_index) {
            let item = TraitItemConst {
                attrs: proc_suite::internal_code(),
                ident: gen_const_ident_with_suffix::<Self>(Some(gen_idx.to_string().as_bytes())),
                default: None,
                ty: CounterType::checked_extract(context, &())?,
                const_token: Default::default(),
                generics: Default::default(),
                colon_token: Default::default(),
                semi_token: Default::default(),
            };
            collect.push(item);
        }
        Ok(collect)
    }

    fn validate_extend(
        &self,
        item: Option<&Vec<TraitItemConst>>,
        towards: &ItemTrait,
        context: Option<&CounterParamsSlice<'a>>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let extracted;
        let counters = match context {
            Some(c) => c,
            None => {
                extracted = CounterParams::checked_extract(towards, &None)?;
                extracted.as_slice().as_ref().as_ref()
            }
        };
        let exp_ty = CounterType::checked_extract(&counters, &())?;
        for gen_idx in counters.iter().map(|c| &c.generic_index) {
            let ident = gen_const_ident_with_suffix::<Self>(Some(gen_idx.to_string().as_bytes()));
            validate_trait_consts! {
                items: item,
                towards: towards,
                ident: ident,
                ty: exp_ty,
                expr: None => {
                    not_found: MetaTraitItemsBugs::OriginalCounterConstNotFound {},
                    wrong_ident: MetaTraitItemsBugs::OriginalCounterConstWrongIdent {},
                    wrong_ty: MetaTraitItemsBugs::OriginalCounterConstWrongType {},
                    has_generics: MetaTraitItemsBugs::OriginalCounterConstHasGenerics {},
                    has_expr: MetaTraitItemsBugs::OriginalCounterConstHasExpr {},
                }
            }
        }
        Ok(())
    }
}

// ===============================================================================
// ``````````````````````````` CUMULATED CONST CHECKER ```````````````````````````
// ===============================================================================

/// Declares the root compile-time validation checkpoint.
///
/// Implementations are expected to provide a unit-valued constant that
/// accumulates all generated validation checkers.
///
/// ## Purpose
///
/// Most generated validation constants are intentionally lazy.
///
/// This allows expansion to remain compatible with generic contexts
/// where eager const evaluation would be undesirable or impossible.
///
/// Consequently, individual checkers are not guaranteed to execute
/// unless they are explicitly referenced.
///
/// `CumulatedConstChecker` serves as the single evaluation entry point
/// that forces all generated validation phases to participate in
/// compile-time checking.
///
/// ## Conceptually
///
/// Implementations construct:
///
/// ```text
/// CumulatedConstChecker
///     -> CheckerA
///     -> CheckerB
///     -> CheckerC
///     -> ...
/// ```
///
/// where each checker may itself recursively evaluate additional
/// affiliate or lineage checkers.
///
/// ## Affiliate Propagation
///
/// Some generated checkers recurse through affiliate relationships:
///
/// ```text
/// Current
///     -> Back(Current)
///         -> Back(Back(Current))
///             -> ...
/// ```
///
/// As a result, evaluating the terminal instance's
/// `CumulatedConstChecker` can force validation of the entire reachable
/// instance graph.
///
/// ## Why this exists
///
/// Rust const evaluation is demand-driven.
///
/// Since generated validation constants are unit-valued and primarily
/// exist for their side effects (compile-time assertions), a dedicated
/// accumulation point is required to guarantee that every validation
/// phase is actually evaluated.
///
/// This constant provides that enforcement point.
#[derive(Debug, Clone)]
pub(crate) struct CumulatedConstChecker;

impl Extension<TraitItemConst, ItemTrait> for CumulatedConstChecker {
    fn raw_extend(&self, _: &ItemTrait, _: &()) -> Result<TraitItemConst, TokenStream> {
        let ident = gen_const_ident::<Self>();
        let ty: Type = parse_quote!(());
        let item = TraitItemConst {
            attrs: proc_suite::internal_code(),
            ident,
            default: None,
            ty,
            const_token: Default::default(),
            generics: Default::default(),
            colon_token: Default::default(),
            semi_token: Default::default(),
        };
        Ok(item)
    }

    fn validate_extend(
        &self,
        item: Option<&TraitItemConst>,
        towards: &ItemTrait,
        _: Option<&()>,
    ) -> Result<(), TokenStream> {
        let ident = gen_const_ident::<Self>();
        let expected_ty: Type = parse_quote!(());

        validate_trait_const! {
            item: item,
            towards: towards,
            ident: ident,
            ty: expected_ty,
            expr: None => {
                not_found: MetaTraitItemsBugs::CumulatedConstCheckerNotFound {},
                wrong_ident: MetaTraitItemsBugs::CumulatedConstCheckerWrongIdent {},
                wrong_ty: MetaTraitItemsBugs::CumulatedConstCheckerNotUnitType {},
                has_generics: MetaTraitItemsBugs::CumulatedConstCheckerHasGenerics {},
                has_expr: MetaTraitItemsBugs::CumulatedConstCheckerHasExpr {},
            }
        }
    }
}

// ===============================================================================
// ```````````````````````` COUNTER PARAMS META EXTRACTION ```````````````````````
// ===============================================================================

/// Reconstructs counter parameters from trait-side metadata after
/// counter-generic replacement.
///
/// During expansion, the original const generic counter parameters are
/// replaced by a single typenum counter type parameter. Consequently,
/// the original counter parameters are no longer available from the
/// transformed trait signature.
///
/// To make them recoverable, earlier metadata phases store:
///
/// - the counter generic indexes and identifiers in
///   [`CountersGenericsMeta`], and
/// - the counter value type in [`CountersTyMeta`].
///
/// Example:
///
/// ```ignore
/// trait Example<Counter> {
///     const __COUNTERS_GENERICS_META:
///         [(usize, &'static str); 2] = [
///             (1, "X"),
///             (3, "Z"),
///         ];
///
///     const __COUNTERS_TY_META: &'static str = "u8";
/// }
/// ```
///
/// From this metadata the extraction reconstructs:
///
/// ```ignore
/// const X: u8,
/// const Z: u8
/// ```
///
/// as [`CounterParam`]s, allowing later expansion phases to regain the
/// original counter parameter information after replacement.
///
/// When the original [`InstanceArgs`] are available, validation also
/// verifies that the reconstructed parameters remain consistent with the
/// user-supplied counter indexes or identifiers.
#[derive(Debug, Clone)]
pub(crate) struct CountersParamMetaExtraction(pub(crate) CounterParams);

impl Extraction<ItemTrait, Option<&InstanceArgs>> for CountersParamMetaExtraction {
    fn raw_extract(
        from: &ItemTrait,
        _: &Option<&InstanceArgs>,
    ) -> Result<Self, proc_macro2::TokenStream> {
        let mut collect = Vec::new();

        let meta_ident = gen_const_ident::<CountersGenericsMeta>();
        let ty_ident = gen_const_ident::<CountersTyMeta>();

        let mut meta_const: Result<&TraitItemConst, TokenStream> =
            Err(MetaTraitItemsBugs::CountersParamMetaExtractionGenericsMetaNotFound {}.into());
        let mut ty_const: Result<&TraitItemConst, TokenStream> =
            Err(MetaTraitItemsBugs::CountersParamMetaExtractionCountersTyMetaNotFound {}.into());

        for item in &from.items {
            let TraitItem::Const(c) = item else {
                continue;
            };

            if c.ident == meta_ident {
                meta_const = Ok(&c);
                continue;
            }

            if c.ident == ty_ident {
                ty_const = Ok(&c);
                continue;
            }
        }

        let const_item = meta_const?;
        let ty_item = ty_const?;

        let Some((_, expr)) = &ty_item.default else {
            return Err(
                MetaTraitItemsBugs::CountersParamMetaExtractionCountersTyMetaDefaultExprNotFound {}
                    .into(),
            );
        };

        let Expr::Lit(lit) = expr else {
            return Err(MetaTraitItemsBugs::CountersParamMetaExtractionCountersTyMetaDefaultExprNotLitExpr {}.into());
        };

        let Lit::Str(str) = &lit.lit else {
            return Err(MetaTraitItemsBugs::CountersParamMetaExtractionCountersTyMetaDefaultExprNotStrLit {}.into());
        };

        let counters_ty_ident = format_ident!("{}", &str.value());
        let counters_ty: Type = parse_quote!(#counters_ty_ident);

        let Some((_, expr)) = &const_item.default else {
            return Err(
                MetaTraitItemsBugs::CountersParamMetaExtractionGenericsMetaDefaultExprNotFound {}
                    .into(),
            );
        };

        let Expr::Array(arr) = expr else {
            return Err(
                MetaTraitItemsBugs::CountersParamMetaExtractionGenericsMetaDefaultExprNotArray {}
                    .into(),
            );
        };
        for elem in &arr.elems {
            let Expr::Tuple(tup) = elem else {
                return Err(MetaTraitItemsBugs::CountersParamMetaExtractionGenericsMetaDefaultExprArrayElemNotTuple {}.into());
            };

            if tup.elems.len() != 2 {
                return Err(MetaTraitItemsBugs::CountersParamMetaExtractionGenericsMetaDefaultExprArrayElemTupleWrongLen {}.into());
            }

            let Some(idx) = tup.elems.get(0) else {
                return Err(MetaTraitItemsBugs::CountersParamMetaExtractionGenericsMetaDefaultExprArrayElemTupleCounterGenericIndexNotFound {}.into());
            };

            let Expr::Lit(lit) = idx else {
                return Err(MetaTraitItemsBugs::CountersParamMetaExtractionGenericsMetaDefaultExprArrayElemTupleCounterGenericIndexNotExprLit {}.into());
            };

            let Lit::Int(int) = &lit.lit else {
                return Err(MetaTraitItemsBugs::CountersParamMetaExtractionGenericsMetaDefaultExprArrayElemTupleCounterGenericIndexNotLitInt {}.into());
            };

            let Some(ident) = tup.elems.get(1) else {
                return Err(MetaTraitItemsBugs::CountersParamMetaExtractionGenericsMetaDefaultExprArrayElemTupleCounterGenericIdentNotFound {}.into());
            };

            let Expr::Lit(lt) = ident else {
                return Err(MetaTraitItemsBugs::CountersParamMetaExtractionGenericsMetaDefaultExprArrayElemTupleCounterGenericIdentNotExprLit {}.into());
            };

            let Lit::Str(str) = &lt.lit else {
                return Err(MetaTraitItemsBugs::CountersParamMetaExtractionGenericsMetaDefaultExprArrayElemTupleCounterGenericIdentNotLitStr {}.into());
            };

            let generic_index = parse_pos_usize(&int)?;
            let ident = format_ident!("{}", &str.value());

            let const_param = ConstParam {
                attrs: proc_suite::internal_code(),
                const_token: Default::default(),
                ident,
                colon_token: Default::default(),
                ty: counters_ty.clone(),
                default: None,
                eq_token: Default::default(),
            };

            collect.push(CounterParam {
                generic_index,
                const_param,
            });
        }

        Ok(Self(collect))
    }

    fn validate_extract(
        &self,
        _: &ItemTrait,
        context: Option<&Option<&InstanceArgs>>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let context = match context {
            Some(Some(c)) => c,
            _ => return Ok(()),
        };

        match context {
            InstanceArgs::Ident(idents) => {
                for (a, b) in idents
                    .idents
                    .iter()
                    .zip(self.0.iter().map(|c| &c.const_param.ident))
                {
                    if a != b {
                        return Err(MetaTraitItemsBugs::CountersParamMetaExtractionInconsistentWithGenericIdentArgs {}.into());
                    }
                }
            }
            InstanceArgs::Index(ints) => {
                for (lit, b) in ints.ints.iter().zip(self.0.iter().map(|c| c.generic_index)) {
                    let a = parse_pos_usize(lit)?;
                    if a != b {
                        return Err(MetaTraitItemsBugs::CountersParamMetaExtractionInconsistentWithGenericIndexArgs {}.into());
                    }
                }
            }
        }

        Ok(())
    }
}

// ===============================================================================
// ```````````````````````````` MARKER SUPERBOUND META ```````````````````````````
// ===============================================================================

/// **This phase currently not utilized**
///
/// Adds and validates the `Marker` superbound on an instance trait.
///
/// Every generated instance trait requires its implementing type to satisfy
/// `Marker`.
///
/// The transformation inserts:
///
/// ```ignore
/// where
///     Self: Marker
/// ```
///
/// The validation phase verifies that the generated trait contains the
/// expected `Self: Marker` predicate. This ensures that the marker contract
/// remains present after all trait-side transformations have completed.
#[derive(Debug, Clone)]
#[allow(unused)]
pub(crate) struct MarkerSuperBound;

impl Transformation<ItemTrait> for MarkerSuperBound {
    fn raw_transform(
        &self,
        transform: &mut ItemTrait,
        _: &(),
    ) -> Result<(), proc_macro2::TokenStream> {
        let crate_of = Instance::support_crate();
        transform
            .generics
            .make_where_clause()
            .predicates
            .push(parse_quote!(Self: #crate_of::Marker));
        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &ItemTrait,
        _: Option<&()>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let crate_of = Instance::support_crate();
        let marker = &parse_quote!(#crate_of::Marker);
        let Some(where_clause) = &transform.generics.where_clause else {
            return Err(MetaTraitItemsBugs::WhereClauseNotFoundForMarkerBound {}.into());
        };
        let preds = &where_clause.predicates;
        let mut found = false;
        for pred in preds {
            let WherePredicate::Type(ty) = pred else {
                continue;
            };
            if ty.bounded_ty != parse_quote!(Self) {
                continue;
            }
            let bounds = &ty.bounds;
            for bound in bounds {
                let TypeParamBound::Trait(trait_) = bound else {
                    continue;
                };
                if trait_ != marker {
                    continue;
                }
                found = true;
            }
        }
        if !found {
            return Err(MetaTraitItemsBugs::MarkerBoundPredNotFound {}.into());
        }
        Ok(())
    }
}
