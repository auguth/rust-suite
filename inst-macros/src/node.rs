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
// ```````````````````````````````````` NODE ARGS ````````````````````````````````
// ===============================================================================

//! Provides the validation and lowering layer for instance models.
//!
//! [`InstanceModel`] contains the parsed instance-parameter structure and
//! provides common normalization and validation operations used by both
//! instance access and instance node lowering.
//!
//! [`AccessArgsRef`] validates an instance model against access-specific rules
//! and lowers it into [`AccessArgs`] representations such as leaf, dynamic,
//! and prefix access.
//!
//! [`NodeArgsRef`] validates an instance model against node-specific rules
//! and lowers it into [`NodeArgs`] representations such as leaf, branch,
//! root, and range-based nodes.
//!
//! The access and node representations share the same [`InstanceModel`] but
//! apply different constraints: access models use runtime parameters and
//! disallow ranges and inference, while node models use compile-time
//! parameters and inference to describe instance traversal and ranges.
//!
//! The lowering implementations select the concrete representation by
//! matching the structure of the validated instance model against the
//! supported access or node forms.

// ===============================================================================
// ``````````````````````````````````` IMPORTS ```````````````````````````````````
// ===============================================================================

// --- Proc Macro Crates ---
use proc_macro2::{Span, TokenStream};
use syn::{LitByteStr, LitInt, spanned::Spanned};

// --- Local Crate ---
use crate::{
    args::{
        AccessArgs, BStrInput, BStrInputList, BranchNode, ComplexInstanceRange, DescendNode,
        DynAccess, ExtendNode, InstMixedItem, InstMixedList, InstanceIdent, InstanceIdents,
        InstanceModel, LeafAccess, LeafNode, NodeArgs, PrefixAccess, PruneNode, RootNode,
        SpreadNode, TraverseNode, TrimNode,
    },
    errors::{ParseBug, ProcParseErr},
};

// --- Proc Suite ---
use proc_suite::{BStringList, IntList};

// ===============================================================================
// ```````````````````````````````` INSTANCE MODEL ```````````````````````````````
// ===============================================================================

impl InstanceModel {
    /// Returns `true` if this instance model is a simple instance parameter group.
    pub fn is_simple(&self) -> bool {
        matches!(self, InstanceModel::Simple(_))
    }

    /// Returns `true` if this instance model contains terminated instance groups.
    pub fn is_terminated(&self) -> bool {
        matches!(
            self,
            InstanceModel::Complex(ComplexInstanceRange::Terminated(_))
        )
    }

    /// Returns `true` if this instance model contains an instance range.
    pub fn is_range(&self) -> bool {
        matches!(self, InstanceModel::Complex(ComplexInstanceRange::Range(_)))
    }

    /// Applies `params` to every instance parameter group in the model.
    ///
    /// A simple model contains one group, a terminated model contains each
    /// terminated group, and a range model contains its start and end groups
    /// when present.
    pub fn get_params<F: FnMut(&InstanceIdents) -> Result<(), TokenStream>>(
        &self,
        mut params: F,
    ) -> Result<(), TokenStream> {
        match &self {
            InstanceModel::Simple(simple) => params(simple)?,
            InstanceModel::Complex(complex) => match complex {
                ComplexInstanceRange::Terminated(term) => {
                    for p in &term.params {
                        params(p)?;
                    }
                }
                ComplexInstanceRange::Range(range) => {
                    if let Some(p) = &range.start {
                        params(p)?;
                    }
                    if let Some(p) = &range.end {
                        params(p)?;
                    }
                }
            },
        }
        Ok(())
    }

    /// Validates that every instance parameter group contains at most four
    /// parameters.
    pub fn max_4_params(&self) -> Result<(), TokenStream> {
        let max = |param: &InstanceIdents| -> Result<(), TokenStream> {
            if param.params.len() > 4 {
                return Err(ProcParseErr::Max4NodeArgs {
                    params: param.clone(),
                }
                .into());
            }
            Ok(())
        };

        self.get_params(|params| max(params))?;
        Ok(())
    }

