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
// ````````````````````` INSTANCE IMPL & LAST INSTANCE ADDONS ````````````````````
// ===============================================================================

//! Generates supplementary implementations alongside each expanded
//! instance implementation.
//!
//! Unlike associated items injected directly into an instance `impl`,
//! this module emits additional standalone implementations that extend
//! the compile-time reflection and lookup capabilities of the generated
//! instance system.
//!
//! These addon implementations are derived from information already
//! available on an expanded instance implementation, such as its counter
//! metadata, identifier expressions, implementing `Self` type, and
//! user-provided self bounds.
//!
//! Example:
//!
//! ```ignore
//! #[instance_impl(1, 2, 3)]
//! #[self_bounds(R: MarkerTrait)]
//! impl<R: MarkerTrait> Example<0, 1, 2> for Phantom<R> {
//!     #[counter(1)]
//!     const X_IDENT: &'static [u8] = b"x";
//!
//!     #[counter(2)]
//!     const Y_IDENT: &'static [u8] = b"yy";
//!
//!     #[counter(3)]
//!     const Z_IDENT: &'static [u8] = b"z";
//! }
//! ```
//!
//! may additionally generate standalone tokens ([`syn::Item`]) such as:
//!
//! ```ignore
//! impl<R> CounterAccess3<
//!     { hash_ident(b"x") },
//!     { hash_ident(b"yy") },
//!     { hash_ident(b"z") },
//!     Phantom<R>,
//! > for Global
//! where
//!     R: MarkerTrait,
//! {
//!     type Counter = (U0, U1, U2);
//! }
//! ```
//!
//! These addon implementations provide auxiliary compile-time lookup and
//! reflection facilities while leaving the original instance
//! implementation unchanged.

// ===============================================================================
// ``````````````````````````````````` IMPORTS ```````````````````````````````````
// ===============================================================================

// --- Std crate ---
use std::collections::HashSet;

// --- Local Crate ---
use crate::{
    Extension, Extraction, Insertion, Instance, ParseError, Transformation, Utilization,
    impls::{
        counters::CounterArgsSlice,
        errors::{AddonBugs, AddonErrors, LastBugs},
        last::CounterArgsAsAssoc,
        utils::{ImplCountersTypeNum, ImplTraitIdent, ImplTraitPath},
    },
    traits::meta::CumulatedConstChecker,
};

use proc_macro2::TokenStream;
// --- Proc Macro Crates ---
use quote::{ToTokens, format_ident};
use syn::{
    AttrStyle, Expr, File, GenericArgument, GenericParam, Generics, ImplItem, Item, ItemConst,
    ItemImpl, Lit, LitInt, Meta, Path, PathArguments, Type, WhereClause, WherePredicate, parse,
    parse_quote, punctuated::Punctuated, spanned::Spanned, token::Comma, visit::Visit,
};

// --- Proc Suite ---
use proc_suite::{BoundsList, ParseDiagnostic, SupportCrate, misc::*};

// ===============================================================================
// ```````````````````````` COUNTERS HASH EXPR UTILIZATION ```````````````````````
// ===============================================================================

/// Extracts the identifier expressions associated with each counter
/// generic index from an implementation.
///
/// Counter generic indexes are provided independently from identifier
/// definitions, therefore the implementation cannot determine which
/// identifier belongs to which counter dimension solely from the const
/// item name.
///
/// Example:
///
/// ```ignore
/// #[instance_impl(1, 2, 3)]
/// impl Example<0, 1, 2> for MyType {
///     #[counter(1)]
///     const X_IDENT: &'static [u8] = b"x";
///
///     #[counter(2)]
///     const Y_IDENT: &'static [u8] = b"yy";
///
///     #[counter(3)]
///     const Z_IDENT: &'static [u8] = b"zzz";
/// }
/// ```
///
/// The counter indexes:
///
/// ```text
/// [1, 2, 3]
/// ```
///
/// are recovered from counter metadata, while:
///
/// ```text
/// X_IDENT
/// Y_IDENT
/// Z_IDENT
/// ```
///
/// are arbitrary user-defined names.
///
/// The `#[counter(...)]` attribute establishes the mapping:
///
/// ```text
/// counter 1 -> b"x"
/// counter 2 -> b"yy"
/// counter 3 -> b"zzz"
/// ```
///
/// This utilization walks the implementation, locates the const item
/// associated with each counter generic index, and returns the
/// expressions in counter order.
///
/// Result:
///
/// ```text
/// [b"x", b"yy", b"zzz"]
/// ```
///
/// These expressions are later used to derive identifier hashes,
/// identifier collections, historical collections, and lineage
/// validation structures.
#[derive(Debug, Clone)]
pub(crate) struct CountersIdentExprs<'a>(pub(crate) Vec<&'a Expr>);

impl<'a> Utilization<'a, ItemImpl, CounterArgsSlice<'a>> for CountersIdentExprs<'a> {
    fn raw_utilize(
        from: &'a ItemImpl,
        context: &'a CounterArgsSlice<'a>,
    ) -> Result<Self, proc_macro2::TokenStream> {
        let mut collect = Vec::new();
        for counter in context.iter() {
            let mut found = false;
            for item in &from.items {
                let ImplItem::Const(c) = item else {
                    continue;
                };
                for attr in &c.attrs {
                    if attr.style != AttrStyle::Outer {
                        continue;
                    }
                    let Meta::List(list) = &attr.meta else {
                        continue;
                    };
                    if !attr.path().is_ident("counter") {
                        continue;
                    }
                    let int = match parse::<LitInt>(list.tokens.clone().into()) {
                        Ok(t) => t,
                        Err(e) => {
                            return Err(e.into_compile_error().into());
                        }
                    };
                    if counter.generic_index != parse_pos_usize(&int)? {
                        continue;
                    }

                    collect.push(&c.expr);
                    found = true
                }
            }
            if !found {
                return Err(AddonErrors::IdentifierUnavailableForCounterIndex {
                    int: counter.const_lit.clone(),
                    index: counter.generic_index,
                    impl_of: from.clone(),
                }
                .into());
            }
        }
        Ok(Self(collect))
    }

    fn validate_utilize(
        &self,
        from: &ItemImpl,
        context: Option<&CounterArgsSlice<'a>>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let extracted;
        let counters = match context {
            Some(c) => c,
            None => {
                extracted = CounterArgsAsAssoc::checked_extract(from, &())?.0;
                extracted.as_slice().as_ref().as_ref()
            }
        };
        if self.0.len() != counters.len() {
            return Err(AddonBugs::ExtractedIdentifierRawExprsLengthMismatch {}.into());
        }
        for expr in &self.0 {
            let mut found = false;

            'exists: for item in &from.items {
                let ImplItem::Const(c) = item else {
                    continue;
                };

                if c.expr != **expr {
                    continue;
                }

                for attr in &c.attrs {
                    let AttrStyle::Outer = attr.style else {
                        continue;
                    };

                    let Meta::List(list) = &attr.meta else {
                        continue;
                    };

                    if list.path.is_ident("counter") {
                        found = true;
                        break 'exists;
                    }
                }
            }

            if !found {
                return Err(AddonBugs::ExtractedIdentifierRawExprIsInconsistent {}.into());
            }
        }
        Ok(())
    }
}

// ===============================================================================
// ````````````````````````````` COUNTER ACCESS ADDON ````````````````````````````
// ===============================================================================

/// Generates a global compile-time lookup implementation exposing an
/// instance's typenum counter tuple.
///
/// Multiple instances of the same instance trait may be implemented for
/// the same Rust type, differing only by their counter tuple.
///
/// For example:
///
/// ```text
/// Phantom<R>
///     |-- Example<(U0,U0,U0)>
///     |-- Example<(U0,U0,U1)>
///     |-- Example<(U0,U1,U0)>
///     |-- ...
/// ```
///
/// Since all of these implementations share the same implementing
/// `Self` type, the Rust type alone is insufficient to uniquely identify
/// an instance.
///
/// Instead, users identify instances through human-readable identifier
/// constants:
///
/// ```text
/// ("x", "yy", "z")
/// ```
///
/// while the generated instance traits internally distinguish instances
/// using typenum counter tuples:
///
/// ```text
/// (U0, U1, U2)
/// ```
///
/// This addon builds the bridge between those two representations by
/// generating a global lookup:
///
/// ```text
/// (identifier hashes..., implementing Self type)
///                  |
///                  v
///         Global : CounterAccessN
///                  |
///                  v
///         Associated counter tuple
/// ```
///
/// Example:
///
/// ```ignore
/// #[instance_impl(1, 2, 3)]
/// #[self_bounds(R: MarkerTrait)]
/// impl<R: MarkerTrait> Example<0, 1, 2> for Phantom<R> {
///     #[counter(1)]
///     const X_IDENT: &'static [u8] = b"x";
///
///     #[counter(2)]
///     const Y_IDENT: &'static [u8] = b"yy";
///
///     #[counter(3)]
///     const Z_IDENT: &'static [u8] = b"z";
/// }
/// ```
///
/// generates:
///
/// ```ignore
/// impl<R> CounterAccess3<
///     { hash_ident(b"x") },
///     { hash_ident(b"yy") },
///     { hash_ident(b"z") },
///     Phantom<R>,
/// > for Global
/// where
///     R: MarkerTrait
/// {
///     type Counter = (U0, U1, U2);
/// }
/// ```
///
/// Only the generic parameters appearing in the implementing `Self` type
/// are preserved. Any bounds required by those parameters must therefore
/// be supplied explicitly through:
///
/// ```ignore
/// #[self_bounds(...)]
/// ```
///
/// because the generated lookup implementation does not retain the
/// original trait's generic parameter list.
///
/// Later expansion phases can recover the corresponding typenum counter
/// tuple entirely from an instance's public identity:
///
/// ```ignore
/// <Global as CounterAccess3<
///     HASH_X,
///     HASH_Y,
///     HASH_Z,
///     Phantom<R>,
/// >>::Counter
/// ```
///
/// Since instance traits replace const generic counters with a typenum
/// counter type parameter, the recovered tuple can be used directly to
/// select the intended instance:
///
/// ```ignore
/// <Phantom<R> as Example<
///     <Global as CounterAccess3<
///         HASH_X,
///         HASH_Y,
///         HASH_Z,
///         Phantom<R>,
///     >>::Counter,
/// >>::...
/// ```
///
/// This provides a canonical compile-time mapping from user-facing
/// identifiers back to the typenum counter tuple required to uniquely
/// select an instance implementation.
#[derive(Debug, Clone)]
pub(crate) struct CounterAccessAddon;

