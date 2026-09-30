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
// ```````````````````````````````` INSTANCE NODE ````````````````````````````````
// ===============================================================================

//! Instance node transformations.
//!
//! An **instance node** is an associated type participating in the instance
//! system. Instance nodes define the relationship between an associated type
//! and an instance hierarchy by either declaring the portion of the hierarchy
//! to be consumed or by providing the concrete hierarchy that satisfies such a
//! declaration.
//!
//! Instance nodes are expressed differently depending on where they appear.
//!
//! - Within an **instance trait**, an associated type becomes a
//!   **subscriber node**, declaring the instance selection associated with that
//!   type.
//! - Within an **instance implementation**, an associated type becomes a
//!   **publisher node**, binding that associated type to a concrete instance
//!   implementation.
//!
//! ## Subscriber nodes
//!
//! Subscriber nodes express **intent** rather than implementation. Their node
//! selector determines which portion of an instance hierarchy is associated
//! with the corresponding associated type.
//!
//! Depending on the selector, a subscriber may represent:
//!
//! - an exact leaf (`leaf`);
//! - a branch (`branch`);
//! - the entire hierarchy (`root`);
//! - a root-to-leaf range (`prune`);
//! - a root-to-branch range (`trim`);
//! - a leaf-to-end range (`extend`);
//! - a branch-to-end range (`spread`);
//! - a leaf-to-branch range (`traverse`); or
//! - a branch-to-leaf range (`descend`).
//!
//! ```ignore
//! #[instance]
//! trait Logger {
//!     #[instance_sub(leaf(...))]
//!     type Events;
//!
//!     #[instance_sub(branch(...))]
//!     type Metrics;
//! }
//! ```
//!
//! Here, `Events` and `Metrics` describe the portions of an instance hierarchy
//! required by the trait without specifying how those selections are
//! implemented.
//!
//! ## Publisher nodes
//!
//! Publisher nodes satisfy subscriber declarations by binding an associated
//! type to a concrete instance implementation.
//!
//! ```ignore
//! #[instance]
//! impl Logger for AppLogger {
//!     #[instance_pub(leaf(...))]
//!     type Events = AppEvents;
//!
//!     #[instance_pub(branch(...))]
//!     type Metrics = AppMetrics;
//! }
//! ```
//!
//! The publisher determines the concrete instance corresponding to the
//! subscriber declared by the trait.
//!
//! ## Deferred publication
//!
//! Implementations are not required to publish every associated type
//! immediately. When publication must be deferred, an implementation may
//! instead declare a subscriber node, allowing the implementation to subscribe
//! to another instance whose publisher will satisfy the declaration.
//!
//! ```ignore
//! #[instance]
//! impl Logger for ProxyLogger {
//!     #[instance_sub(leaf(...))]
//!     type Events = <Defer as DeferTrait>::SimilarEvents;
//! }
//! ```
//!
//! ## Generated bindings
//!
//! Every instance node is transformed into a collection of hidden generated
//! types that encode the node selector and its arguments into the type system.
//! These generated types establish the binding between an associated type and
//! the exact region of an instance hierarchy described by the node.
//!
//! Collectively, the generated bindings provide the infrastructure required to
//! resolve and access a specific instance within the hierarchy. Rather than
//! referring to instance identifier paths directly, users interact through the
//! associated types declared by the instance trait, while the node generated types
//! carry the information necessary to locate the corresponding instance.
//!
//! Subscriber nodes generate the hidden bindings describing the requested
//! instance selection, while publisher nodes generate the bindings that
//! associate those selections with concrete instance implementations. Together,
//! these generated types enable a trait's associated types to act as strongly
//! typed handles for querying and accessing specific instances.

// ===============================================================================
// ``````````````````````````````````` MODULES ```````````````````````````````````
// ===============================================================================

pub(crate) mod access;
mod args;
mod errors;
mod post;
pub(crate) mod publisher;
pub(crate) mod state;
pub(crate) mod subscriber;

// ===============================================================================
// ``````````````````````````````````` IMPORTS ```````````````````````````````````
// ===============================================================================

// --- Std Crate ---
use std::collections::HashSet;