    /// Converts a complex instance model into a simple model when its
    /// structure represents a single instance parameter group.
    ///
    /// A terminated model can be simplified when it contains exactly one
    /// group. A range can be simplified when both endpoints are present and
    /// every corresponding parameter is identical.
    pub fn can_be_simple(&mut self) {
        let InstanceModel::Complex(complex) = &self else {
            return;
        };
        match complex {
            ComplexInstanceRange::Terminated(term) => {
                if term.params.len() != 1 {
                    return;
                }

                let params = term.params[0].params.clone();

                *self = InstanceModel::Simple(InstanceIdents {
                    span: params.span(),
                    paren_token: Default::default(),
                    params,
                })
            }
            ComplexInstanceRange::Range(range) => {
                if range.start.is_none() || range.end.is_none() {
                    return;
                }
                if range
                    .start
                    .as_ref()
                    .unwrap()
                    .params
                    .iter()
                    .zip(range.end.as_ref().unwrap().params.iter())
                    .all(|(start, end)| match (start, end) {
                        (InstanceIdent::Compile(ident0), InstanceIdent::Compile(ident1)) => {
                            ident0 == ident1
                        }
                        (InstanceIdent::Runtime(param0), InstanceIdent::Runtime(param1)) => {
                            param0.ident == param1.ident
                        }
                        (InstanceIdent::Infer(_), InstanceIdent::Infer(_)) => true,
                        _ => false,
                    })
                {
                    *self = InstanceModel::Simple(range.start.clone().unwrap().clone())
                }
            }
        };
    }

    /// Validates that every instance parameter group contains at least one
    /// parameter.
    pub fn atleast_one(&self) -> Result<(), TokenStream> {
        let check = |param: &InstanceIdents| -> Result<(), TokenStream> {
            if param.params.is_empty() {
                return Err(ProcParseErr::RequireNodeArgs {
                    params: param.clone(),
                }
                .into());
            }
            Ok(())
        };

        self.get_params(|params| check(params))?;
        Ok(())
    }

    /// Rejects ranges with neither a start nor an end parameter group.
    ///
    /// An unbounded range with no endpoints is represented by a single
    /// inferred parameter group instead.
    pub fn no_range_roots(&self) -> Result<(), TokenStream> {
        if !self.is_range() {
            return Ok(());
        }

        let InstanceModel::Complex(ComplexInstanceRange::Range(range)) = &self else {
            unreachable!()
        };
        if range.start.is_none() && range.end.is_none() {
            return Err(ProcParseErr::RangeMustBeFullInfer {
                dotdot: range.dotdot_token.clone(),
            }
            .into());
        }
        Ok(())
    }

    /// Validates that a terminated instance model contains only compile-time
    /// identifiers.
    ///
    /// This validation applies only to terminated models; range models may
    /// contain other instance parameter forms.
    pub fn pure_terminated(&self) -> Result<(), TokenStream> {
        let pure = |param: &InstanceIdents| -> Result<(), TokenStream> {
            let mut found = false;
            for p in &param.params {
                if !matches!(p, InstanceIdent::Compile(_)) {
                    found = true;
                }
            }
            if found {
                return Err(ProcParseErr::PureTerminatedElseRange {
                    params: param.clone(),
                }
                .into());
            }
            Ok(())
        };

        if self.is_terminated() {
            self.get_params(|params| pure(params))?;
        }
        Ok(())
    }

    /// Validates the lengths of instance parameter groups.
    ///
    /// Each subsequent group must not contain more parameters than the
    /// preceding group.
    pub fn same_length_elems(&self) -> Result<(), TokenStream> {
        let mut len = None;
        let mut cap = |param: &InstanceIdents| -> Result<(), TokenStream> {
            let local_len = param.params.len();
            if let Some(len) = len {
                if local_len > len {
                    return Err(ProcParseErr::MustHaveSameParamsLen {
                        params: param.clone(),
                        exp: len,
                        found: local_len,
                    }
                    .into());
                }
            }
            len = Some(local_len);
            Ok(())
        };

        self.get_params(|params| cap(params))?;
        Ok(())
    }

    /// Validates that every instance parameter in the model is a compile-time
    /// identifier.
    fn only_compile(&self) -> Result<(), TokenStream> {
        self.get_params(|params| Self::only_compile_params(params))
    }

    /// Validates that every parameter in an instance parameter group is a
    /// compile-time identifier.
    fn only_compile_params(param: &InstanceIdents) -> Result<(), TokenStream> {
        for p in &param.params {
            if !matches!(p, InstanceIdent::Compile(_)) {
                return Err(ProcParseErr::FullConcreteParamsExpected { params: p.clone() }.into());
            }
        }
        Ok(())
    }
}

// ===============================================================================
// ````````````````````````````` INSTANCE ACCESS ARGS ````````````````````````````
// ===============================================================================

