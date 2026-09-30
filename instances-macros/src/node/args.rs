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
// `````````````````````````````` INSTANCE NODE ARGS `````````````````````````````
// ===============================================================================

//! Parsing and validation of instance node attributes.
//!
//! This module implements the parsing infrastructure for
//! `#[instance_sub(...)]` and `#[instance_pub(...)]` attributes. It extracts
//! node selectors, validates their arguments, and converts them into strongly
//! typed selection models used during subscriber and publisher node
//! transformations.
//!
//! Supported node selectors:
//!
//! ```text
//!   Leaf      = Exact node
//!   Branch    = Subtree
//!   Root      = Entire tree
//!
//!   Root   -------> Leaf      = Prune
//!   Root   -------> Branch    = Trim
//!
//!   Leaf   -------> End       = Extend
//!   Branch -------> End       = Spread
//!
//!   Leaf   -------> Branch    = Traverse
//!   Branch -------> Leaf      = Descend
//! ```

// ===============================================================================
// ``````````````````````````````````` IMPORTS ```````````````````````````````````
// ===============================================================================

// --- Std crate ---
use std::{collections::HashSet, fmt::Debug, marker::PhantomData};

// --- Proc Suite ---
use proc_suite::{
    BStringList, DuplicateCheck, IntList, key_schema,
    keys::{KeyInfo, KeySpec},
};

// --- Proc Macro Crates ---
use proc_macro2::TokenStream;
use quote::format_ident;
use syn::{
    AttrStyle, Attribute, File, Ident, ImplItemType, ItemImpl, ItemTrait, Meta, TraitItemType,
    parse, spanned::Spanned,
};

// --- Local Crate
use crate::{
    Extraction,
    node::{NodeBugs, errors::*},
};

// ===============================================================================
// `````````````````````````````````` CONSTANTS ``````````````````````````````````
// ===============================================================================

/// Attribute used to mark an associated type as a subscriber node.
///
/// Supports both a plain marker:
/// `#[instance_sub]`
///
/// and a meta-list form for additional configuration:
/// `#[instance_sub(...)]`
///
/// Only path and meta-list forms are supported; name-value syntax is not.
pub(crate) const SUBSCRIBER_NODE: &str = "instance_sub";

/// Attribute used to mark an associated type as a publisher node.
///
/// Supports only a meta-list form:
/// `#[instance_pub(...)]`
///
/// Only path and meta-list forms are supported; name-value syntax is not.
pub(crate) const PUBLISHER_NODE: &str = "instance_pub";

// ===============================================================================
// ``````````````````````` SUBSCRIBER TRANFORMATION SPACE ````````````````````````
// ===============================================================================

/// Processing context for a subscriber node.
///
/// A subscriber node may originate from either a trait declaration or an
/// implementation block, each carrying the mutable syntax tree required for
/// code generation.
#[derive(Debug)]
pub(crate) enum SubscriberNodeSpace<'a> {
    /// Subscriber node declared in a trait.
    Trait(TraitNodeSpace<'a>),

    /// Subscriber node declared in an impl block.
    Impl(ImplNodeSpace<'a>),
}

/// Processing context for a instance node declared in a trait.
#[derive(Debug)]
pub(crate) struct TraitNodeSpace<'a> {
    /// Trait containing the node.
    pub(crate) trait_of: &'a mut ItemTrait,

    /// Index of the associated type within the trait.
    pub(crate) ty_idx: usize,

    /// File receiving any generated items.
    pub(crate) addons: &'a mut File,
}

/// Processing context for a instance node declared in an impl block.
#[derive(Debug)]
pub(crate) struct ImplNodeSpace<'a> {
    /// Impl block containing the node.
    pub(crate) impl_of: &'a mut ItemImpl,

    /// Index of the associated type within the impl.
    pub(crate) ty_idx: usize,

    /// File receiving any generated items.
    pub(crate) addons: &'a mut File,
}

impl<'a> From<(&'a mut ItemTrait, usize, &'a mut File)> for TraitNodeSpace<'a> {
    fn from(value: (&'a mut ItemTrait, usize, &'a mut File)) -> Self {
        TraitNodeSpace {
            trait_of: value.0,
            ty_idx: value.1,
            addons: value.2,
        }
    }
}

impl<'a> From<(&'a mut ItemImpl, usize, &'a mut File)> for ImplNodeSpace<'a> {
    fn from(value: (&'a mut ItemImpl, usize, &'a mut File)) -> Self {
        ImplNodeSpace {
            impl_of: value.0,
            ty_idx: value.1,
            addons: value.2,
        }
    }
}

impl<'a> From<(&'a mut ItemTrait, usize, &'a mut File)> for SubscriberNodeSpace<'a> {
    fn from(value: (&'a mut ItemTrait, usize, &'a mut File)) -> Self {
        SubscriberNodeSpace::Trait(TraitNodeSpace {
            trait_of: value.0,
            ty_idx: value.1,
            addons: value.2,
        })
    }
}

impl<'a> From<(&'a mut ItemImpl, usize, &'a mut File)> for SubscriberNodeSpace<'a> {
    fn from(value: (&'a mut ItemImpl, usize, &'a mut File)) -> Self {
        SubscriberNodeSpace::Impl(ImplNodeSpace {
            impl_of: value.0,
            ty_idx: value.1,
            addons: value.2,
        })
    }
}

// ===============================================================================
// ```````````````````````````````` NODE KEY-ARGS ````````````````````````````````
// ===============================================================================

/// Parsed arguments describing an instance node selection.
///
/// Each variant represents a distinct selection model over the instance node
/// hierarchy. Together they form a complete set of selectors covering exact
/// nodes, subtrees, ranges, and directional traversals between leaves,
/// branches, the root, and the end of the hierarchy.
///
/// Unrecognized node kinds are preserved as raw tokens by `Unknown`.
#[derive(Debug, Clone)]
pub(crate) enum NodeArgs<Node: NodeOf> {
    /// Select one or more exact leaf nodes.
    Leaf(LeafArgs<Node>),

    /// Select one or more branch subtrees.
    Branch(BranchArgs<Node>),

    /// Select from the root up to an exact leaf.
    Prune(PruneArgs<Node>),

