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

//! Low-Level Instance Proc Macros Crate
//! 
//! For macro usage, use crate `instances`.
//! 
//! Module Doc (TBD)

// ===============================================================================
// ``````````````````````````````````` MODULES ```````````````````````````````````
// ===============================================================================

mod access;
mod args;
mod errors;
mod impls;
mod node;
mod traits;
mod utils;

// ===============================================================================
// ``````````````````````````````````` IMPORTS ```````````````````````````````````
// ===============================================================================

// --- Proc-Suite ---
use proc_suite::{collect, create_item, doc_twin, mutate_item, proc_pipeline};

// --- Proc-Macros Essentials ---
use proc_macro::TokenStream;
use quote::quote;
use syn::{File, ItemImpl, ItemTrait, parse_quote};

// --- Local Crate ---
use crate::{
    access::{
        InstanceAccess, InstanceAccessImpl, InstanceAccessImplDoc, direct::InstanceDirectAccess,
        getter::InstanceGetter, impls_to_items,
    },
    args::{
        InstanceNodeArgs::{Impl, Trait},
        *,
    },
    errors::*,
    impls::{
        InstanceImpl, InstanceImplAddons, InstanceImplDocTarget, LastInstanceImpl,
        LastInstanceSpace, PostInstanceImpl,
    },
    node::{
        InstanceNodeImpl, InstanceNodeImplDocTarget, InstanceNodeTrait, InstanceNodeTraitDocTarget,
        PostInstanceNodeImpl, PostInstanceNodeTrait,
    },
    traits::{InstanceTrait, InstanceTraitDocTarget},
};

// ===============================================================================
// ````````````````````````````` PIPELINE DECLARATION ````````````````````````````
// ===============================================================================

proc_pipeline! {
    /// Primary Struct
    struct Instance {
        extract: Extraction,
        insert: Insertion,
        extend: Extension,
        transform: Transformation,
        utilize: Utilization,
        support: "instances",
    }
}