// --- Proc Macro Crates ---
use proc_macro2::TokenStream;
use syn::{AttrStyle, File, ImplItem, ItemImpl, ItemTrait, Meta, TraitItem, spanned::Spanned};

// --- Local Crate
use crate::{
    Extraction, Transformation,
    node::{
        args::*,
        errors::{NodeBugs, NodeError, StateError},
        post::*,
    },
};

// ===============================================================================
// ````````````````````````````` INSTANCE TRAIT NODE `````````````````````````````
// ===============================================================================

/// Instance Trait Node target consisting of the trait being processed and the
/// auxiliary file used to collect generated addon items.
pub(crate) type InstanceTraitNode<'a> = (&'a mut ItemTrait, &'a mut File);

/// Transformation pass for instance trait nodes.
///
/// Processes the associated type declarations of an instance trait to discover
/// and transform subscriber nodes.
///
/// This pass traverses every associated type declared within an instance trait
/// and inspects its outer attributes to identify node annotations. Subscriber
/// nodes are collected for transformation, while publisher node annotations are
/// rejected because publisher nodes are only valid within implementation
/// blocks.
///
/// For every discovered subscriber node, this pass:
///
/// - validates that the associated type is declared only once;
/// - ensures the subscriber node is annotated at most once;
/// - rejects any publisher node annotations;
/// - extracts the subscriber node arguments;
/// - constructs the corresponding [`SubscriberNodeSpace`]; and
/// - delegates node expansion to [`SubscriberNode`].
///
/// Any items generated during expansion are appended to the auxiliary syntax
/// tree associated with the transformation target.
///
/// ## Example
///
/// Given the following instance trait:
///
/// ```ignore
/// #[instance]
/// trait Logger {
///     #[instance_sub]
///     type Events;
///
///     type State;
/// }
/// ```
///
/// this transformation identifies `Events` as a subscriber node, validates its
/// declaration, and expands it through [`SubscriberNode`]. The associated type
/// `State` is ignored because it does not declare a node.
///
/// Attempting to declare a publisher node within a trait:
///
/// ```ignore
/// #[instance]
/// trait Logger {
///     #[instance_pub]
///     type Events;
/// }
/// ```
///
/// results in an error because publisher nodes are only permitted within
/// instance implementation blocks.
#[derive(Debug, Clone)]
pub(crate) struct InstanceNodeTrait;

impl<'a> Transformation<InstanceTraitNode<'a>> for InstanceNodeTrait {
    fn raw_transform(
        &self,
        transform: &mut InstanceTraitNode<'a>,
        _: &(),
    ) -> Result<(), proc_macro2::TokenStream> {
        let (trait_of, addons) = transform;

        let mut collect = Vec::new();
        let mut seen = HashSet::new();

        for (i, item) in trait_of.items.iter().enumerate() {
            let TraitItem::Type(t) = item else {
                continue;
            };

            let mut is_subscriber = false;

            for a in &t.attrs {
                let AttrStyle::Outer = &a.style else {
                    continue;
                };

                let path = match &a.meta {
                    Meta::Path(p) => p,
                    Meta::List(meta_list) => &meta_list.path,
                    Meta::NameValue(meta_name_value) => &meta_name_value.path,
                };

                if path.is_ident(PublisherNode::IDENT) {
                    return Err(NodeError::PubNotAllowed { path: path.clone() }.into());
                }

                if path.is_ident(SubscriberNode::IDENT) && is_subscriber {
                    return Err(NodeError::MultiSubNode { path: path.clone() }.into());
                }

                if path.is_ident(SubscriberNode::IDENT) {
                    is_subscriber = true;
                    continue;
                }
            }

            if !seen.insert(t.ident.clone()) {
                return Err(NodeError::DuplicateSubNode {
                    ident: t.ident.clone(),
                }
                .into());
            }
            if is_subscriber {
                collect.push(i);
            }
        }

        for i in collect {
            let trait_item = trait_of.items.get(i).unwrap();
            let TraitItem::Type(assoc_ty) = trait_item else {
                return Err(NodeBugs::NodeNotFound {}.into());
            };

            let args = NodeArgs::checked_extract(assoc_ty, &())?;

            let mut space = SubscriberNodeSpace::from((&mut **trait_of, i, &mut **addons));
            SubscriberNode::checked_transform(&Default::default(), &mut space, &args)?;
        }
        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &InstanceTraitNode<'a>,
        _: Option<&()>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let (trait_of, addons) = transform;
        let mut mut_trait_of = (**trait_of).clone();
        let mut mut_addons = (**addons).clone();

        for (i, item) in trait_of.items.iter().enumerate() {
            let TraitItem::Type(t) = item else {
                continue;
            };

            let is_subscriber = t.attrs.iter().try_fold(
                false,
                |found, a| -> Result<bool, proc_macro2::TokenStream> {
                    let AttrStyle::Outer = &a.style else {
                        return Ok(found);
                    };

                    let path = match &a.meta {
                        Meta::Path(path) => path,
                        Meta::List(meta_list) => &meta_list.path,
                        Meta::NameValue(meta_name_value) => &meta_name_value.path,
                    };

                    if path.is_ident(PublisherNode::IDENT) {
                        return Err(NodeBugs::PubNotAllowed {}.into());
                    }

                    Ok(found || path.is_ident(SubscriberNode::IDENT))
                },
            )?;

            if is_subscriber {
                let args = NodeArgs::checked_extract(t, &())?;
                let mut space = SubscriberNodeSpace::from((&mut mut_trait_of, i, &mut mut_addons));
                SubscriberNode::validate_transform(&Default::default(), &mut space, Some(&args))?;
            }
        }
        Ok(())
    }
}