    /// Select from the root up to the end of a branch.
    Trim(TrimArgs<Node>),

    /// Select from an exact leaf to the end.
    Extend(ExtendArgs<Node>),

    /// Select from the beginning of a branch to the end.
    Spread(SpreadArgs<Node>),

    /// Select from an exact leaf until the end of a branch.
    Traverse(TraverseArgs<Node>),

    /// Select from the beginning of a branch until an exact leaf.
    Descend(DescendArgs<Node>),

    /// Select the entire hierarchy.
    Root(RootArgs<Node>),

    /// Preserve an unrecognized node selector as raw tokens.
    Unknown(TokenNodeArgs<Node>),
}

/// Node identifier extracted from an `#[instance_sub]` attribute.
///
/// Stores the parsed node selector (e.g. `leaf`, `branch`, `prune`, `root`)
/// associated with a subscriber node.
#[derive(Debug, Clone)]
pub(crate) struct SubscriberNode(Ident);

/// Node identifier extracted from an `#[instance_pub]` attribute.
///
/// Stores the parsed node selector (e.g. `leaf`, `branch`, `prune`, `root`)
/// associated with a publisher node.
#[derive(Debug, Clone)]
pub(crate) struct PublisherNode(Ident);

/// Identifies the kind of instance node being processed.
///
/// Implementations represent a concrete node attribute (subscriber or
/// publisher) and encapsulate the parsed node selector (for example,
/// `leaf`, `branch`, `prune`, or `root`).
pub(crate) trait NodeOf: Default + Debug + Clone {
    /// Creates a node from the parsed node identifier.
    fn new(i: &Ident) -> Self;

    /// Returns the node identifier associated with this node
    /// (e.g. `leaf`, `branch`, `prune`, `extend`, `root`).
    fn ident(&self) -> &Ident;

    /// Name of the attribute associated with this node type
    /// (e.g. `instance_sub` or `instance_pub`).
    const IDENT: &'static str;
}

// ===============================================================================
// ```````````````````````````````` MISCELLEANEOUS ```````````````````````````````
// ===============================================================================

/// Provides access to the instance counter generic indexes associated with a
/// node selection with additional validation oppurtunities.
pub(crate) trait GetCounterGenericIndexes<'a>: Default {
    /// Returns the instance counter generic indexes.
    fn get(&'a self) -> Result<&'a IntList, TokenStream>;
}

/// Provides access to the outer attributes of a supported associated type.
///
/// This abstraction allows extraction logic to operate uniformly over both
/// trait and impl associated types without depending on their concrete syntax
/// node.
pub(crate) trait ExtractAttr {
    /// Returns the attributes attached to the associated type.
    fn get(&self) -> &Vec<Attribute>;
}

impl ExtractAttr for TraitItemType {
    fn get(&self) -> &Vec<Attribute> {
        &self.attrs
    }
}

impl ExtractAttr for ImplItemType {
    fn get(&self) -> &Vec<Attribute> {
        &self.attrs
    }
}

impl Default for SubscriberNode {
    fn default() -> Self {
        Self(format_ident!("_"))
    }
}
impl NodeOf for SubscriberNode {
    const IDENT: &'static str = SUBSCRIBER_NODE;

    fn new(i: &Ident) -> Self {
        Self(i.clone())
    }

    fn ident(&self) -> &Ident {
        &self.0
    }
}

impl NodeOf for PublisherNode {
    const IDENT: &'static str = PUBLISHER_NODE;
    fn new(i: &Ident) -> Self {
        Self(i.clone())
    }
    fn ident(&self) -> &Ident {
        &self.0
    }
}
impl Default for PublisherNode {
    fn default() -> Self {
        Self(format_ident!("_"))
    }
}

/// Extracts and dispatches the parsed node selector to its corresponding
/// argument model.
///
/// The node selector (`leaf`, `branch`, `prune`, `trim`, `extend`, `spread`,
/// `traverse`, `descend`, or `root`) is first extracted from the instance
/// attribute and then used to parse the matching argument structure. Unknown
/// selectors are preserved as raw tokens.
impl<A: ExtractAttr, Node: NodeOf + Clone + Debug + 'static> Extraction<A> for NodeArgs<Node> {
    fn raw_extract(from: &A, context: &()) -> Result<Self, proc_macro2::TokenStream> {
        let key = Node::checked_extract(from, context)?.ident().to_string();
        match key.as_str() {
            "leaf" => Ok(NodeArgs::Leaf(LeafArgs::checked_extract(from, context)?)),
            "branch" => Ok(NodeArgs::Branch(BranchArgs::checked_extract(
                from, context,
            )?)),
            "prune" => Ok(NodeArgs::Prune(PruneArgs::checked_extract(from, context)?)),
            "trim" => Ok(NodeArgs::Trim(TrimArgs::checked_extract(from, context)?)),
            "extend" => Ok(NodeArgs::Extend(ExtendArgs::checked_extract(
                from, context,
            )?)),
            "spread" => Ok(NodeArgs::Spread(SpreadArgs::checked_extract(
                from, context,
            )?)),
            "traverse" => Ok(NodeArgs::Traverse(TraverseArgs::checked_extract(
                from, context,
            )?)),
            "descend" => Ok(NodeArgs::Descend(DescendArgs::checked_extract(
                from, context,
            )?)),
            "root" => Ok(NodeArgs::Root(RootArgs::checked_extract(from, context)?)),
            _ => Ok(NodeArgs::Unknown(TokenNodeArgs::checked_extract(
                from, context,
            )?)),
        }
    }

    fn validate_extract(&self, _: &A, _: Option<&()>) -> Result<(), proc_macro2::TokenStream> {
        Ok(())
    }
}

// ===============================================================================
// ````````````````````````````` NODE ARGS KEY IDENT `````````````````````````````
// ===============================================================================