// ===============================================================================
// `````````````````````````````` PROCEDURAL-MACROS ``````````````````````````````
// ===============================================================================

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ```````````````````````````````` INSTANCE TRAIT ```````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

#[allow(unused)]
pub(crate) const TRAIT_MACRO_NAME: &'static str = "instance_trait";

/// Proc-Macro Comment (TBD)
#[proc_macro_attribute]
pub fn instance_trait(args: TokenStream, tokens: TokenStream) -> TokenStream {
    let parsed = match InstanceTraitArgs::checked_extract(&tokens, &args) {
        Ok(p) => p,
        Err(e) => return e.into(),
    };

    let mut trait_of = parsed.item;
    let args = parsed.args.as_ref();

    doc_twin(
        "INSTANCE",
        &mut trait_of,
        |trait_of| InstanceTrait::checked_transform(&InstanceTrait, trait_of, &args),
        |doc_trait| {
            InstanceTraitDocTarget::checked_transform(&InstanceTraitDocTarget, doc_trait, &args)
        },
        |_, _| Ok(()),
        false,
        false,
        false,
        false,
    )
    .into()
}

/// Proc-Macro Comment (TBD)
#[proc_macro_attribute]
pub fn instance_trait_explicit_target(args: TokenStream, tokens: TokenStream) -> TokenStream {
    let parsed = match InstanceTraitArgs::checked_extract(&tokens, &args) {
        Ok(p) => p,
        Err(e) => return e.into(),
    };

    let mut trait_of = parsed.item;
    let args = parsed.args.as_ref();

    if let Err(err) = InstanceTrait::checked_transform(&InstanceTrait, &mut trait_of, &args) {
        return err.into();
    };

    quote! { #trait_of }.into()
}

/// Proc-Macro Comment (TBD)
#[proc_macro_attribute]
pub fn instance_trait_explicit_doc_target(args: TokenStream, tokens: TokenStream) -> TokenStream {
    let parsed = match InstanceTraitArgs::checked_extract(&tokens, &args) {
        Ok(p) => p,
        Err(e) => return e.into(),
    };

    let mut trait_of = parsed.item;
    let args = parsed.args.as_ref();

    if let Err(err) =
        InstanceTraitDocTarget::checked_transform(&InstanceTraitDocTarget, &mut trait_of, &args)
    {
        return err.into();
    };

    quote! { #trait_of }.into()
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ```````````````````````````````` INSTANCE IMPL ````````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

pub(crate) const TRAIT_IMPL_MACRO_NAME: &'static str = "instance_impl";

/// Proc-Macro Comment (TBD)
#[proc_macro_attribute]
pub fn instance_impl(args: TokenStream, tokens: TokenStream) -> TokenStream {
    let parsed = match InstanceImplArgs::checked_extract(&tokens, &args) {
        Ok(p) => p,
        Err(e) => return e.into(),
    };

    let mut impl_of = parsed.item;
    let args = parsed.args.as_ref();

    let label = "INSTANCE_IMPL";

    let mut addons = File {
        shebang: None,
        attrs: Vec::new(),
        items: Vec::new(),
    };

    collect!(
        doc_twin(
            label,
            &mut impl_of,
            |impl_of| InstanceImpl::checked_transform(&InstanceImpl, impl_of, &args),
            |doc_impl| {
                InstanceImplDocTarget::checked_transform(&InstanceImplDocTarget, doc_impl, &())
            },
            |not_doc_impl, _| {
                InstanceImplAddons::checked_transform(
                    &InstanceImplAddons,
                    &mut addons,
                    &not_doc_impl,
                )?;
                PostInstanceImpl::checked_transform(&PostInstanceImpl, not_doc_impl, &())?;
                Ok(())
            },
            false,
            false,
            false,
            false,
        ),
        create_item(label, || { Ok(addons) }, false),
    )
    .into()
}

/// Proc-Macro Comment (TBD)
#[proc_macro_attribute]
pub fn instance_impl_explicit_target(args: TokenStream, tokens: TokenStream) -> TokenStream {
    let parsed = match InstanceImplArgs::checked_extract(&tokens, &args) {
        Ok(p) => p,
        Err(e) => return e.into(),
    };

    let mut impl_of = parsed.item;
    let args = parsed.args.as_ref();

    let mut addons = File {
        shebang: None,
        attrs: Vec::new(),
        items: Vec::new(),
    };

    if let Err(err) = InstanceImpl::checked_transform(&InstanceImpl, &mut impl_of, &args) {
        return err.into();
    };

    if let Err(err) =
        InstanceImplAddons::checked_transform(&InstanceImplAddons, &mut addons, &impl_of)
    {
        return err.into();
    }

    if let Err(err) = PostInstanceImpl::checked_transform(&PostInstanceImpl, &mut impl_of, &()) {
        return err.into();
    }

    quote! { #impl_of #addons }.into()
}

/// Proc-Macro Comment (TBD)
#[proc_macro_attribute]
pub fn instance_impl_explicit_doc_target(args: TokenStream, tokens: TokenStream) -> TokenStream {
    let parsed = match InstanceImplArgs::checked_extract(&tokens, &args) {
        Ok(p) => p,
        Err(e) => return e.into(),
    };

    let mut impl_of = parsed.item;

    if let Err(err) =
        InstanceImplDocTarget::checked_transform(&InstanceImplDocTarget, &mut impl_of, &())
    {
        return err.into();
    };

    quote! { #impl_of }.into()
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ```````````````````````````````` LAST INSTANCE ````````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

pub(crate) const LAST_INSTANCE_MACRO_NAME: &'static str = "last_instance";

/// Proc-Macro Comment (TBD)
#[proc_macro_attribute]
pub fn last_instance(args: TokenStream, tokens: TokenStream) -> TokenStream {
    let parsed = match InstanceImplArgs::checked_extract(&tokens, &args) {
        Ok(p) => p,
        Err(e) => return e.into(),
    };

    let mut impl_of = parsed.item;
    let args = parsed.args.as_ref();

    let mut addons = File {
        shebang: None,
        attrs: Vec::new(),
        items: Vec::new(),
    };

    let label = "LAST_INSTANCE";
    collect!(
        mutate_item(
            label,
            &mut impl_of,
            |impl_of| {
                let mut space = LastInstanceSpace {
                    impl_of,
                    addons: &mut addons,
                };
                LastInstanceImpl::checked_transform(&LastInstanceImpl, &mut space, &args)?;
                PostInstanceImpl::checked_transform(&PostInstanceImpl, impl_of, &())?;
                Ok(())
            },
            false,
            false,
        ),
        // Only called by a cfg not doc hence no need for addon items carrying the same flag
        create_item(label, || { Ok(addons) }, false),
    )
    .into()
}

/// Proc-Macro Comment (TBD)
#[proc_macro_attribute]
pub fn last_instance_explicit_target(args: TokenStream, tokens: TokenStream) -> TokenStream {
    let parsed = match InstanceImplArgs::checked_extract(&tokens, &args) {
        Ok(p) => p,
        Err(e) => return e.into(),
    };

    let mut impl_of = parsed.item;
    let args = parsed.args.as_ref();

    let mut addons = File {
        shebang: None,
        attrs: Vec::new(),
        items: Vec::new(),
    };
    let mut space = LastInstanceSpace {
        impl_of: &mut impl_of,
        addons: &mut addons,
    };

    if let Err(err) = LastInstanceImpl::checked_transform(&LastInstanceImpl, &mut space, &args) {
        return err.into();
    }
    if let Err(err) = PostInstanceImpl::checked_transform(&PostInstanceImpl, &mut impl_of, &()) {
        return err.into();
    }

    quote! { #impl_of #addons }.into()
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ```````````````````````````````` INSTANCE NODE ````````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

#[allow(unused)]
pub(crate) const INSTANCE_NODE_MACRO_NAME: &'static str = "instance_node";

/// Proc-Macro Comment (TBD)
#[proc_macro_attribute]
pub fn instance_node(args: TokenStream, tokens: TokenStream) -> TokenStream {
    let parsed = match InstanceNodeArgs::checked_extract(&tokens, &args) {
        Ok(p) => p,
        Err(e) => return e.into(),
    };

    let trait_node = |trait_of: &mut ItemTrait| -> TokenStream {
        let label = "INSTANCE_NODE_TRAIT";

        let mut addons = File {
            shebang: None,
            attrs: Vec::new(),
            items: Vec::new(),
        };

        collect!(
            doc_twin(
                label,
                trait_of,
                |trait_of| InstanceNodeTrait::checked_transform(
                    &InstanceNodeTrait,
                    &mut (trait_of, &mut addons),
                    &()
                ),
                |doc_trait| {
                    InstanceNodeTraitDocTarget::checked_transform(
                        &InstanceNodeTraitDocTarget,
                        doc_trait,
                        &(),
                    )
                },
                |not_doc, _| {
                    PostInstanceNodeTrait::checked_transform(&PostInstanceNodeTrait, not_doc, &())?;
                    Ok(())
                },
                false,
                false,
                false,
                false,
            ),
            create_item(
                label,
                || {
                    for addon in &mut addons.items {
                        *addon = parse_quote!(
                            #[cfg(not(doc))]
                            #addon
                        );
                    }
                    Ok(addons)
                },
                false
            ),
        )
        .into()
    };

    let impl_node = |impl_of: &mut ItemImpl| -> TokenStream {
        let label = "INSTANCE_NODE_IMPL";

        let mut addons = File {
            shebang: None,
            attrs: Vec::new(),
            items: Vec::new(),
        };

        collect!(
            doc_twin(
                label,
                impl_of,
                |impl_of| InstanceNodeImpl::checked_transform(
                    &InstanceNodeImpl,
                    &mut (impl_of, &mut addons),
                    &()
                ),
                |doc_impl| {
                    InstanceNodeImplDocTarget::checked_transform(
                        &InstanceNodeImplDocTarget,
                        doc_impl,
                        &(),
                    )
                },
                |not_doc, _| {
                    PostInstanceNodeImpl::checked_transform(&PostInstanceNodeImpl, not_doc, &())?;
                    Ok(())
                },
                false,
                false,
                false,
                false,
            ),
            create_item(
                label,
                || {
                    for addon in &mut addons.items {
                        *addon = parse_quote!(
                            #[cfg(not(doc))]
                            #addon
                        );
                    }
                    Ok(addons)
                },
                false
            ),
        )
        .into()
    };

    match parsed {
        Trait(mut item_trait) => trait_node(&mut item_trait),
        Impl(mut item_impl) => impl_node(&mut item_impl),
    }
}

/// Proc-Macro Comment (TBD)
#[proc_macro_attribute]
pub fn instance_node_explicit_doc_target(args: TokenStream, tokens: TokenStream) -> TokenStream {
    let parsed = match InstanceNodeArgs::checked_extract(&tokens, &args) {
        Ok(p) => p,
        Err(e) => return e.into(),
    };

    let trait_node = |trait_of: &mut ItemTrait| -> TokenStream {
        if let Err(err) = InstanceNodeTraitDocTarget::checked_transform(
            &InstanceNodeTraitDocTarget,
            trait_of,
            &(),
        ) {
            return err.into();
        }

        quote! { #trait_of }.into()
    };

    let impl_node = |impl_of: &mut ItemImpl| -> TokenStream {
        if let Err(err) =
            InstanceNodeImplDocTarget::checked_transform(&InstanceNodeImplDocTarget, impl_of, &())
        {
            return err.into();
        }

        quote! { #impl_of }.into()
    };

    match parsed {
        Trait(mut item_trait) => trait_node(&mut item_trait),
        Impl(mut item_impl) => impl_node(&mut item_impl),
    }
}

/// Proc-Macro Comment (TBD)
#[proc_macro_attribute]
pub fn instance_node_explicit_target(args: TokenStream, tokens: TokenStream) -> TokenStream {
    let parsed = match InstanceNodeArgs::checked_extract(&tokens, &args) {
        Ok(p) => p,
        Err(e) => return e.into(),
    };

    let trait_node = |trait_of: &mut ItemTrait| -> TokenStream {
        let mut addons = File {
            shebang: None,
            attrs: Vec::new(),
            items: Vec::new(),
        };

        if let Err(err) = InstanceNodeTrait::checked_transform(
            &InstanceNodeTrait,
            &mut (trait_of, &mut addons),
            &(),
        ) {
            return err.into();
        }

        if let Err(err) =
            PostInstanceNodeTrait::checked_transform(&PostInstanceNodeTrait, trait_of, &())
        {
            return err.into();
        }

        quote! { #trait_of #addons }.into()
    };

    let impl_node = |impl_of: &mut ItemImpl| -> TokenStream {
        let mut addons = File {
            shebang: None,
            attrs: Vec::new(),
            items: Vec::new(),
        };

        if let Err(err) =
            InstanceNodeImpl::checked_transform(&InstanceNodeImpl, &mut (impl_of, &mut addons), &())
        {
            return err.into();
        }

        if let Err(err) =
            PostInstanceNodeImpl::checked_transform(&PostInstanceNodeImpl, impl_of, &())
        {
            return err.into();
        }

        quote! { #impl_of #addons }.into()
    };

    match parsed {
        Trait(mut item_trait) => trait_node(&mut item_trait),
        Impl(mut item_impl) => impl_node(&mut item_impl),
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ``````````````````````````````` INSTANCE ACCESS ```````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

#[allow(unused)]
pub(crate) const INSTANCE_ACCESS_MACRO_NAME: &'static str = "instance_access";

/// Proc-Macro Comment (TBD)
#[proc_macro_attribute]
pub fn instance_access(args: TokenStream, tokens: TokenStream) -> TokenStream {
    let parsed = match InstanceAccessArgs::checked_extract(&tokens, &args) {
        Ok(p) => p,
        Err(e) => return e.into(),
    };

    let args = &parsed.args;
    let mut file = File {
        shebang: None,
        attrs: Default::default(),
        items: Default::default(),
    };
    file.items.push(syn::Item::Impl(parsed.item.clone()));

    if let Err(e) = InstanceAccess::checked_transform(&InstanceAccess, &mut file, args) {
        return e.into();
    };

    quote! { #file }.into()
}

/// Proc-Macro Comment (TBD)
#[proc_macro_attribute]
pub fn instance_access_explicit_target(args: TokenStream, tokens: TokenStream) -> TokenStream {
    let parsed = match InstanceAccessArgs::checked_extract(&tokens, &args) {
        Ok(p) => p,
        Err(e) => return e.into(),
    };

    let args = &parsed.args;
    let mut impl_of = parsed.item.clone();

    if let Err(e) = InstanceAccessImpl::checked_transform(&InstanceAccessImpl, &mut impl_of, args) {
        return e.into();
    };

    let items = impls_to_items(impl_of.items);

    quote! { #(#items)* }.into()
}

/// Proc-Macro Comment (TBD)
#[proc_macro_attribute]
pub fn instance_access_explicit_doc_target(args: TokenStream, tokens: TokenStream) -> TokenStream {
    let parsed = match InstanceAccessArgs::checked_extract(&tokens, &args) {
        Ok(p) => p,
        Err(e) => return e.into(),
    };

    let args = &parsed.args;
    let mut impl_of = parsed.item.clone();

    if let Err(e) =
        InstanceAccessImplDoc::checked_transform(&InstanceAccessImplDoc, &mut impl_of, args)
    {
        return e.into();
    };

    let items = impls_to_items(impl_of.items);

    quote! { #(#items)* }.into()
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ```````````````````````````` INSTANCE DIRECT ACCESS ```````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

