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
// ``````````````````````````````````` IMPL INST `````````````````````````````````
// ===============================================================================

//! Provides the transformation pipeline for instance trait implementations.
//!
//! The module transforms an `#[inst(...)]` implementation into its internal
//! expanded representation, including concrete instance-counter generic
//! arguments, generated instance identifier constants, and the
//! `InstanceCounter` associated type.
//!
//! Instance-node associated types are transformed separately into publisher
//! and subscriber node representations. Publisher nodes receive generated
//! [`NodeArgs`], while qualified trait projections are marked as subscriber
//! nodes.
//! 
//! Instance-tuple associated types are transformed separately into instance
//! node representations and additional addon implementations and enum
//! representations for tuples are added to the resulting file.
//!
//! The transformation is performed in separate phases for the implementation
//! generic header and instance-node/tuple associated types, which are unified by
//! [`ImplInst`]. A documentation-only `cfg(not(feature = "inst"))` implementation 
//! is retained for the final terminal instance implementation so rustdoc 
//! can represent the implementation without exposing generated instance-related generics.
//!
//! Support-crate delegation is used to invoke the explicit instance
//! implementation and instance-node macro phases after the initial
//! transformations.

// ===============================================================================
// ``````````````````````````````````` IMPORTS ```````````````````````````````````
// ===============================================================================

// --- Std Crate ---
use std::{collections::HashSet};

// --- Proc Macro Crates ---
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::{
    AngleBracketedGenericArguments, Attribute, Expr, ExprLit, Fields, File, GenericArgument, GenericParam, Ident, ImplItem, ImplItemConst, ImplItemType, Item, ItemEnum, ItemImpl, Lit, LitByteStr, LitInt, Meta, PathArguments, Type, Visibility::Inherited, parse_quote, parse2, punctuated::{Pair, Punctuated}, token::Comma,
};

// --- Local Crate ---
use crate::{
    Extraction, Inst, TraitInst, Transformation, 
    args::{
        ComplexInstanceRange, InstModelLength, InstSpecList, InstSpecPunct, 
        InstanceIdent, InstanceIdents, InstanceModel, NodeArgs, 
        TerminatedInstances
    }, 
    errors::{ImplBug, ImplError, ImplSpace, ParseBug, ProcParseErr}, 
    traits::cfg_feature_disclaimer,
};

// --- Proc Suite ---
use proc_suite::{DuplicateCheck, IdentList, IntList, SupportCrate, misc::*};

// ===============================================================================
// ````````````````````````````` IMPL INST ENTRY-POINT ```````````````````````````
// ===============================================================================

/// Transforms an instance trait implementation through the [`crate::inst`]
/// macro pipeline.
///
/// An instance implementation supplies concrete instance specifications for
/// the instance parameters declared by the implemented trait. These
/// specifications are lowered into const generic instance counters, generated
/// instance identifier constants, and the `InstanceCounter` associated type.
///
/// Instance-node and tuple associated types are also transformed into their generated
/// publisher or subscriber representations along with additional instance 
/// implementations for tuple-nodes.
///
/// Before transformation:
///
/// ```ignore
/// #[inst(Module[0], Function[1])]
/// impl Hasher for MyHasher {
///     #[node(...)]
///     type Hash = ...;
/// 
///     #[tuple(...)]
///     type Tuple = ...;
/// }
/// ```
///
/// The implementation is transformed into an internal `cfg(feature = "inst")` form
/// containing the generated instance metadata:
///
/// ```ignore
/// #[cfg(feature = "inst")]
/// #[support_crate::instance_impl_explicit_target(...)]
/// #[support_crate::instance_node_explicit_target]
/// impl Hasher<0, 1> for MyHasher {
///     const __INST_0_IDENT: &'static [u8] = b"Module";
///     const __INST_1_IDENT: &'static [u8] = b"Function";
///     type InstanceCounter = u8;
///
///     #[instance_pub(...)] // or #[instance_sub]
///     type Hash = ...;
/// 
///     #[instance_pub(...)] // or #[instance_sub]
///     type Tuple = TupleEnum
/// }
/// 
/// enum TupleEnum { .. }
/// 
/// impl InstTuple for TupleEnum { .. }
/// ```
///
/// The trait implementation's instance specifications determine the concrete
/// counter arguments inserted into the implemented trait. Generated
/// identifier constants preserve the instance names for delegated macro
/// expansion.
///
/// Instance-node and tuple associated types are transformed separately from the
/// implementation generic arguments, but both phases are unified by the same
/// [`ImplInst`] macro pipeline.
///
/// A documentation-oriented `cfg(not(feature = "inst"))` copy is retained only 
/// for the final terminal instance implementation. Multiple instance 
/// implementations may exist for the same type, but their generated 
/// instance-related generics are hidden from rustdoc, so only the final 
/// terminal implementation is retained as the documentation representation.
///
/// The transformation also adds the metadata required for delegated macro
/// expansion.
#[derive(Debug, Clone)]
pub struct ImplInst;

impl Transformation<File, TokenStream> for ImplInst {
    fn raw_transform(
        &self,
        transform: &mut File,
        context: &TokenStream,
    ) -> Result<(), proc_macro2::TokenStream> {
        if transform.items.len() > 1 {
            return Err(ParseBug::TooManyItemsToTransform {}.into());
        }
        let Some(Item::Impl(impl_)) = transform
            .items
            .iter_mut()
            .find(|item| matches!(item, Item::Impl(_)))
        else {
            return Err(ImplBug::FileContainsNoImpl {}.into());
        };

        let idents = match context.is_empty() {
            true => None,
            false => {
                let idents = InstSpecList::checked_extract(context, &())?;
                Some(idents)
            }
        };

        let doc_impl = impl_.clone();
        let mut addon = File {shebang: None, attrs: Vec::new(), items: Vec::new()};

        ItemTypeInst::checked_transform(&ItemTypeInst, &mut (&mut addon, impl_), &idents)?;
        cfg_feature_disclaimer(&mut impl_.attrs);
        if let Some(idents) = &idents {
            ImplInstGenericsHeader::checked_transform(&ImplInstGenericsHeader, impl_, idents)?;
        }
        let mut doc_file = File {shebang: None, attrs: Vec::new(), items: Vec::new()};
        doc_file.items.push(Item::Impl(doc_impl));
        ImplInstDoc::checked_transform(&ImplInstDoc, &mut doc_file, &idents)?;

        if doc_file.items.is_empty() && idents.is_some() {
            impl_.attrs.insert(0,Inst::feature());
        }

        if !doc_file.items.is_empty() {
            impl_.attrs.insert(0,Inst::feature());

            for item in doc_file.items {
                transform.items.push(item);
            }
        }

        for item in addon.items {
            transform.items.push(item);
        }

        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &File,
        _: Option<&TokenStream>,
    ) -> Result<(), proc_macro2::TokenStream> {
        if transform.items.is_empty() {
            return Err(ImplBug::TransformedFileInconsistent {}.into());
        }
        Ok(())
    }
}

