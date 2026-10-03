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
// ``````````````````````````````` INSTANCES MACROS ``````````````````````````````
// ===============================================================================

//! Module Comment (TBD)

// ===============================================================================
// ``````````````````````````````````` MODULES ```````````````````````````````````
// ===============================================================================

mod args;
mod errors;
mod expr;
mod impls;
mod mods;
mod node;
mod traits;

// ===============================================================================
// ``````````````````````````````````` IMPORTS ```````````````````````````````````
// ===============================================================================

// --- Proc-Suite ---
use proc_suite::{SupportCrate, proc_pipeline};

// --- Proc-Macros Essentials ---
use proc_macro::TokenStream;
use quote::{quote};
use syn::{File, Item};

// --- Local Crate ---
use crate::{
    args::{InstDirectArgs, InstGetArgs, InstItem},
    expr::{InstDirectExpr, InstGetExpr},
    impls::ImplInst,
    mods::ModInst,
    traits::TraitInst,
};

// ===============================================================================
// ````````````````````````````` PIPELINE DECLARATION ````````````````````````````
// ===============================================================================

proc_pipeline! {
    /// Primary Struct
    pub(crate) struct Inst {
        extract: Extraction,
        insert: Insertion,
        extend: Extension,
        transform: Transformation,
        utilize: Utilization,
        support: "inst",
        delegate: ("instances", "delegate_support_crate_internal_only"),
    }
}

// ===============================================================================
// `````````````````````````````` PROCEDURAL-MACROS ``````````````````````````````
// ===============================================================================

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ``````````````````````````````````` INSTANCE ``````````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

#[allow(unused)]
pub(crate) const INST_MACRO_NAME: &'static str = "inst";

/// Proc-Macro Comment (TBD)
#[proc_macro_attribute]
pub fn inst(args: TokenStream, tokens: TokenStream) -> TokenStream {
    let item = match InstItem::checked_extract(&tokens.into(), &()) {
        Ok(item) => item,
        Err(err) => return err.into(),
    };

    let mut file = File {
        shebang: None,
        items: Vec::new(),
        attrs: Vec::new(),
    };

    match item {
        InstItem::Trait(item_trait) => {
            file.items.push(Item::Trait(item_trait));
            if let Err(err) = TraitInst::checked_transform(&TraitInst, &mut file, &args.into()) {
                return err.into();
            };
        }
        InstItem::Impl(item_impl) => {
            file.items.push(Item::Impl(item_impl));
            if let Err(err) = ImplInst::checked_transform(&ImplInst, &mut file, &args.into()) {
                return err.into();
            };
        }
        InstItem::Mod(item_mod) => {
            file.items.push(Item::Mod(item_mod));
            if let Err(err) = ModInst::checked_transform(&ModInst, &mut file, &args.into()) {
                return err.into();
            };
        }
    }

    quote! { #file }.into()
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ````````````````````````````````` INSTANCE GET ````````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

#[allow(unused)]
pub(crate) const INST_GET_MACRO_NAME: &'static str = "inst_get";

/// Proc-Macro Comment (TBD)
#[proc_macro]
pub fn inst_get(tokens: TokenStream) -> TokenStream {
    let item = match InstGetArgs::checked_extract(&tokens.into(), &()) {
        Ok(item) => item,
        Err(err) => return err.into(),
    };

    let mut tokens = proc_macro2::TokenStream::new();

    if let Err(err) = InstGetExpr::checked_transform(&InstGetExpr, &mut tokens, &item) {
        return err.into();
    }

    tokens.into()
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ``````````````````````````````` INSTANCE DIRECT ```````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

#[allow(unused)]
pub(crate) const INST_DIRECT_MACRO_NAME: &'static str = "inst_direct";

/// Proc-Macro Comment (TBD)
#[proc_macro]
pub fn inst_direct(tokens: TokenStream) -> TokenStream {
    let item = match InstDirectArgs::checked_extract(&tokens.into(), &()) {
        Ok(item) => item,
        Err(err) => return err.into(),
    };

    let mut tokens = proc_macro2::TokenStream::new();

    if let Err(err) = InstDirectExpr::checked_transform(&InstDirectExpr, &mut tokens, &item) {
        return err.into();
    }

    tokens.into()
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ```````````````````````````````` DELEGATE MACRO ```````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

#[allow(unused)]
pub(crate) const DELEGATE_MACRO_NAME: &'static str = "delegate_support_crate_internal_only";

/// Proc-Macro Comment (TBD)
#[proc_macro_attribute]
pub fn delegate_support_crate_internal_only(_: TokenStream, tokens: TokenStream) -> TokenStream {
    let mut item = match syn::parse::<Item>(tokens) {
        Ok(item) => item,
        Err(err) => return err.into_compile_error().into(),
    };

    Inst::delegate_inner(&mut item);

    quote! { #item }.into()
}
