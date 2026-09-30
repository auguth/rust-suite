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
// ```````````````````` INSTANCE ASSOCIATE ENUMS (SUM TYPES) `````````````````````
// ===============================================================================

//! Provides sum-type boundaries for instance-dependent associated types.
//!
//! Every instance trait is extended with three hidden associated types:
//! [`GlobalTerminalAssoc`], [`GlobalTerminalExact`], and [`SelfTerminalAssoc`]. Together,
//! these identify the terminal instance associated with `Self`.
//!
//! Associated types marked with `#[sum]` are linked to the corresponding
//! associated type of [`SelfTerminalAssoc`] through an associated-type equality
//! constraint. The terminal-anchored instance is therefore required to expose
//! the same associated types as the current instance.
//!
//! This makes the terminal instance a canonical type-level boundary rather
//! than merely a conversion boundary. A `#[sum]` associated type on `Self` and
//! the corresponding associated type on [`SelfTerminalAssoc`] are required to
//! be the same type.
//!
//! The resulting sum type is therefore represented by a common terminal
//! instance: different instance implementations may resolve through different
//! instance anchors, while their `#[sum]` associated types are unified with
//! the corresponding associated types of the terminal-anchored instance.
//!
//! In addition to establishing these equality relationships, this module
//! removes the `#[sum]` marker after expansion so that it does not remain in
//! the resulting trait definition.

// ===============================================================================
// ``````````````````````````````````` IMPORTS ```````````````````````````````````
// ===============================================================================

// --- Proc-Macro crates ---
use proc_macro2::TokenStream;
use syn::{
    AngleBracketedGenericArguments, GenericArgument, ItemTrait, TraitBound, TraitItem,
    TraitItemType, Type, parse_quote, punctuated::Punctuated,
};

// --- Local Crate ---
use crate::{
    Instance, Transformation,
    traits::{
        counters::CounterParamsSlice,
        errors::{SumTypeBugs, SumTypeErr},
        utils::*,
    },
};

// --- Proc-Suite ---
use proc_suite::{misc::*, pipeline::SupportCrate};

// ===============================================================================
// ```````````````````````````````` MARKER STRUCTS ```````````````````````````````
// ===============================================================================

/// Identifies the global instance associated with the current instance.
///
/// For example, for:
///
/// ```ignore
/// trait Example<I> { /* ... */ }
/// ```
///
/// `GlobalTerminalAssoc` represents the associated type that provides the global
/// instance for `Self`:
///
/// ```ignore
/// type GlobalTerminalAssoc: TerminalAccess<Self>;
/// ```
///
/// typically is `instances::Global` of the support crate of this
/// proc macro crate.
pub(crate) struct GlobalTerminalAssoc;

/// Identifies the exact terminal counter of the global instance.
///
/// The terminal counter is obtained from [`GlobalTerminalAssoc`] and wrapped in
/// `Exact`, giving the concrete counter used to construct the terminal
/// instance.
///
/// For example:
///
/// ```ignore
/// type GlobalTerminalExact:
///     Exact<
///         <Self::GlobalTerminalAssoc as TerminalAccess<Self>>::Terminal
///     >;
/// ```
pub(crate) struct GlobalTerminalExact;

/// Identifies the current trait instantiated with its terminal counter.
///
/// The instance counter corresponding to the first instance parameter is
/// replaced with [`GlobalTerminalExact`].
///
/// For `#[sum]` associated types, the corresponding associated types of the
/// terminal-anchored instance are additionally constrained to be equal to
/// those of `Self`.
///
/// For example, given:
///
/// ```ignore
/// trait Example<I> {
///     #[sum]
///     type Value;
/// }
/// ```
///
/// this represents:
///
/// ```ignore
/// type SelfTerminalAssoc:
///     Example<
///         Self::GlobalTerminalExact,
///         Value = Self::Value,
///     >;
/// ```
///
/// Thus, [`SelfTerminalAssoc`] represents the current trait instantiated at
/// the terminal counter while preserving the `#[sum]` associated types through
/// explicit associated-type equality constraints.
///
/// Usually, it is the `Self` implementation of the instance trait itself.
pub(crate) struct SelfTerminalAssoc;