/// Extracts the node selector from an instance node attribute.
///
/// Searches the supported instance node attribute (`#[instance_sub(...)]` or
/// `#[instance_pub(...)]`), parses its key, and returns the corresponding node
/// identifier (e.g. `leaf`, `branch`, `prune`, `extend`, or `root`) wrapped by
/// the target [`NodeOf`] implementation.
impl<A: ExtractAttr, T: 'static> Extraction<A> for T
where
    T: NodeOf,
{
    fn raw_extract(from: &A, _: &()) -> Result<Self, proc_macro2::TokenStream> {
        for attr in from.get() {
            let AttrStyle::Outer = attr.style else {
                continue;
            };

            let path = match &attr.meta {
                Meta::Path(path) => path,
                Meta::List(meta_list) => &meta_list.path,
                Meta::NameValue(_) => continue,
            };

            if !path.is_ident(Self::IDENT) {
                continue;
            }

            let tokens = match &attr.meta {
                Meta::Path(_) => &TokenStream::new(),
                Meta::List(meta_list) => &meta_list.tokens,
                Meta::NameValue(_) => continue,
            };

            let k_info = match parse::<KeyInfo>(tokens.clone().into()) {
                Ok(v) => v,
                Err(_) => {
                    return Err(KeysError::ExpectedKeys {
                        span: tokens.span(),
                    }
                    .into());
                }
            };

            return Ok(Self::new(&k_info.key));
        }
        return Err(NodeBugs::NodeAttrNotFound {}.into());
    }

    fn validate_extract(&self, _: &A, _: Option<&()>) -> Result<(), proc_macro2::TokenStream> {
        Ok(())
    }
}

// ===============================================================================
// ````````````````````````````` LEAF NODE KEY-ARGS ``````````````````````````````
// ===============================================================================

use leaf_key::LeafKeySpec;

mod leaf_key {
    use super::*;
    key_schema! {
        /// Key specification for the `leaf` node selector.
        ///
        /// Defines the accepted keys and valid argument combinations for a leaf
        /// selection.
        pub(crate) LeafKeySpec {
            key: "leaf",

            allow_empty: false,

            sub_keys: {
                index : IntList,
                ident : [BStringList],
            },

            combinations: {
                Indexed(index, ident),
            }
        }
    }
}

/// Arguments for a `leaf` node selection.
///
/// Represents one or more exact identifier paths together with their
/// corresponding instance counter generic indexes.
///
/// Example:
/// ```text
/// indexes = [0, 1]
///
/// idents = [
///     ["Account", "Free"],
///     ["Account", "Reserved"],
/// ]
///
/// Tree
/// Root
/// |--Account
///     |-- Free      <- leaf / exact
///     |-- Reserved  <- leaf / exact
/// ```
///
/// Each identifier path is an exact leaf selected by the node, while
/// `indexes` specifies the instance counter generic indexes associated with
/// those paths.
#[derive(Debug, Clone, Default)]
pub(crate) struct LeafArgs<Node: NodeOf> {
    /// Marker for the associated node attribute type.
    node: PhantomData<Node>,

    /// Instance counter generic indexes associated with each identifier path.
    pub(crate) indexes: IntList,

    /// Exact identifier paths selected by the leaf node.
    pub(crate) idents: Vec<BStringList>,
}

impl<A: ExtractAttr, Node: NodeOf + Clone + Debug + 'static> Extraction<A> for LeafArgs<Node> {
    fn raw_extract(from: &A, _: &()) -> Result<Self, proc_macro2::TokenStream> {
        for attr in from.get() {
            let AttrStyle::Outer = attr.style else {
                continue;
            };
            let Meta::List(list) = &attr.meta else {
                continue;
            };

            if !list.path.is_ident(Node::IDENT) {
                continue;
            }

            let k_info = match parse::<KeyInfo>(list.tokens.clone().into()) {
                Ok(v) => v,
                Err(_) => {
                    return Err(KeysError::ExpectedLeaf {
                        span: if list.tokens.is_empty() {
                            list.path.span()
                        } else {
                            list.tokens.span()
                        },
                    }
                    .into());
                }
            };

            let args = LeafKeySpec::parse(&k_info)?;

            let (indexes, idents) = match args {
                LeafKeySpec::Indexed(int_list, value_group) => {
                    let len = int_list.ints.len();
                    let mut collect = Vec::new();
                    let mut seen = HashSet::new();
                    for list in value_group {
                        if len != list.bytes.len() {
                            return Err(SubscriberError::IndexIdentsLenMismatch {
                                idents: list.clone(),
                                exp_len: len,
                            }
                            .into());
                        }
                        if !seen.insert(list.bytes.clone()) {
                            return Err(SubscriberError::DuplicateIdentInstance {
                                idents: list.clone(),
                            }
                            .into());
                        }
                        collect.push(list);
                    }
                    (int_list, collect)
                }
            };

            return Ok(Self {
                node: PhantomData,
                indexes,
                idents,
            });
        }
        return Err(NodeBugs::NodeAttrNotFound {}.into());
    }

    fn validate_extract(&self, _: &A, _: Option<&()>) -> Result<(), proc_macro2::TokenStream> {
        Ok(())
    }
}

impl<'a, Node: NodeOf> GetCounterGenericIndexes<'a> for LeafArgs<Node> {
    fn get(&'a self) -> Result<&'a IntList, TokenStream> {
        let indexes = &self.indexes;
        indexes.duplicate_check(Some(NodeSpace::DuplicateIndexes {}.into()))?;
        Ok(indexes)
    }
}

// ===============================================================================
// ```````````````````````````` BRANCH NODE KEY-ARGS `````````````````````````````
// ===============================================================================

use branch_key::BranchKeySpec;
mod branch_key {
    use super::*;
    key_schema! {
        /// Key specification for the `branch` node selector.
        ///
        /// Defines the accepted keys and valid argument combinations for a branch
        /// selection.
                pub(crate) BranchKeySpec {
            key: "branch",

            allow_empty: false,

            sub_keys: {
                index : IntList,
                parent : BStringList,
            },

            combinations: {
                Parent(index, parent),
            }
        }
    }
}

/// Arguments for a `branch` node selection.
///
/// Represents a subtree rooted at a branch identifier together with the
/// associated instance counter generic indexes.
///
/// Example:
/// ```text
/// indexes = [0, 1]
///
/// parent = ["Account"]
///
/// Tree
/// Root
/// |-- Account        <- branch / start
///     |-- Free
///     |-- Reserved
///     |-- Frozen     <- end
/// |-- User
///     |-- Awake
///     |-- Asleep
/// ```
///
/// The `parent` identifies the root of the selected subtree, while `indexes`
/// specifies the instance counter generic indexes associated with that branch.
#[derive(Debug, Clone, Default)]
pub(crate) struct BranchArgs<Node: NodeOf> {
    /// Marker for the associated node attribute type.
    node: PhantomData<Node>,