// ===============================================================================
// `````````````````````````` POST INSTANCE TRAIT NODE ```````````````````````````
// ===============================================================================

/// Post-transformation pass for instance trait nodes.
///
/// Performs the cleanup stage following the primary transformation of instance
/// trait nodes.
///
/// Unlike the primary transformation pass, this pass does not perform any node
/// discovery or code generation. Instead, it delegates cleanup to the
/// individual node attribute removal transformations, such as
/// [`SubscriberNodeAttrRemoval`].
///
/// Each attribute removal transformation is responsible for determining whether
/// its corresponding node annotation should be removed from the syntax tree.
/// The decision is made by inspecting the remaining procedural macro
/// attributes applied to the associated type. If another transformation phase
/// still depends on the node annotation, the attribute is preserved;
/// otherwise, it is removed.
///
/// This deferred cleanup strategy allows multiple transformation phases to
/// share the same node annotations without prematurely discarding information
/// required by later passes. Once no remaining transformation depends on a
/// node annotation, the corresponding attribute removal transformation removes
/// it from the syntax tree.
///
/// ## Example
///
/// Before post-processing:
///
/// ```ignore
/// #[instance]
/// trait Logger {
///     #[instance_sub]
///     type Events;
/// }
/// ```
///
/// If no subsequent transformation requires the `#[instance_sub]` annotation,
/// this pass delegates to [`SubscriberNodeAttrRemoval`] to remove it,
/// producing:
///
/// ```ignore
/// trait Logger {
///     type Events;
/// }
/// ```
///
/// If a later transformation still depends on the annotation, it is preserved
/// until the corresponding post-transformation pass determines that it is safe
/// to remove.
#[derive(Debug, Clone)]
pub(crate) struct PostInstanceNodeTrait;

impl Transformation<ItemTrait> for PostInstanceNodeTrait {
    fn raw_transform(
        &self,
        transform: &mut ItemTrait,
        context: &(),
    ) -> Result<(), proc_macro2::TokenStream> {
        let space = &mut transform.into();
        SubscriberNodeAttrRemoval::checked_transform(&SubscriberNodeAttrRemoval, space, context)?;
        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &ItemTrait,
        context: Option<&()>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let mut mut_trait_of = transform.clone();
        let space = (&mut mut_trait_of).into();
        SubscriberNodeAttrRemoval::validate_transform(&SubscriberNodeAttrRemoval, &space, context)?;
        Ok(())
    }
}

// ===============================================================================
// ``````````````````````````` INSTANCE DOC TRAIT NODE ```````````````````````````
// ===============================================================================

