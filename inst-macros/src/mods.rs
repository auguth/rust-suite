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
// ```````````````````````````````````` MOD INST `````````````````````````````````
// ===============================================================================

//! Provides the transformation pipeline for instance access modules.
//!
//! The module transforms an instance access module containing inherent
//! implementations for instance-node associated types. Each implementation
//! is identified by a `#[node(<inst-args>)]` annotation that specifies the
//! concrete instance arguments for the implemented instance node.
//!
//! The transformation resolves these instance arguments against the instance
//! parameters declared by the access module and generates the corresponding
//! instance access arguments used by the delegated access transformation.
//!
//! Each instance access implementation is represented in two forms: an
//! internal `cfg(feature = "inst")` implementation for the actual 
//! instance access transformation and a `cfg(not(feature = "inst"))` 
//! implementation for rustdoc. The documentation representation includes 
//! generated documentation describing the instance accessor and its arguments.
//!
//! Module-level documentation is generated separately by [`ModInstDoc`], while
//! [`InstArguments`] renders the concrete instance arguments used by the
//! documentation representation.
//!
//! The module also validates the transformed access implementations to ensure
//! that internal and documentation targets are present in matching numbers,
//! delegated transformation metadata is retained, and the generated
//! implementations contain the required `cfg` attributes and documentation.

// ===============================================================================
// ``````````````````````````````````` IMPORTS ```````````````````````````````````
// ===============================================================================

// --- Proc Macro Crates ---
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{Attribute, File, ImplItem, Item, ItemImpl, ItemMod, WherePredicate, parse_quote};

// --- Local Crate ---
use crate::{
    Extraction, Inst, Transformation, args::{InstAccess, InstanceModel}, errors::{ModBug, ModError, ParseBug}, node::AccessArgsRef, traits::{append_placeholder_args, cfg_feature_disclaimer},
};

// --- Proc Suite ---
use proc_suite::{DocAttr, InsertDocs, SupportCrate};

// ===============================================================================
// ````````````````````````````` MOD INST ENTRY-POINT ````````````````````````````
// ===============================================================================

/// Transforms an instance access module through the [`crate::inst`] macro
/// pipeline.
///
/// The module contains inherent implementations annotated with
/// `#[node(<inst-args>)]`. Each node annotation identifies the concrete
/// instance arguments for the instance-node associated type implemented by
/// that inherent implementation.
///
/// For each node implementation, this transformation:
///
/// 1. extracts and validates the instance arguments from `#[node(...)]`;
/// 2. resolves those arguments against the instance parameters declared by
///    the access module;
/// 3. generates the corresponding instance access arguments;
/// 4. attaches the explicit instance-access target attribute used by the
///    delegated access transformation;
/// 5. creates a `cfg(feature = "inst")` implementation for the actual 
///    transformation;
/// 6. creates a `cfg(not(feature = "inst"))` implementation for exposing 
///    the documentation representation of the same instance access.
/// 7. appends documentations for `cfg(not(feature = "inst"))` target for 
///    each impl items for clarity.
///
/// The original `#[node(...)]` annotation is consumed during transformation
/// and is not retained on either generated implementation.
#[derive(Debug, Clone)]
pub struct ModInst;

impl Transformation<File, TokenStream> for ModInst {
    fn raw_transform(
        &self,
        transform: &mut File,
        context: &TokenStream,
    ) -> Result<(), proc_macro2::TokenStream> {
        if transform.items.len() > 1 {
            return Err(ParseBug::TooManyItemsToTransform {}.into());
        }
        let Some(Item::Mod(mod_)) = transform
            .items
            .iter_mut()
            .find(|item| matches!(item, Item::Mod(_)))
        else {
            return Err(ModBug::FileContainsNoMod {}.into());
        };

        let depth = InstAccess::checked_extract(context, &())?;

        ModInst::checked_transform(&ModInst, mod_, &depth)?;
        ModInstDoc::checked_transform(&ModInstDoc, mod_, &depth)?;

        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &File,
        _: Option<&TokenStream>,
    ) -> Result<(), proc_macro2::TokenStream> {
        if transform.items.len() > 1 {
            return Err(ModBug::TransformedFileInconsistent {}.into());
        }
        Ok(())
    }
}

// ===============================================================================
// ````````````````````````````` MOD INST DOC-TARGET `````````````````````````````
// ===============================================================================

/// Transforms an instance access module into its documentation
/// representation.
///
/// This transformation adds module-level documentation describing the
/// instance access module, its instance-node inherent implementations, and the
/// `inst_get!()` interface used to access them.
///
/// The documentation is retained on the transformed module and validated to
/// ensure that the generated module documentation is present.
#[derive(Debug, Clone)]
pub struct ModInstDoc;