    /// Instance counter generic indexes associated with the branch selection.
    pub(crate) indexes: IntList,

    /// Identifier path of the branch root.
    pub(crate) parent: BStringList,
}

impl<A: ExtractAttr, Node: NodeOf + Clone + Debug + 'static> Extraction<A> for BranchArgs<Node> {
    fn raw_extract(from: &A, _: &()) -> Result<Self, proc_macro2::TokenStream> {
        for attr in from.get() {
            let AttrStyle::Outer = attr.style else {
                continue;
            };
            let Meta::List(list) = &attr.meta else {
                continue;
            };

            if !list.path.is_ident(Node::IDENT) {
                continue;
            }

            let k_info = match parse::<KeyInfo>(list.tokens.clone().into()) {
                Ok(v) => v,
                Err(_) => {
                    return Err(KeysError::ExpectedBranch {
                        span: if list.tokens.is_empty() {
                            list.path.span()
                        } else {
                            list.tokens.span()
                        },
                    }
                    .into());
                }
            };

            let args = BranchKeySpec::parse(&k_info)?;

            let BranchKeySpec::Parent(indexes, parent) = args;

            let idents_len = parent.bytes.len();
            let idx_len = indexes.ints.len();

            if idents_len > idx_len {
                return Err(BranchError::ParentMustBeOfLesserLen {
                    idents: parent.clone(),
                    less_than_len: idx_len,
                }
                .into());
            }

            if idents_len == idx_len {
                return Err(BranchError::ParentNotLeafIdent {
                    idents: parent.clone(),
                    less_than_len: idx_len,
                }
                .into());
            }

            return Ok(Self {
                node: PhantomData,
                indexes,
                parent,
            });
        }
        return Err(NodeBugs::NodeAttrNotFound {}.into());
    }

    fn validate_extract(&self, _: &A, _: Option<&()>) -> Result<(), proc_macro2::TokenStream> {
        Ok(())
    }
}

impl<'a, Node: NodeOf> GetCounterGenericIndexes<'a> for BranchArgs<Node> {
    fn get(&'a self) -> Result<&'a IntList, TokenStream> {
        let indexes = &self.indexes;
        indexes.duplicate_check(Some(NodeSpace::DuplicateIndexes {}.into()))?;
        Ok(indexes)
    }
}

// ===============================================================================
// ````````````````````````````` PRUNE NODE KEY-ARGS `````````````````````````````
// ===============================================================================

use prune_key::PruneKeySpec;
mod prune_key {
    use super::*;
    key_schema! {
        /// Key specification for the `prune` node selector.
        ///
        /// Defines the accepted keys and valid argument combinations for a prune
        /// selection.
        pub(crate) PruneKeySpec {
            key: "prune",

            allow_empty: false,

            sub_keys: {
                index : IntList,
                until : BStringList,
            },

            combinations: {
                Suffix(index, until),
            }
        }
    }
}

/// Arguments for a `prune` node selection.
///
/// Represents a selection from the root of the hierarchy up to an exact leaf,
/// together with the associated instance counter generic indexes.
///
/// Example:
/// ```text
/// indexes = [0, 1, 2]
///
/// until = ["Account", "Free", "Locked"]
///
/// Tree
/// Root                <- start
/// |--Account
///     |-- Free
///     |   |--Locked   <- until / end
///     |--Reserved
///
/// Selection
/// Root ---> Account ---> Free ---> Locked
/// ```
///
/// The `until` path identifies the inclusive leaf at which the selection
/// terminates, while `indexes` specifies the instance counter generic indexes
/// associated with that path.
#[derive(Debug, Clone, Default)]
pub(crate) struct PruneArgs<Node: NodeOf> {
    /// Marker for the associated node attribute type.
    node: PhantomData<Node>,

    /// Instance counter generic indexes associated with the selection.
    pub(crate) indexes: IntList,

    /// Exact leaf where the selection ends.
    pub(crate) until: BStringList,
}

impl<A: ExtractAttr, Node: NodeOf + Clone + Debug + 'static> Extraction<A> for PruneArgs<Node> {
    fn raw_extract(from: &A, _: &()) -> Result<Self, proc_macro2::TokenStream> {
        for attr in from.get() {
            let AttrStyle::Outer = attr.style else {
                continue;
            };
            let Meta::List(list) = &attr.meta else {
                continue;
            };

            if !list.path.is_ident(Node::IDENT) {
                continue;
            }

            let k_info = match parse::<KeyInfo>(list.tokens.clone().into()) {
                Ok(v) => v,
                Err(_) => {
                    return Err(KeysError::ExpectedPrune {
                        span: if list.tokens.is_empty() {
                            list.path.span()
                        } else {
                            list.tokens.span()
                        },
                    }
                    .into());
                }
            };

            let args = PruneKeySpec::parse(&k_info)?;

            let PruneKeySpec::Suffix(indexes, until) = args;
            let idx_len = indexes.ints.len();
            let found = until.bytes.len();
            if found != idx_len {
                return Err(SubscriberError::IndexIdentsLenMismatch {
                    idents: until.clone(),
                    exp_len: idx_len,
                }
                .into());
            }

            return Ok(Self {
                node: PhantomData,
                indexes,
                until,
            });
        }
        return Err(NodeBugs::NodeAttrNotFound {}.into());
    }

    fn validate_extract(&self, _: &A, _: Option<&()>) -> Result<(), proc_macro2::TokenStream> {
        Ok(())
    }
}