/// Documentation-target transformation for instance trait nodes.
///
/// Produces the documentation representation of an instance trait.
///
/// During expansion, every instance trait is emitted in two forms:
///
/// - a **compilation target**, compiled under `#[cfg(not(doc))]`, which may
///   contain additional generated implementation details required by the
///   transformation pipeline; and
/// - a **documentation target**, compiled under `#[cfg(doc)]`, which presents
///   the user-facing definition.
///
/// This transformation operates exclusively on the documentation target. It
/// delegates to [`PostInstanceNodeTrait`] to remove all internal node
/// annotations, producing a clean trait definition suitable for generated
/// documentation.
///
/// Unlike the compilation target, this pass performs no node discovery or code
/// generation. Its sole responsibility is to prepare the documentation-facing
/// syntax tree by removing transformation-only node annotations while
/// preserving the public API.
#[derive(Clone, Debug)]
pub(crate) struct InstanceNodeTraitDocTarget;

impl Transformation<ItemTrait> for InstanceNodeTraitDocTarget {
    fn raw_transform(
        &self,
        transform: &mut ItemTrait,
        context: &(),
    ) -> Result<(), proc_macro2::TokenStream> {
        PostInstanceNodeTrait::checked_transform(&PostInstanceNodeTrait, transform, context)?;
        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &ItemTrait,
        context: Option<&()>,
    ) -> Result<(), proc_macro2::TokenStream> {
        PostInstanceNodeTrait::validate_transform(&PostInstanceNodeTrait, transform, context)?;
        Ok(())
    }
}

// ===============================================================================
// ````````````````````````````` INSTANCE IMPL NODE ``````````````````````````````
// ===============================================================================

/// Instance Impl Node target consisting of the impl being processed and the
/// auxiliary file used to collect generated addon items.
pub(crate) type InstanceImplNode<'a> = (&'a mut ItemImpl, &'a mut File);

/// Transformation pass for instance implementation nodes.
///
/// Processes the associated type definitions of an instance implementation to
/// discover and transform subscriber and publisher nodes.
///
/// This pass traverses every associated type defined within an instance
/// implementation and inspects its node annotations. Subscriber and publisher
/// nodes are identified, validated, and collected for transformation.
///
/// During traversal, this pass:
///
/// - ensures a subscriber node is annotated at most once;
/// - ensures a publisher node is annotated at most once;
/// - rejects associated types annotated as both subscriber and publisher
///   nodes;
/// - rejects subscriber node arguments, as subscriber nodes within
///   implementations do not accept arguments;
/// - extracts publisher node arguments; and
/// - collects subscriber and publisher nodes for transformation.
///
/// After validation, each subscriber node is transformed by constructing a
/// [`SubscriberNodeSpace`] and delegating expansion to [`SubscriberNode`].
/// Likewise, each publisher node is transformed by constructing an
/// [`ImplNodeSpace`] and delegating expansion to [`PublisherNode`].
///
/// Any items generated during expansion are appended to the auxiliary syntax
/// tree associated with the transformation target.
///
/// ## Example
///
/// Given the following instance implementation:
///
/// ```ignore
/// #[instance]
/// impl Logger for App {
///     #[instance_sub]
///     type Events = ...;
///
///     #[instance_pub(...)]
///     type Output = ...;
///
///     type State = ...;
/// }
/// ```
///
/// this transformation identifies `Events` as a subscriber node and `Output`
/// as a publisher node. Both nodes are validated and expanded through their
/// respective transformation passes, while `State` is ignored because it does
/// not declare a node.
#[derive(Debug, Clone)]
pub(crate) struct InstanceNodeImpl;