/// References the instance model used to construct an [`AccessArgs`]
/// representation.
///
/// The referenced [`InstanceModel`] is validated against the access depth and
/// index information before being lowered into a concrete access form.
pub struct AccessArgsRef(pub InstanceModel);

impl AccessArgsRef {
    /// Validates the instance model against the supplied depth and index
    /// information.
    ///
    /// The validation enforces access-specific restrictions such as runtime
    /// parameter placement, disallowed inference and ranges, matching
    /// parameter lengths, and the maximum parameter count.
    fn validate(&mut self, depth: &InstMixedList, indexes: &IntList) -> Result<(), TokenStream> {
        let mut indexed_found = false;
        for item in &depth.items {
            match item {
                InstMixedItem::Indexed(_) => {
                    indexed_found = true;
                }
                InstMixedItem::Ident(ident) if indexed_found => {
                    return Err(ProcParseErr::PlainIdentAfterIndexed {
                        ident: ident.clone(),
                    }
                    .into());
                }
                _ => {}
            }
        }

        self.force_runtime()?;
        self.0.can_be_simple();
        self.0.atleast_one()?;
        self.0.no_range_roots()?;
        self.0.pure_terminated()?;
        self.no_infers()?;
        self.no_ranges()?;
        self.capped_elems(depth, indexes)?;
        self.0.same_length_elems()?;
        self.runtime_of_depth(depth)?;
        self.0.max_4_params()?;
        Ok(())
    }

    /// Rejects range-based instance models, which are only valid for nodes.
    fn no_ranges(&self) -> Result<(), TokenStream> {
        if self.0.is_range() {
            return Err(ProcParseErr::RangeNotInAccess {
                model: self.0.clone(),
            }
            .into());
        }
        Ok(())
    }

    /// Rejects inferred instance parameters from access arguments.
    ///
    /// Dynamic instances must be expressed explicitly as runtime parameters
    /// instead of using `_` inference.
    fn no_infers(&self) -> Result<(), TokenStream> {
        let check = |param: &InstanceIdents| -> Result<(), TokenStream> {
            for p in &param.params {
                if let InstanceIdent::Infer(infer) = p {
                    return Err(ProcParseErr::InferNotInAccess {
                        infer: infer.clone(),
                    }
                    .into());
                }
            }
            Ok(())
        };

        self.0.get_params(|params| check(params))?;
        Ok(())
    }

    /// Validates the ordering of runtime and compile-time instance parameters.
    ///
    /// Once a runtime parameter is encountered, subsequent parameters must not
    /// introduce compile-time child identifiers.
    fn force_runtime(&self) -> Result<(), TokenStream> {
        let force = |param: &InstanceIdents| -> Result<(), TokenStream> {
            let mut found = false;
            for p in &param.params {
                match p {
                    InstanceIdent::Compile(ident) => {
                        if found {
                            return Err(ProcParseErr::NoCompileIdentAfterRuntime {
                                ident: ident.clone(),
                            }
                            .into());
                        } else {
                            continue;
                        }
                    }
                    InstanceIdent::Runtime(_) => found = true,
                    _ => {
                        continue;
                    }
                }
            }
            Ok(())
        };

        self.0.get_params(|params| force(params))?;
        Ok(())
    }

    /// Validates that every instance parameter group has the same length as
    /// the supplied depth and index lists.
    fn capped_elems(&self, depth: &InstMixedList, indexes: &IntList) -> Result<(), TokenStream> {
        let depth_len = depth.items.len();
        let depth_cap = |param: &InstanceIdents| -> Result<(), TokenStream> {
            let len = param.params.len();
            if len != depth_len {
                return Err(ProcParseErr::DepthLenParamsRequired {
                    params: param.clone(),
                    exp: depth_len,
                    found: len,
                }
                .into());
            }
            Ok(())
        };
        self.0.get_params(|params| depth_cap(params))?;

        let indexes_len = indexes.ints.len();
        let indexes_cap = |param: &InstanceIdents| -> Result<(), TokenStream> {
            let len = param.params.len();
            if len != indexes_len {
                return Err(ParseBug::IndexesLengthInconsistent {}.into());
            }
            Ok(())
        };
        self.0.get_params(|params| indexes_cap(params))?;

        Ok(())
    }