impl<'a, Node: NodeOf> GetCounterGenericIndexes<'a> for PruneArgs<Node> {
    fn get(&'a self) -> Result<&'a IntList, TokenStream> {
        let indexes = &self.indexes;
        indexes.duplicate_check(Some(NodeSpace::DuplicateIndexes {}.into()))?;
        Ok(indexes)
    }
}

// ===============================================================================
// ````````````````````````````` TRIM NODE KEY-ARGS ``````````````````````````````
// ===============================================================================

use trim_key::TrimKeySpec;
mod trim_key {
    use super::*;
    key_schema! {
        /// Key specification for the `trim` node selector.
        ///
        /// Defines the accepted keys and valid argument combinations for a trim
        /// selection.
        pub(crate) TrimKeySpec {
            key: "trim",

            allow_empty: false,

            sub_keys: {
                index : IntList,
                until : BStringList,
            },

            combinations: {
                Suffix(index, until),
            }
        }
    }
}

/// Arguments for a `trim` node selection.
///
/// Represents a selection from the root of the hierarchy up to the end of a
/// branch, together with the associated instance counter generic indexes.
///
/// Example:
/// ```text
/// indexes = [0, 1, 2]
///
/// until = ["Account", "Free"]
///
/// Tree
/// Root                  <- start  
/// |-- Account
///     |-- Free          <- branch
///     |   |-- Locked
///     |   |-- Frozen    <- end
///     |-- Reserved
///
/// Selection
/// Root ---> Account ---> Free ---> * ---> Frozen
/// ```
///
/// The `until` path identifies the branch whose subtree marks the end of the
/// selection, while `indexes` specifies the associated instance counter
/// generic indexes.
#[derive(Debug, Clone, Default)]
pub(crate) struct TrimArgs<Node: NodeOf> {
    /// Marker for the associated node attribute type.
    node: PhantomData<Node>,

    /// Instance counter generic indexes associated with the selection.
    pub(crate) indexes: IntList,

    /// Branch where the selection ends.
    pub(crate) until: BStringList,
}

impl<A: ExtractAttr, Node: NodeOf + Clone + Debug + 'static> Extraction<A> for TrimArgs<Node> {
    fn raw_extract(from: &A, _: &()) -> Result<Self, proc_macro2::TokenStream> {
        for attr in from.get() {
            let AttrStyle::Outer = attr.style else {
                continue;
            };
            let Meta::List(list) = &attr.meta else {
                continue;
            };

            if !list.path.is_ident(Node::IDENT) {
                continue;
            }

            let k_info = match parse::<KeyInfo>(list.tokens.clone().into()) {
                Ok(v) => v,
                Err(_) => {
                    return Err(KeysError::ExpectedTrim {
                        span: if list.tokens.is_empty() {
                            list.path.span()
                        } else {
                            list.tokens.span()
                        },
                    }
                    .into());
                }
            };

            let args = TrimKeySpec::parse(&k_info)?;

            let TrimKeySpec::Suffix(indexes, until) = args;
            let idx_len = indexes.ints.len();
            let idents_len = until.bytes.len();
            if idents_len > idx_len {
                return Err(TrimError::ParentMustBeOfLesserLen {
                    idents: until.clone(),
                    less_than_len: idx_len,
                }
                .into());
            }

            if idents_len == idx_len {
                return Err(TrimError::ParentNotPruneIdent {
                    idents: until.clone(),
                    less_than_len: idx_len,
                }
                .into());
            }

            return Ok(Self {
                node: PhantomData,
                indexes,
                until,
            });
        }
        return Err(NodeBugs::NodeAttrNotFound {}.into());
    }

    fn validate_extract(&self, _: &A, _: Option<&()>) -> Result<(), proc_macro2::TokenStream> {
        Ok(())
    }
}

impl<'a, Node: NodeOf> GetCounterGenericIndexes<'a> for TrimArgs<Node> {
    fn get(&'a self) -> Result<&'a IntList, TokenStream> {
        let indexes = &self.indexes;
        indexes.duplicate_check(Some(NodeSpace::DuplicateIndexes {}.into()))?;
        Ok(indexes)
    }
}

// ===============================================================================
// ````````````````````````````` EXTEND NODE KEY-ARGS ````````````````````````````
// ===============================================================================

use extend_key::ExtendKeySpec;
mod extend_key {
    use super::*;
    key_schema! {
        /// Key specification for the `extend` node selector.
        ///
        /// Defines the accepted keys and valid argument combinations for an extend
        /// selection.
        pub(crate) ExtendKeySpec {
            key: "extend",

            allow_empty: false,

            sub_keys: {
                index : IntList,
                from : BStringList,
            },

            combinations: {
                Prefix(index, from),
            }
        }
    }
}

/// Arguments for an `extend` node selection.
///
/// Represents a selection from an exact leaf to the end of the hierarchy,
/// together with the associated instance counter generic indexes.
///
/// Example:
/// ```text
/// indexes = [0, 1, 2]
///
/// from = ["Account", "Free", "Locked"]
///
/// Tree
/// Root
/// |-- Account
///     |-- Free
///     |   |-- Locked      <- from / start
///     |   |-- Unlocked    
///     |-- Reserved
///         |-- Frozen      <- end
///
/// Selection
/// Account --> Free ---> Locked ---> End
/// ```
///
/// The `from` path identifies the inclusive leaf where the selection begins,
/// while `indexes` specifies the associated instance counter generic indexes.
#[derive(Debug, Clone, Default)]
pub(crate) struct ExtendArgs<Node: NodeOf> {
    /// Marker for the associated node attribute type.
    node: PhantomData<Node>,

    /// Instance counter generic indexes associated with the selection.
    pub(crate) indexes: IntList,