impl<'a> Transformation<InstanceImplNode<'a>> for InstanceNodeImpl {
    fn raw_transform(
        &self,
        transform: &mut InstanceImplNode<'a>,
        _: &(),
    ) -> Result<(), proc_macro2::TokenStream> {
        let (impl_of, addons) = transform;

        let mut sub_collect = Vec::new();
        let mut pub_collect = Vec::new();
        for (i, item) in impl_of.items.iter().enumerate() {
            let ImplItem::Type(t) = item else {
                continue;
            };

            let mut sub_node = false;
            let mut pub_node = false;
            for a in &t.attrs {
                let AttrStyle::Outer = &a.style else {
                    continue;
                };

                let path = match &a.meta {
                    Meta::Path(path) => path,
                    Meta::List(meta_list) => &meta_list.path,
                    Meta::NameValue(meta_name_value) => &meta_name_value.path,
                };

                if path.is_ident(SubscriberNode::IDENT) && sub_node {
                    return Err(NodeError::MultiSubNode { path: path.clone() }.into());
                }

                if path.is_ident(PublisherNode::IDENT) && pub_node {
                    return Err(NodeError::MultiPubNode { path: path.clone() }.into());
                }

                if path.is_ident(SubscriberNode::IDENT) {
                    sub_node = true;
                }

                if path.is_ident(PublisherNode::IDENT) {
                    pub_node = true;
                }

                if sub_node && pub_node {
                    return Err(NodeError::PubSubOnSameNode { ty: t.clone() }.into());
                }

                if sub_node {
                    if let Meta::List(list) = &a.meta {
                        if !list.tokens.is_empty() {
                            return Err(StateError::RemoveNodeArgs {
                                span: list.tokens.span(),
                            }
                            .into());
                        }
                    }
                }
            }

            if sub_node {
                sub_collect.push(i);
            }

            if pub_node {
                pub_collect.push(i);
            }
        }

        for i in sub_collect {
            let mut space = SubscriberNodeSpace::from((&mut **impl_of, i, &mut **addons));
            SubscriberNode::checked_transform(
                &Default::default(),
                &mut space,
                &NodeArgs::Unknown(TokenStream::new().into()),
            )?;
        }

        for i in pub_collect {
            let impl_item = impl_of.items.get(i).unwrap();
            let ImplItem::Type(assoc_ty) = impl_item else {
                return Err(NodeBugs::NodeNotFound {}.into());
            };

            let args = NodeArgs::checked_extract(assoc_ty, &())?;

            let mut space = ImplNodeSpace::from((&mut **impl_of, i, &mut **addons));
            PublisherNode::checked_transform(&Default::default(), &mut space, &args)?;
        }

        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &InstanceImplNode<'a>,
        _: Option<&()>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let (impl_of, addons) = transform;
        let mut mut_impl_of = (**impl_of).clone();
        let mut mut_addons = (**addons).clone();

        for (i, item) in impl_of.items.iter().enumerate() {
            let ImplItem::Type(t) = item else {
                continue;
            };

            let sub_node = t.attrs.iter().any(|a| {
                let AttrStyle::Outer = &a.style else {
                    return false;
                };

                let path = match &a.meta {
                    Meta::Path(path) => path,
                    Meta::List(meta_list) => &meta_list.path,
                    Meta::NameValue(meta_name_value) => &meta_name_value.path,
                };

                path.is_ident(SubscriberNode::IDENT)
            });

            let pub_node = t.attrs.iter().any(|a| {
                let AttrStyle::Outer = &a.style else {
                    return false;
                };

                let path = match &a.meta {
                    Meta::Path(path) => path,
                    Meta::List(meta_list) => &meta_list.path,
                    Meta::NameValue(meta_name_value) => &meta_name_value.path,
                };

                path.is_ident(PublisherNode::IDENT)
            });

            if sub_node && pub_node {
                return Err(NodeBugs::PubSubOnSameNode {}.into());
            }

            if sub_node {
                let space = SubscriberNodeSpace::from((&mut mut_impl_of, i, &mut mut_addons));
                SubscriberNode::validate_transform(
                    &Default::default(),
                    &space,
                    Some(&NodeArgs::Unknown(TokenStream::new().into())),
                )?;
            }

            if pub_node {
                let args = NodeArgs::checked_extract(t, &())?;
                let space = ImplNodeSpace::from((&mut mut_impl_of, i, &mut mut_addons));
                PublisherNode::validate_transform(&Default::default(), &space, Some(&args))?;
            }
        }
        Ok(())
    }
}

// ===============================================================================
// ``````````````````````````` POST INSTANCE IMPL NODE ```````````````````````````
// ===============================================================================