impl Transformation<ItemMod, InstAccess> for ModInstDoc {
    fn raw_transform(
        &self,
        transform: &mut ItemMod,
        context: &InstAccess,
    ) -> Result<(), proc_macro2::TokenStream> {
        transform.attrs.prepend_docs(vec![
            DocAttr::Heading {
                level: 0,
                content: vec![
                    DocAttr::Raw("Instance Accessor Module ".to_string()),
                    DocAttr::Ref(context.path.to_token_stream().to_string()),
                    DocAttr::Raw(format!("(`{}`)", context.items.to_token_stream())),
                ],
            },
            DocAttr::Raw(
                "Source contains inherent implementations for instance-node \
                 associated types, accessible through "
                    .to_string(),
            ),
            DocAttr::Link {
                title: "inst_get!()".to_string(),
                url: "inst::inst_get".to_string(),
            },
            DocAttr::Raw(
                " , since original methods of the trait cannot be accessed directly.".to_string(),
            ),
            DocAttr::LineBreak,
            DocAttr::Raw("*See* ".to_string()),
            DocAttr::Link {
                title: "`inst::mod`".to_string(),
                url: "inst::mod".to_string(),
            },
            DocAttr::Raw(" *for details.*".to_string()),
            DocAttr::LineBreak,
        ]);

        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &ItemMod,
        _: Option<&InstAccess>,
    ) -> Result<(), proc_macro2::TokenStream> {
        if !transform.clone().has_docs() {
            return Err(ModBug::ModuleDocsNotAppended {}.into());
        }
        Ok(())
    }
}

// ===============================================================================
// ``````````````````````````````` MOD INST ACTUAL ```````````````````````````````
// ===============================================================================

/// Renders the instance arguments of a simple [`InstanceModel`] as tokens for
/// documentation purposes.
///
/// Compile-time instance identifiers are emitted directly, while runtime
/// instance parameters are rendered using the `dyn` argument form expected by
/// instance accessors.
///
/// Complex instance models and inferred instance parameters are not valid for
/// instance accessor arguments.
struct InstArguments<'a>(&'a InstanceModel);

impl ToTokens for InstArguments<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let InstanceModel::Simple(simple) = &self.0 else {
            unreachable!("complex instance models are not possible for arguments");
        };

        for (i, param) in simple.params.iter().enumerate() {
            if i != 0 {
                tokens.extend(quote!(,));
            }

            match param {
                crate::args::InstanceIdent::Compile(ident) => {
                    ident.to_tokens(tokens);
                }
                crate::args::InstanceIdent::Runtime(dynamic_expr) => {
                    let expr = &dynamic_expr.ident;
                    quote!(dyn #expr).to_tokens(tokens);
                }
                crate::args::InstanceIdent::Infer(_) => {
                    unreachable!("inferred instance parameters are not possible here");
                }
            }
        }
    }
}

