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
// ``````````````````````````` INSTANCE ACCESS WRAPPERS ``````````````````````````
// ===============================================================================

//! Defines wrappers that convert supported implementation items [`ImplItem`]
//! into standalone Rust items [`Item`] and apply the configuration attributes
//! required by the generated documentation and non-documentation representations.

// ===============================================================================
// ``````````````````````````````````` IMPORTS ```````````````````````````````````
// ===============================================================================

// --- Proc Macro Crates ---
use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{
    Attribute, ImplItem, ImplItemConst, ImplItemFn, ImplItemType, Item, ItemConst, ItemFn,
    ItemType, parse_quote,
};

// ===============================================================================
// ``````````````````````````````````` STRUCTS ```````````````````````````````````
// ===============================================================================

/// Wraps an associated function as a standalone [`ItemFn`].
pub(super) struct WrapperFn(pub ItemFn);

/// Wraps an associated constant as a standalone [`ItemConst`].
pub(super) struct WrapperConst(pub ItemConst);

/// Wraps an associated type as a standalone [`ItemType`].
pub(super) struct WrapperType(pub ItemType);

/// Represents an implementation item converted into a standalone item wrapper.
pub(super) enum Wrapper {
    Fn(WrapperFn),
    Const(WrapperConst),
    Type(WrapperType),
}

impl From<ImplItemFn> for WrapperFn {
    fn from(item: ImplItemFn) -> Self {
        Self(ItemFn {
            attrs: item.attrs,
            vis: item.vis,
            sig: item.sig,
            block: Box::new(item.block),
        })
    }
}

impl From<ImplItemConst> for WrapperConst {
    fn from(item: ImplItemConst) -> Self {
        Self(ItemConst {
            attrs: item.attrs,
            vis: item.vis,
            const_token: item.const_token,
            ident: item.ident,
            generics: item.generics,
            colon_token: item.colon_token,
            ty: Box::new(item.ty),
            eq_token: item.eq_token,
            expr: Box::new(item.expr),
            semi_token: item.semi_token,
        })
    }
}

impl From<ImplItemType> for WrapperType {
    fn from(item: ImplItemType) -> Self {
        Self(ItemType {
            attrs: item.attrs,
            vis: item.vis,
            type_token: item.type_token,
            ident: item.ident,
            generics: item.generics,
            eq_token: item.eq_token,
            ty: Box::new(item.ty),
            semi_token: item.semi_token,
        })
    }
}

impl ToTokens for WrapperFn {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.0.to_tokens(tokens);
    }
}

impl ToTokens for WrapperConst {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.0.to_tokens(tokens);
    }
}

impl ToTokens for WrapperType {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.0.to_tokens(tokens);
    }
}

impl From<ImplItemFn> for Wrapper {
    fn from(item: ImplItemFn) -> Self {
        Self::Fn(item.into())
    }
}

impl From<ImplItemConst> for Wrapper {
    fn from(item: ImplItemConst) -> Self {
        Self::Const(item.into())
    }
}

impl From<ImplItemType> for Wrapper {
    fn from(item: ImplItemType) -> Self {
        Self::Type(item.into())
    }
}

impl ToTokens for Wrapper {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Fn(item) => {
                item.to_tokens(tokens);
            }

            Self::Const(item) => {
                item.to_tokens(tokens);
            }

            Self::Type(item) => {
                item.to_tokens(tokens);
            }
        }
    }
}

/// Converts supported implementation items into their corresponding wrappers.
///
/// Only functions, constants, and associated types are retained; all other
/// implementation items are discarded.
fn wrap_impl_items(items: Vec<ImplItem>) -> Vec<Wrapper> {
    items
        .into_iter()
        .filter_map(|item| match item {
            ImplItem::Fn(item) => Some(Wrapper::from(item)),

            ImplItem::Const(item) => Some(Wrapper::from(item)),

            ImplItem::Type(item) => Some(Wrapper::from(item)),

            _ => None,
        })
        .collect()
}

/// Converts implementation items into standalone [`Item`] representations.
///
/// The supported implementation items are first wrapped to normalize their
/// representation, then unwrapped into their corresponding standalone item
/// variants.
pub(crate) fn impls_to_items(impls: Vec<ImplItem>) -> Vec<Item> {
    wrap_impl_items(impls)
        .into_iter()
        .map(|wrapper| match wrapper {
            Wrapper::Fn(item) => Item::Fn(item.0),
            Wrapper::Const(item) => Item::Const(item.0),
            Wrapper::Type(item) => Item::Type(item.0),
        })
        .collect()
}

/// Appends the appropriate documentation configuration attribute to each item.
///
/// When `doc` is `true`, items are enabled only during documentation builds
/// through `#[cfg(doc)]`; otherwise, they are enabled only outside documentation
/// builds through `#[cfg(not(doc))]`.
pub(super) fn append_cfg(items: &mut Vec<Item>, doc: bool) {
    for item in items {
        let attr: Attribute = if doc {
            parse_quote!(#[cfg(doc)])
        } else {
            parse_quote!(#[cfg(not(doc))])
        };

        match item {
            Item::Fn(item) => item.attrs.push(attr),
            Item::Const(item) => item.attrs.push(attr),
            Item::Type(item) => item.attrs.push(attr),
            _ => {}
        }
    }
}