    /// Validates that every runtime instance parameter has a corresponding
    /// indexed depth declaration.
    ///
    /// A runtime identifier declared only by name must also have its depth
    /// index explicitly specified.
    fn runtime_of_depth(&self, depth: &InstMixedList) -> Result<(), TokenStream> {
        let force = |param: &InstanceIdents| -> Result<(), TokenStream> {
            for p in &param.params {
                if let InstanceIdent::Runtime(param) = p {
                    let dyn_ident = &param.ident;
                    if !depth.items.iter().any(|item| {
                        let InstMixedItem::Indexed(indexed) = item else {
                            return false;
                        };
                        indexed.ident == *dyn_ident
                    }) {
                        if let Some(ident) = depth.items.iter().find_map(|item| {
                            if let InstMixedItem::Ident(ident) = item {
                                if ident == dyn_ident {
                                    return Some(ident);
                                }
                            };
                            None
                        }) {
                            return Err(ProcParseErr::RequiresIndexed {
                                ident: ident.clone(),
                            }
                            .into());
                        }
                        return Err(ProcParseErr::RequiresDepthArgument {
                            ident: dyn_ident.clone(),
                        }
                        .into());
                    }
                }
            }
            Ok(())
        };

        self.0.get_params(|params| force(params))?;
        Ok(())
    }

    /// Validates the model and selects the access representation capable of
    /// serving it.
    ///
    /// Access representations are attempted in dynamic, prefix, and leaf
    /// forms. The first applicable representation is returned.
    pub fn try_access(
        &mut self,
        depth: &InstMixedList,
        indexes: &IntList,
    ) -> Result<AccessArgs, TokenStream> {
        self.validate(depth, indexes)?;

        if let Some(node) = self.try_dyn(depth, indexes) {
            return Ok(node);
        }

        if let Some(node) = self.try_prefix(depth, indexes) {
            return Ok(node);
        }

        if let Some(node) = self.try_leaf(indexes) {
            return Ok(node);
        }

        return Err(ProcParseErr::UnsupportedAccessArgs {
            model: self.0.clone(),
        }
        .into());
    }

    /// Attempts to lower the instance model into a leaf access.
    ///
    /// A leaf access requires a simple instance model containing only
    /// compile-time identifiers.
    pub fn try_leaf(&self, indexes: &IntList) -> Option<AccessArgs> {
        if !self.0.is_simple() {
            return None;
        }

        if self.0.only_compile().is_ok() {
            let mut collect = BStringList::default();
            let InstanceModel::Simple(simple) = &self.0 else {
                unreachable!()
            };
            for p in &simple.params {
                let InstanceIdent::Compile(ident) = p else {
                    unreachable!()
                };
                let ident_str = ident.to_string();
                let ident_bytes = ident_str.as_bytes();
                collect
                    .bytes
                    .push(LitByteStr::new(ident_bytes, ident.span()));
            }

            return Some(AccessArgs::Leaf(LeafAccess {
                indexes: indexes.clone(),
                idents: collect,
            }));
        }

        return None;
    }

    /// Attempts to lower the instance model into a dynamic root access.
    ///
    /// A dynamic access requires a simple instance model whose parameters are
    /// runtime values. The corresponding depth indexes are resolved from the
    /// supplied depth declarations.
    fn try_dyn(&self, depth: &InstMixedList, indexes: &IntList) -> Option<AccessArgs> {
        if !self.0.is_simple() {
            return None;
        }

        let all_runtimes = |param: &InstanceIdents| -> Result<(), TokenStream> {
            for param in &param.params {
                if !matches!(param, InstanceIdent::Runtime(_)) {
                    return Err(TokenStream::new());
                }
            }
            Ok(())
        };

        if self.0.get_params(|params| all_runtimes(params)).is_err() {
            return None;
        }

        let mut idents = BStrInputList::default();
        let InstanceModel::Simple(simple) = &self.0 else {
            unreachable!()
        };
        for p in &simple.params {
            match p {
                InstanceIdent::Runtime(dynamic_param) => {
                    idents
                        .exprs
                        .push(BStrInput::Ident(dynamic_param.ident.clone()));
                }
                _ => {}
            }
        }

        let mut depths = IntList::default();
        for dynamic in &idents.exprs {
            let BStrInput::Ident(ident) = dynamic else {
                unreachable!()
            };

            let depth = depth.items.iter().find_map(|depth| {
                let InstMixedItem::Indexed(indexed) = depth else {
                    return None;
                };

                if indexed.ident != *ident {
                    return None;
                };

                Some(indexed.index.clone())
            });

            let Some(depth) = depth else {
                unreachable!("validation for access args ref is not done")
            };

            depths
                .ints
                .push(LitInt::new(&depth.to_string(), Span::call_site().into()))
        }

        return Some(AccessArgs::Root(DynAccess {
            indexes: indexes.clone(),
            idents,
            depths: Some(depths),
        }));
    }

