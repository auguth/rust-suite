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
// `````````````````````````````` POST INSTANCE NODE `````````````````````````````
// ===============================================================================

//! Post-transformation cleanup for instance node attributes.
//!
//! Removes `#[instance_sub(...)]` and `#[instance_pub(...)]` marker attributes
//! after they have been consumed to identify subscriber and publisher instance
//! nodes.

// ===============================================================================
// ``````````````````````````````````` IMPORTS ```````````````````````````````````
// ===============================================================================

// --- Local Crate ---
use crate::{
    Transformation,
    node::{args::*, errors::PostBugs},
};

// --- Proc Macro Crates ---
use syn::{ImplItem, ItemImpl, ItemTrait, Meta, TraitItem};

// ===============================================================================
// ```````````````````` INSTANCE NODE POST-TRANFORMATION SPACE ```````````````````
// ===============================================================================

pub(crate) enum InstanceNodePostSpace<'a> {
    Trait(&'a mut ItemTrait),
    Impl(&'a mut ItemImpl),
}

impl<'a> From<&'a mut ItemTrait> for InstanceNodePostSpace<'a> {
    fn from(value: &'a mut ItemTrait) -> Self {
        InstanceNodePostSpace::Trait(value)
    }
}

impl<'a> From<&'a mut ItemImpl> for InstanceNodePostSpace<'a> {
    fn from(value: &'a mut ItemImpl) -> Self {
        InstanceNodePostSpace::Impl(value)
    }
}

// ===============================================================================
// ````````````````````````` SUBSCRIBER NODE ATTR REMOVAL ````````````````````````
// ===============================================================================

/// Removes all `#[instance_sub(...)]` attributes after subscriber node
/// processing.
///
/// Instance node attributes are not procedural macros. They exist solely to
/// identify associated types that participate as subscriber instance nodes
/// during transformation. Once processed, the attributes have no further
/// purpose and are removed from the syntax tree before code generation.
#[derive(Debug, Clone)]
pub struct SubscriberNodeAttrRemoval;

impl<'a> Transformation<InstanceNodePostSpace<'a>> for SubscriberNodeAttrRemoval {
    fn raw_transform(
        &self,
        transform: &mut InstanceNodePostSpace<'a>,
        _: &(),
    ) -> Result<(), proc_macro2::TokenStream> {
        match transform {
            InstanceNodePostSpace::Trait(trait_of) => {
                for item in &mut trait_of.items {
                    let TraitItem::Type(c) = item else {
                        continue;
                    };

                    c.attrs.retain(|attr| {
                        let path = match &attr.meta {
                            Meta::Path(path) => path,
                            Meta::List(meta_list) => &meta_list.path,
                            Meta::NameValue(meta_name_value) => &meta_name_value.path,
                        };

                        !path.is_ident(SubscriberNode::IDENT)
                    });
                }
            }
            InstanceNodePostSpace::Impl(impl_of) => {
                for item in &mut impl_of.items {
                    let ImplItem::Type(c) = item else {
                        continue;
                    };

                    c.attrs.retain(|attr| {
                        let path = match &attr.meta {
                            Meta::Path(path) => path,
                            Meta::List(meta_list) => &meta_list.path,
                            Meta::NameValue(meta_name_value) => &meta_name_value.path,
                        };

                        !path.is_ident(SubscriberNode::IDENT)
                    });
                }
            }
        }

        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &InstanceNodePostSpace<'a>,
        _: Option<&()>,
    ) -> Result<(), proc_macro2::TokenStream> {
        match transform {
            InstanceNodePostSpace::Trait(trait_of) => {
                for item in &trait_of.items {
                    let TraitItem::Type(c) = item else {
                        continue;
                    };

                    if c.attrs.iter().any(|attr| {
                        let path = match &attr.meta {
                            Meta::Path(path) => path,
                            Meta::List(meta_list) => &meta_list.path,
                            Meta::NameValue(meta_name_value) => &meta_name_value.path,
                        };

                        path.is_ident(SubscriberNode::IDENT)
                    }) {
                        return Err(PostBugs::SubNodeExists {}.into());
                    }
                }
            }
            InstanceNodePostSpace::Impl(impl_of) => {
                for item in &impl_of.items {
                    let ImplItem::Type(c) = item else {
                        continue;
                    };

                    if c.attrs.iter().any(|attr| {
                        let path = match &attr.meta {
                            Meta::Path(path) => path,
                            Meta::List(meta_list) => &meta_list.path,
                            Meta::NameValue(meta_name_value) => &meta_name_value.path,
                        };

                        path.is_ident(SubscriberNode::IDENT)
                    }) {
                        return Err(PostBugs::SubNodeExists {}.into());
                    }
                }
            }
        }

        Ok(())
    }
}