// ===============================================================================
// `````````````````````````````` INSTANCE SUM TYPES `````````````````````````````
// ===============================================================================

/// Adds sum-type support to an instance trait by applying
/// [`SumTypeExtension`] with its instance counter parameters.
#[derive(Clone, Debug)]
pub(crate) struct InstanceSumTypes;

impl<'a> Transformation<ItemTrait, CounterParamsSlice<'a>> for InstanceSumTypes {
    fn raw_transform(
        &self,
        transform: &mut ItemTrait,
        context: &CounterParamsSlice<'a>,
    ) -> Result<(), TokenStream> {
        SumTypeExtension::checked_transform(&SumTypeExtension, transform, context)?;
        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &ItemTrait,
        context: Option<&CounterParamsSlice<'a>>,
    ) -> Result<(), TokenStream> {
        SumTypeExtension::validate_transform(&SumTypeExtension, transform, context)?;
        Ok(())
    }
}

// ===============================================================================
// `````````````````````````````` SUM TYPE EXTENSION `````````````````````````````
// ===============================================================================

/// Extends an instance trait with its terminal-anchored sum-type machinery.
///
/// For every instance trait, this extension adds three hidden associated types:
///
/// - [`GlobalTerminalAssoc`] - the global instance associated with `Self`.
/// - [`GlobalTerminalExact`] - the exact terminal counter obtained from
///   [`GlobalTerminalAssoc`].
/// - [`SelfTerminalAssoc`] - the trait instance obtained by replacing the
///   instance counter with [`GlobalTerminalExact`] and constraining each
///   `#[sum]` associated type to be equal to the corresponding associated
///   type of `Self`.
///
/// For example, given:
///
/// ```ignore
/// trait Example<I> {
///     #[sum]
///     type Value;
/// }
/// ```
///
/// the extension conceptually adds:
///
/// ```ignore
/// trait Example<I> {
///     type GlobalTerminalAssoc: TerminalAccess<Self>;
///
///     type GlobalTerminalExact:
///         Exact<<Self::GlobalTerminalAssoc as TerminalAccess<Self>>::Terminal>;
///
///     type SelfTerminalAssoc:
///         Example<
///             Self::GlobalTerminalExact,
///             Value = Self::Value,
///         >;
/// }
/// ```
///
/// Thus, [`SelfTerminalAssoc`] represents the same instance trait anchored to
/// the terminal counter, with its `#[sum]` associated types explicitly unified
/// with those of `Self`.
///
/// The resulting relationship is:
///
/// ```text
/// Self
///   |-- GlobalTerminalAssoc
///          |-- GlobalTerminalExact
///                    |
///             SelfTerminalAssoc
///                    | associated-type equality
///              Self::#[sum] types
/// ```
///
/// For every `#[sum]` associated type `Assoc`, the generated bound establishes:
///
/// ```text
/// <Self::SelfTerminalAssoc as Trait<..., Self::GlobalTerminalExact, ...>>::Assoc
///     ==
/// Self::Assoc
/// ```
///
/// Therefore, the instance-associated type and its terminal-instance
/// counterpart are the same type rather than merely being convertible types.
///
/// This equality is established at the trait level, allowing the terminal
/// associated type to be used wherever the corresponding instance associated
/// type is expected without requiring an explicit `From` or `Into` conversion.
///
/// `SumTypeExtension` is responsible for establishing these type-level
/// equality relationships; it does not itself define any runtime conversions.
#[derive(Clone, Debug)]
pub(crate) struct SumTypeExtension;