    /// Exact leaf where the selection begins.
    pub(crate) from: BStringList,
}

impl<A: ExtractAttr, Node: NodeOf + Clone + Debug + 'static> Extraction<A> for ExtendArgs<Node> {
    fn raw_extract(from: &A, _: &()) -> Result<Self, proc_macro2::TokenStream> {
        for attr in from.get() {
            let AttrStyle::Outer = attr.style else {
                continue;
            };
            let Meta::List(list) = &attr.meta else {
                continue;
            };

            if !list.path.is_ident(Node::IDENT) {
                continue;
            }

            let k_info = match parse::<KeyInfo>(list.tokens.clone().into()) {
                Ok(v) => v,
                Err(_) => {
                    return Err(KeysError::ExpectedExtend {
                        span: if list.tokens.is_empty() {
                            list.path.span()
                        } else {
                            list.tokens.span()
                        },
                    }
                    .into());
                }
            };

            let args = ExtendKeySpec::parse(&k_info)?;
            let ExtendKeySpec::Prefix(indexes, from) = args;
            let idx_len = indexes.ints.len();
            let found = from.bytes.len();
            if found != idx_len {
                return Err(SubscriberError::IndexIdentsLenMismatch {
                    idents: from.clone(),
                    exp_len: idx_len,
                }
                .into());
            }
            return Ok(Self {
                node: PhantomData,
                indexes,
                from,
            });
        }
        return Err(NodeBugs::NodeAttrNotFound {}.into());
    }

    fn validate_extract(&self, _: &A, _: Option<&()>) -> Result<(), proc_macro2::TokenStream> {
        Ok(())
    }
}

impl<'a, Node: NodeOf> GetCounterGenericIndexes<'a> for ExtendArgs<Node> {
    fn get(&'a self) -> Result<&'a IntList, TokenStream> {
        let indexes = &self.indexes;
        indexes.duplicate_check(Some(NodeSpace::DuplicateIndexes {}.into()))?;
        Ok(indexes)
    }
}

// ===============================================================================
// ```````````````````````````` SPREAD NODE KEY-ARGS `````````````````````````````
// ===============================================================================

use spread_key::SpreadKeySpec;
mod spread_key {
    use super::*;
    key_schema! {
        /// Key specification for the `spread` node selector.
        ///
        /// Defines the accepted keys and valid argument combinations for a spread
        /// selection.
        pub(crate) SpreadKeySpec {
            key: "spread",

            allow_empty: false,

            sub_keys: {
                index : IntList,
                from : BStringList,
            },

            combinations: {
                Prefix(index, from),
            }
        }
    }
}

/// Arguments for a `spread` node selection.
///
/// Represents a selection from the beginning of a branch to the end of the
/// hierarchy, together with the associated instance counter generic indexes.
///
/// Example:
/// ```text
/// indexes = [0, 1, 2]
///
/// from = ["Account", "Free"]
///
/// Tree
/// Root
/// |-- Account
///     |-- Free          <- branch / start
///     |   |-- Locked
///     |   |-- Frozen
///     |-- Reserved
///         |-- Archived  <- end
///
/// Selection
/// Account--> Free --> * --->  Archived
/// ```
///
/// The `from` path identifies the branch whose subtree forms the beginning of
/// the selection, while `indexes` specifies the associated instance counter
/// generic indexes.
#[derive(Debug, Clone, Default)]
pub(crate) struct SpreadArgs<Node: NodeOf> {
    /// Marker for the associated node attribute type.
    node: PhantomData<Node>,

    /// Instance counter generic indexes associated with the selection.
    pub(crate) indexes: IntList,

    /// Branch where the selection begins.
    pub(crate) from: BStringList,
}

impl<A: ExtractAttr, Node: NodeOf + Clone + Debug + 'static> Extraction<A> for SpreadArgs<Node> {
    fn raw_extract(from: &A, _: &()) -> Result<Self, proc_macro2::TokenStream> {
        for attr in from.get() {
            let AttrStyle::Outer = attr.style else {
                continue;
            };
            let Meta::List(list) = &attr.meta else {
                continue;
            };

            if !list.path.is_ident(Node::IDENT) {
                continue;
            }

            let k_info = match parse::<KeyInfo>(list.tokens.clone().into()) {
                Ok(v) => v,
                Err(_) => {
                    return Err(KeysError::ExpectedSpread {
                        span: if list.tokens.is_empty() {
                            list.path.span()
                        } else {
                            list.tokens.span()
                        },
                    }
                    .into());
                }
            };

            let args = SpreadKeySpec::parse(&k_info)?;
            let SpreadKeySpec::Prefix(indexes, from) = args;
            let idx_len = indexes.ints.len();
            let idents_len = from.bytes.len();
            if idents_len > idx_len {
                return Err(SpreadError::ParentMustBeOfLesserLen {
                    idents: from.clone(),
                    less_than_len: idx_len,
                }
                .into());
            }

            if idents_len == idx_len {
                return Err(SpreadError::ParentNotExtendIdent {
                    idents: from.clone(),
                    less_than_len: idx_len,
                }
                .into());
            }
            return Ok(Self {
                node: PhantomData,
                indexes,
                from,
            });
        }
        return Err(NodeBugs::NodeAttrNotFound {}.into());
    }

    fn validate_extract(&self, _: &A, _: Option<&()>) -> Result<(), proc_macro2::TokenStream> {
        Ok(())
    }
}

impl<'a, Node: NodeOf> GetCounterGenericIndexes<'a> for SpreadArgs<Node> {
    fn get(&'a self) -> Result<&'a IntList, TokenStream> {
        let indexes = &self.indexes;
        indexes.duplicate_check(Some(NodeSpace::DuplicateIndexes {}.into()))?;
        Ok(indexes)
    }
}

// ===============================================================================
// ``````````````````````````` TRAVERSE NODE KEY-ARGS ````````````````````````````
// ===============================================================================

use traverse_key::TraverseKeySpec;
mod traverse_key {
    use super::*;
    key_schema! {
        /// Key specification for the `traverse` node selector.
        ///
        /// Defines the accepted keys and valid argument combinations for a traverse
        /// selection.
        pub(crate) TraverseKeySpec {
            key: "traverse",

            allow_empty: false,

            sub_keys: {
                index : IntList,
                from : BStringList,
                until: BStringList,
            },

            combinations: {
                Range(index, from, until)
            }
        }
    }
}

/// Arguments for a `traverse` node selection.
///
/// Represents a selection from an exact leaf to the end of a branch, together
/// with the associated instance counter generic indexes.
///
/// Example:
/// ```text
/// indexes = [0, 1, 2]
///
/// from  = ["Account", "Free", "Locked"]
/// until = ["Account", "Free"]
///
/// Tree
/// Root
/// |-- Account
///     |-- Free          <- branch (until / end)
///     |   |-- Locked    <- leaf (from/ start)
///     |   |-- Frozen
///     |-- Reserved
///
/// Selection
/// Account --> Free --> * --> Locked
/// ```
///
/// The `from` path identifies the inclusive leaf where the selection begins,
/// while `until` identifies the branch whose subtree marks the end of the
/// selection. `indexes` specifies the associated instance counter generic
/// indexes.
#[derive(Debug, Clone, Default)]
pub(crate) struct TraverseArgs<Node: NodeOf> {
    /// Marker for the associated node attribute type.
    node: PhantomData<Node>,