impl<'a> Insertion<File, ItemImpl> for ItemImpl {
    fn raw_insert(&self, to: &mut File, _: &ItemImpl) -> Result<(), proc_macro2::TokenStream> {
        to.items.push(Item::Impl(self.clone()));
        Ok(())
    }

    fn validate_inserted(
        _of: Option<&Self>,
        _to: &File,
        _context: Option<&ItemImpl>,
    ) -> Result<(), proc_macro2::TokenStream> {
        Ok(())
    }
}

impl<'a> Extension<ItemImpl, File, ItemImpl> for CounterAccessAddon {
    fn raw_extend(
        &self,
        _: &File,
        context: &ItemImpl,
    ) -> Result<ItemImpl, proc_macro2::TokenStream> {
        let self_ty = &context.self_ty;
        let generics = &context.generics;
        let new_params = extract_generic_idents_from_ty(self_ty, generics);
        let attrs = &context.attrs;
        let mut new_predicates = Punctuated::<WherePredicate, Comma>::new();
        for attr in attrs {
            if attr.style != AttrStyle::Outer {
                continue;
            }
            let Meta::List(list) = &attr.meta else {
                continue;
            };
            if !list.path.is_ident("self_bounds") {
                continue;
            }
            let bounds = match parse::<BoundsList>(list.tokens.clone().into()) {
                Ok(t) => t,
                Err(_) => {
                    return Err(<BoundsList as ParseDiagnostic>::span_parse_error(
                        &list.tokens.span(),
                        Some(ParseError::BoundsList.into()),
                    ));
                }
            };
            new_predicates.extend(bounds.bounds);
        }

        let new_generics = Generics {
            lt_token: Default::default(),
            params: new_params,
            gt_token: Default::default(),
            where_clause: Some(WhereClause {
                where_token: Default::default(),
                predicates: new_predicates,
            }),
        };

        let (impl_generics, _, where_clause) = new_generics.split_for_impl();

        let counters = CounterArgsAsAssoc::checked_extract(context, &())?.0;
        let counters_len = counters.len();
        let counters_slc = counters.as_slice();
        let counters_ty = ImplCountersTypeNum::checked_utilize(context, &counters_slc)?.0;

        let trait_ident = format_ident!("CounterAccess{}", counters_len);
        let crate_of = Instance::support_crate();

        let hash_args = counter_access_impl_ident_hash_args(context, &counters_slc)?;

        Ok(parse_quote!(
            impl #impl_generics #crate_of::#trait_ident < #(#hash_args),* , #self_ty > for #crate_of::Global
            #where_clause
            {
                type Counter = #counters_ty;
            }
        ))
    }

    fn validate_extend(
        &self,
        item: Option<&ItemImpl>,
        towards: &File,
        context: Option<&ItemImpl>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let Some(context) = context else {
            return Ok(());
        };

        let counters = CounterArgsAsAssoc::checked_extract(context, &())?.0;
        let counters_len = counters.len();
        let exp_trait_ident = format_ident!("CounterAccess{}", counters_len);

        let validate_impl = |impl_of: &ItemImpl, context: &ItemImpl| -> Result<(), TokenStream> {
            let crate_of = Instance::support_crate();

            // trait ident
            let trait_of = ImplTraitIdent::checked_utilize(impl_of, &())?.0;
            if *trait_of != exp_trait_ident {
                return Err(AddonBugs::InvalidCounterAccessImplIdent {}.into());
            };

            // self as instance's global
            let self_ty = &*impl_of.self_ty;
            let exp_self_ty: Type = parse_quote!(#crate_of::Global);
            if self_ty.to_token_stream().to_string() != exp_self_ty.to_token_stream().to_string() {
                return Err(AddonBugs::InvalidGlobalTypePathForCounterAccess {}.into());
            }

            // trait full path
            let trait_path = ImplTraitPath::checked_utilize(impl_of, &())?.0;
            let exp_path: Path = parse_quote!(#crate_of::#exp_trait_ident);
            if trait_path.segments.len() != exp_path.segments.len() {
                return Err(AddonBugs::InvalidCounterAccessImplPathSegmentsLen {}.into());
            }
            for (found, exp) in trait_path.segments.iter().zip(&exp_path.segments) {
                if found.ident != exp.ident {
                    return Err(AddonBugs::InvalidCounterAccessImplPathSegment {}.into());
                }
            }

            // trait generic args
            let Some(last_path) = trait_path.segments.last() else {
                return Err(AddonBugs::InvalidCounterAccessImplTraitSegment {}.into());
            };
            let PathArguments::AngleBracketed(found_angle_args) = &last_path.arguments else {
                return Err(AddonBugs::CounterAccessImplHasInvalidGenericArgs {}.into());
            };
            if found_angle_args.args.len() != counters_len + 1 {
                return Err(AddonBugs::CounterAccessImplExpectedGenericArgsLenInvalid {}.into());
            }
            let mut args_iter = found_angle_args.args.iter().rev();

            // trait generic arg original instance implementing type
            let Some(orig_ty) = args_iter.next() else {
                return Err(
                    AddonBugs::CounterAccessImplOriginalTypeGenericArgUnavailable {}.into(),
                );
            };
            let GenericArgument::Type(orig_ty) = orig_ty else {
                return Err(AddonBugs::CounterAccessImplOriginalTypeNotTypeGenericArg {}.into());
            };
            if orig_ty.to_token_stream().to_string()
                != context.self_ty.to_token_stream().to_string()
            {
                return Err(AddonBugs::CounterAccessImplOriginalTypeGenericArgInvalid {}.into());
            }

            // trait generic args remaining hash ident consts exprs
            let counters_slc = counters.as_slice();
            let exp_hash_args = counter_access_impl_ident_hash_args(context, &counters_slc)?;
            if args_iter.len() != exp_hash_args.len() {
                return Err(
                    AddonBugs::CounterAccessImplExpectedConstGenericArgsLenInvalid {}.into(),
                );
            }
            for (found, exp) in args_iter.zip(exp_hash_args.iter().rev()) {
                let GenericArgument::Const(c) = found else {
                    return Err(AddonBugs::CounterAccessImplExpectedConstGenericFoundElse {}.into());
                };

                if c != exp {
                    return Err(AddonBugs::CounterAccessImplHashExprInvalid {}.into());
                }
            }

            // original instance type's generic where clauses
            let exp_where_preds = {
                let mut collect = Vec::new();
                for attr in &context.attrs {
                    if attr.style != AttrStyle::Outer {
                        continue;
                    }
                    let Meta::List(list) = &attr.meta else {
                        continue;
                    };
                    if !list.path.is_ident("self_bounds") {
                        continue;
                    }
                    let Ok(bounds) = parse::<BoundsList>(list.tokens.clone().into()) else {
                        return Err(<BoundsList as ParseDiagnostic>::span_parse_error(
                            &list.tokens.span(),
                            Some(ParseError::BoundsList.into()),
                        ));
                    };
                    for bound in bounds.bounds {
                        collect.push(bound);
                    }
                }
                collect
            };
            let found_where_preds = {
                let mut collect = Vec::new();
                if let Some(where_clause) = &impl_of.generics.where_clause {
                    for pred in &where_clause.predicates {
                        collect.push(pred);
                    }
                }
                collect
            };
            for expected in &exp_where_preds {
                let found = found_where_preds.iter().any(|actual| {
                    expected.to_token_stream().to_string() == actual.to_token_stream().to_string()
                });

                if !found {
                    return Err(
                        AddonBugs::CounterAccessImplOriginalTypeGenericWhereClausesInvalid {}
                            .into(),
                    );
                }
            }

            // original instance type's generic params idents
            let exp_gen_params =
                { extract_generic_idents_from_ty(&context.self_ty, &context.generics) };
            let found_gen_params = &impl_of.generics.params;
            for expected in &exp_gen_params {
                let found = found_gen_params.iter().any(|actual| actual == expected);

                if !found {
                    return Err(
                        AddonBugs::CounterAccessImplOriginalTypeGenericParamsInvalid {}.into(),
                    );
                }
            }

            // trait's only assoc type
            if impl_of.items.is_empty() {
                return Err(AddonBugs::CounterAccessImplAddonWithNoAssocItems {}.into());
            }
            let mut iter = impl_of.items.iter();
            let Some(first) = iter.next() else {
                return Err(AddonBugs::CounterAccessImplAddonWithNoAssocItems {}.into());
            };
            let ImplItem::Type(ty) = first else {
                return Err(AddonBugs::CounterAccessImplCounterAssocNotType {}.into());
            };
            let exp_ident = format_ident!("Counter");
            if ty.ident != exp_ident {
                return Err(AddonBugs::CounterAccessImplAddonTypeAssocIdentInvalid {}.into());
            }
            let found_ty = ImplCountersTypeNum::checked_utilize(context, &counters_slc)?.0;
            if found_ty.to_token_stream().to_string() != ty.ty.to_token_stream().to_string() {
                return Err(AddonBugs::CounterAccessImplInvalidCounterTy {}.into());
            }
            if iter.next().is_some() {
                return Err(AddonBugs::CounterAccessImplHasExcessAssocs {}.into());
            }

            Ok(())
        };

        if let Some(impl_of) = item {
            validate_impl(impl_of, context)?;
        };

        let mut found = false;
        for item in &towards.items {
            let Item::Impl(impl_of) = item else {
                continue;
            };
            let trait_of = ImplTraitIdent::checked_utilize(impl_of, &())?.0;

            if *trait_of == exp_trait_ident {
                validate_impl(impl_of, context)?;
                found = true
            }
        }
        if !found {
            return Err(AddonBugs::CounterAccessImplAddonNotFound {}.into());
        }
        Ok(())
    }
}

/// Extracts the generic parameters referenced by a given `Self` type.
///
/// Only generic identifiers appearing in `Self` are retained. Bounds,
/// defaults, and generics used solely by the implemented trait are
/// omitted.
///
/// ```ignore
/// impl<'a, T, R: Marker, const B: bool> Example<...> for Phantom<&'a R> {}
/// ```
///
/// yields:
///
/// ```ignore
/// <'a, R>
/// ```
pub(crate) fn extract_generic_idents_from_ty(
    ty: &Type,
    generics: &Generics,
) -> Punctuated<GenericParam, Comma> {
    struct TypeIdentVisitor {
        idents: HashSet<String>,
    }

    impl<'b> Visit<'b> for TypeIdentVisitor {
        fn visit_ident(&mut self, ident: &'b syn::Ident) {
            self.idents.insert(ident.to_string());
        }
    }

    // Collect all identifiers appearing in the type.
    let mut visitor = TypeIdentVisitor {
        idents: HashSet::new(),
    };

    visitor.visit_type(ty);

    let type_idents = visitor.idents;

    // Rebuild generic params containing only the name/kind.
    let mut params = Punctuated::new();

    for param in &generics.params {
        match param {
            GenericParam::Type(param) => {
                if type_idents.contains(&param.ident.to_string()) {
                    params.push(GenericParam::Type(syn::TypeParam {
                        attrs: Vec::new(),
                        ident: param.ident.clone(),
                        colon_token: None,
                        bounds: Punctuated::new(),
                        eq_token: None,
                        default: None,
                    }));
                }
            }

            GenericParam::Lifetime(param) => {
                if type_idents.contains(&param.lifetime.ident.to_string()) {
                    params.push(GenericParam::Lifetime(syn::LifetimeParam {
                        attrs: Vec::new(),
                        lifetime: param.lifetime.clone(),
                        colon_token: None,
                        bounds: Punctuated::new(),
                    }));
                }
            }

            GenericParam::Const(param) => {
                if type_idents.contains(&param.ident.to_string()) {
                    params.push(GenericParam::Const(syn::ConstParam {
                        attrs: Vec::new(),
                        const_token: param.const_token,
                        ident: param.ident.clone(),
                        colon_token: param.colon_token,
                        ty: param.ty.clone(),
                        eq_token: None,
                        default: None,
                    }));
                }
            }
        }
    }

    params
}

/// Builds the hashed identifier expressions used as const generic
/// arguments.
///
/// Each counter identifier expression is transformed into
/// `{ hash_ident(...) }`.
fn counter_access_impl_ident_hash_args<'a>(
    impl_of: &ItemImpl,
    counters: CounterArgsSlice<'a>,
) -> Result<Vec<Expr>, TokenStream> {
    let crate_of = Instance::support_crate();
    let counters_exprs = CountersIdentExprs::checked_utilize(impl_of, &counters)?.0;
    let mut collect = Vec::<Expr>::new();
    for expr in counters_exprs {
        collect.push(parse_quote!( { #crate_of::hash_ident(#expr) } ))
    }
    Ok(collect)
}

// ===============================================================================
// ````````````````````````` IDENT HASH-LEN ACCESS ADDON `````````````````````````
// ===============================================================================

/// Generates compile-time lookup implementations describing the identifier
/// length associated with each counter generic position of an instance.
///
/// Unlike [`CounterAccessAddon`], which exposes the complete typenum counter
/// tuple associated with an instance, this addon provides access to the
/// identifier length associated with each individual counter generic index.
///
/// For every counter generic index of an instance, an implementation is
/// generated that associates the instance's identifier hash path and that
/// generic index with its corresponding identifier length.
///
/// Consider the following instance:
///
/// ```ignore
/// InstanceTrait<A, B, C>
/// ```
///
/// where each counter generic corresponds to one identifier dimension:
///
/// ```text
/// A  B  C
/// 0  1  2
/// ```
///
/// For each counter generic position `N`, this addon generates the
/// corresponding `IdentHashLenAccessN` implementation.
///
/// For example, if the identifier hashes are (as numerics):
///
/// ```text
/// A = 3
/// B = 5
/// C = 7
/// ```
///
/// the generated access metadata is conceptually:
///
/// ```text
/// identifier hash path + generic index
///                    |
///          Global : IdentHashLenAccessN
///                    |
///             identifier length
/// ```
///
/// The generic index identifies which identifier dimension is being queried,
/// while the identifier hash sequence identifies the instance whose metadata
/// is being accessed.
///
/// Therefore, for:
///
/// ```ignore
/// <Global as IdentHashLenAccess3<
///     HASH_A,
///     HASH_B,
///     HASH_C,
///     1,
///     InstanceType,
/// >>::Length
/// ```
///
/// the identifier hash sequence identifies `InstanceType`'s instance path,
/// `1` selects the second counter generic position, and `Length` resolves to
/// the typenum value representing that identifier's length.
///
/// The generated implementation stores the length as a typenum value so that
/// it can participate directly in subsequent compile-time type-level
/// operations.
///
/// As with the other access addons, only the generic parameters appearing in
/// the implementing `Self` type are preserved. Any bounds required by those
/// parameters are recovered from:
///
/// `ignore
/// #[self_bounds(...)]
/// `
///
/// Each counter generic index therefore receives its own compile-time lookup
/// implementation, while all implementations belonging to the same instance
/// share the same ordered identifier hash path.
///
/// This provides the canonical compile-time mapping from an instance
/// identifier path and counter generic position to the length of the
/// corresponding identifier.
#[derive(Debug, Clone)]
pub(crate) struct IdentHashLenAddon;

impl<'a> Transformation<File, ItemImpl> for IdentHashLenAddon {
    fn raw_transform(
        &self,
        transform: &mut File,
        context: &ItemImpl,
    ) -> Result<(), proc_macro2::TokenStream> {
        let self_ty = &context.self_ty;
        let generics = &context.generics;
        let new_params = extract_generic_idents_from_ty(self_ty, generics);
        let attrs = &context.attrs;
        let mut new_predicates = Punctuated::<WherePredicate, Comma>::new();
        for attr in attrs {
            if attr.style != AttrStyle::Outer {
                continue;
            }
            let Meta::List(list) = &attr.meta else {
                continue;
            };
            if !list.path.is_ident("self_bounds") {
                continue;
            }
            let bounds = match parse::<BoundsList>(list.tokens.clone().into()) {
                Ok(t) => t,
                Err(_) => {
                    return Err(<BoundsList as ParseDiagnostic>::span_parse_error(
                        &list.tokens.span(),
                        Some(ParseError::BoundsList.into()),
                    ));
                }
            };
            new_predicates.extend(bounds.bounds);
        }

        let new_generics = Generics {
            lt_token: Default::default(),
            params: new_params,
            gt_token: Default::default(),
            where_clause: Some(WhereClause {
                where_token: Default::default(),
                predicates: new_predicates,
            }),
        };

        let (impl_generics, _, where_clause) = new_generics.split_for_impl();

        let counters = CounterArgsAsAssoc::checked_extract(context, &())?.0;
        let counters_len = counters.len();
        let counters_slc = counters.as_slice();

        let trait_ident = format_ident!("IdentHashLenAccess{}", counters_len);
        let crate_of = Instance::support_crate();

        let hash_args = counter_access_impl_ident_hash_args(context, &counters_slc)?;

        for counter in counters_slc {
            let index = counter.generic_index;

            let lit = parse_pos_usize(&counter.const_lit)? + 1;
            let typenum = format_ident!("U{lit}");
            let ty: Type = parse_quote!(#crate_of::#typenum);

            let impl_of = parse_quote!(
                impl #impl_generics #crate_of::#trait_ident < #(#hash_args),* , #index, #self_ty > for #crate_of::Global
                #where_clause
                {
                    type Length = #ty;
                }
            );

            transform.items.push(Item::Impl(impl_of));
        }

        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &File,
        context: Option<&ItemImpl>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let Some(context) = context else {
            return Ok(());
        };

        let counters = CounterArgsAsAssoc::checked_extract(context, &())?.0;
        let counters_len = counters.len();
        let exp_trait_ident = format_ident!("IdentHashLenAccess{}", counters_len);

        let mut found_counters = Vec::new();

        let mut validate_impl = |impl_of: &ItemImpl,
                                 context: &ItemImpl|
         -> Result<(), TokenStream> {
            let crate_of = Instance::support_crate();

            // trait ident
            let trait_of = ImplTraitIdent::checked_utilize(impl_of, &())?.0;
            if *trait_of != exp_trait_ident {
                return Err(AddonBugs::InvalidIdentHashLenAccessImplIdent {}.into());
            };

            // self as instance's global
            let self_ty = &*impl_of.self_ty;
            let exp_self_ty: Type = parse_quote!(#crate_of::Global);
            if self_ty.to_token_stream().to_string() != exp_self_ty.to_token_stream().to_string() {
                return Err(AddonBugs::InvalidGlobalTypePathForIdentHashLenAccess {}.into());
            }

            // trait full path
            let trait_path = ImplTraitPath::checked_utilize(impl_of, &())?.0;
            let exp_path: Path = parse_quote!(#crate_of::#exp_trait_ident);
            if trait_path.segments.len() != exp_path.segments.len() {
                return Err(AddonBugs::InvalidIdentHashLenAccessImplPathSegmentsLen {}.into());
            }
            for (found, exp) in trait_path.segments.iter().zip(&exp_path.segments) {
                if found.ident != exp.ident {
                    return Err(AddonBugs::InvalidIdentHashLenAccessImplPathSegment {}.into());
                }
            }

            // trait generic args
            let Some(last_path) = trait_path.segments.last() else {
                return Err(AddonBugs::InvalidIdentHashLenAccessImplTraitSegment {}.into());
            };
            let PathArguments::AngleBracketed(found_angle_args) = &last_path.arguments else {
                return Err(AddonBugs::IdentHashLenAccessImplHasInvalidGenericArgs {}.into());
            };
            if found_angle_args.args.len() != counters_len + 2 {
                return Err(
                    AddonBugs::IdentHashLenAccessImplExpectedGenericArgsLenInvalid {}.into(),
                );
            }
            let mut args_iter = found_angle_args.args.iter().rev();

            // trait generic arg original instance implementing type
            let Some(orig_ty) = args_iter.next() else {
                return Err(
                    AddonBugs::IdentHashLenAccessImplOriginalTypeGenericArgUnavailable {}.into(),
                );
            };
            let GenericArgument::Type(orig_ty) = orig_ty else {
                return Err(
                    AddonBugs::IdentHashLenAccessImplOriginalTypeNotTypeGenericArg {}.into(),
                );
            };
            if orig_ty.to_token_stream().to_string()
                != context.self_ty.to_token_stream().to_string()
            {
                return Err(
                    AddonBugs::IdentHashLenAccessImplOriginalTypeGenericArgInvalid {}.into(),
                );
            }

            // trait generic arg counter index
            let Some(counter_index) = args_iter.next() else {
                return Err(
                    AddonBugs::IdentHashLenAccessCounterIndexGenericArgUnavailable {}.into(),
                );
            };
            let GenericArgument::Const(counter_lit) = counter_index else {
                return Err(AddonBugs::IdentHashLenAccessCounterIndexNotConstGenericArg {}.into());
            };
            let Expr::Lit(c_lit) = counter_lit else {
                return Err(AddonBugs::IdentHashLenAccessCounterIndexNotConstExprLit {}.into());
            };
            let Lit::Int(c_int) = &c_lit.lit else {
                return Err(AddonBugs::IdentHashLenAccessCounterIndexNotConstLitInt {}.into());
            };
            let index = &parse_pos_usize(c_int)?;
            found_counters.push(index.clone());

            // trait generic args remaining hash ident consts exprs
            let counters_slc = counters.as_slice();
            let exp_hash_args = counter_access_impl_ident_hash_args(context, &counters_slc)?;
            if args_iter.len() != exp_hash_args.len() {
                return Err(
                    AddonBugs::IdentHashLenAccessImplExpectedConstGenericArgsLenInvalid {}.into(),
                );
            }
            for (found, exp) in args_iter.zip(exp_hash_args.iter().rev()) {
                let GenericArgument::Const(c) = found else {
                    return Err(
                        AddonBugs::IdentHashLenAccessImplExpectedConstGenericFoundElse {}.into(),
                    );
                };

                if c != exp {
                    return Err(AddonBugs::IdentHashLenAccessImplHashExprInvalid {}.into());
                }
            }

            // original instance type's generic where clauses
            let exp_where_preds = {
                let mut collect = Vec::new();
                for attr in &context.attrs {
                    if attr.style != AttrStyle::Outer {
                        continue;
                    }
                    let Meta::List(list) = &attr.meta else {
                        continue;
                    };
                    if !list.path.is_ident("self_bounds") {
                        continue;
                    }
                    let Ok(bounds) = parse::<BoundsList>(list.tokens.clone().into()) else {
                        return Err(<BoundsList as ParseDiagnostic>::span_parse_error(
                            &list.tokens.span(),
                            Some(ParseError::BoundsList.into()),
                        ));
                    };
                    for bound in bounds.bounds {
                        collect.push(bound);
                    }
                }
                collect
            };
            let found_where_preds = {
                let mut collect = Vec::new();
                if let Some(where_clause) = &impl_of.generics.where_clause {
                    for pred in &where_clause.predicates {
                        collect.push(pred);
                    }
                }
                collect
            };
            for expected in &exp_where_preds {
                let found = found_where_preds.iter().any(|actual| {
                    expected.to_token_stream().to_string() == actual.to_token_stream().to_string()
                });

                if !found {
                    return Err(
                        AddonBugs::IdentHashLenAccessImplOriginalTypeGenericWhereClausesInvalid {}
                            .into(),
                    );
                }
            }

            // original instance type's generic params idents
            let exp_gen_params =
                { extract_generic_idents_from_ty(&context.self_ty, &context.generics) };
            let found_gen_params = &impl_of.generics.params;
            for expected in &exp_gen_params {
                let found = found_gen_params.iter().any(|actual| actual == expected);

                if !found {
                    return Err(
                        AddonBugs::IdentHashLenAccessImplOriginalTypeGenericParamsInvalid {}.into(),
                    );
                }
            }

            // trait's only assoc type
            if impl_of.items.is_empty() {
                return Err(AddonBugs::IdentHashLenAccessImplAddonWithNoAssocItems {}.into());
            }
            let mut iter = impl_of.items.iter();
            let Some(first) = iter.next() else {
                return Err(AddonBugs::IdentHashLenAccessImplAddonWithNoAssocItems {}.into());
            };
            let ImplItem::Type(ty) = first else {
                return Err(AddonBugs::IdentHashLenAccessImplLengthAssocNotType {}.into());
            };
            let exp_ident = format_ident!("Length");
            if ty.ident != exp_ident {
                return Err(AddonBugs::IdentHashLenAccessImplAddonTypeAssocIdentInvalid {}.into());
            }

            let const_lit = counters_slc.iter().find_map(|item| {
                if item.generic_index == *index {
                    return Some(&item.const_lit);
                }
                None
            });
            let Some(const_lit) = const_lit else {
                return Err(AddonBugs::IdentHashLenAccessGivenCounterIndexInvalid {}.into());
            };

            let lit = parse_pos_usize(const_lit)? + 1;
            let typenum = format_ident!("U{lit}");
            let found_ty: Type = parse_quote!(#crate_of::#typenum);

            if found_ty.to_token_stream().to_string() != ty.ty.to_token_stream().to_string() {
                return Err(AddonBugs::IdentHashLenAccessImplInvalidLengthTy {}.into());
            }
            if iter.next().is_some() {
                return Err(AddonBugs::IdentHashLenAccessImplHasExcessAssocs {}.into());
            }

            Ok(())
        };

        let mut found = false;
        for item in &transform.items {
            let Item::Impl(impl_of) = item else {
                continue;
            };
            let trait_of = ImplTraitIdent::checked_utilize(impl_of, &())?.0;

            if *trait_of == exp_trait_ident {
                validate_impl(impl_of, context)?;
                found = true;
            }
        }
        if !found {
            return Err(AddonBugs::IdentHashLenAccessImplAddonNotFound {}.into());
        }

        for counter in counters {
            let exp = counter.generic_index;
            let mut got = false;
            for found in &found_counters {
                if *found == exp {
                    got = true
                }
            }
            if !got {
                return Err(AddonBugs::IdentHashLenAccessImplAddonMissing {}.into());
            }
        }

        Ok(())
    }
}

// ===============================================================================
// `````````````````````````````` ONSET ACCESS ADDON `````````````````````````````
// ===============================================================================

/// Generates compile-time lookup implementations describing the onset
/// instance for every contiguous trailing zero counter dimension.
///
/// Unlike [`BoundaryAccessAddon`], which is generated from explicit
/// `#[last_instance(...)]` declarations, this addon is derived
/// automatically from an instance's counter tuple.
///
/// Consider the following sequence:
///
/// ```text
/// InstanceTrait<0,0,0>
/// InstanceTrait<0,0,1>
/// InstanceTrait<0,0,2>
/// InstanceTrait<0,1,0>
/// InstanceTrait<0,2,0>
/// InstanceTrait<1,0,0>
/// ```
///
/// Every maximal contiguous trailing sequence of zero-valued counters
/// denotes the onset of a new sub-sequence of instances.
///
/// For example:
///
/// ```text
/// InstanceTrait<0,1,0>
/// ```
///
/// has a trailing zero sequence beginning at counter `2`, while:
///
/// ```text
/// InstanceTrait<1,0,0>
/// ```
///
/// has a trailing zero sequence beginning at counter `1`,
/// encompassing both counters `1` and `2`.
///
/// Consequently, this addon generates one lookup implementation for
/// every counter dimension belonging to the contiguous trailing zero
/// sequence.
///
/// ```text
/// InstanceTrait<1,0,0>
///        |-- OnSetAccess1
///        |-- OnSetAccess2
/// ```
///
/// The onset instance represents the first child instance reachable
/// beneath a partially resolved parent identifier sequence.
///
/// Once the identifier hashes preceding counter dimension `N` have been
/// resolved, the first child of that parent always begins with counter
/// `N` equal to zero. Therefore, the parent identifier sequence alone is
/// sufficient to determine the typenum counter tuple of that child's
/// beginning instance, without requiring any remaining identifier
/// hashes.
///
/// Each generated implementation maps:
///
/// ```text
/// (identifier hashes before N..., implementing Self type)
///                     |
///                     v
///           Global : OnSetAccessN
///                     |
///                     v
///         onset hash + typenum counter tuple
/// ```
///
/// Example:
///
/// ```ignore
/// impl<R> OnSetAccess2<
///     { hash_ident(b"x") },
///     { hash_ident(b"y") },
///     Phantom<R>,
/// > for Global
/// where
///     R: MarkerTrait,
/// {
///     const INITIAL: u64 = { hash_ident(b"z") };
///     type OnSet = (U0, U1, U0);
/// }
/// ```
///
/// Here, `INITIAL` is the identifier hash associated with the onset of
/// counter dimension `2`, while `OnSet` exposes the typenum counter
/// tuple of the onset instance.
///
/// As with [`CounterAccessAddon`], only the generic parameters
/// appearing in the implementing `Self` type are preserved. Any bounds
/// required by those parameters are recovered from:
///
/// ```ignore
/// #[self_bounds(...)]
/// ```
///
/// Later expansion phases can recover both the onset identifier hash
/// and the corresponding typenum counter tuple:
///
/// ```ignore
/// <Global as OnSetAccess2<
///     HASH_X,
///     HASH_Y,
///     Phantom<R>,
/// >>::INITIAL
///
/// <Global as OnSetAccess2<
///     HASH_X,
///     HASH_Y,
///     Phantom<R>,
/// >>::OnSet
/// ```
///
/// This provides the canonical compile-time mapping from a partially
/// resolved parent identifier sequence to the beginning of its child
/// instance hierarchy.
#[derive(Debug, Clone)]
pub(crate) struct OnSetAccessAddon;

impl<'a> Transformation<File, ItemImpl> for OnSetAccessAddon {
    fn raw_transform(
        &self,
        transform: &mut File,
        context: &ItemImpl,
    ) -> Result<(), proc_macro2::TokenStream> {
        let counters = CounterArgsAsAssoc::checked_extract(context, &())?.0;
        let counters = &counters.as_slice();

        let (trailing, _) = counters.iter().enumerate().rev().try_fold(
            (Vec::new(), false),
            |(mut acc, done), (idx, item)| -> Result<_, TokenStream> {
                if done {
                    return Ok((acc, done));
                }

                if parse_pos_usize(&item.const_lit)? == 0 {
                    acc.push(idx);
                    Ok((acc, false))
                } else {
                    // Stop after the first non-zero, whether or not we've
                    // collected any trailing zeros.
                    Ok((acc, true))
                }
            },
        )?;

        let hash_exprs = counter_access_impl_ident_hash_args(context, counters)?;
        let self_ty = &context.self_ty;
        let generics = &context.generics;
        let new_params = extract_generic_idents_from_ty(self_ty, generics);
        let attrs = &context.attrs;
        let mut new_predicates = Punctuated::<WherePredicate, Comma>::new();
        for attr in attrs {
            if attr.style != AttrStyle::Outer {
                continue;
            }
            let Meta::List(list) = &attr.meta else {
                continue;
            };
            if !list.path.is_ident("self_bounds") {
                continue;
            }
            let bounds = match parse::<BoundsList>(list.tokens.clone().into()) {
                Ok(t) => t,
                Err(_) => {
                    return Err(<BoundsList as ParseDiagnostic>::span_parse_error(
                        &list.tokens.span(),
                        Some(ParseError::BoundsList.into()),
                    ));
                }
            };
            new_predicates.extend(bounds.bounds);
        }

        let new_generics = Generics {
            lt_token: Default::default(),
            params: new_params,
            gt_token: Default::default(),
            where_clause: Some(WhereClause {
                where_token: Default::default(),
                predicates: new_predicates,
            }),
        };

        let (impl_generics, _, where_clause) = new_generics.split_for_impl();
        let typenum = ImplCountersTypeNum::checked_utilize(context, counters)?.0;

        for index in trailing {
            let Some(initial_hash) = hash_exprs.get(index) else {
                return Err(AddonBugs::OnSetAccessHashOfIndexFromHashExprsUnavailable {}.into());
            };
            let hash_args = hash_exprs.iter().take(index);

            let trait_ident = format_ident!("OnSetAccess{}", index);
            let crate_of = Instance::support_crate();

            let item = if hash_args.len() == 0 {
                parse_quote! {
                    impl #impl_generics
                        #crate_of::#trait_ident<#self_ty>
                        for #crate_of::Global
                        #where_clause
                    {
                        const INITIAL: u64 = #initial_hash;
                        type OnSet = #typenum;
                    }
                }
            } else {
                parse_quote! {
                    impl #impl_generics
                        #crate_of::#trait_ident<#(#hash_args),*, #self_ty>
                        for #crate_of::Global
                        #where_clause
                    {
                        const INITIAL: u64 = #initial_hash;
                        type OnSet = #typenum;
                    }
                }
            };

            transform.items.push(Item::Impl(item));
        }

        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &File,
        context: Option<&ItemImpl>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let Some(context) = context else {
            return Ok(());
        };
        let counters = CounterArgsAsAssoc::checked_extract(context, &())?.0;
        let counters = &counters.as_slice();

        let hash_exprs = counter_access_impl_ident_hash_args(context, counters)?;

        let validate_impl = |impl_of: &ItemImpl, index: usize| -> Result<(), TokenStream> {
            let crate_of = Instance::support_crate();

            // self as instance's global
            let self_ty = &*impl_of.self_ty;
            let exp_self_ty: Type = parse_quote!(#crate_of::Global);
            if self_ty.to_token_stream().to_string() != exp_self_ty.to_token_stream().to_string() {
                return Err(AddonBugs::InvalidGlobalTypePathForOnSetAccess {}.into());
            }

            // trait full path
            let exp_trait_ident = format_ident!("OnSetAccess{}", index);
            let trait_path = ImplTraitPath::checked_utilize(impl_of, &())?.0;
            let exp_path: Path = parse_quote!(#crate_of::#exp_trait_ident);
            if trait_path.segments.len() != exp_path.segments.len() {
                return Err(AddonBugs::InvalidOnSetAccessImplPathSegmentsLen {}.into());
            }

            for (found, exp) in trait_path.segments.iter().zip(&exp_path.segments) {
                if found.ident != exp.ident {
                    return Err(AddonBugs::InvalidOnSetAccessImplPathSegment {}.into());
                }
            }

            // trait generic args
            let Some(last_path) = trait_path.segments.last() else {
                return Err(AddonBugs::InvalidOnSetAccessImplTraitSegment {}.into());
            };
            let PathArguments::AngleBracketed(found_angle_args) = &last_path.arguments else {
                return Err(AddonBugs::OnSetAccessImplHasInvalidGenericArgs {}.into());
            };

            let mut iter_args = found_angle_args.args.iter().rev();
            // trait generic arg original instance implementing type
            let Some(orig_ty) = iter_args.next() else {
                return Err(AddonBugs::OnSetAccessImplOriginalTypeGenericArgUnavailable {}.into());
            };
            let GenericArgument::Type(orig_ty) = orig_ty else {
                return Err(AddonBugs::OnSetAccessImplOriginalTypeNotTypeGenericArg {}.into());
            };

            if orig_ty.to_token_stream().to_string()
                != context.self_ty.to_token_stream().to_string()
            {
                return Err(AddonBugs::OnSetAccessImplOriginalTypeGenericArgInvalid {}.into());
            }

            // hash-exprs
            let hash_args = hash_exprs.iter().take(index);
            let iter_args = iter_args.rev();
            if iter_args.len() != hash_args.len() {
                return Err(AddonBugs::OnSetAccessHashExprArgsLenInvalid {}.into());
            }

            for (arg, exp) in iter_args.zip(hash_args) {
                let GenericArgument::Const(c) = arg else {
                    return Err(AddonBugs::OnSetAccessHashExprArgNotConst {}.into());
                };

                if c.to_token_stream().to_string() != exp.to_token_stream().to_string() {
                    return Err(AddonBugs::OnSetAccessHashExprArgConstInvalid {}.into());
                }
            }

            // original instance type's generic where clauses
            let exp_where_preds = {
                let mut collect = Vec::new();
                for attr in &context.attrs {
                    if attr.style != AttrStyle::Outer {
                        continue;
                    }
                    let Meta::List(list) = &attr.meta else {
                        continue;
                    };
                    if !list.path.is_ident("self_bounds") {
                        continue;
                    }
                    let Ok(bounds) = parse::<BoundsList>(list.tokens.clone().into()) else {
                        return Err(<BoundsList as ParseDiagnostic>::span_parse_error(
                            &list.tokens.span(),
                            Some(ParseError::BoundsList.into()),
                        ));
                    };
                    for bound in bounds.bounds {
                        collect.push(bound);
                    }
                }
                collect
            };
            let found_where_preds = {
                let mut collect = Vec::new();
                if let Some(where_clause) = &impl_of.generics.where_clause {
                    for pred in &where_clause.predicates {
                        collect.push(pred);
                    }
                }
                collect
            };
            for expected in &exp_where_preds {
                let found = found_where_preds.iter().any(|actual| {
                    expected.to_token_stream().to_string() == actual.to_token_stream().to_string()
                });

                if !found {
                    return Err(
                        AddonBugs::OnSetAccessImplOriginalTypeGenericWhereClausesInvalid {}.into(),
                    );
                }
            }

            // original instance type's generic params idents
            let exp_gen_params =
                { extract_generic_idents_from_ty(&context.self_ty, &context.generics) };
            let found_gen_params = &impl_of.generics.params;
            for expected in &exp_gen_params {
                let found = found_gen_params.iter().any(|actual| actual == expected);

                if !found {
                    return Err(
                        AddonBugs::OnSetAccessImplOriginalTypeGenericParamsInvalid {}.into(),
                    );
                }
            }

            // trait's only assoc type
            if impl_of.items.is_empty() {
                return Err(AddonBugs::OnSetAccessImplAddonWithNoAssocItems {}.into());
            }
            if impl_of.items.len() != 2 {
                return Err(AddonBugs::OnSetAccessImplHasExcessAssocs {}.into());
            }
            let Some(ty) = impl_of.items.iter().find_map(|item| {
                let ImplItem::Type(ty) = item else {
                    return None;
                };
                return Some(ty);
            }) else {
                return Err(AddonBugs::OnSetAccessImplCounterAssocTypeNotFound {}.into());
            };

            let exp_ident = format_ident!("OnSet");
            if ty.ident != exp_ident {
                return Err(AddonBugs::OnSetAccessImplAddonTypeAssocIdentInvalid {}.into());
            }
            let found_ty = ImplCountersTypeNum::checked_utilize(context, counters)?.0;
            if found_ty.to_token_stream().to_string() != ty.ty.to_token_stream().to_string() {
                return Err(AddonBugs::OnSetAccessImplInvalidCounterTy {}.into());
            }

            // trait's only const type
            let Some(c) = impl_of.items.iter().find_map(|item| {
                let ImplItem::Const(c) = item else {
                    return None;
                };
                return Some(c);
            }) else {
                return Err(AddonBugs::OnSetAccessImplHashAssocConstNotFound {}.into());
            };

            let exp_ident = format_ident!("INITIAL");
            if c.ident != exp_ident {
                return Err(AddonBugs::OnSetAccessImplAddonConstAssocIdentInvalid {}.into());
            }
            let exp_ty: Type = parse_quote!(u64);
            if exp_ty.to_token_stream().to_string() != c.ty.to_token_stream().to_string() {
                return Err(AddonBugs::OnSetAccessImplInvalidHashConstTy {}.into());
            }

            let Some(exp_expr) = hash_exprs.get(index) else {
                return Err(AddonBugs::OnSetAccessHashOfIndexFromHashExprsUnavailable {}.into());
            };
            if exp_expr.to_token_stream().to_string() != c.expr.to_token_stream().to_string() {
                return Err(AddonBugs::OnSetAccessImplInvalidConstAssocExpr {}.into());
            }

            Ok(())
        };

        let (trailing, _) = counters.iter().enumerate().rev().try_fold(
            (Vec::new(), false),
            |(mut acc, done), (idx, item)| -> Result<_, TokenStream> {
                if done {
                    return Ok((acc, done));
                }

                if parse_pos_usize(&item.const_lit)? == 0 {
                    acc.push(idx);
                    Ok((acc, false))
                } else {
                    // Stop after the first non-zero, whether or not we've
                    // collected any trailing zeros.
                    Ok((acc, true))
                }
            },
        )?;

        for index in trailing {
            let exp_trait_ident = format_ident!("OnSetAccess{}", index);
            let mut found = false;
            for item in &transform.items {
                let Item::Impl(impl_of) = item else {
                    continue;
                };

                let Some((_, trait_path, _)) = &impl_of.trait_ else {
                    continue;
                };

                let Some(seg) = trait_path.segments.last() else {
                    continue;
                };

                if seg.ident == exp_trait_ident {
                    validate_impl(impl_of, index)?;
                    found = true;
                    break;
                }
            }

            if !found {
                return Err(AddonBugs::OnSetAccessImplNotFound {}.into());
            };
        }
        Ok(())
    }
}

// ===============================================================================
// ``````````````````````` TERMINAL INSTANCE ACCESS ADDON ````````````````````````
// ===============================================================================

/// Generates a compile-time lookup implementation for the unique
/// terminal instance of an implementing `Self` type.
///
/// Unlike [`CounterAccessAddon`], which maps an identifier hash sequence
/// and implementing type to an instance's typenum counter tuple, this
/// addon is generated only when the first instance counter is declared
/// terminal through `#[last_instance(...)]`.
///
/// Consider the following sequence:
///
/// ```text
/// InstanceTrait<0,0,0>
/// InstanceTrait<0,0,1>
/// InstanceTrait<0,0,2>   #[last_instance(2)]
/// InstanceTrait<0,1,0>   #[last_instance(2)]
/// InstanceTrait<0,2,0>   #[last_instance(1)]
/// InstanceTrait<1,0,0>   #[last_instance(0,1,2)] or #[last_instance(0)]
/// ```
///
/// Marking only counters `2` or `3` as terminal does not uniquely
/// identify the instance, since earlier counters may still advance and
/// produce additional implementations.
///
/// Only once counter `0` is terminal can no further instance exist for
/// the implementing `Self` type.
///
/// Example:
///
/// ```ignore
/// #[last_instance(0,1,2)] // or #[last_instance(0)]
/// impl InstanceTrait<1,0,0> for InstanceTy { ... }
/// ```
///
/// generates:
///
/// ```ignore
/// impl TerminalAccess<InstanceTy> for instances::Global
/// {
///     type Terminal = (U1, U0, U0); // the type-num counter
/// }
/// ```
///
/// As with [`CounterAccessAddon`], only the generic parameters appearing
/// in the implementing `Self` type are preserved. Any bounds required by
/// those parameters are recovered from `#[self_bounds(...)]`.
///
/// Later expansion phases may therefore recover the terminal instance's
/// typenum counter ([`typenum`](crate::impls::typenum)) tuple directly
/// from the implementing type:
///
/// ```ignore
/// <Global as TerminalAccess<InstanceTy>>::Terminal
/// ```
///
/// This provides the canonical compile-time mapping from the unique
/// terminal implementing type to its typenum counter tuple, eliminating
/// the need for identifier-based lookup once the first counter has been
/// declared terminal.
#[derive(Debug, Clone)]
pub(crate) struct TerminalAccessAddon;

impl<'a> Transformation<File, (&ItemImpl, CounterArgsSlice<'a>, &LitInt)> for TerminalAccessAddon {
    fn raw_transform(
        &self,
        transform: &mut File,
        context: &(&ItemImpl, CounterArgsSlice<'a>, &LitInt),
    ) -> Result<(), proc_macro2::TokenStream> {
        let (context, counters, lit) = context;
        let given = parse_pos_usize(lit)?;

        let Some(first) = counters.first() else {
            return Err(LastBugs::CounterArgsAssocIsEmptyForFirstCounterAccess {}.into());
        };

        if first.generic_index != given {
            return Ok(());
        }

        let self_ty = &context.self_ty;
        let generics = &context.generics;
        let new_params = extract_generic_idents_from_ty(self_ty, generics);
        let attrs = &context.attrs;
        let mut new_predicates = Punctuated::<WherePredicate, Comma>::new();
        for attr in attrs {
            if attr.style != AttrStyle::Outer {
                continue;
            }
            let Meta::List(list) = &attr.meta else {
                continue;
            };
            if !list.path.is_ident("self_bounds") {
                continue;
            }
            let bounds = match parse::<BoundsList>(list.tokens.clone().into()) {
                Ok(t) => t,
                Err(_) => {
                    return Err(<BoundsList as ParseDiagnostic>::span_parse_error(
                        &list.tokens.span(),
                        Some(ParseError::BoundsList.into()),
                    ));
                }
            };
            new_predicates.extend(bounds.bounds);
        }

        let new_generics = Generics {
            lt_token: Default::default(),
            params: new_params,
            gt_token: Default::default(),
            where_clause: Some(WhereClause {
                where_token: Default::default(),
                predicates: new_predicates,
            }),
        };

        let (impl_generics, _, where_clause) = new_generics.split_for_impl();

        let typenum = ImplCountersTypeNum::checked_utilize(context, counters)?.0;
        let trait_ident = format_ident!("TerminalAccess");
        let crate_of = Instance::support_crate();

        let item = parse_quote!(
            impl #impl_generics #crate_of::#trait_ident <#self_ty > for #crate_of::Global
            #where_clause
            {
                type Terminal = #typenum;
            }
        );

        transform.items.push(Item::Impl(item));

        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &File,
        context: Option<&(&ItemImpl, CounterArgsSlice<'a>, &LitInt)>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let Some((context, counters, lit)) = context else {
            return Ok(());
        };
        let given = parse_pos_usize(lit)?;

        let Some(first) = counters.first() else {
            return Err(LastBugs::CounterArgsAssocIsEmptyForFirstCounterAccess {}.into());
        };

        let exp_trait_ident = format_ident!("TerminalAccess");

        if first.generic_index != given {
            for item in &transform.items {
                let Item::Impl(impl_of) = item else {
                    continue;
                };

                let Some((_, trait_p, _)) = &impl_of.trait_ else {
                    continue;
                };

                let Some(seg) = trait_p.segments.last() else {
                    continue;
                };

                if seg.ident == exp_trait_ident {
                    return Err(
                        AddonBugs::TerminalAccessImplAttemptedForNonTerminalCounter {}.into(),
                    );
                }
            }
            return Ok(());
        }

        let validate_impl = |impl_of: &ItemImpl, context: &ItemImpl| -> Result<(), TokenStream> {
            let crate_of = Instance::support_crate();

            // trait ident
            let trait_of = ImplTraitIdent::checked_utilize(impl_of, &())?.0;
            if *trait_of != exp_trait_ident {
                return Err(AddonBugs::InvalidTerminalAccessImplIdent {}.into());
            };

            // self as instance's global
            let self_ty = &*impl_of.self_ty;
            let exp_self_ty: Type = parse_quote!(#crate_of::Global);
            if self_ty.to_token_stream().to_string() != exp_self_ty.to_token_stream().to_string() {
                return Err(AddonBugs::InvalidGlobalTypePathForTerminalAccess {}.into());
            }

            // trait full path
            let trait_path = ImplTraitPath::checked_utilize(impl_of, &())?.0;
            let exp_path: Path = parse_quote!(#crate_of::#exp_trait_ident);
            if trait_path.segments.len() != exp_path.segments.len() {
                return Err(AddonBugs::InvalidTerminalAccessImplPathSegmentsLen {}.into());
            }

            for (found, exp) in trait_path.segments.iter().zip(&exp_path.segments) {
                if found.ident != exp.ident {
                    return Err(AddonBugs::InvalidTerminalAccessImplPathSegment {}.into());
                }
            }

            // trait generic args
            let Some(last_path) = trait_path.segments.last() else {
                return Err(AddonBugs::InvalidTerminalAccessImplTraitSegment {}.into());
            };
            let PathArguments::AngleBracketed(found_angle_args) = &last_path.arguments else {
                return Err(AddonBugs::TerminalAccessImplHasInvalidGenericArgs {}.into());
            };
            if found_angle_args.args.len() != 1 {
                return Err(AddonBugs::TerminalAccessImplExpectedGenericArgsLenInvalid {}.into());
            }

            // trait generic arg original instance implementing type
            let Some(orig_ty) = found_angle_args.args.first() else {
                return Err(
                    AddonBugs::TerminalAccessImplOriginalTypeGenericArgUnavailable {}.into(),
                );
            };
            let GenericArgument::Type(orig_ty) = orig_ty else {
                return Err(AddonBugs::TerminalAccessImplOriginalTypeNotTypeGenericArg {}.into());
            };

            if orig_ty.to_token_stream().to_string()
                != context.self_ty.to_token_stream().to_string()
            {
                return Err(AddonBugs::TerminalAccessImplOriginalTypeGenericArgInvalid {}.into());
            }

            // original instance type's generic where clauses
            let exp_where_preds = {
                let mut collect = Vec::new();
                for attr in &context.attrs {
                    if attr.style != AttrStyle::Outer {
                        continue;
                    }
                    let Meta::List(list) = &attr.meta else {
                        continue;
                    };
                    if !list.path.is_ident("self_bounds") {
                        continue;
                    }
                    let Ok(bounds) = parse::<BoundsList>(list.tokens.clone().into()) else {
                        return Err(<BoundsList as ParseDiagnostic>::span_parse_error(
                            &list.tokens.span(),
                            Some(ParseError::BoundsList.into()),
                        ));
                    };
                    for bound in bounds.bounds {
                        collect.push(bound);
                    }
                }
                collect
            };
            let found_where_preds = {
                let mut collect = Vec::new();
                if let Some(where_clause) = &impl_of.generics.where_clause {
                    for pred in &where_clause.predicates {
                        collect.push(pred);
                    }
                }
                collect
            };
            for expected in &exp_where_preds {
                let found = found_where_preds.iter().any(|actual| {
                    expected.to_token_stream().to_string() == actual.to_token_stream().to_string()
                });

                if !found {
                    return Err(
                        AddonBugs::TerminalAccessImplOriginalTypeGenericWhereClausesInvalid {}
                            .into(),
                    );
                }
            }

            // original instance type's generic params idents
            let exp_gen_params =
                { extract_generic_idents_from_ty(&context.self_ty, &context.generics) };
            let found_gen_params = &impl_of.generics.params;
            for expected in &exp_gen_params {
                let found = found_gen_params.iter().any(|actual| actual == expected);

                if !found {
                    return Err(
                        AddonBugs::TerminalAccessImplOriginalTypeGenericParamsInvalid {}.into(),
                    );
                }
            }

            // trait's only assoc type
            if impl_of.items.is_empty() {
                return Err(AddonBugs::TerminalAccessImplAddonWithNoAssocItems {}.into());
            }
            let mut iter = impl_of.items.iter();
            let Some(first) = iter.next() else {
                return Err(AddonBugs::TerminalAccessImplAddonWithNoAssocItems {}.into());
            };
            let ImplItem::Type(ty) = first else {
                return Err(AddonBugs::TerminalAccessImplCounterAssocNotType {}.into());
            };
            let exp_ident = format_ident!("Terminal");
            if ty.ident != exp_ident {
                return Err(AddonBugs::TerminalAccessImplAddonTypeAssocIdentInvalid {}.into());
            }
            let found_ty = ImplCountersTypeNum::checked_utilize(context, counters)?.0;
            if found_ty.to_token_stream().to_string() != ty.ty.to_token_stream().to_string() {
                return Err(AddonBugs::TerminalAccessImplInvalidCounterTy {}.into());
            }
            if iter.next().is_some() {
                return Err(AddonBugs::TerminalAccessImplHasExcessAssocs {}.into());
            }

            Ok(())
        };

        let mut found = false;
        for item in &transform.items {
            let Item::Impl(impl_of) = item else {
                continue;
            };

            let Some((_, trait_path, _)) = &impl_of.trait_ else {
                continue;
            };

            let Some(seg) = trait_path.segments.last() else {
                continue;
            };

            if seg.ident == exp_trait_ident {
                validate_impl(impl_of, context)?;
                found = true;
                break;
            }
        }

        if !found {
            return Err(AddonBugs::TerminalAccessImplNotFound {}.into());
        };

        Ok(())
    }
}

// ===============================================================================
// ``````````````````````` TERMINAL INSTANCE CHECKER ADDON ```````````````````````
// ===============================================================================

/// Generates the compile-time entry point that validates the complete
/// instance hierarchy.
///
/// Every instance implementation owns a cumulative checker generated by
/// [`CumulatedConstChecker`]. Each cumulative checker first
/// validates the current instance, then recursively evaluates the
/// cumulative checker of its immediate predecessor.
///
/// Consider the following instance hierarchy:
///
/// ```text
/// Try<0,0,0>
/// Try<0,0,1>
/// Try<0,0,2>   #[last_instance(3)]
/// Try<0,1,0>   #[last_instance(3)]
/// Try<0,2,0>   #[last_instance(2)]
/// Try<1,0,0>   #[last_instance(1,2,3)]   <-- terminal instance
/// ```
///
/// The cumulative checker generated for the terminal instance behaves
/// conceptually as:
///
/// ```ignore
/// const INSTANCE_CUMULATED_CHECKER: () = {
///     check_current_instance();
///
///     <PreviousInstance as Try<...>>::INSTANCE_CUMULATED_CHECKER
/// };
/// ```
///
/// which expands recursively into:
///
/// ```text
/// Try<1,0,0>
///     |-- validates Try<1,0,0>
///           |-- validates Try<0,2,0>
///                 |-- validates Try<0,1,0>
///                       |-- validates Try<0,0,2>
///                             |-- validates Try<0,0,1>
///                                   |-- validates Try<0,0,0>
/// ```
///
/// However, associated constants are evaluated only when referenced.
/// Merely generating the recursive checker does not execute it.
///
/// This addon therefore generates a hidden top-level constant:
///
/// ```ignore
/// #[doc(hidden)]
/// const _: () =
///     <Phantom<R> as Try<'a, 1, 0, 0, T, B>>::INSTANCE_CUMULATED_CHECKER;
/// ```
///
/// Evaluating this constant forces evaluation of the terminal instance's
/// cumulative checker, which recursively evaluates every predecessor
/// checker, thereby validating the entire generated instance graph.
///
/// This constant is generated only when:
///
/// - the implementation is terminal for the first counter dimension
///   (that is, `#[last_instance(...)]` includes the first counter), and
/// - the implementation declares no generic parameters.
///
/// Generic implementations are excluded because top-level constants
/// cannot currently be parameterized by generic arguments. Their
/// cumulative checker must instead be evaluated later from a
/// monomorphic resolution site after all generic parameters have been
/// substituted.
#[derive(Debug, Clone)]
pub(crate) struct TerminalCheckerAddon;

impl<'a> Transformation<File, (&ItemImpl, CounterArgsSlice<'a>, &LitInt)> for TerminalCheckerAddon {
    fn raw_transform(
        &self,
        transform: &mut File,
        context: &(&ItemImpl, CounterArgsSlice<'a>, &LitInt),
    ) -> Result<(), proc_macro2::TokenStream> {
        let (context, counters, lit) = context;
        let given = parse_pos_usize(lit)?;

        let Some(first) = counters.first() else {
            return Err(LastBugs::CounterArgsAssocIsEmptyForFirstCounterAccess {}.into());
        };

        if first.generic_index != given {
            return Ok(());
        }

        let self_ty = &context.self_ty;
        let generics = &context.generics;

        if !generics.params.is_empty() {
            return Ok(());
        }

        let trait_path = ImplTraitPath::checked_utilize(context, &())?.0;

        let cumulated_check = gen_const_ident::<CumulatedConstChecker>();
        let checker = ItemConst {
            attrs: proc_suite::internal_code(),
            vis: syn::Visibility::Inherited,
            ident: format_ident!("_"),
            ty: parse_quote!(()),
            expr: parse_quote!(<#self_ty as #trait_path>::#cumulated_check),
            generics: Default::default(),
            const_token: Default::default(),
            colon_token: Default::default(),
            eq_token: Default::default(),
            semi_token: Default::default(),
        };

        transform.items.push(Item::Const(checker));

        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &File,
        context: Option<&(&ItemImpl, CounterArgsSlice<'a>, &LitInt)>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let Some((context, counters, lit)) = context else {
            return Ok(());
        };

        if !context.generics.params.is_empty() {
            return Ok(());
        }

        let given = parse_pos_usize(lit)?;

        let Some(first) = counters.first() else {
            return Err(LastBugs::CounterArgsAssocIsEmptyForFirstCounterAccess {}.into());
        };

        let exp_const_ident = format_ident!("_");
        if first.generic_index != given {
            for item in &transform.items {
                let Item::Const(c) = item else {
                    continue;
                };

                if c.ident == exp_const_ident {
                    return Err(
                        AddonBugs::TerminalCheckerConstAttemptedForNonTerminalCounter {}.into(),
                    );
                }
            }
            return Ok(());
        }

        let validate_const = |c: &ItemConst, context: &ItemImpl| -> Result<(), TokenStream> {
            // trait ident
            if c.ident != exp_const_ident {
                return Err(AddonBugs::TerminalCheckerConstInvalidIdent {}.into());
            };

            let expr = &*c.expr;

            let self_ty = &context.self_ty;
            let generics = &context.generics;

            if !generics.params.is_empty() {
                return Err(AddonBugs::TerminalCheckerConstAttemptedForGenericImpl {}.into());
            }

            let trait_path = ImplTraitPath::checked_utilize(context, &())?.0;

            let cumulated_check = gen_const_ident::<CumulatedConstChecker>();

            let exp_expr: Expr = parse_quote!(<#self_ty as #trait_path>::#cumulated_check);

            if expr.to_token_stream().to_string() != exp_expr.to_token_stream().to_string() {
                return Err(AddonBugs::TerminalCheckerConstInvalidExpr {}.into());
            }

            Ok(())
        };

        let mut found = false;
        for item in &transform.items {
            let Item::Const(c) = item else {
                continue;
            };

            if c.ident == exp_const_ident {
                validate_const(c, context)?;
                found = true;
                break;
            }
        }

        if !found {
            return Err(AddonBugs::TerminalCheckerConstNotFound {}.into());
        };

        Ok(())
    }
}

// ===============================================================================
// ```````````````````````` BOUNDARY INSTANCE ACCESS ADDON ```````````````````````
// ===============================================================================

/// Generates compile-time lookup implementations describing the
/// boundary instance established by a terminal counter.
///
/// Unlike [`TerminalAccessAddon`], which is generated only when the
/// first counter becomes terminal and therefore identifies the unique
/// terminal instance of an implementing `Self` type, this addon is
/// generated for every `#[last_instance(...)]` declaration.
///
/// Consider the following sequence:
///
/// ```text
/// InstanceTrait<0,0,0>
/// InstanceTrait<0,0,1>
/// InstanceTrait<0,0,2>   #[last_instance(2)]
/// InstanceTrait<0,1,0>   #[last_instance(2)]
/// InstanceTrait<0,2,0>   #[last_instance(1)]
/// InstanceTrait<1,0,0>   #[last_instance(0,1,2)]
/// ```
///
/// Declaring:
///
/// ```ignore
/// #[last_instance(1)]
/// ```
///
/// establishes the boundary beginning at counter `1`.
///
/// Once a counter becomes terminal, no later instance may advance that
/// counter while preserving all preceding counter values. Consequently,
/// every trailing counter dimension is also bounded by the same
/// terminal instance.
///
/// Therefore this addon generates one lookup implementation for the
/// declared terminal counter and every subsequent counter dimension:
///
/// ```text
/// #[last_instance(1)]
///        |-- BoundaryAccess1
///        |-- BoundaryAccess2
///        |-- ...
/// ```
///
/// The boundary instance represents the final child instance reachable
/// beneath a partially resolved parent identifier sequence.
///
/// Once the identifier hashes preceding counter dimension `N` have been
/// resolved, every child instance sharing those parent identifiers forms
/// a contiguous hierarchy. The terminal instance of that hierarchy is
/// the boundary beyond which counter `N` can no longer advance.
///
/// Consequently, the parent identifier sequence alone is sufficient to
/// recover the typenum counter tuple of that child hierarchy's boundary
/// instance, without requiring any remaining identifier hashes.
///
/// Each generated implementation maps:
///
/// ```text
/// (identifier hashes before N..., implementing Self type)
///                     |
///                     v
///         Global : BoundaryAccessN
///                     |
///                     v
///       boundary hash + typenum counter tuple
/// ```
///
/// Example:
///
/// ```ignore
/// impl<R> BoundaryAccess1<
///     { hash_ident(b"x") },
///     Phantom<R>,
/// > for Global
/// where
///     R: MarkerTrait,
/// {
///     const FINAL: u64 = { hash_ident(b"y2") };
///     type Boundary = (U0, U2, U4);
/// }
/// ```
///
/// Here, `FINAL` is the identifier hash of the boundary for counter
/// dimension `1`, while `Boundary` exposes the typenum counter tuple of
/// the boundary instance.
///
/// As with [`CounterAccessAddon`], only the generic parameters appearing
/// in the implementing `Self` type are preserved. Any bounds required by
/// those parameters are recovered from:
///
/// ```ignore
/// #[self_bounds(...)]
/// ```
///
/// Later expansion phases can recover both the terminating identifier
/// hash and the corresponding typenum counter tuple:
///
/// ```ignore
/// <Global as BoundaryAccess1<
///     HASH_X,
///     Phantom<R>,
/// >>::FINAL
///
/// <Global as BoundaryAccess1<
///     HASH_X,
///     Phantom<R>,
/// >>::Boundary
/// ```
///
/// This provides the canonical compile-time mapping from a partially
/// resolved parent identifier sequence to the boundary (terminal) child
/// instance of that hierarchy.
#[derive(Debug, Clone)]
pub(crate) struct BoundaryAccessAddon;

impl<'a> Transformation<File, (&ItemImpl, CounterArgsSlice<'a>, &LitInt)> for BoundaryAccessAddon {
    fn raw_transform(
        &self,
        transform: &mut File,
        context: &(&ItemImpl, CounterArgsSlice<'a>, &LitInt),
    ) -> Result<(), proc_macro2::TokenStream> {
        let (context, counters, lit) = context;
        let given = parse_pos_usize(lit)?;

        let counters_len = counters.len();
        let Some(index) = counters.iter().enumerate().find_map(|(idx, counter)| {
            if counter.generic_index == given {
                return Some(idx);
            }
            None
        }) else {
            return Err(AddonBugs::LastInstanceCounterInvalid {}.into());
        };

        let hash_exprs = counter_access_impl_ident_hash_args(context, counters)?;
        let self_ty = &context.self_ty;
        let generics = &context.generics;
        let new_params = extract_generic_idents_from_ty(self_ty, generics);
        let attrs = &context.attrs;
        let mut new_predicates = Punctuated::<WherePredicate, Comma>::new();
        for attr in attrs {
            if attr.style != AttrStyle::Outer {
                continue;
            }
            let Meta::List(list) = &attr.meta else {
                continue;
            };
            if !list.path.is_ident("self_bounds") {
                continue;
            }
            let bounds = match parse::<BoundsList>(list.tokens.clone().into()) {
                Ok(t) => t,
                Err(_) => {
                    return Err(<BoundsList as ParseDiagnostic>::span_parse_error(
                        &list.tokens.span(),
                        Some(ParseError::BoundsList.into()),
                    ));
                }
            };
            new_predicates.extend(bounds.bounds);
        }

        let new_generics = Generics {
            lt_token: Default::default(),
            params: new_params,
            gt_token: Default::default(),
            where_clause: Some(WhereClause {
                where_token: Default::default(),
                predicates: new_predicates,
            }),
        };

        let (impl_generics, _, where_clause) = new_generics.split_for_impl();

        let typenum = ImplCountersTypeNum::checked_utilize(context, counters)?.0;
        let crate_of = Instance::support_crate();

        for index in index..counters_len {
            let Some(boundary_hash) = hash_exprs.get(index) else {
                return Err(AddonBugs::BoundaryAccessHashOfIndexFromHashExprsUnavailable {}.into());
            };
            let hash_args = hash_exprs.iter().take(index);
            let trait_ident = format_ident!("BoundaryAccess{}", index);

            let item = if hash_args.len() == 0 {
                parse_quote! {
                    impl #impl_generics
                        #crate_of::#trait_ident<#self_ty>
                        for #crate_of::Global
                        #where_clause
                    {
                        const FINAL: u64 = #boundary_hash;
                        type Boundary = #typenum;
                    }
                }
            } else {
                parse_quote! {
                    impl #impl_generics
                        #crate_of::#trait_ident<#(#hash_args),*, #self_ty>
                        for #crate_of::Global
                        #where_clause
                    {
                        const FINAL: u64 = #boundary_hash;
                        type Boundary = #typenum;
                    }
                }
            };

            transform.items.push(Item::Impl(item));
        }

        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &File,
        context: Option<&(&ItemImpl, CounterArgsSlice<'a>, &LitInt)>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let Some((context, counters, lit)) = context else {
            return Ok(());
        };

        let given = parse_pos_usize(lit)?;

        let hash_exprs = counter_access_impl_ident_hash_args(context, counters)?;

        let validate_impl = |impl_of: &ItemImpl, index: usize| -> Result<(), TokenStream> {
            let crate_of = Instance::support_crate();

            // self as instance's global
            let self_ty = &*impl_of.self_ty;
            let exp_self_ty: Type = parse_quote!(#crate_of::Global);
            if self_ty.to_token_stream().to_string() != exp_self_ty.to_token_stream().to_string() {
                return Err(AddonBugs::InvalidGlobalTypePathForBoundaryAccess {}.into());
            }

            // trait full path
            let exp_trait_ident = format_ident!("BoundaryAccess{}", index);
            let trait_path = ImplTraitPath::checked_utilize(impl_of, &())?.0;
            let exp_path: Path = parse_quote!(#crate_of::#exp_trait_ident);
            if trait_path.segments.len() != exp_path.segments.len() {
                return Err(AddonBugs::InvalidBoundaryAccessImplPathSegmentsLen {}.into());
            }

            for (found, exp) in trait_path.segments.iter().zip(&exp_path.segments) {
                if found.ident != exp.ident {
                    return Err(AddonBugs::InvalidBoundaryAccessImplPathSegment {}.into());
                }
            }

            // trait generic args
            let Some(last_path) = trait_path.segments.last() else {
                return Err(AddonBugs::InvalidBoundaryAccessImplTraitSegment {}.into());
            };
            let PathArguments::AngleBracketed(found_angle_args) = &last_path.arguments else {
                return Err(AddonBugs::BoundaryAccessImplHasInvalidGenericArgs {}.into());
            };

            let mut iter_args = found_angle_args.args.iter().rev();
            // trait generic arg original instance implementing type
            let Some(orig_ty) = iter_args.next() else {
                return Err(
                    AddonBugs::BoundaryAccessImplOriginalTypeGenericArgUnavailable {}.into(),
                );
            };
            let GenericArgument::Type(orig_ty) = orig_ty else {
                return Err(AddonBugs::BoundaryAccessImplOriginalTypeNotTypeGenericArg {}.into());
            };

            if orig_ty.to_token_stream().to_string()
                != context.self_ty.to_token_stream().to_string()
            {
                return Err(AddonBugs::BoundaryAccessImplOriginalTypeGenericArgInvalid {}.into());
            }

            // hash-exprs
            let hash_args = hash_exprs.iter().take(index);
            let iter_args = iter_args.rev();
            if iter_args.len() != hash_args.len() {
                return Err(AddonBugs::BoundaryAccessHashExprArgsLenInvalid {}.into());
            }

            for (arg, exp) in iter_args.zip(hash_args) {
                let GenericArgument::Const(c) = arg else {
                    return Err(AddonBugs::BoundaryAccessHashExprArgNotConst {}.into());
                };

                if c.to_token_stream().to_string() != exp.to_token_stream().to_string() {
                    return Err(AddonBugs::BoundaryAccessHashExprArgConstInvalid {}.into());
                }
            }

            // original instance type's generic where clauses
            let exp_where_preds = {
                let mut collect = Vec::new();
                for attr in &context.attrs {
                    if attr.style != AttrStyle::Outer {
                        continue;
                    }
                    let Meta::List(list) = &attr.meta else {
                        continue;
                    };
                    if !list.path.is_ident("self_bounds") {
                        continue;
                    }
                    let Ok(bounds) = parse::<BoundsList>(list.tokens.clone().into()) else {
                        return Err(<BoundsList as ParseDiagnostic>::span_parse_error(
                            &list.tokens.span(),
                            Some(ParseError::BoundsList.into()),
                        ));
                    };
                    for bound in bounds.bounds {
                        collect.push(bound);
                    }
                }
                collect
            };
            let found_where_preds = {
                let mut collect = Vec::new();
                if let Some(where_clause) = &impl_of.generics.where_clause {
                    for pred in &where_clause.predicates {
                        collect.push(pred);
                    }
                }
                collect
            };
            for expected in &exp_where_preds {
                let found = found_where_preds.iter().any(|actual| {
                    expected.to_token_stream().to_string() == actual.to_token_stream().to_string()
                });

                if !found {
                    return Err(
                        AddonBugs::BoundaryAccessImplOriginalTypeGenericWhereClausesInvalid {}
                            .into(),
                    );
                }
            }

            // original instance type's generic params idents
            let exp_gen_params =
                { extract_generic_idents_from_ty(&context.self_ty, &context.generics) };
            let found_gen_params = &impl_of.generics.params;
            for expected in &exp_gen_params {
                let found = found_gen_params.iter().any(|actual| actual == expected);

                if !found {
                    return Err(
                        AddonBugs::BoundaryAccessImplOriginalTypeGenericParamsInvalid {}.into(),
                    );
                }
            }

            // trait's only assoc type
            if impl_of.items.is_empty() {
                return Err(AddonBugs::BoundaryAccessImplAddonWithNoAssocItems {}.into());
            }
            if impl_of.items.len() != 2 {
                return Err(AddonBugs::BoundaryAccessImplHasExcessAssocs {}.into());
            }
            let Some(ty) = impl_of.items.iter().find_map(|item| {
                let ImplItem::Type(ty) = item else {
                    return None;
                };
                return Some(ty);
            }) else {
                return Err(AddonBugs::BoundaryAccessImplCounterAssocTypeNotFound {}.into());
            };

            let exp_ident = format_ident!("Boundary");
            if ty.ident != exp_ident {
                return Err(AddonBugs::BoundaryAccessImplAddonTypeAssocIdentInvalid {}.into());
            }
            let found_ty = ImplCountersTypeNum::checked_utilize(context, counters)?.0;
            if found_ty.to_token_stream().to_string() != ty.ty.to_token_stream().to_string() {
                return Err(AddonBugs::BoundaryAccessImplInvalidCounterTy {}.into());
            }

            // trait's only const type
            let Some(c) = impl_of.items.iter().find_map(|item| {
                let ImplItem::Const(c) = item else {
                    return None;
                };
                return Some(c);
            }) else {
                return Err(AddonBugs::BoundaryAccessImplHashAssocConstNotFound {}.into());
            };

            let exp_ident = format_ident!("FINAL");
            if c.ident != exp_ident {
                return Err(AddonBugs::BoundaryAccessImplAddonConstAssocIdentInvalid {}.into());
            }
            let exp_ty: Type = parse_quote!(u64);
            if exp_ty.to_token_stream().to_string() != c.ty.to_token_stream().to_string() {
                return Err(AddonBugs::BoundaryAccessImplInvalidHashConstTy {}.into());
            }

            let Some(exp_expr) = hash_exprs.get(index) else {
                return Err(AddonBugs::BoundaryAccessHashOfIndexFromHashExprsUnavailable {}.into());
            };
            if exp_expr.to_token_stream().to_string() != c.expr.to_token_stream().to_string() {
                return Err(AddonBugs::BoundaryAccessImplInvalidConstAssocExpr {}.into());
            }

            Ok(())
        };

        let counters_len = counters.len();
        let Some(index) = counters.iter().enumerate().find_map(|(idx, counter)| {
            if counter.generic_index == given {
                return Some(idx);
            }
            None
        }) else {
            return Err(AddonBugs::LastInstanceCounterInvalid {}.into());
        };

        for index in index..counters_len {
            let exp_trait_ident = format_ident!("BoundaryAccess{}", index);
            let mut found = false;
            for item in &transform.items {
                let Item::Impl(impl_of) = item else {
                    continue;
                };

                let Some((_, trait_path, _)) = &impl_of.trait_ else {
                    continue;
                };

                let Some(seg) = trait_path.segments.last() else {
                    continue;
                };

                if seg.ident == exp_trait_ident {
                    validate_impl(impl_of, index)?;
                    found = true;
                    break;
                }
            }

            if !found {
                return Err(AddonBugs::BoundaryAccessImplNotFound {}.into());
            };
        }
        Ok(())
    }
}