// ===============================================================================
// ```````````````````````````` IMPL INST DOC-TARGET ````````````````````````````
// ===============================================================================

/// Transforms an instance implementation into its documentation representation.
///
/// This transformation operates on the cfg(not(feature = "inst")) copy
/// produced by [ImplInst].
///
/// A documentation implementation is generated only when it is required for
/// a terminal instance. For a non-instance implementation, the documentation
/// implementation is instead generated for its associated nodes.
///
/// During the transformation, internal #[node(...)], #[sum(...)], and
/// #[tuple(...)] annotations are removed from associated types. For an
/// associated type with a #[tuple(...)] annotation, the tuple type is
/// replaced with its first tuple element.
///
/// The resulting implementation therefore exposes the user-facing
/// documentation representation without the internal instance transformation
/// annotations.
#[derive(Debug, Clone)]
pub struct ImplInstDoc;

impl Transformation<File, Option<InstSpecList>> for ImplInstDoc {
    fn raw_transform(
        &self,
        transform: &mut File,
        context: &Option<InstSpecList>,
    ) -> Result<(), proc_macro2::TokenStream> {
        if transform.items.len() > 1 {
            return Err(ParseBug::TooManyItemsToTransform {}.into());
        }
        let Some(Item::Impl(impl_)) = transform
            .items
            .iter_mut()
            .find(|item| matches!(item, Item::Impl(_)))
        else {
            return Err(ImplBug::FileContainsNoImpl {}.into());
        };

        for ty in &mut impl_.items {
            let ImplItem::Type(ty) = ty else {
                continue;
            };
            ty.attrs.retain(|attr| !attr.path().is_ident(&Inst::node()));
            ty.attrs.retain(|attr| !attr.path().is_ident(&Inst::sum()));

            if !ty.attrs.iter().any(|attr| attr.path().is_ident(&Inst::tuple())) {
                continue;
            };

            ty.attrs.retain(|attr| !attr.path().is_ident(&Inst::tuple()));

            if let Type::Tuple(tup) = &ty.ty {
                ty.ty = tup.elems[0].clone();
            };
        }

        let push_attr;
        if let Some(idents) = context {
            if idents
                .idents
                .pairs()
                .next()
                .is_some_and(|pair| matches!(pair.punct(), Some(InstSpecPunct::SemiColon(_))))
            {
                push_attr = true;
            
            } else {
                push_attr = false;
            };
        } else {
            push_attr = true;
        }

        if push_attr {
            impl_.attrs.insert(0, Inst::not_feature())
        } else {
            transform.items = Vec::new()
        }

        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &File,
        context: Option<&Option<InstSpecList>>,
    ) -> Result<(), proc_macro2::TokenStream> {

        let Some(context) = context else {
            return Ok(())
        };

        let pushed_attr;
        match context {
            Some(idents) => {
                if idents
                    .idents
                    .pairs()
                    .next()
                    .is_some_and(|pair| matches!(pair.punct(), Some(InstSpecPunct::SemiColon(_))))
                {
                    pushed_attr = true;
                    
                } else {
                    pushed_attr = false;
                };
            },
            None => {
                pushed_attr = true;
            },
        }

        if pushed_attr {
            if transform.items.len() > 1 {
                return Err(ImplBug::DocFileTooManyItemsToTransform {}.into());
            }
            let Some(Item::Impl(impl_)) = transform
                .items
                .iter()
                .find(|item| matches!(item, Item::Impl(_)))
            else {
                return Err(ImplBug::DocFileContainsNoImpl {}.into());
            };

            if !impl_.attrs.iter().any(|attr| {
                match &attr.meta {
                    Meta::List(list) => {
                        if !list.path.is_ident("cfg") {
                            return false
                        }

                        list.tokens.to_string() == quote!(not(feature = "inst")).to_string()
                    }
                    _ => false,
                }
            }) {
                return Err(ImplBug::DocImplExpectedToContainDocAttr {}.into())
            }
        } else {
            if !transform.items.is_empty() {
                return Err(ImplBug::DocFileExpectedToBeEmpty {}.into())
            }
        }

        Ok(())
    }
}

// ===============================================================================
// `````````````````````````` INST IMPL GENERICS HEADER ``````````````````````````
// ===============================================================================

/// Transforms the generic header of an instance trait implementation.
///
/// For every instance specification, this phase inserts the corresponding
/// instance counter into the implemented trait's generic arguments. It also
/// generates instance identifier constants and the `InstanceCounter`
/// associated type used by the delegated implementation transformation.
///
/// Before transformation:
///
/// ```ignore
/// #[inst(Module[0], Function[1])]
/// impl Hasher<'a> for MyHasher {
///     // ...
/// }
/// ```
///
/// After transformation:
///
/// ```ignore
/// #[instance_impl_explicit_target(1, 2)]
/// impl<'a> Hasher<'a, 0, 1> for MyHasher {
///     #[counter(1)]
///     const __INST_0_IDENT: &'static [u8] = b"Module";
///
///     #[counter(2)]
///     const __INST_1_IDENT: &'static [u8] = b"Function";
///
///     type InstanceCounter = u8;
/// }
/// ```
///
/// The instance counters are inserted after all lifetime arguments and their
/// generic argument positions are recorded in the
/// `instance_impl_explicit_target` attribute.
///
/// For a terminated instance specification, the position of the final
/// instance counter is additionally recorded by
/// `last_instance_explicit_target`. This identifies the final terminal
/// implementation used to produce the documentation-oriented 
/// `cfg(not(feature = "inst"))` representation.
///
/// The generated instance identifier constants preserve the declared
/// instance names as byte strings and are annotated with their corresponding
/// counter indexes.
///
/// The `instance_impl_explicit_target` and
/// `last_instance_explicit_target` attributes record the generated metadata,
/// while [`SupportCrate::delegate_attribute`] appends the delegate macro for
/// the final transformation.
#[derive(Clone, Debug)]
struct ImplInstGenericsHeader;