    /// Instance counter generic indexes associated with the selection.
    pub(crate) indexes: IntList,

    /// Exact leaf where the selection begins.
    pub(crate) from: BStringList,

    /// Branch where the selection ends.
    pub(crate) until: BStringList,
}

impl<A: ExtractAttr, Node: NodeOf + Clone + Debug + 'static> Extraction<A> for TraverseArgs<Node> {
    fn raw_extract(from: &A, _: &()) -> Result<Self, proc_macro2::TokenStream> {
        for attr in from.get() {
            let AttrStyle::Outer = attr.style else {
                continue;
            };
            let Meta::List(list) = &attr.meta else {
                continue;
            };

            if !list.path.is_ident(Node::IDENT) {
                continue;
            }

            let k_info = match parse::<KeyInfo>(list.tokens.clone().into()) {
                Ok(v) => v,
                Err(_) => {
                    return Err(KeysError::ExpectedTraverse {
                        span: if list.tokens.is_empty() {
                            list.path.span()
                        } else {
                            list.tokens.span()
                        },
                    }
                    .into());
                }
            };

            let args = TraverseKeySpec::parse(&k_info)?;
            let TraverseKeySpec::Range(indexes, from, until) = args;
            let idx_len = indexes.ints.len();
            let from_len = from.bytes.len();
            let until_len = until.bytes.len();
            if from_len != idx_len {
                return Err(SubscriberError::IndexIdentsLenMismatch {
                    idents: from.clone(),
                    exp_len: idx_len,
                }
                .into());
            }
            if until_len > idx_len {
                return Err(TraverseError::ParentMustBeOfLesserLen {
                    idents: until.clone(),
                    less_than_len: idx_len,
                }
                .into());
            }

            if until_len == idx_len {
                return Err(TraverseError::ParentNotDescendIdent {
                    idents: until.clone(),
                    less_than_len: idx_len,
                }
                .into());
            }

            return Ok(Self {
                node: PhantomData,
                indexes,
                from,
                until,
            });
        }
        return Err(NodeBugs::NodeAttrNotFound {}.into());
    }

    fn validate_extract(&self, _: &A, _: Option<&()>) -> Result<(), proc_macro2::TokenStream> {
        Ok(())
    }
}

impl<'a, Node: NodeOf> GetCounterGenericIndexes<'a> for TraverseArgs<Node> {
    fn get(&'a self) -> Result<&'a IntList, TokenStream> {
        let indexes = &self.indexes;
        indexes.duplicate_check(Some(NodeSpace::DuplicateIndexes {}.into()))?;
        Ok(indexes)
    }
}

// ===============================================================================
// ```````````````````````````` DESCEND NODE KEY-ARGS ````````````````````````````
// ===============================================================================

use descend_key::DescendKeySpec;
mod descend_key {
    use super::*;
    key_schema! {
        /// Key specification for the `descend` node selector.
        ///
        /// Defines the accepted keys and valid argument combinations for a descend
        /// selection.
        pub(crate) DescendKeySpec {
            key: "descend",

            allow_empty: false,

            sub_keys: {
                index : IntList,
                from : BStringList,
                until: BStringList,
            },

            combinations: {
                Range(index, from, until)
            }
        }
    }
}

/// Arguments for a `descend` node selection.
///
/// Represents a selection from the beginning of a branch to an exact leaf,
/// together with the associated instance counter generic indexes.
///
/// Example:
/// ```text
/// indexes = [0, 1, 2]
///
/// from  = ["Account", "Free"]
/// until = ["Account", "Free", "Locked"]
///
/// Tree
/// Root
/// |-- Account
///     |-- Free          <- branch (from/ start)
///     |   |-- Locked    <- leaf (until / end)
///     |   |-- Frozen
///     |-- Reserved
///
/// Selection
/// Account --> Free --> * --->  Locked
/// ```
///
/// The `from` path identifies the branch where the selection begins, while
/// `until` identifies the inclusive leaf where the selection ends. `indexes`
/// specifies the associated instance counter generic indexes.
#[derive(Debug, Clone, Default)]
pub(crate) struct DescendArgs<Node: NodeOf> {
    /// Marker for the associated node attribute type.
    node: PhantomData<Node>,

    /// Instance counter generic indexes associated with the selection.
    pub(crate) indexes: IntList,

    /// Branch where the selection begins.
    pub(crate) from: BStringList,

    /// Exact leaf where the selection ends.
    pub(crate) until: BStringList,
}

impl<A: ExtractAttr, Node: NodeOf + Clone + Debug + 'static> Extraction<A> for DescendArgs<Node> {
    fn raw_extract(from: &A, _: &()) -> Result<Self, proc_macro2::TokenStream> {
        for attr in from.get() {
            let AttrStyle::Outer = attr.style else {
                continue;
            };
            let Meta::List(list) = &attr.meta else {
                continue;
            };

            if !list.path.is_ident(Node::IDENT) {
                continue;
            }

            let k_info = match parse::<KeyInfo>(list.tokens.clone().into()) {
                Ok(v) => v,
                Err(_) => {
                    return Err(KeysError::ExpectedDescend {
                        span: if list.tokens.is_empty() {
                            list.path.span()
                        } else {
                            list.tokens.span()
                        },
                    }
                    .into());
                }
            };

            let args = DescendKeySpec::parse(&k_info)?;
            let DescendKeySpec::Range(indexes, from, until) = args;
            let idx_len = indexes.ints.len();
            let from_len = from.bytes.len();
            let until_len = until.bytes.len();

            if from_len > idx_len {
                return Err(DescendError::ParentMustBeOfLesserLen {
                    idents: from.clone(),
                    less_than_len: idx_len,
                }
                .into());
            }

            if from_len == idx_len {
                return Err(DescendError::ParentNotTraverseIdent {
                    idents: from.clone(),
                    less_than_len: idx_len,
                }
                .into());
            }

            if until_len != idx_len {
                return Err(SubscriberError::IndexIdentsLenMismatch {
                    idents: until.clone(),
                    exp_len: idx_len,
                }
                .into());
            }

            return Ok(Self {
                node: PhantomData,
                indexes,
                from,
                until,
            });
        }
        return Err(NodeBugs::NodeAttrNotFound {}.into());
    }