    /// Attempts to lower the instance model into a prefix access.
    ///
    /// A prefix access separates compile-time parent identifiers from the
    /// runtime child parameters. The depth indexes of the runtime children are
    /// resolved from the supplied depth declarations.
    fn try_prefix(&self, depth: &InstMixedList, indexes: &IntList) -> Option<AccessArgs> {
        if !self.0.is_simple() {
            return None;
        }

        let last_runtime = |param: &InstanceIdents| -> Result<(), TokenStream> {
            let Some(last) = param.params.last() else {
                unreachable!("validation for node args is not done")
            };
            if !matches!(last, InstanceIdent::Runtime(_)) {
                return Err(TokenStream::new());
            }
            Ok(())
        };

        if self.0.get_params(|params| last_runtime(params)).is_err() {
            return None;
        }

        let mut parents = BStringList::default();
        let mut childs = BStrInputList::default();
        let InstanceModel::Simple(simple) = &self.0 else {
            unreachable!()
        };
        for p in &simple.params {
            match p {
                InstanceIdent::Compile(ident) => {
                    let ident_str = ident.to_string();
                    let ident_bytes = ident_str.as_bytes();
                    parents
                        .bytes
                        .push(LitByteStr::new(ident_bytes, ident.span()));
                }
                InstanceIdent::Runtime(dynamic_param) => {
                    childs
                        .exprs
                        .push(BStrInput::Ident(dynamic_param.ident.clone()));
                }
                _ => {}
            }
        }

        let mut depths = IntList::default();
        for child in &childs.exprs {
            let BStrInput::Ident(ident) = child else {
                unreachable!()
            };

            let depth = depth.items.iter().find_map(|depth| {
                let InstMixedItem::Indexed(indexed) = depth else {
                    return None;
                };

                if indexed.ident != *ident {
                    return None;
                };

                Some(indexed.index.clone())
            });

            let Some(depth) = depth else {
                unreachable!("validation for access args ref is not done")
            };

            depths
                .ints
                .push(LitInt::new(&depth.to_string(), Span::call_site().into()))
        }

        return Some(AccessArgs::Branch(PrefixAccess {
            indexes: indexes.clone(),
            parents,
            childs,
            depths: Some(depths),
        }));
    }
}

// ===============================================================================
// `````````````````````````````` INSTANCE NODE ARGS `````````````````````````````
// ===============================================================================

/// References an instance model while lowering it into [`NodeArgs`].
///
/// The referenced model is validated against the supplied instance indexes
/// before being matched to a concrete node representation.
pub struct NodeArgsRef(pub InstanceModel);

impl NodeArgsRef {
    /// Validates the instance model against the supplied instance indexes.
    ///
    /// Normalizes models that can be represented simply, then enforces the
    /// node-specific restrictions on inference, runtime parameters, ranges,
    /// parameter lengths, and maximum parameter count.
    fn validate(&mut self, indexes: &IntList) -> Result<(), TokenStream> {
        self.force_infers()?;
        self.0.can_be_simple();
        self.0.atleast_one()?;
        self.0.no_range_roots()?;
        self.0.pure_terminated()?;
        self.no_runtime()?;
        self.indexes_capped_elems(indexes)?;
        self.0.same_length_elems()?;
        self.0.max_4_params()?;
        Ok(())
    }

    /// Rejects runtime instance parameters from node arguments.
    ///
    /// Node parameters must be compile-time identifiers or inferred
    /// parameters.
    fn no_runtime(&self) -> Result<(), TokenStream> {
        let check = |param: &InstanceIdents| -> Result<(), TokenStream> {
            for p in &param.params {
                if let InstanceIdent::Runtime(runtime) = p {
                    return Err(ProcParseErr::RuntimeNotInNode {
                        runtime: runtime.clone(),
                    }
                    .into());
                }
            }
            Ok(())
        };

        self.0.get_params(|params| check(params))?;
        Ok(())
    }