impl Transformation<ItemImpl, InstSpecList> for ImplInstGenericsHeader {
    fn raw_transform(
        &self,
        transform: &mut ItemImpl,
        context: &InstSpecList,
    ) -> Result<(), proc_macro2::TokenStream> {
        let Some((_, path, _)) = &mut transform.trait_ else {
            return Err(ImplError::TraitImplExpected {
                self_ty: *transform.self_ty.clone(),
            }
            .into());
        };

        let Some(seg) = path.segments.last_mut() else {
            return Err(ImplError::TraitImplExpected {
                self_ty: *transform.self_ty.clone(),
            }
            .into());
        };

        let angle = match &mut seg.arguments {
            syn::PathArguments::None => {
                seg.arguments =
                    syn::PathArguments::AngleBracketed(AngleBracketedGenericArguments {
                        colon2_token: Default::default(),
                        lt_token: Default::default(),
                        args: Punctuated::new(),
                        gt_token: Default::default(),
                    });
                let PathArguments::AngleBracketed(angle) = &mut seg.arguments else {
                    unreachable!()
                };
                angle
            }
            syn::PathArguments::AngleBracketed(angle) => angle,
            syn::PathArguments::Parenthesized(_) => {
                return Err(ImplError::NoParenTraitImpl { seg: seg.clone() }.into());
            }
        };

        let mut seen_non_lifetime = false;
        for arg in &mut angle.args {
            if let GenericArgument::Lifetime(lt) = arg {
                if seen_non_lifetime {
                    return Err(
                        ProcParseErr::LifetimeShouldBeDeclaredFront { lt: lt.clone() }.into(),
                    );
                }
            } else {
                seen_non_lifetime = true;
            }
        }

        let index = angle
            .args
            .iter()
            .position(|arg| !matches!(arg, GenericArgument::Lifetime(_)))
            .unwrap_or(angle.args.len());

        let mut last_inst_idx = None;

        let mut indexes = Vec::<LitInt>::new();
        let mut collect_ident = Vec::new();
        let mut collect_bstr = Vec::new();

        for (offset, pair) in context.idents.pairs().enumerate() {
            let (bound, punct) = match pair {
                Pair::Punctuated(bound, punct) => (bound, Some(punct)),
                Pair::End(bound) => (bound, None),
            };

            let offsetted = index + offset;

            let id_str = offset.to_string();
            let id = id_str.as_bytes();
            let ident = gen_const_ident_with_suffix::<TraitInst>(Some(id));
            collect_ident.push(ident);

            let counter = &bound.index;
            angle
                .args
                .insert(offsetted, GenericArgument::Const(parse_quote!(#counter)));

            let index = LitInt::new(&offsetted.to_string(), Span::call_site());
            indexes.push(parse_quote!(#index));

            let bstr_str = bound.ident.to_string();
            let bstr_bytes = bstr_str.as_bytes();
            collect_bstr.push(Lit::ByteStr(LitByteStr::new(
                bstr_bytes,
                bound.ident.span(),
            )));

            if matches!(punct, Some(InstSpecPunct::SemiColon(_))) {
                if last_inst_idx.is_none() {
                    last_inst_idx = Some(offsetted)
                }
            }
        }

        let attrs = &mut transform.attrs;

        for ((ident, bstr), index) in collect_ident.iter().zip(collect_bstr).zip(&indexes) {
            transform.items.push(syn::ImplItem::Const(ImplItemConst {
                attrs: vec![parse_quote!(#[counter(#index)])],
                vis: Inherited,
                defaultness: Default::default(),
                const_token: Default::default(),
                ident: format_ident!("{}_IDENT", ident),
                generics: Default::default(),
                colon_token: Default::default(),
                ty: parse_quote!(&'static [u8]),
                eq_token: Default::default(),
                expr: Expr::Lit(ExprLit {
                    attrs: Default::default(),
                    lit: bstr,
                }),
                semi_token: Default::default(),
            }));
        }

        transform.items.push(syn::ImplItem::Type(ImplItemType {
            attrs: Default::default(),
            vis: Inherited,
            defaultness: Default::default(),
            type_token: Default::default(),
            ident: format_ident!("InstanceCounter"),
            generics: Default::default(),
            eq_token: Default::default(),
            ty: parse_quote!(u8),
            semi_token: Default::default(),
        }));

        let support_crate = Inst::support_crate();

        if let Some(last) = last_inst_idx {
            let last = LitInt::new(&last.to_string(), Span::call_site());
            attrs.insert(
                0,
                parse_quote!(#[#support_crate::last_instance_explicit_target(#last)]),
            );
        }

        attrs.insert(
            0,
            parse_quote!(#[#support_crate::instance_impl_explicit_target(#(#indexes),*)]),
        );
        Inst::delegate_attribute(attrs);

        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &ItemImpl,
        context: Option<&InstSpecList>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let support_crate = Inst::support_crate();
        let Some(delegate_macro) = Inst::delegate_macro() else {
            return Err(ImplBug::DelegateMacroNotInitialized {}.into());
        };

        let Some((idx, inst_attr)) = transform.attrs.iter().enumerate().find_map(|(i, attr)| {
            if *attr.path() == parse_quote!(#support_crate::instance_impl_explicit_target) {
                return Some((i, attr));
            }
            return None;
        }) else {
            return Err(ImplBug::InstanceImplMacroNotFound {}.into());
        };

        if !transform.attrs.iter().enumerate().any(|(i, attr)| {
            if i <= idx {
                return false;
            }
            *attr.path() == parse_quote!(#support_crate::#delegate_macro)
        }) {
            return Err(ImplBug::DelegateMacroNotFound {}.into());
        }

        let tokens = match &inst_attr.meta {
            syn::Meta::List(meta_list) => &meta_list.tokens,
            _ => return Err(ImplBug::InstanceImplMacroArgsNotFound {}.into()),
        };

        let Ok(indexes) = IntList::checked_extract(tokens, &()) else {
            return Err(ImplBug::InstanceImplMacroIndexesParseFailed {}.into());
        };

        let Some((_, path, _)) = &transform.trait_ else {
            return Err(ImplBug::TraitImplExpected {}.into());
        };

        let Some(seg) = path.segments.last() else {
            return Err(ImplBug::TraitImplExpected {}.into());
        };

        let args = &seg.arguments;
        let PathArguments::AngleBracketed(angle) = args else {
            return Err(ImplBug::TraitImplAngleArgsExpected {}.into());
        };
        let mut seen_non_lifetime = false;
        for arg in angle.args.iter() {
            if matches!(arg, GenericArgument::Lifetime(_)) {
                if seen_non_lifetime {
                    return Err(ParseBug::LifetimeShouldBeDeclaredFront {}.into());
                }
            } else {
                seen_non_lifetime = true;
            }
        }
        let index = angle
            .args
            .iter()
            .position(|arg| !matches!(arg, GenericArgument::Lifetime(_)))
            .unwrap_or(angle.args.len());

        let Some(context) = context else {
            return Ok(());
        };

        struct Counter {
            index: usize,
            counter: LitInt,
            bstr: LitByteStr,
            bstr_const: Ident,
        }

        let mut last_inst_idx = None;
        let mut bounds = Vec::new();
        for (offset, pair) in context.idents.pairs().enumerate() {
            let (bound, punct) = match pair {
                Pair::Punctuated(bound, punct) => (bound, Some(punct)),
                Pair::End(bound) => (bound, None),
            };

            let index = index + offset;

            if matches!(punct, Some(InstSpecPunct::SemiColon(_))) {
                if last_inst_idx.is_none() {
                    last_inst_idx = Some(index)
                }
            }

            let id_str = offset.to_string();
            let id = id_str.as_bytes();
            let bstr_const = gen_const_ident_with_suffix::<TraitInst>(Some(id));

            let counter = bound.index.clone();

            let bstr_str = &bound.ident.to_string();
            let bstr_bytes = bstr_str.as_bytes();
            let bstr = LitByteStr::new(bstr_bytes, bound.ident.span());

            bounds.push(Counter {
                index,
                counter,
                bstr,
                bstr_const,
            });
        }

        if let Some(last_inst) = last_inst_idx {
            let Some((last_inst_attr_idx, last_inst_attr)) =
                transform.attrs.iter().enumerate().find_map(|(i, attr)| {
                    if *attr.path() == parse_quote!(#support_crate::last_instance_explicit_target) {
                        return Some((i, attr));
                    }
                    None
                })
            else {
                return Err(ImplBug::InstanceLastImplMacroNotFound {}.into());
            };

            let tokens = match &last_inst_attr.meta {
                syn::Meta::List(meta_list) => &meta_list.tokens,
                _ => return Err(ImplBug::InstanceLastImplMacroArgNotFound {}.into()),
            };

            let Ok(last_idx) = parse2::<LitInt>(tokens.clone()) else {
                return Err(ImplBug::InstanceLastImplMacroIndexParseFailed {}.into());
            };

            if !transform.attrs.iter().enumerate().any(|(i, attr)| {
                if i <= last_inst_attr_idx {
                    return false;
                }
                if *attr.path() == parse_quote!(#support_crate::#delegate_macro) {
                    return true;
                }
                false
            }) {
                return Err(ImplBug::InstanceLastImplDelegateMacroNotFound {}.into());
            }

            if parse_pos_usize(&last_idx)? != last_inst {
                return Err(ImplBug::InstanceLastImplMacroIndexInconsistent {}.into());
            }
        }

        if indexes.ints.len() != bounds.len() {
            return Err(ImplBug::InstanceImplIndexesArgsLenInconsistent {}.into());
        }

        for (index, bound) in indexes.ints.iter().zip(bounds) {
            let idx = bound.index;
            if parse_pos_usize(index)? != idx {
                return Err(ImplBug::InstanceImplIndexesArgsWrong {}.into());
            }

            let Some(arg) = &angle.args.get(idx) else {
                return Err(ImplBug::InstanceImplCounterGenericArgMissing {}.into());
            };

            let GenericArgument::Const(c) = arg else {
                return Err(ImplBug::InstanceImplCounterGenericArgNotConst {}.into());
            };

            let Expr::Lit(lit) = c else {
                return Err(ImplBug::InstanceImplCounterGenericArgNotLit {}.into());
            };

            let Lit::Int(int) = &lit.lit else {
                return Err(ImplBug::InstanceImplCounterGenericArgNotLitInt {}.into());
            };

            if int != &bound.counter {
                return Err(ImplBug::InstanceImplCounterGenericArgInvalid {}.into());
            };

            let Some(c) = transform.items.iter().find_map(|item| {
                let ImplItem::Const(c) = item else {
                    return None;
                };

                if c.ident != format_ident!("{}_IDENT", bound.bstr_const) {
                    return None;
                }

                Some(c)
            }) else {
                return Err(ImplBug::InstanceImplCounterIdentConstItemMissing {}.into());
            };

            let Expr::Lit(lit) = &c.expr else {
                return Err(ImplBug::InstanceImplCounterIdentConstNotLit {}.into());
            };

            let Lit::ByteStr(b_str) = &lit.lit else {
                return Err(ImplBug::InstanceImplCounterIdentConstNotLitBStr {}.into());
            };

            if b_str != &bound.bstr {
                return Err(ImplBug::InstanceImplCounterIdentConstInvalid {}.into());
            };

            if c.ty != parse_quote!(&'static [u8]) {
                return Err(ImplBug::InstanceImplCounterIdentConstTypeInvalid {}.into());
            }

            let Some(counter_attr) = c.attrs.iter().find(|attr| attr.path().is_ident("counter"))
            else {
                return Err(ImplBug::InstanceImplCounterIdentConstCounterAttrMissing {}.into());
            };

            let Meta::List(meta_list) = &counter_attr.meta else {
                return Err(ImplBug::InstanceImplCounterIdentConstCounterAttrInvalid {}.into());
            };

            let Ok(counter_idx) = parse2::<LitInt>(meta_list.tokens.clone()) else {
                return Err(
                    ImplBug::InstanceImplCounterIdentConstCounterAttrArgNotParsed {}.into(),
                );
            };

            if parse_pos_usize(&counter_idx)? != idx {
                return Err(ImplBug::InstanceImplCounterIdentConstCounterAttrArgInvalid {}.into());
            }
        }

        let Some(inst_counter) = transform.items.iter().find_map(|item| {
            let ImplItem::Type(ty) = item else {
                return None;
            };

            if ty.ident != format_ident!("InstanceCounter") {
                return None;
            }

            return Some(ty);
        }) else {
            return Err(ImplBug::InstanceImplInstanceCounterTypeMissing {}.into());
        };

        if inst_counter.ty != parse_quote!(u8) {
            return Err(ImplBug::InstanceImplInstanceCounterTypeInvalid {}.into());
        }
        Ok(())
    }
}

// ===============================================================================
// ``````````````````````````` INST IMPL ASSOCIATE TYPES `````````````````````````
// ===============================================================================

/// Transforms instance-associated types declared on an instance trait
/// implementation.
///
/// An associated type may declare either `#[node(...)]` or `#[tuple]`, but
/// never both. These annotations are only valid on associated types; applying
/// either annotation to an associated constant or function is rejected.
///
/// The transformation first determines whether the implementation contains an
/// instance node or tuple declaration. When no such declaration is present,
/// the transformation requires an instance trait context; otherwise the
/// implementation must contain at least one instance declaration.
///
/// After validating the instance declaration form, the transformation
/// delegates node-associated types to [`ItemTypeNodeInst`] and tuple-associated
/// types to [`ItemTypeTupleInst`]. Each specialized transformation is
/// responsible for lowering its corresponding representation and generating
/// the required instance metadata.
///
/// The same delegated transformations are validated after transformation to
/// ensure that the resulting implementation conforms to the expected node and
/// tuple representations.
#[derive(Debug, Clone)]
struct ItemTypeInst;

type AddonSpace<'a> = (&'a mut File, &'a mut ItemImpl);

impl<'a> Transformation<AddonSpace<'a>, Option<InstSpecList>> for ItemTypeInst {
    fn raw_transform(&self, transform: &mut AddonSpace<'a>, context: &Option<InstSpecList>) -> Result<(), proc_macro2::TokenStream> {
        let impl_ = &mut transform.1;

        let node = Inst::node();
        let tuple = Inst::tuple();

        let mut found = false;

        for item in &impl_.items {
            match item {
                ImplItem::Const(c) => {
                    if let Some(attr) = c.attrs.iter().find(|attr| attr.path().is_ident(&node))
                    {
                        return Err(
                            ImplError::NodeOnlyInTypeAssoc { attr: attr.clone() }.into()
                        );
                    }

                    if let Some(attr) = c.attrs.iter().find(|attr| attr.path().is_ident(&tuple))
                    {
                        return Err(
                            ImplError::TupleOnlyInTypeAssoc { attr: attr.clone() }.into()
                        );
                    }
                }
                ImplItem::Fn(f) => {
                    if let Some(attr) = f.attrs.iter().find(|attr| attr.path().is_ident(&node))
                    {
                        return Err(
                            ImplError::NodeOnlyInTypeAssoc { attr: attr.clone() }.into()
                        );
                    }
                    if let Some(attr) = f.attrs.iter().find(|attr| attr.path().is_ident(&tuple))
                    {
                        return Err(
                            ImplError::TupleOnlyInTypeAssoc { attr: attr.clone() }.into()
                        );
                    }
                }
                ImplItem::Type(t) => {
                    let node = t.attrs.iter().find(|attr| attr.path().is_ident(&node));
                    let tuple = t.attrs.iter().find(|attr| attr.path().is_ident(&tuple));

                    if node.is_some() && tuple.is_some() {
                        return Err(ImplError::EitherTupleOrNode { attr: tuple.unwrap().clone() } .into())
                    }

                    if node.is_some() || tuple.is_some() {
                        found = true;
                    }
                }
                _ => {}
            }
        }
        if !found && context.is_none() {
            return Err(ImplError::InstEitherTraitOrNodeOrTuple {}.into());
        }

        ItemTypeNodeInst::checked_transform(&ItemTypeNodeInst, &mut transform.1, &())?;
        ItemTypeTupleInst::checked_transform(&ItemTypeTupleInst, transform, &())?;
        Ok(())
    }

    fn validate_transform(&self, transform: &AddonSpace<'a>, _: Option<&Option<InstSpecList>>) -> Result<(), proc_macro2::TokenStream> {
        ItemTypeNodeInst::validate_transform(&ItemTypeNodeInst, &transform.1, None)?;
        ItemTypeTupleInst::validate_transform(&ItemTypeTupleInst, transform,None)?;
        Ok(())
    }
}

// ===============================================================================
// ````````````````````````````` INST ASSOC TYPE NODE ````````````````````````````
// ===============================================================================

/// Transforms instance-node associated types declared on an instance trait
/// implementation.
///
/// Each `#[node(...)]` attribute is parsed as an [`InstanceModel`] and lowered
/// into [`NodeArgs`]. The resulting node arguments are attached as either an
/// internal `#[instance_sub]` annotation for qualified trait-associated types
/// or an `#[instance_pub(...)]` annotation for public instance nodes.
///
/// Subscriber nodes use qualified associated types and are marked with
/// `#[instance_sub]`. Publisher nodes use unqualified associated types and
/// receive their generated [`NodeArgs`] through `#[instance_pub(...)]`.
///
/// `#[node(...)]` is only valid on associated types. The associated type must
/// resolve to a path type, and subscriber nodes must use a qualified trait
/// projection.
///
/// Before transformation:
///
/// ```ignore
/// impl Hasher for MyHasher {
///     #[node(Crypto, _)]
///     type Hash = Crypto::Hasher;
/// }
/// ```
///
/// After transformation:
///
/// ```ignore
/// #[instance_node_explicit_target]
/// impl Hasher for MyHasher {
///     #[instance_pub(...)]
///     type Hash = Crypto::Hasher;
/// }
/// ```
///
/// For a qualified trait projection, the node is instead marked as an
/// instance subscriber:
///
/// ```ignore
/// #[instance_sub]
/// type Hash = <Crypto as Hasher>::Hash;
/// ```
///
/// The `instance_node_explicit_target` attribute identifies the node
/// transformation phase, while [`SupportCrate::delegate_attribute`] appends
/// the delegate macro for the final transformation.
#[derive(Debug, Clone)]
struct ItemTypeNodeInst;

impl Transformation<ItemImpl> for ItemTypeNodeInst {
    fn raw_transform(
        &self,
        transform: &mut ItemImpl,
        _: &(),
    ) -> Result<(), proc_macro2::TokenStream> {
        let mut ty_items = Vec::new();
        let node = Inst::node();
        for item in &mut transform.items {
            match item {
                ImplItem::Const(c) => {
                    if let Some(attr) = c.attrs.iter().find(|attr| attr.path().is_ident(&node)) {
                        return Err(ImplError::NodeOnlyInTypeAssoc { attr: attr.clone() }.into());
                    }
                }
                ImplItem::Fn(f) => {
                    if let Some(attr) = f.attrs.iter().find(|attr| attr.path().is_ident(&node)) {
                        return Err(ImplError::NodeOnlyInTypeAssoc { attr: attr.clone() }.into());
                    }
                }
                ImplItem::Type(t) => {
                    if t.attrs.iter().any(|attr| attr.path().is_ident(&node)) {
                        ty_items.push(t);
                    }
                }
                _ => {}
            }
        }

        if ty_items.is_empty() {
            return Ok(())
        }

        let mut set = HashSet::new();
        for param in transform.generics.params.iter() {
            let GenericParam::Type(ty) = param else {
                continue;
            };
            set.insert(ty.ident.to_string());
        }

        for ty in ty_items {
            let mut node_attr = None;
            let mut build = Vec::new();
            for attr in &ty.attrs {
                if !attr.path().is_ident(&Inst::node()) {
                    build.push(attr.clone());
                }
                if node_attr.is_some() {
                    return Err(ImplError::DoubleNode { attr: attr.clone() }.into());
                };
                node_attr = Some(attr.clone());
            }
            ty.attrs = build;
            let Some(node_attr) = node_attr else {
                return Err(ImplBug::InconsistencyFindingNodeAttribute {}.into());
            };

            let tokens = match &node_attr.meta {
                syn::Meta::List(meta_list) => &meta_list.tokens,
                syn::Meta::Path(_) => {
                    return Err(ImplError::NodeRequiresArgs {
                        attr: node_attr.clone(),
                    }
                    .into());
                }
                syn::Meta::NameValue(meta) => {
                    return Err(ImplError::NodeRequiresArgsNotValue { meta: meta.clone() }.into());
                }
            };
            if tokens.is_empty() {
                return Err(ImplError::NodeRequiresArgs {
                    attr: node_attr.clone(),
                }
                .into());
            }

            let attrs = &mut ty.attrs;

            let Type::Path(path) = &ty.ty else {
                return Err(ImplError::ExpectedNodeTypeImpl { ty: ty.ty.clone() }.into());
            };

            if let Some(qself) = &path.qself {

                if qself.as_token.is_some() {

                    let sub_target = Inst::subscriber();
                    attrs.insert(0, parse_quote!(#[#sub_target]));

                    continue;
                } else {
                    return Err(ImplError::QualifiedButNotTrait {
                        qself: *qself.ty.clone(),
                    }
                    .into());
                }
            }


            let inst = InstanceModel::checked_extract(tokens, &())?;

            // Indexes are placeholder, for impl nodes in instances-macro their length is only taken
            let mut indexes = IntList::default();
            for i in 0..InstModelLength::checked_extract(&inst, &())?.0 {
                indexes
                    .ints
                    .push(LitInt::new(&i.to_string(), Span::call_site()));
            }

            let node_args = NodeArgs::checked_extract(&inst, &indexes)?;

            if path.path.segments.len() == 2 {
                let first = path.path.segments.first().unwrap();
                if first.arguments.is_none() {
                    if set.contains(&first.ident.to_string()) {
                        return Err(ImplError::GenericFoundExpectsTraitQualifier {
                            seg: first.clone(),
                        }
                        .into());
                    }
                }
            }

            let pub_target = Inst::publisher();
            attrs.insert(0, parse_quote!(#[#pub_target(#node_args)]));
        }

        let attrs = &mut transform.attrs;
        let support_crate = Inst::support_crate();
        let node_target = Inst::node_target();
        attrs.insert(
            0,
            parse_quote!(#[#support_crate::#node_target]),
        );
        Inst::delegate_attribute(attrs);

        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &ItemImpl,
        _: Option<&()>,
    ) -> Result<(), proc_macro2::TokenStream> {
        enum ItemOf<'a> {
            Sub((&'a ImplItemType, &'a Attribute)),
            Pub((&'a ImplItemType, &'a Attribute)),
        }
        let items = transform.items.iter().filter_map(|item| {
            let ImplItem::Type(ty) = item else {
                return None;
            };
            if let Some(attr) = ty
                .attrs
                .iter()
                .find(|attr| attr.meta.path().is_ident(&Inst::subscriber()))
            {
                return Some(ItemOf::Sub((ty, attr)));
            }
            if let Some(attr) = ty
                .attrs
                .iter()
                .find(|attr| attr.meta.path().is_ident(&Inst::publisher()))
            {
                return Some(ItemOf::Pub((ty, attr)));
            }
            return None;
        });

        if items.clone().next().is_none() {
            return Ok(())
        }

        let support_crate = Inst::support_crate();
        let Some(delegate_macro) = Inst::delegate_macro() else {
            return Err(ImplBug::DelegateMacroNotInitialized {}.into());
        };

        let Some((idx, inst_attr)) = transform.attrs.iter().enumerate().find_map(|(i, attr)| {
            let node_target = Inst::node_target();
            if *attr.path() == parse_quote!(#support_crate::#node_target) {
                return Some((i, attr));
            }
            None
        }) else {
            return Err(ImplBug::InstanceImplNodeMacroNotFound {}.into());
        };

        if !transform.attrs.iter().enumerate().any(|(i, attr)| {
            if i <= idx {
                return false;
            }
            *attr.path() == parse_quote!(#support_crate::#delegate_macro)
        }) {
            return Err(ImplBug::DelegateMacroNotFound {}.into());
        }

        let syn::Meta::Path(_) = &inst_attr.meta else {
            return Err(ImplBug::InstanceImplNodeMacroArgsFound {}.into());
        };

        for item in items {
            match item {
                ItemOf::Sub((sub, attr)) => {
                    let Type::Path(path) = &sub.ty else {
                        return Err(ImplBug::ExpectedNodeTypeImpl {}.into());
                    };
                    let Some(qself) = &path.qself else {
                        return Err(ImplBug::ExpectedNodeTypeImplSub {}.into());
                    };

                    if qself.as_token.is_none() {
                        return Err(ImplBug::ExpectedNodeTypeImplSub {}.into());
                    }

                    let Meta::Path(_) = &attr.meta else {
                        return Err(ImplBug::ExpectedNodeTypeImplSubAttrPath {}.into());
                    };
                }
                ItemOf::Pub((publ, attr)) => {
                    let Type::Path(path) = &publ.ty else {
                        return Err(ImplBug::ExpectedNodeTypeImpl {}.into());
                    };

                    if path.qself.is_some() {
                        return Err(ImplBug::ExpectedNodeTypeImplPub {}.into());
                    }

                    let Meta::List(_) = &attr.meta else {
                        return Err(ImplBug::ExpectedNodeTypeImplPubAttrList {}.into());
                    };
                }
            }
        }
        Ok(())
    }
}

// ===============================================================================
// ```````````````````````````` INST ASSOC TYPE TUPLE ````````````````````````````
// ===============================================================================

/// Transforms tuple-associated types declared on an instance trait
/// implementation into instance tuple representations.
///
/// Each `#[tuple]` attribute identifies an associated type whose tuple members
/// represent the available instance variants. The tuple members are collected
/// and lowered into a generated enum, with one variant for each tuple member.
/// Each generated enum variant is associated with its corresponding instance
/// index through an `InstTuple` implementation.
///
/// `#[tuple]` is only valid on associated types. The associated type must
/// contain a non-empty tuple of unqualified path types. Qualified paths,
/// generic arguments, and non-path tuple elements are not supported.
///
/// Before transformation:
///
/// ```ignore
/// impl Config for Test {
///     #[tuple]
///     type Take = (Sha256, Sha512, Sha768);
/// }
/// ```
///
/// After transformation, the associated type refers to a generated enum:
///
/// ```ignore
/// #[instance_node_explicit_target]
/// impl Config for Test {
///     #[instance_pub(...)]
///     type Take = __ItemTypeTupleInstSha256Sha512Sha768;
/// }
/// ```
///
/// The generated enum contains one tuple variant for each instance type:
///
/// ```ignore
/// pub enum __ItemTypeTupleInstSha256Sha512Sha768 {
///     Sha256(Sha256),
///     Sha512(Sha512),
///     Sha768(Sha768),
/// }
/// #[inst(Sha256[0])] 
/// impl InstTuple for Sha256 { type Variant = Sha256 }
/// 
/// #[inst(Sha512[1])] 
/// impl InstTuple for Sha512 { type Variant = Sha512 }
/// 
/// #[inst(Sha768[2];)] // terminal index has semicolon 
/// impl InstTuple for Sha768 { type Variant = Sha768 }
/// ```
///
/// An `InstTuple` implementation is generated for each variant, associating
/// the variant with its instance index. The generated enum and its
/// implementations are gated behind the `inst` feature.
///
/// The transformation also marks the resulting associated type as an instance
/// publisher and registers the containing implementation with the instance
/// node transformation phase.
#[derive(Debug, Clone)]
struct ItemTypeTupleInst;

impl<'a> Transformation<AddonSpace<'a>> for ItemTypeTupleInst {
    fn raw_transform(
        &self,
        transform: &mut AddonSpace<'a>,
        _: &(),
    ) -> Result<(), proc_macro2::TokenStream> {
        let (file, transform) = transform;

        let mut ty_items = Vec::new();
        let tuple = Inst::tuple();
        for item in &mut transform.items {
            match item {
                ImplItem::Const(c) => {
                    if let Some(attr) = c.attrs.iter().find(|attr| attr.path().is_ident(&tuple)) {
                        return Err(ImplError::TupleOnlyInTypeAssoc { attr: attr.clone() }.into());
                    }
                }
                ImplItem::Fn(f) => {
                    if let Some(attr) = f.attrs.iter().find(|attr| attr.path().is_ident(&tuple)) {
                        return Err(ImplError::TupleOnlyInTypeAssoc { attr: attr.clone() }.into());
                    }
                }
                ImplItem::Type(t) => {
                    if t.attrs.iter().any(|attr| attr.path().is_ident(&tuple)) {
                        ty_items.push(t);
                    }
                }
                _ => {}
            }
        }

        if ty_items.is_empty() {
            return Ok(())
        }

        let support_crate = Inst::support_crate();

        for ty in ty_items {

            if let Some(where_clause) = &ty.generics.where_clause {
                return Err(ImplError::WhereClauseNotSupportInTuple { where_: where_clause.clone() }.into());
            }

            let mut tuple_attr = None;
            let mut build = Vec::new();
            for attr in &ty.attrs {
                if !attr.path().is_ident(&tuple) {
                    build.push(attr.clone());
                    continue;
                }
                if tuple_attr.is_some() {
                    return Err(ImplError::DoubleTuple { attr: attr.clone() }.into());
                };
                tuple_attr = Some(attr.clone());
            }
            ty.attrs = build;
            let Some(tuple_attr) = tuple_attr else {
                return Err(ImplBug::InconsistencyFindingTupleAttribute {}.into());
            };

            match &tuple_attr.meta {
                syn::Meta::Path(_) => {}
                _ => {
                    return Err(ImplError::TupleArgsAreInfered { attr: tuple_attr.clone() } .into());
                }
            };

            // Indexes are placeholder, for impl nodes in instances-macro their length is only taken
            let mut indexes = IntList::default();
            indexes.ints.push(parse_quote!(0));

            let attrs = &mut ty.attrs;

            if let Type::Path(path) = &ty.ty {
                if let Some(qself) = &path.qself {
                    if qself.as_token.is_none() {
                        return Err(ImplError::TupleSubAssocQualifiedButNotTrait {
                            qself: *qself.ty.clone(),
                        }
                        .into());
                    }
                } else {
                    return Err(ImplError::ExpectedTupleTypeImpl { ty: ty.ty.clone() }.into())
                }

                let sub_target = Inst::subscriber();
                attrs.insert(0, parse_quote!(#[#sub_target]));

                continue;
            }

            let Type::Tuple(tuple_ty ) = &ty.ty else {
                return Err(ImplError::ExpectedTupleTypeImpl { ty: ty.ty.clone() }.into())
            };

            if tuple_ty.elems.is_empty() {
                return Err(ImplError::ExpectedTupleTypeImpl { ty: ty.ty.clone() }.into())
            }

            let mut collect_tuples = IdentList::default();
            for ty in &tuple_ty.elems {
                let Type::Path(path) = ty else {
                    return Err(ImplError::TupleArgMustBeStruct {ty: ty.clone()}.into());
                };

                if let Some(qself) = &path.qself {
                    return Err(ImplError::TupleArgMustNotBeQualified {qself: *qself.ty.clone()}.into());
                }

                let len = path.path.segments.len();
                if len > 1 {
                    return Err(ImplError::TupleArgPathQualifyElsewhere {seg: path.path.segments[len -2].clone()}.into());
                }

                let seg = path.path.segments.last().unwrap();

                if !matches!(&seg.arguments, PathArguments::None) {
                    return Err(ImplError::TupleArgGenericArgsNotSupported {path_args: seg.arguments.clone()}.into());
                }

                collect_tuples.idents.push(seg.ident.clone())
            }

            collect_tuples.duplicate_check(Some(ImplSpace::IdentDuplicateFound{}.into()))?;

            let mut tuples_iter = collect_tuples.idents.iter();
            let mut span = tuples_iter.next().unwrap().span();
            for ident in tuples_iter {
                span = span.join(ident.span()).unwrap_or(span);
            };

            let mut collect: Punctuated<InstanceIdents, Comma> = Punctuated::new();
            for ident in &collect_tuples.idents {
                let mut inner: Punctuated<InstanceIdent, Comma> = Punctuated::new();
                inner.push(InstanceIdent::Compile(ident.clone()));
                let idents = InstanceIdents { span: ident.span(), paren_token: Default::default(), params: inner };
                collect.push(idents);
            }
            let inst = InstanceModel::Complex(ComplexInstanceRange::Terminated(TerminatedInstances { span, bracket_token: Default::default(), params: collect }));
            let args = NodeArgs::checked_extract(&inst.into(), &indexes)?;

            let bytes = collect_tuples.idents
                .iter()
                .flat_map(|ident| ident.to_string().into_bytes())
                .collect::<Vec<u8>>();
            let bytes = &bytes;

            let enum_ident = gen_type_ident_with_suffix::<Self>(Some(bytes));

            let mut enum_: ItemEnum = parse_quote!(pub enum #enum_ident {});
            enum_.attrs.push(Inst::feature());
            enum_.attrs.push(parse_quote!(#[allow(non_camel_case_types)]));

            let tuple_len = collect_tuples.idents.len();

            for (i, ident) in collect_tuples.idents.iter().enumerate() {
                enum_.variants.push(parse_quote!(#ident(#ident)));

                let mut impl_: ItemImpl = parse_quote!(
                    impl #support_crate::InstTuple for #enum_ident {
                        type Variant = #ident;
                    }
                );
                let index = i.to_string();
                let index_lit = LitInt::new(&index, ident.span());
                if i < tuple_len - 1 {
                    impl_.attrs.push(parse_quote!(#[#support_crate::inst(#ident[#index_lit])]));
                } else {
                    impl_.attrs.push(parse_quote!(#[#support_crate::inst(#ident[#index_lit];)]));
                }
                impl_.attrs.push(parse_quote!(#[allow(non_camel_case_types)]));
                impl_.attrs.insert(0, Inst::feature());

                file.items.push(Item::Impl(impl_));
            }
            file.items.push(Item::Enum(enum_));

            let pub_target = Inst::publisher();
            ty.attrs.insert(0, parse_quote!(#[#pub_target(#args)]));

            ty.ty = parse_quote!(#enum_ident);
        }

        let attrs = &mut transform.attrs;
        let support_crate = Inst::support_crate();
        let node_target = Inst::node_target();

        attrs.insert(
            0,
            parse_quote!(#[#support_crate::#node_target]),
        );
        Inst::delegate_attribute(attrs);

        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &AddonSpace<'a>,
        _: Option<&()>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let (file, transform) = transform;
        ItemTypeNodeInst::validate_transform(&ItemTypeNodeInst, transform, None)?;

        let items = file.items.iter().filter_map(|item| {
            let Item::Enum(en) = item else {
                return None;
            };
            Some(en)
        });

        if items.clone().next().is_none() {
            return Ok(())
        }

        let has_attrs = |attrs: &Vec<Attribute>| -> Result<(), TokenStream> {
            if !attrs.iter().any(|attr| {
                match &attr.meta {
                    Meta::List(list) => {
                        if !list.path.is_ident("cfg") {
                            return false
                        }

                        list.tokens.to_string() == quote!(feature = "inst").to_string()
                    }
                    _ => false,
                }
            }) {
                return Err(ImplBug::ImplTupleAddonNotInstFeatureGated {}.into())
            }

            if !attrs.iter().any(|attr| {
                match &attr.meta {
                    Meta::List(list) => {
                        if !list.path.is_ident("allow") {
                            return false
                        }

                        list.tokens.to_string() == quote!(non_camel_case_types).to_string()
                    }
                    _ => false,
                }
            }) {
                return Err(ImplBug::ImplTupleAddonNonCamelCaseAttrMissing {}.into())
            }

            Ok(())
        };

        for en in items {
            has_attrs(&en.attrs)?;

            let enum_ident = &en.ident;
            let variants_len = en.variants.len();
            let variants = &en.variants;

            let impl_items = file.items.iter().filter_map(|item| {
                let Item::Impl(impl_) = item else {
                    return None
                };

                if *impl_.self_ty == parse_quote!(#enum_ident) {
                    return Some(impl_)
                };

                return None
            });

            if impl_items.clone().count() != variants_len {
                return Err(ImplBug::TupleEnumAddonInstImplsInconsistent {}.into())
            }

            let support_crate = Inst::support_crate();

            let mut collected_variants = Vec::new();
            for impl_ in impl_items {

                if impl_.items.len() > 1 {
                    return Err(ImplBug::TupleEnumAddonInstImplItemsExtraFound {}.into())
                }

                has_attrs(&impl_.attrs)?;

                let Some(tokens) = impl_.attrs.iter().find_map(|attr| {
                    match &attr.meta {
                        Meta::List(list) => {
                            if list.path != parse_quote!(#support_crate::inst){
                                return None
                            }
                            Some(&list.tokens)
                        }
                        _ => None,
                    }
                }) else {
                    return Err(ImplBug::ImplTupleAddonInstImplNotInstAttributed {}.into())
                };

                let args = InstSpecList::checked_extract(tokens, &())?;

                if args.idents.len() != 1 {
                    return Err(ImplBug::ImplTupleAddonInstImplInstArgInconsistent {}.into())
                }

                let pair = args.idents.pairs().next().unwrap();
                let arg_ident = pair.value().ident.clone();
                let arg_punc = pair.punct().cloned().cloned();

                let arg_index = parse_pos_usize(&args.idents[0].index)?;

                let Some((_, trait_path, _)) = &impl_.trait_ else {
                    return Err(ImplBug::ImplTupleAddonInstImplNotTraitImpl {}.into())
                };

                if *trait_path != parse_quote!(#support_crate::InstTuple) {
                    return Err(ImplBug::ImplTupleAddonInstTraitImplInvalidTrait {}.into())
                }

                let ImplItem::Type(ty) = &impl_.items[0] else {
                    return Err(ImplBug::TupleEnumAddonInstImplItemNotType {}.into())
                };

                if ty.ty != parse_quote!(#arg_ident) {
                    return Err(ImplBug::TupleEnumAddonInstImplItemTypeInvalid {}.into())
                }

                if ty.ident != format_ident!("Variant") {
                    return Err(ImplBug::TupleEnumAddonInstImplItemIdentInvalid {}.into())
                }

                collected_variants.push((arg_ident, arg_index, arg_punc));
            }

            collected_variants.sort_by_key(|(_, index, _)| *index);


            for ((i, variant), (ident, index, punct)) in variants.iter().enumerate().zip(collected_variants) {

                if i != index {
                    return Err(ImplBug::TupleEnumAddonVariantIndexInvalid {}.into())
                }

                if i == variants_len - 1 {
                    let Some(InstSpecPunct::SemiColon(_)) = punct else {
                        return Err(ImplBug::TupleEnumAddonVariantLastInstPunctNotTerminated {}.into())
                    };
                } else {
                    if let Some(InstSpecPunct::SemiColon(_)) = punct {
                        return Err(ImplBug::TupleEnumAddonVariantInstPunctInvalid {}.into())
                    }
                }

                let variant_ident = &variant.ident;

                if *variant_ident != ident {
                    return Err(ImplBug::TupleEnumAddonVariantIdentInvalid {}.into())
                }

                let Fields::Unnamed(unamed) = &variant.fields else {
                    return Err(ImplBug::TupleEnumAddonVariantFieldsNotUnamed {}.into())
                };

                if unamed.unnamed.len() != 1 {
                    return Err(ImplBug::TupleEnumAddonVariantFieldInconsistent {}.into())
                }

                let field_ty = &unamed.unnamed[0].ty;

                if *field_ty != parse_quote!(#ident) {
                    return Err(ImplBug::TupleEnumAddonVariantTypeInvalid {}.into())
                }
            }

            let Some(impl_ty) = transform.items.iter().find_map(|item| {
                let ImplItem::Type(ty) = item else {
                    return None
                };

                if ty.ty == parse_quote!(#enum_ident) {
                    return Some(ty)
                }

                None
            }) else {
                    return Err(ImplBug::TupleEnumInstNodeTypeNotFound {}.into())
            };

            let Some(_) = impl_ty.attrs.iter().find_map(|attr| {
                match &attr.meta {
                    Meta::List(list) => {
                        if !list.path.is_ident(&Inst::publisher()) {
                            return None
                        }
                        Some(&list.tokens)
                    }
                    _ => None,
                }
            }) else {
                return Err(ImplBug::TupleEnumInstNodeTypePubNodeAttrsNotFound {}.into())
            };

        }

        Ok(())
    }
}