// ===============================================================================
// ````````````````````````` PUBLISHER NODE ATTR REMOVAL `````````````````````````
// ===============================================================================

/// Removes all `#[instance_pub(...)]` attributes after publisher node
/// processing.
///
/// Instance node attributes are not procedural macros. They exist solely to
/// identify associated types that participate as publisher instance nodes
/// during transformation. Once processed, the attributes have no further
/// purpose and are removed from the syntax tree before code generation.
#[derive(Debug, Clone)]
pub struct PublisherNodeAttrRemoval;

impl<'a> Transformation<InstanceNodePostSpace<'a>> for PublisherNodeAttrRemoval {
    fn raw_transform(
        &self,
        transform: &mut InstanceNodePostSpace<'a>,
        _: &(),
    ) -> Result<(), proc_macro2::TokenStream> {
        match transform {
            InstanceNodePostSpace::Trait(trait_of) => {
                for item in &mut trait_of.items {
                    let TraitItem::Type(c) = item else {
                        continue;
                    };

                    c.attrs.retain(|attr| {
                        let path = match &attr.meta {
                            Meta::Path(path) => path,
                            Meta::List(meta_list) => &meta_list.path,
                            Meta::NameValue(meta_name_value) => &meta_name_value.path,
                        };

                        !path.is_ident(PublisherNode::IDENT)
                    });
                }
            }
            InstanceNodePostSpace::Impl(impl_of) => {
                for item in &mut impl_of.items {
                    let ImplItem::Type(c) = item else {
                        continue;
                    };

                    c.attrs.retain(|attr| {
                        let path = match &attr.meta {
                            Meta::Path(path) => path,
                            Meta::List(meta_list) => &meta_list.path,
                            Meta::NameValue(meta_name_value) => &meta_name_value.path,
                        };

                        !path.is_ident(PublisherNode::IDENT)
                    });
                }
            }
        }

        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &InstanceNodePostSpace<'a>,
        _: Option<&()>,
    ) -> Result<(), proc_macro2::TokenStream> {
        match transform {
            InstanceNodePostSpace::Trait(trait_of) => {
                for item in &trait_of.items {
                    let TraitItem::Type(c) = item else {
                        continue;
                    };

                    if c.attrs.iter().any(|attr| {
                        let path = match &attr.meta {
                            Meta::Path(path) => path,
                            Meta::List(meta_list) => &meta_list.path,
                            Meta::NameValue(meta_name_value) => &meta_name_value.path,
                        };

                        path.is_ident(PublisherNode::IDENT)
                    }) {
                        return Err(PostBugs::PubNodeExists {}.into());
                    }
                }
            }
            InstanceNodePostSpace::Impl(impl_of) => {
                for item in &impl_of.items {
                    let ImplItem::Type(c) = item else {
                        continue;
                    };

                    if c.attrs.iter().any(|attr| {
                        let path = match &attr.meta {
                            Meta::Path(path) => path,
                            Meta::List(meta_list) => &meta_list.path,
                            Meta::NameValue(meta_name_value) => &meta_name_value.path,
                        };

                        path.is_ident(PublisherNode::IDENT)
                    }) {
                        return Err(PostBugs::PubNodeExists {}.into());
                    }
                }
            }
        }

        Ok(())
    }
}