impl<'a> Transformation<ItemTrait, CounterParamsSlice<'a>> for SumTypeExtension {
    fn raw_transform(
        &self,
        transform: &mut ItemTrait,
        context: &CounterParamsSlice<'a>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let trait_ident = &transform.ident;
        let crate_of = Instance::support_crate();

        let Some(first) = context.first() else {
            return Err(SumTypeBugs::CounterParamsAreEmpty {}.into());
        };

        let (_, ty_gen, _) = transform.generics.split_for_impl();

        // GlobalTerminalAssoc

        let global_assoc = gen_type_ident::<GlobalTerminalAssoc>();

        transform.items.push(TraitItem::Type(TraitItemType {
            attrs: proc_suite::internal_code(),
            ident: global_assoc.clone(),
            default: None,
            type_token: Default::default(),
            generics: Default::default(),
            colon_token: Default::default(),
            bounds: {
                let mut bounds = Punctuated::new();

                bounds.push(parse_quote!(
                    #crate_of::TerminalAccess<Self>
                ));

                bounds
            },
            semi_token: Default::default(),
        }));

        // GlobalTerminalExact

        let terminal_counter = gen_type_ident::<GlobalTerminalExact>();

        transform.items.push(TraitItem::Type(TraitItemType {
            attrs: proc_suite::internal_code(),
            ident: terminal_counter.clone(),
            default: None,
            type_token: Default::default(),
            generics: Default::default(),
            colon_token: Default::default(),
            bounds: {
                let mut bounds = Punctuated::new();

                bounds.push(parse_quote!(
                    #crate_of::Exact<
                        <Self::#global_assoc as #crate_of::TerminalAccess<Self>>::Terminal
                    >
                ));

                bounds
            },
            semi_token: Default::default(),
        }));

        // SelfTerminalAssoc

        let sum_inst = gen_type_ident::<SelfTerminalAssoc>();

        let mut sum_inst_args: AngleBracketedGenericArguments = parse_quote!(#ty_gen);

        let Some(arg) = sum_inst_args.args.iter_mut().nth(first.generic_index) else {
            return Err(SumTypeBugs::TypeNumGenericNotFound {}.into());
        };

        *arg = GenericArgument::Type(parse_quote!(Self::#terminal_counter));

        let mut collect = Vec::new();
        for item in &transform.items {
            let TraitItem::Type(ty) = item else {
                continue;
            };

            if !ty.attrs.iter().any(|attr| attr.path().is_ident("sum")) {
                continue;
            }

            let ty_ident = &ty.ident;

            collect.push(ty_ident.clone());
        }

        let mut new_args = sum_inst_args.clone();
        for ident in &collect {
            new_args.args.push(parse_quote!(#ident = Self::#ident));
        }

        transform.items.push(TraitItem::Type(TraitItemType {
            attrs: proc_suite::internal_code(),
            ident: sum_inst.clone(),
            default: None,
            type_token: Default::default(),
            generics: Default::default(),
            colon_token: Default::default(),
            bounds: {
                let mut bounds = Punctuated::new();

                bounds.push(parse_quote!(
                    #trait_ident #new_args
                ));

                bounds
            },
            semi_token: Default::default(),
        }));

        for item in &mut transform.items {
            let TraitItem::Type(ty) = item else {
                continue;
            };

            let ty_ident = &ty.ident;

            if !collect.iter().any(|try_ident| *try_ident == *ty_ident) {
                continue;
            }

            let output: Type =
                parse_quote!(<Self::#sum_inst as #trait_ident #sum_inst_args>::#ty_ident);
            ty.bounds.push(parse_quote!(From<#output>));
            ty.bounds.push(parse_quote!(Into<#output>));
        }

        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &ItemTrait,
        context: Option<&CounterParamsSlice<'a>>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let crate_of = Instance::support_crate();

        let global_assoc = gen_type_ident::<GlobalTerminalAssoc>();
        let terminal_counter = gen_type_ident::<GlobalTerminalExact>();
        let sum_inst = gen_type_ident::<SelfTerminalAssoc>();

        let (_, ty_gen, _) = transform.generics.split_for_impl();

        // GlobalTerminalAssoc

        let global_assoc_bound: TraitBound = parse_quote!(
            #crate_of::TerminalAccess<Self>
        );

        validate_trait_type! {
            item: None,
            towards: transform,
            ident: global_assoc,
            bounds: vec![&global_assoc_bound] => {
                not_found: SumTypeBugs::GlobalAssocNotFound {},
                wrong_ident: SumTypeBugs::GlobalAssocWrongIdent {},
                has_generics: SumTypeBugs::GlobalAssocHasGenerics {},
                bound_not_found: SumTypeBugs::GlobalAssocMissingBound {},
            }
        }?;

        // GlobalTerminalExact

        let terminal_counter_bound: TraitBound = parse_quote!(
            #crate_of::Exact<
                <Self::#global_assoc as #crate_of::TerminalAccess<Self>>::Terminal
            >
        );

        validate_trait_type! {
            item: None,
            towards: transform,
            ident: terminal_counter,
            bounds: vec![&terminal_counter_bound] => {
                not_found: SumTypeBugs::GlobalTerminalExactNotFound {},
                wrong_ident: SumTypeBugs::GlobalTerminalExactWrongIdent {},
                has_generics: SumTypeBugs::GlobalTerminalExactHasGenerics {},
                bound_not_found: SumTypeBugs::GlobalTerminalExactMissingBound {},
            }
        }?;

        // `SelfTerminalAssoc` must use the same trait as the current trait.
        // The equality bindings (`Assoc = Self::Assoc`) are intentionally not
        // included in `sum_inst_args`; they are validated separately below.

        let trait_ident = &transform.ident;

        validate_trait_type! {
            item: None,
            towards: transform,
            ident: sum_inst,
            bounds: Vec::<&TraitBound>::new() => {
                not_found: SumTypeBugs::SelfTerminalNotFound {},
                wrong_ident: SumTypeBugs::SelfTerminalWrongIdent {},
                has_generics: SumTypeBugs::SelfTerminalHasGenerics {},
                bound_not_found: SumTypeBugs::SelfTerminalMissingBound {},
            }
        }?;

        // SelfTerminalAssoc
        let Some(context) = context else {
            return Ok(());
        };

        let Some(first) = context.first() else {
            return Err(SumTypeBugs::CounterParamsAreEmpty {}.into());
        };

        let mut sum_inst_args: AngleBracketedGenericArguments = parse_quote!(#ty_gen);

        let Some(arg) = sum_inst_args.args.iter_mut().nth(first.generic_index) else {
            return Err(SumTypeBugs::TypeNumGenericNotFound {}.into());
        };

        *arg = GenericArgument::Type(parse_quote!(Self::#terminal_counter));

        let mut collect = Vec::new();
        for item in &transform.items {
            let TraitItem::Type(ty) = item else {
                continue;
            };

            if !ty.attrs.iter().any(|attr| attr.path().is_ident("sum")) {
                continue;
            }

            let ty_ident = &ty.ident;

            collect.push(ty_ident);
        }

        for ident in &collect {
            sum_inst_args.args.push(parse_quote!(#ident = Self::#ident));
        }

        // Validate the trait bound of `SelfTerminalAssoc` manually.
        //
        // The expected bound is:
        //
        //     Trait<terminal arguments, Assoc = Self::Assoc, ...>
        //
        // Only the ordinary generic arguments are validated here. The associated-type
        // equality bindings are generated from `collect` and are deliberately handled
        // separately.

        let sum_inst_item = transform.items.iter().find_map(|item| {
            let TraitItem::Type(ty) = item else {
                return None;
            };

            (ty.ident == sum_inst).then_some(ty)
        });

        let Some(sum_inst_item) = sum_inst_item else {
            return Err(SumTypeBugs::SelfTerminalNotFound {}.into());
        };

        let Some(trait_bound) = sum_inst_item.bounds.iter().find_map(|bound| {
            let syn::TypeParamBound::Trait(bound) = bound else {
                return None;
            };

            (bound.path.segments.last()?.ident == *trait_ident).then_some(bound)
        }) else {
            return Err(SumTypeBugs::SelfTerminalMissingBound {}.into());
        };

        let Some(segment) = trait_bound.path.segments.last() else {
            return Err(SumTypeBugs::SelfTerminalMissingBound {}.into());
        };

        let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
            return Err(SumTypeBugs::SelfTerminalInvalidBound {}.into());
        };

        // `sum_inst_args` contains only the ordinary generic arguments.
        // `args` contains those arguments followed by the associated-type equality
        // bindings. Therefore, validate the ordinary arguments by comparing only
        // the prefix corresponding to `sum_inst_args`.

        let expected_args = &sum_inst_args.args;
        let actual_args = &args.args;

        if !(expected_args.len() <= actual_args.len()) {
            return Err(SumTypeBugs::SelfTerminalInvalidBound {}.into());
        }

        for (expected, actual) in expected_args.iter().zip(actual_args.iter()) {
            if expected != actual {
                return Err(SumTypeBugs::SelfTerminalInvalidBound {}.into());
            }
        }

        // The remaining arguments are the expected `#[sum]` associated-type
        // equality bindings:
        //
        //     Assoc = Self::Assoc
        //
        // Their order is not significant. Each expected equality is therefore
        // searched for independently among the actual associated-type arguments.
        // The actual bound may contain additional non-equality arguments before,
        // between, or after them.

        let actual_equalities = actual_args.iter().filter_map(|arg| {
            let GenericArgument::AssocType(assoc) = arg else {
                return None;
            };

            Some(assoc)
        });

        for ident in &collect {
            let expected_type: Type = parse_quote!(Self::#ident);

            let Some(_) = actual_equalities
                .clone()
                .find(|assoc| assoc.ident == **ident && assoc.ty == expected_type)
            else {
                return Err(SumTypeBugs::SelfTerminalMissingBound {}.into());
            };
        }

        Ok(())
    }
}

// ===============================================================================
// ``````````````````````````````` SUM ATTR REMOVAL ``````````````````````````````
// ===============================================================================

/// Removes the internal `#[sum]` marker from associated types.
///
/// And implies the associated type as a sum type via documentation regardless
/// of the target usage.
#[derive(Clone, Debug)]
pub(crate) struct SumAttrRemoval;

impl<'a> Transformation<ItemTrait> for SumAttrRemoval {
    fn raw_transform(
        &self,
        transform: &mut ItemTrait,
        _: &(),
    ) -> Result<(), proc_macro2::TokenStream> {
        for item in &mut transform.items {
            let TraitItem::Type(ty) = item else {
                let attrs = match item {
                    TraitItem::Const(c) => &c.attrs,
                    TraitItem::Fn(f) => &f.attrs,
                    TraitItem::Macro(m) => &m.attrs,
                    _ => {
                        continue;
                    }
                };

                if attrs.iter().any(|attr| attr.path().is_ident("sum")) {
                    return Err(SumTypeErr::SumOnlyForAssocTypes { item: item.clone() }.into());
                };

                continue;
            };

            if ty.attrs.iter().any(|attr| attr.path().is_ident("sum")) {
                // Remove #[sum], preserving all other attributes.
                ty.attrs.retain(|attr| !attr.path().is_ident("sum"));
            }
        }
        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &ItemTrait,
        _: Option<&()>,
    ) -> Result<(), proc_macro2::TokenStream> {
        for item in &transform.items {
            let TraitItem::Type(ty) = item else {
                continue;
            };

            for attr in &ty.attrs {
                if attr.path().is_ident("sum") {
                    return Err(SumTypeBugs::SumAttrNotRemoved {}.into());
                }
            }
        }
        Ok(())
    }
}