/// Transforms each instance-node inherent implementation contained in the
/// instance access module.
///
/// Implementations are selected by the presence of a `#[node(...)]`
/// annotation. The annotation must contain a valid [`InstanceModel`], whose
/// arguments are resolved against the instance-node declarations associated
/// with the access module.
///
/// Each selected implementation is split into two forms: an internal
/// `cfg(feature = "inst")` implementation carrying the actual instance-access 
/// target, and a `cfg(not(feature = "inst"))` implementation carrying the 
/// documentation target and generated accessor documentation for each impl items.
///
/// The transformed internal implementation replaces the original
/// implementation, while the documentation implementation is appended to
/// the module.
/// 
/// Additionally, to avoid documentation issues, every instance accessor function
/// is replaced with a { todo!() } block during documentation builds, with a
/// disclaimer to use --features inst for the actual implementation.
impl Transformation<ItemMod, InstAccess> for ModInst {
    fn raw_transform(
        &self,
        transform: &mut ItemMod,
        context: &InstAccess,
    ) -> Result<(), proc_macro2::TokenStream> {
        let Some((_, items)) = &mut transform.content else {
            return Err(ModError::RequiresInstAccessImpls {
                module: transform.clone(),
            }
            .into());
        };

        let node = Inst::node();
        let impl_indexes: Vec<usize> = items
            .iter()
            .enumerate()
            .filter_map(|(index, item)| match item {
                Item::Impl(impl_)
                    if impl_.attrs.iter().any(|attr| attr.path().is_ident(&node)) =>
                {
                    Some(index)
                }
                _ => None,
            })
            .collect();

        if impl_indexes.is_empty() {
            return Err(ModError::RequiresInstAccessImpls {
                module: transform.clone(),
            }
            .into());
        }

        for index in impl_indexes {
            let mut impl_ = match &items[index] {
                Item::Impl(impl_) => impl_.clone(),
                _ => unreachable!(),
            };

            let mut node_attr = None;
            let mut build = Vec::new();

            for attr in &impl_.attrs {
                if attr.path().is_ident(&node) {
                    if node_attr.is_some() {
                        return Err(ModError::DoubleNode { attr: attr.clone() }.into());
                    }

                    node_attr = Some(attr.clone());
                } else {
                    build.push(attr.clone());
                }
            }

            impl_.attrs = build;

            let Some(node_attr) = node_attr else {
                return Err(ModBug::InconsistencyFindingNodeAttribute {}.into());
            };

            let tokens = match &node_attr.meta {
                syn::Meta::List(meta_list) => &meta_list.tokens,

                syn::Meta::Path(_) => {
                    return Err(ModError::NodeRequiresArgs {
                        attr: node_attr.clone(),
                    }
                    .into());
                }

                syn::Meta::NameValue(meta) => {
                    return Err(ModError::NodeRequiresArgsNotValue { meta: meta.clone() }.into());
                }
            };

            if tokens.is_empty() {
                return Err(ModError::NodeRequiresArgs {
                    attr: node_attr.clone(),
                }
                .into());
            }

            let inst = InstanceModel::checked_extract(tokens, &())?;

            let support_crate = Inst::support_crate();

            let direct_model = |impl_: &mut ItemImpl, not_doc: bool| -> Result<(), TokenStream> {
                let Some(where_clause) = &mut impl_.generics.where_clause else {
                    return Err(ModError::ExpectedWhereClauseForSelf {
                        generics: impl_.generics.clone(),
                    }
                    .into());
                };

                let Some(bounds) = where_clause.predicates.iter_mut().find_map(|pred| {
                    let WherePredicate::Type(ty) = pred else {
                        return None;
                    };

                    if ty.bounded_ty != parse_quote!(Self) {
                        return None;
                    }

                    Some(&mut ty.bounds)
                }) else {
                    return Err(ModError::ExpectedSelfInWhereClause {
                        where_: where_clause.clone(),
                        exp: context.path.clone(),
                    }
                    .into());
                };

                let indexes = append_placeholder_args(&mut *bounds, &context.path, &inst, not_doc)?;

                let mut access = AccessArgsRef(inst.clone());

                let args = access.try_access(&context.items, &indexes)?;

                Inst::delegate_attribute(&mut impl_.attrs);

                if not_doc {
                    let access_target = Inst::access_target();
                    impl_.attrs.insert(
                        0,
                        parse_quote!(
                            #[#support_crate::#access_target(#args)]
                        ),
                    );
                    impl_.attrs.insert(0, Inst::feature());
                } else {
                    let access_doc_target = Inst::access_doc_target();
                    impl_.attrs.insert(
                        0,
                        parse_quote!(
                            #[#support_crate::#access_doc_target(#args)]
                        ),
                    );
                    impl_.attrs.insert(0, Inst::not_feature());
                }

                for item in &mut impl_.items {

                    if !not_doc  {
                        if let ImplItem::Fn(f) = item {
                            f.block = parse_quote!(
                                {
                                    #[cfg(not(doc))]
                                    {
                                        compile_error!(
                                            "`#[inst(...)]` macro requires the `inst` feature for build, \
                                            recompile with `--features inst`"
                                        );
                                        todo!("expected build to be compiled with `--features inst`")
                                    }

                                    #[cfg(doc)] 
                                    {
                                        todo!("expected build to be compiled for documentation only");
                                    }
                                }
                            );
                        }
                    }

                    let attrs = match item {
                        syn::ImplItem::Const(c) => &mut c.attrs,
                        syn::ImplItem::Fn(f) => &mut f.attrs,
                        syn::ImplItem::Type(t) => &mut t.attrs,
                        _ => continue,
                    };

                    if not_doc {
                        cfg_feature_disclaimer(attrs);
                        attrs.insert(0,Inst::feature());
                    } else {
                        attrs.prepend_docs(vec![
                            DocAttr::Heading {
                                level: 0,
                                content: vec![
                                    DocAttr::Raw(
                                        "Instance Accessor Item ".to_string(),
                                    ),
                                    DocAttr::Ref(
                                        context
                                            .path
                                            .to_token_stream()
                                            .to_string(),
                                    ),
                                    DocAttr::Raw(format!(
                                        " `({})`",
                                        InstArguments(&access.0)
                                            .to_token_stream()
                                            .to_string()
                                    )),
                                ],
                            },
                            DocAttr::Raw(
                                "Instance accessor item with well-defined arguments. Dynamic arguments if exists (`dyn Param`) must be supplied through "
                                    .to_string(),
                            ),
                            DocAttr::Link {
                                title: "inst_get!()".to_string(),
                                url: "inst::inst_get".to_string(),
                            },
                            DocAttr::Raw(
                                " macro.".to_string(),
                            ),
                            DocAttr::LineBreak,
                            DocAttr::Raw(
                                "*See* ".to_string(),
                            ),
                            DocAttr::Link {
                                title: "`inst::mod`".to_string(),
                                url: "inst::mod".to_string(),
                            },
                            DocAttr::Raw(
                                " *for details.*".to_string(),
                            ),
                            DocAttr::LineBreak,
                        ]);
                        attrs.insert(0,Inst::not_feature());

                    }
                }

                impl_.attrs.retain(|attr| !attr.path().is_ident(&node));

                Ok(())
            };

            let mut doc_impl = impl_.clone();
            direct_model(&mut impl_, true)?;
            direct_model(&mut doc_impl, false)?;
            items[index] = Item::Impl(impl_);
            items.push(Item::Impl(doc_impl));
        }

        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &ItemMod,
        _: Option<&InstAccess>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let Some((_, items)) = &transform.content else {
            return Err(ModBug::RequiresInstAccessImpls {}.into());
        };

        let support_crate = Inst::support_crate();
        let access_target = Inst::access_target();
        let access_doc_target = Inst::access_doc_target();
        let Some(delegate_target) = Inst::delegate_macro() else {
            return Err(ModBug::DelegateMacroNotInitialized {}.into())
        };

        let mut impl_items = Vec::new();
        let mut impl_doc_items = Vec::new();
        for item in items {
            match item {
                Item::Impl(impl_) => {
                    if impl_.attrs.iter().any(|attr| {
                        *attr.path()
                            == parse_quote!(#support_crate::#access_target)
                    }) {
                        impl_items.push(impl_);
                    }
                    if impl_.attrs.iter().any(|attr| {
                        *attr.path()
                            == parse_quote!(#support_crate::#access_doc_target)
                    }) {
                        impl_doc_items.push(impl_);
                    }
                }
                _ => {}
            }
        }

        if impl_items.is_empty() || impl_doc_items.is_empty() {
            return Err(ModBug::RequiresInstAccessImpls {}.into());
        }

        if impl_items.len() != impl_doc_items.len() {
            return Err(ModBug::InconsistentTranformedImplTargetsLength {}.into());
        }

        let find = |doc: bool, attrs: &Vec<Attribute>| -> bool {
            attrs.iter().any(|attr| {
                if *attr == Inst::feature() {
                    if doc {false} else {true}
                } else if *attr == Inst::not_feature() {
                    if doc {true} else {false}

                } else {
                    false
                }
            }) 
        };

        for impl_ in impl_items {
            let Some((idx, _)) = impl_.attrs.iter().enumerate().find_map(|(i, attr)| {
                if *attr.path() == parse_quote!(#support_crate::#access_target) {
                    return Some((i, attr));
                }
                None
            }) else {
                unreachable!()
            };

            if !impl_.attrs.iter().enumerate().any(|(i, attr)| {
                if i <= idx {
                    return false;
                };
                *attr.path() == parse_quote!(#support_crate::#delegate_target)
            }) {
                return Err(ModBug::DelegateMacroNotFound {}.into());
            }

            for item in &impl_.items {
                let attrs = match item {
                    syn::ImplItem::Const(c) => &c.attrs,
                    syn::ImplItem::Fn(f) => &f.attrs,
                    syn::ImplItem::Type(t) => &t.attrs,
                    _ => {
                        continue;
                    }
                };

                if !find(false, attrs) {
                    return Err(ModBug::ItemNotDocTargetAttrNotFound {}.into());
                }

                if !attrs.clone().has_docs() {
                    return Err(ModBug::ItemNotDocTargetHasNoDocs {}.into());
                }
            }
        }

        for impl_ in impl_doc_items {
            let Some((idx, _)) = impl_.attrs.iter().enumerate().find_map(|(i, attr)| {
                if *attr.path() == parse_quote!(#support_crate::#access_doc_target)
                {
                    return Some((i, attr));
                }
                None
            }) else {
                unreachable!()
            };

            if !impl_.attrs.iter().enumerate().any(|(i, attr)| {
                if i <= idx {
                    return false;
                };
                *attr.path() == parse_quote!(#support_crate::#delegate_target)
            }) {
                return Err(ModBug::DelegateMacroNotFound {}.into());
            }

            for item in &impl_.items {
                let attrs = match item {
                    syn::ImplItem::Const(c) => &c.attrs,
                    syn::ImplItem::Fn(f) => &f.attrs,
                    syn::ImplItem::Type(t) => &t.attrs,
                    _ => {
                        continue;
                    }
                };
                if !find(true, attrs) {
                    return Err(ModBug::ItemDocTargetAttrNotFound {}.into());
                }

                if !attrs.clone().has_docs() {
                    return Err(ModBug::ItemDocTargetHasNoDocs {}.into());
                }
            }
        }
        Ok(())
    }
}