    /// Ensures that an inferred parameter terminates its parameter group.
    ///
    /// Once `_` is encountered, no compile-time identifier may follow it.
    fn force_infers(&self) -> Result<(), TokenStream> {
        let force = |param: &InstanceIdents| -> Result<(), TokenStream> {
            let mut found = false;
            for p in &param.params {
                match p {
                    InstanceIdent::Compile(ident) => {
                        if found {
                            return Err(ProcParseErr::NoCompileIdentAfterInfer {
                                ident: ident.clone(),
                            }
                            .into());
                        } else {
                            continue;
                        }
                    }
                    InstanceIdent::Infer(_) => found = true,
                    _ => {
                        continue;
                    }
                }
            }
            Ok(())
        };

        self.0.get_params(|params| force(params))?;
        Ok(())
    }

    /// Validates that each instance parameter group fits within the supplied
    /// instance index list.
    fn indexes_capped_elems(&self, indexes: &IntList) -> Result<(), TokenStream> {
        let cap = |param: &InstanceIdents| -> Result<(), TokenStream> {
            let len = param.params.len();
            if len > indexes.ints.len() {
                return Err(ParseBug::IndexesLengthInconsistent {}.into());
            }
            Ok(())
        };

        self.0.get_params(|params| cap(params))?;
        Ok(())
    }

    /// Attempts to lower the instance model into a [`NodeArgs`] variant.
    ///
    /// Node representations are tested from the most specialized forms to
    /// the more general forms until one matches the instance model.
    pub fn try_node(&mut self, indexes: &IntList) -> Result<NodeArgs, TokenStream> {
        self.validate(indexes)?;

        if let Some(node) = self.try_leaves(indexes) {
            return Ok(node);
        }

        if let Some(node) = self.try_spread(indexes) {
            return Ok(node);
        }

        if let Some(node) = self.try_traverse(indexes) {
            return Ok(node);
        }

        if let Some(node) = self.try_extend(indexes) {
            return Ok(node);
        }

        if let Some(node) = self.try_trim(indexes) {
            return Ok(node);
        }

        if let Some(node) = self.try_descend(indexes) {
            return Ok(node);
        }

        if let Some(node) = self.try_prune(indexes) {
            return Ok(node);
        }

        if let Some(node) = self.try_root(indexes) {
            return Ok(node);
        }

        if let Some(node) = self.try_branch(indexes) {
            return Ok(node);
        }

        if let Some(node) = self.try_leaf(indexes) {
            return Ok(node);
        }

        Err(ProcParseErr::UnsupportedNodeArgs {
            model: self.0.clone(),
        }
        .into())
    }

    /// Attempts to lower a simple all-inferred model into a root node.
    fn try_root(&self, indexes: &IntList) -> Option<NodeArgs> {
        if !self.0.is_simple() {
            return None;
        }

        let only_term = |param: &InstanceIdents| -> Result<(), TokenStream> {
            for p in &param.params {
                if !matches!(p, InstanceIdent::Infer(_)) {
                    return Err(TokenStream::new());
                }
            }
            Ok(())
        };

        if self.0.get_params(|params| only_term(params)).is_ok() {
            return Some(NodeArgs::Root(RootNode {
                indexes: indexes.clone(),
            }));
        }

        return None;
    }

    /// Attempts to lower a simple all-compile-time model into a leaf node.
    pub fn try_leaf(&self, indexes: &IntList) -> Option<NodeArgs> {
        if !self.0.is_simple() {
            return None;
        }
        if self.0.only_compile().is_ok() {
            let mut collect = BStringList::default();
            let InstanceModel::Simple(simple) = &self.0 else {
                unreachable!()
            };
            for p in &simple.params {
                let InstanceIdent::Compile(ident) = p else {
                    unreachable!()
                };
                let ident_str = ident.to_string();
                let ident_bytes = ident_str.as_bytes();
                collect
                    .bytes
                    .push(LitByteStr::new(ident_bytes, ident.span()));
            }

            return Some(NodeArgs::Leaf(LeafNode {
                indexes: indexes.clone(),
                idents: vec![collect],
            }));
        }

        return None;
    }

    /// Attempts to lower a terminated all-compile-time model into multiple
    /// leaf identifiers.
    pub fn try_leaves(&self, indexes: &IntList) -> Option<NodeArgs> {
        if !self.0.is_terminated() {
            return None;
        }
        if self.0.only_compile().is_ok() {
            let mut leaves = Vec::new();
            let InstanceModel::Complex(ComplexInstanceRange::Terminated(term)) = &self.0 else {
                unreachable!()
            };
            for p in &term.params {
                let mut leaf = BStringList::default();
                for p in &p.params {
                    let InstanceIdent::Compile(ident) = p else {
                        unreachable!()
                    };
                    let ident_str = ident.to_string();
                    let ident_bytes = ident_str.as_bytes();
                    leaf.bytes.push(LitByteStr::new(ident_bytes, ident.span()));
                }
                leaves.push(leaf);
            }

            return Some(NodeArgs::Leaf(LeafNode {
                indexes: indexes.clone(),
                idents: leaves,
            }));
        }

        return None;
    }