#[allow(unused)]
pub(crate) const INSTANCE_DIRECT_ACCESS_MACRO_NAME: &'static str = "instance_direct_access";

/// Proc-Macro Comment (TBD)
#[proc_macro]
pub fn instance_direct_access(input: TokenStream) -> TokenStream {
    let parsed = match InstanceDirectAccessArgs::checked_extract(&input.into(), &()) {
        Ok(p) => p,
        Err(e) => return e.into(),
    };
    let args = &parsed.leaf;
    let mut expr = parsed.expr.clone();

    if let Err(e) = InstanceDirectAccess::checked_transform(&InstanceDirectAccess, &mut expr, args)
    {
        return e.into();
    }

    quote! { #expr }.into()
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ``````````````````````````````` INSTANCE GETTER ```````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

#[allow(unused)]
pub(crate) const INSTANCE_GETTER_MACRO_NAME: &'static str = "instance_get_access";

/// Proc-Macro Comment (TBD)
#[proc_macro]
pub fn instance_get_access(input: TokenStream) -> TokenStream {
    let parsed = match InstanceGetterArgs::checked_extract(&input.into(), &()) {
        Ok(p) => p,
        Err(e) => return e.into(),
    };
    let args = &parsed.input;
    let mut expr = parsed.expr.clone();

    if let Err(e) = InstanceGetter::checked_transform(&InstanceGetter, &mut expr, args) {
        return e.into();
    }

    quote! { #expr }.into()
}