/// Post-transformation pass for instance implementation nodes.
///
/// Performs the cleanup stage following the primary transformation of instance
/// implementation nodes.
///
/// Unlike the primary transformation pass, this pass performs no node
/// discovery or code generation. Instead, it delegates cleanup to the
/// individual node attribute removal transformations, namely
/// [`SubscriberNodeAttrRemoval`] and [`PublisherNodeAttrRemoval`].
///
/// Each attribute removal transformation is responsible for determining
/// whether its corresponding node annotation should be removed from the syntax
/// tree. The decision is made by inspecting the remaining transformation
/// annotations applied to the associated type. If a subsequent transformation
/// phase still depends on a node annotation, the attribute is preserved;
/// otherwise, it is removed.
///
/// This deferred cleanup strategy allows multiple transformation phases to
/// share the same node annotations without prematurely discarding information
/// required by later passes. Once no remaining transformation depends on a
/// node annotation, the corresponding attribute removal transformation removes
/// it from the syntax tree.
///
/// ## Example
///
/// Before post-processing:
///
/// ```ignore
/// #[instance]
/// impl Logger for App {
///     #[instance_sub]
///     type Events = ...;
///
///     #[instance_pub(...)]
///     type Output = ...;
/// }
/// ```
///
/// If no subsequent transformation depends on these node annotations, this
/// pass delegates to [`SubscriberNodeAttrRemoval`] and
/// [`PublisherNodeAttrRemoval`] to remove them, producing:
///
/// ```ignore
/// #[instance]
/// impl Logger for App {
///     type Events = ...;
///     type Output = ...;
/// }
/// ```
///
/// If a later transformation still requires either annotation, it is preserved
/// until the corresponding post-transformation pass determines that it is safe
/// to remove.
#[derive(Debug, Clone)]
pub(crate) struct PostInstanceNodeImpl;

impl Transformation<ItemImpl> for PostInstanceNodeImpl {
    fn raw_transform(
        &self,
        transform: &mut ItemImpl,
        context: &(),
    ) -> Result<(), proc_macro2::TokenStream> {
        let space = &mut transform.into();
        SubscriberNodeAttrRemoval::checked_transform(&SubscriberNodeAttrRemoval, space, context)?;
        PublisherNodeAttrRemoval::checked_transform(&PublisherNodeAttrRemoval, space, context)?;
        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &ItemImpl,
        context: Option<&()>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let mut mut_impl_of = transform.clone();
        let space = (&mut mut_impl_of).into();
        SubscriberNodeAttrRemoval::validate_transform(&SubscriberNodeAttrRemoval, &space, context)?;
        PublisherNodeAttrRemoval::validate_transform(&PublisherNodeAttrRemoval, &space, context)?;
        Ok(())
    }
}

// ===============================================================================
// `````````````````````````` INSTANCE IMPL TRAIT NODE ```````````````````````````
// ===============================================================================

/// Documentation-target transformation for instance implementation nodes.
///
/// Produces the documentation representation of an instance implementation.
///
/// During expansion, every instance implementation is emitted in two forms:
///
/// - a **compilation target**, compiled under `#[cfg(not(doc))]`, which may
///   contain additional generated implementation details required by the
///   transformation pipeline; and
/// - a **documentation target**, compiled under `#[cfg(doc)]`, which presents
///   the user-facing implementation.
///
/// This transformation operates exclusively on the documentation target. It
/// delegates to [`PostInstanceNodeImpl`] to remove all internal node
/// annotations, producing a clean implementation suitable for generated
/// documentation.
///
/// Unlike the compilation target, this pass performs no node discovery or code
/// generation. Its sole responsibility is to prepare the documentation-facing
/// syntax tree by removing transformation-only node annotations while
/// preserving the public API.
#[derive(Clone, Debug)]
pub(crate) struct InstanceNodeImplDocTarget;

impl Transformation<ItemImpl> for InstanceNodeImplDocTarget {
    fn raw_transform(
        &self,
        transform: &mut ItemImpl,
        context: &(),
    ) -> Result<(), proc_macro2::TokenStream> {
        PostInstanceNodeImpl::checked_transform(&PostInstanceNodeImpl, transform, context)?;
        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &ItemImpl,
        context: Option<&()>,
    ) -> Result<(), proc_macro2::TokenStream> {
        PostInstanceNodeImpl::validate_transform(&PostInstanceNodeImpl, transform, context)?;
        Ok(())
    }
}