    /// Attempts to lower a simple model ending in an inferred parameter into
    /// a branch node.
    fn try_branch(&self, indexes: &IntList) -> Option<NodeArgs> {
        if !self.0.is_simple() {
            return None;
        }
        let last_infer = |param: &InstanceIdents| -> Result<(), TokenStream> {
            let Some(last) = param.params.last() else {
                unreachable!("validation for node args is not done")
            };
            if !matches!(last, InstanceIdent::Infer(_)) {
                return Err(TokenStream::new());
            }
            Ok(())
        };

        if self.0.get_params(|params| last_infer(params)).is_ok() {
            let mut collect = BStringList::default();
            let InstanceModel::Simple(simple) = &self.0 else {
                unreachable!()
            };
            for p in &simple.params {
                let InstanceIdent::Compile(ident) = p else {
                    break;
                };
                let ident_str = ident.to_string();
                let ident_bytes = ident_str.as_bytes();
                collect
                    .bytes
                    .push(LitByteStr::new(ident_bytes, ident.span()));
            }

            return Some(NodeArgs::Branch(BranchNode {
                indexes: indexes.clone(),
                parents: collect,
            }));
        }

        return None;
    }

    /// Attempts to lower an open-ended range with a compile-time end into a
    /// prune node.
    fn try_prune(&self, indexes: &IntList) -> Option<NodeArgs> {
        if !self.0.is_range() {
            return None;
        }

        let InstanceModel::Complex(ComplexInstanceRange::Range(range)) = &self.0 else {
            unreachable!()
        };

        if range.start.is_some() {
            return None;
        };

        let end = range.end.as_ref();

        if !end.is_some_and(|end| {
            end.params
                .iter()
                .all(|param| matches!(param, InstanceIdent::Compile(_)))
        }) {
            return None;
        }

        let mut collect = BStringList::default();
        for p in end.unwrap().params.iter() {
            let InstanceIdent::Compile(ident) = p else {
                unreachable!()
            };
            let ident_str = ident.to_string();
            let ident_bytes = ident_str.as_bytes();
            collect
                .bytes
                .push(LitByteStr::new(ident_bytes, ident.span()));
        }

        return Some(NodeArgs::Prune(PruneNode {
            indexes: indexes.clone(),
            until: collect,
        }));
    }

    /// Attempts to lower a bounded range with compile-time endpoints into a
    /// descend node.
    fn try_descend(&self, indexes: &IntList) -> Option<NodeArgs> {
        if !self.0.is_range() {
            return None;
        }

        let InstanceModel::Complex(ComplexInstanceRange::Range(range)) = &self.0 else {
            unreachable!()
        };

        if range.end.is_none() || range.start.is_none() {
            return None;
        };

        let end = range.end.as_ref();

        if !end.is_some_and(|end| {
            end.params
                .iter()
                .all(|param| matches!(param, InstanceIdent::Compile(_)))
        }) {
            return None;
        }

        let mut until = BStringList::default();
        for p in end.unwrap().params.iter() {
            let InstanceIdent::Compile(ident) = p else {
                unreachable!()
            };
            let ident_str = ident.to_string();
            let ident_bytes = ident_str.as_bytes();
            until.bytes.push(LitByteStr::new(ident_bytes, ident.span()));
        }

        let start = range.start.as_ref();

        let mut from = BStringList::default();
        for p in start.unwrap().params.iter() {
            let InstanceIdent::Compile(ident) = p else {
                break;
            };
            let ident_str = ident.to_string();
            let ident_bytes = ident_str.as_bytes();
            from.bytes.push(LitByteStr::new(ident_bytes, ident.span()));
        }

        return Some(NodeArgs::Descend(DescendNode {
            indexes: indexes.clone(),
            from,
            until,
        }));
    }