    fn validate_extract(&self, _: &A, _: Option<&()>) -> Result<(), proc_macro2::TokenStream> {
        Ok(())
    }
}

impl<'a, Node: NodeOf> GetCounterGenericIndexes<'a> for DescendArgs<Node> {
    fn get(&'a self) -> Result<&'a IntList, TokenStream> {
        let indexes = &self.indexes;
        indexes.duplicate_check(Some(NodeSpace::DuplicateIndexes {}.into()))?;
        Ok(indexes)
    }
}

// ===============================================================================
// ````````````````````````````` ROOT NODE KEY-ARGS ``````````````````````````````
// ===============================================================================

use root_key::RootKeySpec;
mod root_key {
    use super::*;
    key_schema! {
        /// Key specification for the `root` node selector.
        ///
        /// Defines the accepted keys and valid argument combinations for a root
        /// selection.
        pub(crate) RootKeySpec {
            key: "root",

            allow_empty: false,

            sub_keys: {
                index : IntList,
            },

            combinations: {
                Full(index)
            }
        }
    }
}

/// Arguments for a `root` node selection.
///
/// Represents a selection of the entire hierarchy together with the associated
/// instance counter generic indexes.
///
/// Example:
/// ```text
/// indexes = [0, 1]
///
/// Tree
/// Root                <- start
/// |-- Account
/// |   |-- Free
/// |   |-- Reserved
/// |-- Treasury
///     |-- Pot         <- end
///
/// Selection
/// Root --->  End
/// ```
///
/// `indexes` specifies the instance counter generic indexes associated with
/// the complete hierarchy.
#[derive(Debug, Clone, Default)]
pub(crate) struct RootArgs<Node: NodeOf> {
    /// Marker for the associated node attribute type.
    node: PhantomData<Node>,

    /// Instance counter generic indexes associated with the selection.
    pub(crate) indexes: IntList,
}

impl<A: ExtractAttr, Node: NodeOf + Clone + Debug + 'static> Extraction<A> for RootArgs<Node> {
    fn raw_extract(from: &A, _: &()) -> Result<Self, proc_macro2::TokenStream> {
        for attr in from.get() {
            let AttrStyle::Outer = attr.style else {
                continue;
            };
            let Meta::List(list) = &attr.meta else {
                continue;
            };

            if !list.path.is_ident(Node::IDENT) {
                continue;
            }

            let k_info = match parse::<KeyInfo>(list.tokens.clone().into()) {
                Ok(v) => v,
                Err(_) => {
                    return Err(KeysError::ExpectedRoot {
                        span: if list.tokens.is_empty() {
                            list.path.span()
                        } else {
                            list.tokens.span()
                        },
                    }
                    .into());
                }
            };

            let args = RootKeySpec::parse(&k_info)?;
            let RootKeySpec::Full(indexes) = args;
            return Ok(Self {
                node: PhantomData,
                indexes,
            });
        }
        return Err(NodeBugs::NodeAttrNotFound {}.into());
    }

    fn validate_extract(&self, _: &A, _: Option<&()>) -> Result<(), proc_macro2::TokenStream> {
        Ok(())
    }
}

impl<'a, Node: NodeOf> GetCounterGenericIndexes<'a> for RootArgs<Node> {
    fn get(&'a self) -> Result<&'a IntList, TokenStream> {
        let indexes = &self.indexes;
        indexes.duplicate_check(Some(NodeSpace::DuplicateIndexes {}.into()))?;
        Ok(indexes)
    }
}

// ===============================================================================
// ````````````````````````````` TOKEN NODE KEY-ARGS `````````````````````````````
// ===============================================================================

/// Raw arguments for an unrecognized node selector.
///
/// Preserves the original token stream when a node selector is not recognized,
/// allowing it to be reported or processed later without losing the user's
/// input.
#[derive(Debug, Clone, Default)]
pub(crate) struct TokenNodeArgs<Node: NodeOf> {
    /// Marker for the associated node attribute type.
    node: PhantomData<Node>,

    /// Raw tokens associated with the unrecognized node selector.
    pub(crate) tokens: TokenStream,
}

impl<Node: NodeOf> From<TokenStream> for TokenNodeArgs<Node> {
    fn from(value: TokenStream) -> Self {
        Self {
            node: PhantomData,
            tokens: value,
        }
    }
}

impl<A: ExtractAttr, Node: NodeOf + Clone + Debug + 'static> Extraction<A> for TokenNodeArgs<Node> {
    fn raw_extract(from: &A, _: &()) -> Result<Self, proc_macro2::TokenStream> {
        for attr in from.get() {
            let AttrStyle::Outer = attr.style else {
                continue;
            };
            let path = match &attr.meta {
                Meta::Path(path) => path,
                Meta::List(meta_list) => &meta_list.path,
                Meta::NameValue(_) => continue,
            };

            if !path.is_ident(Node::IDENT) {
                continue;
            }

            let tokens = match &attr.meta {
                Meta::Path(_) => &TokenStream::new(),
                Meta::List(meta_list) => &meta_list.tokens,
                Meta::NameValue(_) => continue,
            };

            if !tokens.is_empty() {
                return Ok(Self {
                    node: PhantomData,
                    tokens: tokens.clone(),
                });
            } else {
                return Err(KeysError::ExpectedKeys { span: path.span() }.into());
            }
        }
        return Err(NodeBugs::NodeAttrNotFound {}.into());
    }

    fn validate_extract(&self, _: &A, _: Option<&()>) -> Result<(), proc_macro2::TokenStream> {
        Ok(())
    }
}