    /// Attempts to lower an open-ended range ending in an inferred parameter
    /// into a trim node.
    fn try_trim(&self, indexes: &IntList) -> Option<NodeArgs> {
        if !self.0.is_range() {
            return None;
        }

        let InstanceModel::Complex(ComplexInstanceRange::Range(range)) = &self.0 else {
            unreachable!()
        };

        if range.start.is_some() {
            return None;
        };

        let end = range.end.as_ref();

        if !end.is_some_and(|end| {
            end.params
                .iter()
                .last()
                .is_some_and(|last| matches!(last, InstanceIdent::Infer(_)))
        }) {
            return None;
        }

        let mut collect = BStringList::default();
        for p in end.unwrap().params.iter() {
            let InstanceIdent::Compile(ident) = p else {
                break;
            };
            let ident_str = ident.to_string();
            let ident_bytes = ident_str.as_bytes();
            collect
                .bytes
                .push(LitByteStr::new(ident_bytes, ident.span()));
        }

        return Some(NodeArgs::Trim(TrimNode {
            indexes: indexes.clone(),
            until: collect,
        }));
    }

    /// Attempts to lower a range with only a compile-time start into an
    /// extend node.
    fn try_extend(&self, indexes: &IntList) -> Option<NodeArgs> {
        if !self.0.is_range() {
            return None;
        }

        let InstanceModel::Complex(ComplexInstanceRange::Range(range)) = &self.0 else {
            unreachable!()
        };

        if range.end.is_some() {
            return None;
        };

        let start = range.start.as_ref();

        if !start.is_some_and(|start| {
            start
                .params
                .iter()
                .all(|param| matches!(param, InstanceIdent::Compile(_)))
        }) {
            return None;
        }

        let mut collect = BStringList::default();
        for p in start.unwrap().params.iter() {
            let InstanceIdent::Compile(ident) = p else {
                unreachable!()
            };
            let ident_str = ident.to_string();
            let ident_bytes = ident_str.as_bytes();
            collect
                .bytes
                .push(LitByteStr::new(ident_bytes, ident.span()));
        }

        return Some(NodeArgs::Extend(ExtendNode {
            indexes: indexes.clone(),
            from: collect,
        }));
    }

    /// Attempts to lower a range with a compile-time prefix followed by an
    /// inferred parameter into a spread node.
    fn try_spread(&self, indexes: &IntList) -> Option<NodeArgs> {
        if !self.0.is_range() {
            return None;
        }

        let InstanceModel::Complex(ComplexInstanceRange::Range(range)) = &self.0 else {
            unreachable!()
        };

        if range.end.is_some() {
            return None;
        };

        let start = range.start.as_ref();

        if !start.is_some_and(|start| {
            start
                .params
                .iter()
                .last()
                .is_some_and(|last| matches!(last, InstanceIdent::Infer(_)))
        }) {
            return None;
        }

        let mut collect = BStringList::default();
        for p in start.unwrap().params.iter() {
            let InstanceIdent::Compile(ident) = p else {
                break;
            };
            let ident_str = ident.to_string();
            let ident_bytes = ident_str.as_bytes();
            collect
                .bytes
                .push(LitByteStr::new(ident_bytes, ident.span()));
        }

        return Some(NodeArgs::Spread(SpreadNode {
            indexes: indexes.clone(),
            from: collect,
        }));
    }

    /// Attempts to lower a bounded range with compile-time endpoints into a
    /// traverse node.
    fn try_traverse(&self, indexes: &IntList) -> Option<NodeArgs> {
        if !self.0.is_range() {
            return None;
        }

        let InstanceModel::Complex(ComplexInstanceRange::Range(range)) = &self.0 else {
            unreachable!()
        };

        if range.end.is_none() || range.start.is_none() {
            return None;
        };

        let end = range.end.as_ref();

        let mut until = BStringList::default();
        for p in end.unwrap().params.iter() {
            let InstanceIdent::Compile(ident) = p else {
                break;
            };
            let ident_str = ident.to_string();
            let ident_bytes = ident_str.as_bytes();
            until.bytes.push(LitByteStr::new(ident_bytes, ident.span()));
        }

        let start = range.start.as_ref();

        if !start.is_some_and(|start| {
            start
                .params
                .iter()
                .all(|param| matches!(param, InstanceIdent::Compile(_)))
        }) {
            return None;
        }

        let mut from = BStringList::default();
        for p in start.unwrap().params.iter() {
            let InstanceIdent::Compile(ident) = p else {
                unreachable!()
            };
            let ident_str = ident.to_string();
            let ident_bytes = ident_str.as_bytes();
            from.bytes.push(LitByteStr::new(ident_bytes, ident.span()));
        }

        return Some(NodeArgs::Traverse(TraverseNode {
            indexes: indexes.clone(),
            from,
            until,
        }));
    }
}
