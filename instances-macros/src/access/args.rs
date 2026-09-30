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
// ````````````````````````````` INSTANCE ACCESS ARGS ````````````````````````````
// ===============================================================================

//! Defines the syntax representations and extraction helpers for instance
//! access arguments, including supported access forms, their key-specific
//! arguments, and normalized components such as indexes, identifiers, prefixes,
//! and hierarchy depths.

// ===============================================================================
// ``````````````````````````````````` IMPORTS ```````````````````````````````````
// ===============================================================================

// --- Std crate ---
use std::fmt::Debug;

// --- Proc Suite ---
use proc_suite::{
    BStringList, DuplicateCheck, ErrorInfo, ExprList, IntList, ParseDiagnostic, key_schema,
    keys::{KeyInfo, KeySpec},
    misc::*,
};

// --- Proc Macro Crates ---
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    Block,
    Expr::{self},
    ExprCall, ExprPath, ExprReference, ExprReturn, ExprTry, ExprTuple, Ident, Stmt, parse,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    token::Comma,
};

// --- Local Crate
use crate::{
    Extraction, Utilization,
    access::errors::{ArgumentErrors, ArgumentSpace, KeysBugs, KeysError},
};

// ===============================================================================
// ``````````````````````````` BYTE-STR-EXPR ACCESS ARG ``````````````````````````
// ===============================================================================

/// Represents a expression access argument as either a pure expression of
/// type `&'static [u8]` or an explcit identifier/variable referring to the
/// byte-string value. To avoid ambiguity expressions can use `{expr}` to force
/// an ExprPath ambiguiated as an identifier.
#[derive(Debug, Clone)]
pub(crate) enum BStrInput {
    /// An expression of type `&[u8]` supplied directly in the access arguments.
    Raw(Expr),

    /// An identifier referring to the byte-string value.
    Ident(Ident),
}

/// Represents a comma-separated list of [`BStrInput`] access arguments.
#[derive(Debug, Clone, Default)]
pub(crate) struct BStrInputList {
    /// Byte-string expression and identifiers forming the access argument list.
    pub(crate) exprs: Punctuated<BStrInput, Comma>,
}

impl Parse for BStrInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let expr: Expr = input.parse()?;

        match expr {
            Expr::Path(path) if path.qself.is_none() && path.path.segments.len() == 1 => {
                Ok(Self::Ident(path.path.segments[0].ident.clone()))
            }

            expr => Ok(Self::Raw(Expr::Block(syn::ExprBlock {
                attrs: vec![],
                label: None,
                block: Block {
                    brace_token: Default::default(),
                    stmts: vec![Stmt::Expr(expr, None)],
                },
            }))),
        }
    }
}

impl ToTokens for BStrInput {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Raw(lit) => lit.to_tokens(tokens),
            Self::Ident(ident) => ident.to_tokens(tokens),
        }
    }
}

impl Parse for BStrInputList {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            exprs: Punctuated::<BStrInput, Comma>::parse_terminated(input)?,
        })
    }
}

impl ToTokens for BStrInputList {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.exprs.to_tokens(tokens);
    }
}

impl ParseDiagnostic for BStrInput {
    const KIND: &'static str = "byte string input";
    const EXPECTED: &'static str = "an expression of type `&[u8]` or an identifier";
    const EXAMPLE: &[&'static str] = &[r#"b"foo""#, "bytes"];
    const DEFAULT_ERROR_INFO: ErrorInfo = ArgumentSpace::BStrInputParseFail.to_error_info();
}

impl ParseDiagnostic for BStrInputList {
    const KIND: &'static str = "byte string input list";
    const EXPECTED: &'static str =
        "a comma-separated list of expressions of type `&[u8]` or identifiers";
    const EXAMPLE: &[&'static str] = &[r#"b"foo", b"bar""#, "bytes, other_bytes"];
    const DEFAULT_ERROR_INFO: ErrorInfo = ArgumentSpace::BStrInputListParseFail.to_error_info();
}

impl TryFrom<ExprList> for BStrInputList {
    type Error = TokenStream;

    fn try_from(value: ExprList) -> Result<Self, Self::Error> {
        let mut exprs = Punctuated::new();

        for expr in value.exprs {
            let input = match expr {
                Expr::Path(path) if path.qself.is_none() && path.path.segments.len() == 1 => {
                    BStrInput::Ident(path.path.segments[0].ident.clone())
                }

                Expr::Path(path) => {
                    return Err(
                        ArgumentErrors::ExpectedSingleSegmentIdent { span: path.span() }.into(),
                    );
                }

                expr => BStrInput::Raw(expr),
            };

            exprs.push(input);
        }

        Ok(Self { exprs })
    }
}

// ===============================================================================
// ``````````````````````````````` ASSOC-ACCESSOR ````````````````````````````````
// ===============================================================================

/// Supported accessor expressions accepted by the
/// [`call`](crate::call) proc-macro phases.
///
/// Rather than restricting users to a single expression form, the macro
/// accepts several equivalent accessor expressions and normalizes them
/// into this enum.
///
/// The parsed accessor is subsequently analyzed to recover:
///
/// - the referenced instance trait,
/// - the implementing `Self` type (whether a concrete type or an
///   associated type),
/// - and the associated item being accessed.
///
/// For example, all of the following are accepted:
///
/// ```ignore
/// <MyType as Example<...>>::VALUE
/// <MyType as Example<...>>::value()
/// &<MyType as Example<...>>::VALUE
/// (<MyType as Example<...>>::VALUE)
/// <MyType as Example<...>>::value()?
/// return <MyType as Example<...>>::VALUE
/// ```
///
/// The underlying associated-item access is typically represented by an
/// `ExprPath` or an ExprCall`]. Rust, however, permits these base
/// expressions to be wrapped by higher-level expression forms such as
/// references, tuples, `return`, and the `?` operator.
///
/// Normalizing these forms allows later expansion phases to uniformly
/// recover the underlying instance trait, implementing `Self` type, and
/// associated item regardless of the surrounding expression syntax.
#[derive(Clone, Debug)]
pub(crate) enum AssocAccessor {
    Fn(ExprCall),
    Path(ExprPath),
    Tuple(ExprTuple),
    Ref(ExprReference),
    Return(ExprReturn),
    Try(ExprTry),
}

impl ToTokens for AssocAccessor {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            AssocAccessor::Fn(expr_call) => tokens.extend(quote! {#expr_call}),
            AssocAccessor::Path(expr_path) => tokens.extend(quote! {#expr_path}),
            AssocAccessor::Tuple(expr_tuple) => tokens.extend(quote! {#expr_tuple}),
            AssocAccessor::Ref(expr_reference) => tokens.extend(quote! {#expr_reference}),
            AssocAccessor::Return(expr_return) => tokens.extend(quote! {#expr_return}),
            AssocAccessor::Try(expr_try) => tokens.extend(quote! {#expr_try}),
        }
    }
}

impl TryFrom<Expr> for AssocAccessor {
    type Error = syn::Error;

    fn try_from(value: Expr) -> Result<Self, Self::Error> {
        let result = match value {
            Expr::Call(expr_call) => {
                // panic!("{}", &expr_call.to_token_stream());
                AssocAccessor::Fn(expr_call)
            }
            Expr::Path(expr_path) => AssocAccessor::Path(expr_path),
            Expr::Reference(expr_reference) => AssocAccessor::Ref(expr_reference),
            Expr::Return(expr_return) => AssocAccessor::Return(expr_return),
            Expr::Try(expr_try) => AssocAccessor::Try(expr_try),
            Expr::Tuple(expr_tuple) => AssocAccessor::Tuple(expr_tuple),
            Expr::Group(expr_group) => Self::try_from(*expr_group.expr)?,
            e => {
                return Err(ArgumentErrors::AccessorExprUnsupported { expr: e }.into());
            }
        };
        Ok(result)
    }
}

/// Mutable counterpart of [`AssocAccessor`].
///
/// Each supported [`Expr`] variant owns a concrete syntax node whose
/// internal components carry the expression's semantic meaning.
///
/// For example, an [`ExprPath`] contains structures such as `QSelf`, where
/// this enum provides mutable access to those concrete syntax nodes so
/// their individual semantic components can be rewritten in place.
///
/// Such modifications may change the meaning of the entire expression
/// while preserving its syntactic variant.
#[derive(Debug)]
pub(crate) enum AssocAccessorMut<'a> {
    Fn(&'a mut ExprCall),
    Path(&'a mut ExprPath),
    Tuple(&'a mut ExprTuple),
    Ref(&'a mut ExprReference),
    Return(&'a mut ExprReturn),
    Try(&'a mut ExprTry),
}

impl<'a> TryFrom<&'a mut Expr> for AssocAccessorMut<'a> {
    type Error = syn::Error;

    fn try_from(value: &'a mut Expr) -> Result<Self, Self::Error> {
        let result = match value {
            Expr::Call(expr_call) => AssocAccessorMut::Fn(expr_call),
            Expr::Path(expr_path) => AssocAccessorMut::Path(expr_path),
            Expr::Reference(expr_reference) => AssocAccessorMut::Ref(expr_reference),
            Expr::Return(expr_return) => AssocAccessorMut::Return(expr_return),
            Expr::Try(expr_try) => AssocAccessorMut::Try(expr_try),
            Expr::Tuple(expr_tuple) => AssocAccessorMut::Tuple(expr_tuple),
            Expr::Group(expr_group) => Self::try_from(&mut *expr_group.expr)?,
            e => {
                return Err(ArgumentErrors::AccessorExprUnsupported { expr: e.clone() }.into());
            }
        };
        Ok(result)
    }
}

impl<'a> From<&'a mut AssocAccessor> for AssocAccessorMut<'a> {
    fn from(value: &'a mut AssocAccessor) -> Self {
        match value {
            AssocAccessor::Fn(expr_call) => AssocAccessorMut::Fn(expr_call),
            AssocAccessor::Path(expr_path) => AssocAccessorMut::Path(expr_path),
            AssocAccessor::Tuple(expr_tuple) => AssocAccessorMut::Tuple(expr_tuple),
            AssocAccessor::Ref(expr_reference) => AssocAccessorMut::Ref(expr_reference),
            AssocAccessor::Return(expr_return) => AssocAccessorMut::Return(expr_return),
            AssocAccessor::Try(expr_try) => AssocAccessorMut::Try(expr_try),
        }
    }
}

impl ToTokens for AssocAccessorMut<'_> {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            Self::Fn(expr) => expr.to_tokens(tokens),
            Self::Path(expr) => expr.to_tokens(tokens),
            Self::Tuple(expr) => expr.to_tokens(tokens),
            Self::Ref(expr) => expr.to_tokens(tokens),
            Self::Return(expr) => expr.to_tokens(tokens),
            Self::Try(expr) => expr.to_tokens(tokens),
        }
    }
}

// ===============================================================================
// `````````````````````````` INSTANCE ACCESS KEY-ARGS ```````````````````````````
// ===============================================================================

/// Represents the complete set of supported instance access argument forms.
#[derive(Debug, Clone)]
pub(crate) enum AccessArgs {
    /// Selects an instance directly from a leaf access path.
    Leaf(LeafAccess),

    /// Selects an instance through a parent-to-child branch path.
    Branch(BranchAccess),

    /// Selects an instance by pruning the specified counter path.
    Prune(PruneAccess),

    /// Selects an instance by trimming the specified counter path.
    Trim(TrimAccess),

    /// Selects an instance by extending the specified counter path.
    Extend(ExtendAccess),

    /// Selects an instance by spreading across the specified counter path.
    Spread(SpreadAccess),

    /// Selects an instance by traversing through the specified counter path.
    Traverse(TraverseAccess),

    /// Selects an instance by descending through the specified counter path.
    Descend(DescendAccess),

    /// Selects an instance from the root of the instance hierarchy.
    Root(RootAccess),

    /// Represents an unrecognized access argument for deferred error handling.
    Unknown(TokenAccess),
}

impl Extraction<TokenStream> for AccessArgs {
    fn raw_extract(from: &TokenStream, context: &()) -> Result<Self, proc_macro2::TokenStream> {
        let key = AccessArgKeyIdent::checked_extract(from, context)?
            .0
            .to_string();

        let value = match key.as_str() {
            "leaf" => AccessArgs::Leaf(LeafAccess::checked_extract(from, context)?),
            "branch" => AccessArgs::Branch(BranchAccess::checked_extract(from, context)?),
            "prune" => AccessArgs::Prune(PruneAccess::checked_extract(from, context)?),
            "trim" => AccessArgs::Trim(TrimAccess::checked_extract(from, context)?),
            "extend" => AccessArgs::Extend(ExtendAccess::checked_extract(from, context)?),
            "spread" => AccessArgs::Spread(SpreadAccess::checked_extract(from, context)?),
            "traverse" => AccessArgs::Traverse(TraverseAccess::checked_extract(from, context)?),
            "descend" => AccessArgs::Descend(DescendAccess::checked_extract(from, context)?),
            "root" => AccessArgs::Root(RootAccess::checked_extract(from, context)?),
            _ => AccessArgs::Unknown(TokenAccess::checked_extract(from, context)?),
        };

        Ok(value)
    }

    fn validate_extract(
        &self,
        _: &TokenStream,
        _: Option<&()>,
    ) -> Result<(), proc_macro2::TokenStream> {
        Ok(())
    }
}

/// Contains the prefix and identifier arguments extracted from an access form.
#[derive(Debug, Clone)]
pub(crate) struct AccessArgsIdents {
    /// Prefix values identifying the parent portion of the access path.
    pub(crate) prefixes: BStringList,

    /// Identifier expressions selecting values within the accessed counters.
    pub(crate) idents: BStrInputList,
}

impl Extraction<AccessArgs> for AccessArgsIdents {
    fn raw_extract(from: &AccessArgs, _: &()) -> Result<Self, proc_macro2::TokenStream> {
        let prefixes_def = Default::default();
        let idents_def = Default::default();
        match from {
            AccessArgs::Leaf(leaf) => Ok(Self {
                prefixes: leaf.idents.clone(),
                idents: idents_def,
            }),

            AccessArgs::Branch(branch) => {
                let BranchAccess {
                    parents, childs, ..
                } = branch;

                Ok(Self {
                    prefixes: parents.clone(),
                    idents: childs.clone().into(),
                })
            }

            AccessArgs::Prune(prune) => Ok(Self {
                prefixes: prefixes_def,
                idents: prune.idents.clone(),
            }),

            AccessArgs::Trim(trim) => Ok(Self {
                prefixes: prefixes_def,
                idents: trim.idents.clone(),
            }),

            AccessArgs::Extend(extend) => Ok(Self {
                prefixes: prefixes_def,
                idents: extend.idents.clone(),
            }),

            AccessArgs::Spread(spread) => Ok(Self {
                prefixes: prefixes_def,
                idents: spread.idents.clone(),
            }),

            AccessArgs::Traverse(traverse) => Ok(Self {
                prefixes: match &traverse.prefix {
                    Some(p) => p.clone(),
                    None => prefixes_def,
                },
                idents: traverse.idents.clone(),
            }),

            AccessArgs::Descend(descend) => Ok(Self {
                prefixes: match &descend.prefix {
                    Some(p) => p.clone(),
                    None => prefixes_def,
                },
                idents: descend.idents.clone(),
            }),

            AccessArgs::Root(root) => Ok(Self {
                prefixes: prefixes_def,
                idents: root.idents.clone(),
            }),

            AccessArgs::Unknown(token) => Err(KeysError::ExpectedKeys {
                span: token.tokens.span(),
            }
            .into()),
        }
    }

    fn validate_extract(
        &self,
        _: &AccessArgs,
        _: Option<&()>,
    ) -> Result<(), proc_macro2::TokenStream> {
        Ok(())
    }
}

/// Contains the counter indexes specified by an access argument.
#[derive(Debug, Clone)]
pub(crate) struct AccessArgsIndexes<'a>(pub(crate) &'a IntList);

impl<'a> Utilization<'a, AccessArgs> for AccessArgsIndexes<'a> {
    fn raw_utilize(from: &'a AccessArgs, _: &()) -> Result<Self, proc_macro2::TokenStream> {
        let indexes = match from {
            AccessArgs::Leaf(leaf) => &leaf.indexes,
            AccessArgs::Branch(branch) => &branch.indexes,
            AccessArgs::Prune(prune) => &prune.indexes,
            AccessArgs::Trim(trim) => &trim.indexes,
            AccessArgs::Extend(extend) => &extend.indexes,
            AccessArgs::Spread(spread) => &spread.indexes,
            AccessArgs::Traverse(traverse) => &traverse.indexes,
            AccessArgs::Descend(descend) => &descend.indexes,
            AccessArgs::Root(root) => &root.indexes,
            AccessArgs::Unknown(token) => {
                return Err(KeysError::ExpectedKeys {
                    span: token.tokens.span(),
                }
                .into());
            }
        };
        Ok(Self(indexes))
    }

    fn validate_utilize(
        &self,
        _: &AccessArgs,
        _: Option<&()>,
    ) -> Result<(), proc_macro2::TokenStream> {
        Ok(())
    }
}

/// Contains the inner and outer hierarchy depths specified by an access argument.
#[derive(Debug, Clone)]
pub(crate) struct AccessArgsDepths(pub(crate) (u8, u8));

impl Extraction<AccessArgs> for AccessArgsDepths {
    fn raw_extract(from: &AccessArgs, _: &()) -> Result<Self, proc_macro2::TokenStream> {
        let depth = match from {
            AccessArgs::Branch(branch) => branch.depths.clone(),
            AccessArgs::Prune(prune) => prune.depths.clone(),
            AccessArgs::Trim(trim) => trim.depths.clone(),
            AccessArgs::Extend(extend) => extend.depths.clone(),
            AccessArgs::Spread(spread) => spread.depths.clone(),
            AccessArgs::Traverse(traverse) => traverse.depths.clone(),
            AccessArgs::Descend(descend) => descend.depths.clone(),
            AccessArgs::Root(root) => root.depths.clone(),
            AccessArgs::Leaf(_) => return Err(KeysBugs::ExpectedDynKeyArgs {}.into()),
            AccessArgs::Unknown(token) => {
                return Err(KeysError::ExpectedKeys {
                    span: token.tokens.span(),
                }
                .into());
            }
        };

        const DEFAULT_DEPTH_1: u8 = 8;
        const MAX_DEPTH_1: u8 = 32;
        const DEFAULT_DEPTH_2: u8 = 4;
        const MAX_DEPTH_2: u8 = 16;

        let actual = match depth {
            Some(given) => match given.ints.len() {
                0 => (DEFAULT_DEPTH_1, DEFAULT_DEPTH_2),
                1 => {
                    let item = &given.ints[0];
                    let depth = parse_pos_u8(item)?;
                    if depth == 0 {
                        return Err(ArgumentErrors::DepthIsZero { lit: item.clone() }.into());
                    }
                    if depth > MAX_DEPTH_1 {
                        return Err(ArgumentErrors::MaxDepthReached {
                            lit: item.clone(),
                            max: MAX_DEPTH_1,
                        }
                        .into());
                    }
                    (depth, DEFAULT_DEPTH_2)
                }
                2 => {
                    let (item_2, item_1) = (&given.ints[0], &given.ints[1]);
                    let (depth_2, depth_1) = (parse_pos_u8(item_2)?, parse_pos_u8(item_1)?);
                    if depth_2 == 0 {
                        return Err(ArgumentErrors::DepthIsZero {
                            lit: item_2.clone(),
                        }
                        .into());
                    }
                    if depth_1 == 0 {
                        return Err(ArgumentErrors::DepthIsZero {
                            lit: item_1.clone(),
                        }
                        .into());
                    }
                    if depth_1 > MAX_DEPTH_1 {
                        return Err(ArgumentErrors::MaxDepthReached {
                            lit: item_1.clone(),
                            max: MAX_DEPTH_1,
                        }
                        .into());
                    }
                    if depth_2 > MAX_DEPTH_2 {
                        return Err(ArgumentErrors::MaxDepthReached {
                            lit: item_2.clone(),
                            max: MAX_DEPTH_2,
                        }
                        .into());
                    }
                    (depth_1, depth_2)
                }
                _ => {
                    return Err(ArgumentErrors::MaxTwoDepthsOnly {
                        depths: given.clone(),
                    }
                    .into());
                }
            },
            None => (DEFAULT_DEPTH_1, DEFAULT_DEPTH_2),
        };
        Ok(Self(actual))
    }

    fn validate_extract(
        &self,
        _: &AccessArgs,
        _: Option<&()>,
    ) -> Result<(), proc_macro2::TokenStream> {
        Ok(())
    }
}

// ===============================================================================
// ```````````````````````````` ACCESS ARGS KEY IDENT ````````````````````````````
// ===============================================================================

/// Contains the key identifier identifying the requested access-argument form.
#[derive(Debug, Clone)]
pub(crate) struct AccessArgKeyIdent(Ident);

impl Extraction<TokenStream> for AccessArgKeyIdent {
    fn raw_extract(from: &TokenStream, _: &()) -> Result<Self, proc_macro2::TokenStream> {
        let k_info = match parse::<KeyInfo>(from.clone().into()) {
            Ok(v) => v,
            Err(e) => return Err(KeysError::ExpectedKeys { span: e.span() }.into()),
        };

        return Ok(Self(k_info.key));
    }

    fn validate_extract(
        &self,
        _: &TokenStream,
        _: Option<&()>,
    ) -> Result<(), proc_macro2::TokenStream> {
        Ok(())
    }
}

// ===============================================================================
// ```````````````````````````` LEAF ACCESS KEY-ARGS `````````````````````````````
// ===============================================================================

use leaf_key::LeafAccessSpec;

mod leaf_key {
    use super::*;
    key_schema! {
        pub(crate) LeafAccessSpec {
            key: "leaf",

            allow_empty: false,

            sub_keys: {
                index : IntList,
                ident : BStringList,
            },

            combinations: {
                Indexed(index, ident),
            }
        }
    }
}

#[derive(Debug, Clone, Default)]
/// Represents the arguments for leaf instance access.
pub(crate) struct LeafAccess {
    /// Counter indexes identifying the leaf counters being accessed.
    pub(crate) indexes: IntList,

    /// Byte-string identifiers selecting values from those counters.
    pub(crate) idents: BStringList,
}

impl Extraction<TokenStream> for LeafAccess {
    fn raw_extract(from: &TokenStream, _: &()) -> Result<Self, proc_macro2::TokenStream> {
        let k_info = match parse::<KeyInfo>(from.clone().into()) {
            Ok(v) => v,
            Err(e) => return Err(KeysError::ExpectedLeaf { span: e.span() }.into()),
        };

        let args = LeafAccessSpec::parse(&k_info)?;

        let (indexes, idents) = match args {
            LeafAccessSpec::Indexed(ints, idents) => (ints, idents),
        };

        indexes.duplicate_check(Some(ArgumentSpace::DuplicateCounterIndexes {}.into()))?;
        if indexes.ints.len() != idents.bytes.len() {
            return Err(ArgumentErrors::IdentsAndIndexesNotSameLen { indexes, idents }.into());
        }

        return Ok(Self { indexes, idents });
    }

    fn validate_extract(
        &self,
        _: &TokenStream,
        _: Option<&()>,
    ) -> Result<(), proc_macro2::TokenStream> {
        Ok(())
    }
}

// ===============================================================================
// ``````````````````````````` BRANCH ACCESS KEY-ARGS ````````````````````````````
// ===============================================================================

use branch_key::BranchAccessSpec;

mod branch_key {
    use super::*;
    key_schema! {
        pub(crate) BranchAccessSpec {
            key: "branch",

            allow_empty: false,

            sub_keys: {
                index : IntList,
                parent : BStringList,
                child : BStrInputList,
                depth: IntList,
            },

            combinations: {
                Indexed(index, parent, child),
                Leveled(index, parent, child, depth),
            }
        }
    }
}

#[derive(Debug, Clone, Default)]
/// Represents the arguments for branch instance access.
pub(crate) struct BranchAccess {
    /// Counter indexes defining the branch path.
    pub(crate) indexes: IntList,

    /// Parent identifiers defining the parent portion of the branch path.
    pub(crate) parents: BStringList,

    /// Child expressions defining the child portion of the branch path.
    pub(crate) childs: BStrInputList,

    /// Optional hierarchy depths associated with the child path.
    pub(crate) depths: Option<IntList>,
}

impl Extraction<TokenStream> for BranchAccess {
    fn raw_extract(from: &TokenStream, _: &()) -> Result<Self, proc_macro2::TokenStream> {
        let k_info = match parse::<KeyInfo>(from.clone().into()) {
            Ok(v) => v,
            Err(e) => return Err(KeysError::ExpectedBranch { span: e.span() }.into()),
        };

        let args = BranchAccessSpec::parse(&k_info)?;

        let (indexes, parents, childs, depths) = match args {
            BranchAccessSpec::Indexed(ints, parent, child) => (ints, parent, child, None),
            BranchAccessSpec::Leveled(ints, parent, child, depth) => {
                if depth.ints.len() != child.exprs.len() {
                    return Err(ArgumentErrors::DepthArgsInconsistent {
                        lens: depth,
                        idents: child,
                    }
                    .into());
                }
                (ints, parent, child, Some(depth))
            }
        };

        indexes.duplicate_check(Some(ArgumentSpace::DuplicateCounterIndexes {}.into()))?;
        let parent_len = parents.bytes.len();
        let child_len = childs.exprs.len();
        let indexes_len = indexes.ints.len();
        if parent_len >= indexes_len {
            return Err(ArgumentErrors::UnexpectedParentDepth {
                span: parents.bytes.span(),
            }
            .into());
        }
        if child_len >= indexes_len {
            return Err(ArgumentErrors::UnexpectedChildDepth {
                span: childs.exprs.span(),
            }
            .into());
        }

        if parent_len + child_len != indexes_len {
            return Err(ArgumentErrors::UnexpectedPathDepth { span: from.span() }.into());
        }

        return Ok(Self {
            indexes,
            parents,
            childs,
            depths,
        });
    }

    fn validate_extract(
        &self,
        _: &TokenStream,
        _: Option<&()>,
    ) -> Result<(), proc_macro2::TokenStream> {
        Ok(())
    }
}

// ===============================================================================
// ``````````````````````````` PRUNE ACCESS KEY-ARGS ````````````````````````````
// ===============================================================================

use prune_key::PruneAccessSpec;

mod prune_key {
    use super::*;
    key_schema! {
        pub(crate) PruneAccessSpec {
            key: "prune",

            allow_empty: false,

            sub_keys: {
                index : IntList,
                ident : BStrInputList,
                depth: IntList,
            },

            combinations: {
                Indexed(index, ident),
                Leveled(index, ident, depth),
            }
        }
    }
}

#[derive(Debug, Clone, Default)]
/// Represents the arguments for prune instance access.
pub(crate) struct PruneAccess {
    /// Counter indexes identifying the counters to traverse.
    pub(crate) indexes: IntList,

    /// Identifier expressions selecting values from the counters.
    pub(crate) idents: BStrInputList,

    /// Optional hierarchy depths for the selected counters.
    pub(crate) depths: Option<IntList>,
}

impl Extraction<TokenStream> for PruneAccess {
    fn raw_extract(from: &TokenStream, _: &()) -> Result<Self, proc_macro2::TokenStream> {
        let k_info = match parse::<KeyInfo>(from.clone().into()) {
            Ok(v) => v,
            Err(e) => return Err(KeysError::ExpectedPrune { span: e.span() }.into()),
        };

        let args = PruneAccessSpec::parse(&k_info)?;

        let (indexes, idents, depths) = match args {
            PruneAccessSpec::Indexed(ints, idents) => (ints, idents, None),
            PruneAccessSpec::Leveled(ints, idents, depth) => {
                if depth.ints.len() != idents.exprs.len() {
                    return Err(ArgumentErrors::DepthArgsInconsistent {
                        lens: depth,
                        idents,
                    }
                    .into());
                }

                (ints, idents, Some(depth))
            }
        };

        indexes.duplicate_check(Some(ArgumentSpace::DuplicateCounterIndexes {}.into()))?;
        if indexes.ints.len() != idents.exprs.len() {
            return Err(ArgumentErrors::BStrExprAndIndexesNotSameLen { indexes, idents }.into());
        }

        return Ok(Self {
            indexes,
            idents,
            depths,
        });
    }

    fn validate_extract(
        &self,
        _: &TokenStream,
        _: Option<&()>,
    ) -> Result<(), proc_macro2::TokenStream> {
        Ok(())
    }
}

// ===============================================================================
// ``````````````````````````` TRIM ACCESS KEY-ARGS ````````````````````````````
// ===============================================================================

use trim_key::TrimAccessSpec;

mod trim_key {
    use super::*;
    key_schema! {
        pub(crate) TrimAccessSpec {
            key: "trim",

            allow_empty: false,

            sub_keys: {
                index : IntList,
                ident : BStrInputList,
                depth: IntList,
            },

            combinations: {
                Indexed(index, ident),
                Leveled(index, ident, depth),
            }
        }
    }
}

#[derive(Debug, Clone, Default)]
/// Represents the arguments for trim instance access.
pub(crate) struct TrimAccess {
    /// Counter indexes identifying the counters to traverse.
    pub(crate) indexes: IntList,

    /// Identifier expressions selecting values from the counters.
    pub(crate) idents: BStrInputList,

    /// Optional hierarchy depths for the selected counters.
    pub(crate) depths: Option<IntList>,
}

impl Extraction<TokenStream> for TrimAccess {
    fn raw_extract(from: &TokenStream, _: &()) -> Result<Self, proc_macro2::TokenStream> {
        let k_info = match parse::<KeyInfo>(from.clone().into()) {
            Ok(v) => v,
            Err(e) => return Err(KeysError::ExpectedTrim { span: e.span() }.into()),
        };

        let args = TrimAccessSpec::parse(&k_info)?;

        let (indexes, idents, depths) = match args {
            TrimAccessSpec::Indexed(ints, idents) => (ints, idents, None),
            TrimAccessSpec::Leveled(ints, idents, depth) => {
                if depth.ints.len() != idents.exprs.len() {
                    return Err(ArgumentErrors::DepthArgsInconsistent {
                        lens: depth,
                        idents,
                    }
                    .into());
                }

                (ints, idents, Some(depth))
            }
        };

        indexes.duplicate_check(Some(ArgumentSpace::DuplicateCounterIndexes {}.into()))?;
        if indexes.ints.len() != idents.exprs.len() {
            return Err(ArgumentErrors::BStrExprAndIndexesNotSameLen { indexes, idents }.into());
        }

        return Ok(Self {
            indexes,
            idents,
            depths,
        });
    }

    fn validate_extract(
        &self,
        _: &TokenStream,
        _: Option<&()>,
    ) -> Result<(), proc_macro2::TokenStream> {
        Ok(())
    }
}

// ===============================================================================
// ``````````````````````````` EXTEND ACCESS KEY-ARGS ````````````````````````````
// ===============================================================================

use extend_key::ExtendAccessSpec;

mod extend_key {
    use super::*;

    key_schema! {
        pub(crate) ExtendAccessSpec {
            key: "extend",

            allow_empty: false,

            sub_keys: {
                index : IntList,
                ident : BStrInputList,
                depth: IntList,
            },

            combinations: {
                Indexed(index, ident),
                Leveled(index, ident, depth),
            }
        }
    }
}

#[derive(Debug, Clone, Default)]
/// Represents the arguments for extend instance access.
pub(crate) struct ExtendAccess {
    /// Counter indexes identifying the counters to traverse.
    pub(crate) indexes: IntList,

    /// Identifier expressions selecting values from the counters.
    pub(crate) idents: BStrInputList,

    /// Optional hierarchy depths for the selected counters.
    pub(crate) depths: Option<IntList>,
}

impl Extraction<TokenStream> for ExtendAccess {
    fn raw_extract(from: &TokenStream, _: &()) -> Result<Self, proc_macro2::TokenStream> {
        let k_info = match parse::<KeyInfo>(from.clone().into()) {
            Ok(v) => v,
            Err(e) => return Err(KeysError::ExpectedExtend { span: e.span() }.into()),
        };

        let args = ExtendAccessSpec::parse(&k_info)?;

        let (indexes, idents, depths) = match args {
            ExtendAccessSpec::Indexed(ints, idents) => (ints, idents, None),

            ExtendAccessSpec::Leveled(ints, idents, depth) => {
                if depth.ints.len() != idents.exprs.len() {
                    return Err(ArgumentErrors::DepthArgsInconsistent {
                        lens: depth,
                        idents,
                    }
                    .into());
                }

                (ints, idents, Some(depth))
            }
        };

        indexes.duplicate_check(Some(ArgumentSpace::DuplicateCounterIndexes {}.into()))?;

        if indexes.ints.len() != idents.exprs.len() {
            return Err(ArgumentErrors::BStrExprAndIndexesNotSameLen { indexes, idents }.into());
        }

        Ok(Self {
            indexes,
            idents,
            depths,
        })
    }

    fn validate_extract(
        &self,
        _: &TokenStream,
        _: Option<&()>,
    ) -> Result<(), proc_macro2::TokenStream> {
        Ok(())
    }
}

// ===============================================================================
// ``````````````````````````` SPREAD ACCESS KEY-ARGS ````````````````````````````
// ===============================================================================

use spread_key::SpreadAccessSpec;

mod spread_key {
    use super::*;

    key_schema! {
        pub(crate) SpreadAccessSpec {
            key: "spread",

            allow_empty: false,

            sub_keys: {
                index : IntList,
                ident : BStrInputList,
                depth: IntList,
            },

            combinations: {
                Indexed(index, ident),
                Leveled(index, ident, depth),
            }
        }
    }
}

#[derive(Debug, Clone, Default)]
/// Represents the arguments for spread instance access.
pub(crate) struct SpreadAccess {
    /// Counter indexes identifying the counters to traverse.
    pub(crate) indexes: IntList,

    /// Identifier expressions selecting values from the counters.
    pub(crate) idents: BStrInputList,

    /// Optional hierarchy depths for the selected counters.
    pub(crate) depths: Option<IntList>,
}

impl Extraction<TokenStream> for SpreadAccess {
    fn raw_extract(from: &TokenStream, _: &()) -> Result<Self, proc_macro2::TokenStream> {
        let k_info = match parse::<KeyInfo>(from.clone().into()) {
            Ok(v) => v,
            Err(e) => return Err(KeysError::ExpectedSpread { span: e.span() }.into()),
        };

        let args = SpreadAccessSpec::parse(&k_info)?;

        let (indexes, idents, depths) = match args {
            SpreadAccessSpec::Indexed(ints, idents) => (ints, idents, None),

            SpreadAccessSpec::Leveled(ints, idents, depth) => {
                if depth.ints.len() != idents.exprs.len() {
                    return Err(ArgumentErrors::DepthArgsInconsistent {
                        lens: depth,
                        idents,
                    }
                    .into());
                }

                (ints, idents, Some(depth))
            }
        };

        indexes.duplicate_check(Some(ArgumentSpace::DuplicateCounterIndexes {}.into()))?;

        if indexes.ints.len() != idents.exprs.len() {
            return Err(ArgumentErrors::BStrExprAndIndexesNotSameLen { indexes, idents }.into());
        }

        Ok(Self {
            indexes,
            idents,
            depths,
        })
    }

    fn validate_extract(
        &self,
        _: &TokenStream,
        _: Option<&()>,
    ) -> Result<(), proc_macro2::TokenStream> {
        Ok(())
    }
}

// ===============================================================================
// ```````````````````````````` DYN ACCESS KEY-ARGS `````````````````````````````
// ===============================================================================

use root_key::RootAccessSpec;

mod root_key {
    use super::*;

    key_schema! {
        pub(crate) RootAccessSpec {
            key: "root",

            allow_empty: false,

            sub_keys: {
                index : IntList,
                ident : BStrInputList,
                depth: IntList,
            },

            combinations: {
                Indexed(index, ident),
                Leveled(index, ident, depth),
            }
        }
    }
}

#[derive(Debug, Clone, Default)]
/// Represents the arguments for root instance access.
pub(crate) struct RootAccess {
    /// Counter indexes identifying the counters to traverse from the root.
    pub(crate) indexes: IntList,

    /// Identifier expressions selecting values from the counters.
    pub(crate) idents: BStrInputList,

    /// Optional hierarchy depths for the selected counters.
    pub(crate) depths: Option<IntList>,
}

impl Extraction<TokenStream> for RootAccess {
    fn raw_extract(from: &TokenStream, _: &()) -> Result<Self, proc_macro2::TokenStream> {
        let k_info = match parse::<KeyInfo>(from.clone().into()) {
            Ok(v) => v,
            Err(e) => return Err(KeysError::ExpectedRoot { span: e.span() }.into()),
        };

        let args = RootAccessSpec::parse(&k_info)?;

        let (indexes, idents, depths) = match args {
            RootAccessSpec::Indexed(ints, idents) => (ints, idents, None),

            RootAccessSpec::Leveled(ints, idents, depth) => {
                if depth.ints.len() != idents.exprs.len() {
                    return Err(ArgumentErrors::DepthArgsInconsistent {
                        lens: depth,
                        idents,
                    }
                    .into());
                }

                (ints, idents, Some(depth))
            }
        };

        indexes.duplicate_check(Some(ArgumentSpace::DuplicateCounterIndexes {}.into()))?;

        if indexes.ints.len() != idents.exprs.len() {
            return Err(ArgumentErrors::BStrExprAndIndexesNotSameLen { indexes, idents }.into());
        }

        Ok(Self {
            indexes,
            idents,
            depths,
        })
    }

    fn validate_extract(
        &self,
        _: &TokenStream,
        _: Option<&()>,
    ) -> Result<(), proc_macro2::TokenStream> {
        Ok(())
    }
}

// ===============================================================================
// ```````````````````````````` TRAVERSE ACCESS KEY-ARGS `````````````````````````````
// ===============================================================================

use traverse_key::TraverseAccessSpec;

mod traverse_key {
    use super::*;

    key_schema! {
        pub(crate) TraverseAccessSpec {
            key: "traverse",

            allow_empty: false,

            sub_keys: {
                index : IntList,
                ident : BStrInputList,
                prefix : BStringList,
                depth: IntList,
            },

            combinations: {
                Indexed(index, ident),
                Leveled(index, ident, depth),
                Prefixed(index, prefix, ident),
                PrefixLeveled(index, prefix, ident, depth),
            }
        }
    }
}

#[derive(Debug, Clone, Default)]
/// Represents the arguments for traverse instance access.
pub(crate) struct TraverseAccess {
    /// Counter indexes defining the traversal path.
    pub(crate) indexes: IntList,

    /// Optional prefix values restricting the traversal path.
    pub(crate) prefix: Option<BStringList>,

    /// Identifier expressions selecting values from the traversed counters.
    pub(crate) idents: BStrInputList,

    /// Optional hierarchy depths associated with the traversal.
    pub(crate) depths: Option<IntList>,
}

impl Extraction<TokenStream> for TraverseAccess {
    fn raw_extract(from: &TokenStream, _: &()) -> Result<Self, proc_macro2::TokenStream> {
        let k_info = match parse::<KeyInfo>(from.clone().into()) {
            Ok(v) => v,
            Err(e) => return Err(KeysError::ExpectedTraverse { span: e.span() }.into()),
        };

        let args = TraverseAccessSpec::parse(&k_info)?;

        let (indexes, prefix, idents, depths) = match args {
            TraverseAccessSpec::Indexed(ints, idents) => (ints, None, idents, None),

            TraverseAccessSpec::Leveled(ints, idents, depth) => {
                if depth.ints.len() != idents.exprs.len() {
                    return Err(ArgumentErrors::DepthArgsInconsistent {
                        lens: depth,
                        idents,
                    }
                    .into());
                }

                (ints, None, idents, Some(depth))
            }
            TraverseAccessSpec::Prefixed(ints, prefix, idents) => {
                (ints, Some(prefix), idents, None)
            }
            TraverseAccessSpec::PrefixLeveled(ints, prefix, idents, depth) => {
                if depth.ints.len() != idents.exprs.len() {
                    return Err(ArgumentErrors::DepthArgsInconsistent {
                        lens: depth,
                        idents,
                    }
                    .into());
                }

                (ints, Some(prefix), idents, Some(depth))
            }
        };

        indexes.duplicate_check(Some(ArgumentSpace::DuplicateCounterIndexes {}.into()))?;

        match &prefix {
            Some(prefix) => {
                let parent_len = prefix.bytes.len();
                let child_len = idents.exprs.len();
                let indexes_len = indexes.ints.len();
                if parent_len >= indexes_len {
                    return Err(ArgumentErrors::UnexpectedPrefixDepth {
                        span: prefix.bytes.span(),
                    }
                    .into());
                }
                if child_len >= indexes_len {
                    return Err(ArgumentErrors::UnexpectedPrefixIdentDepth {
                        span: idents.exprs.span(),
                    }
                    .into());
                }

                if parent_len + child_len != indexes_len {
                    return Err(
                        ArgumentErrors::UnexpectedPrefixPathDepth { span: from.span() }.into(),
                    );
                }
            }
            None => {
                if indexes.ints.len() != idents.exprs.len() {
                    return Err(
                        ArgumentErrors::BStrExprAndIndexesNotSameLen { indexes, idents }.into(),
                    );
                }
            }
        }

        Ok(Self {
            indexes,
            prefix,
            idents,
            depths,
        })
    }

    fn validate_extract(
        &self,
        _: &TokenStream,
        _: Option<&()>,
    ) -> Result<(), proc_macro2::TokenStream> {
        Ok(())
    }
}

// ===============================================================================
// ```````````````````````````` DESCEND ACCESS KEY-ARGS `````````````````````````````
// ===============================================================================

use descend_key::DescendAccessSpec;

mod descend_key {
    use super::*;

    key_schema! {
        pub(crate) DescendAccessSpec {
            key: "descend",

            allow_empty: false,

            sub_keys: {
                index : IntList,
                ident : BStrInputList,
                prefix : BStringList,
                depth: IntList,
            },

            combinations: {
                Indexed(index, ident),
                Leveled(index, ident, depth),
                Prefixed(index, prefix, ident),
                PrefixLeveled(index, prefix, ident, depth),
            }
        }
    }
}

#[derive(Debug, Clone, Default)]
/// Represents the arguments for descend instance access.
pub(crate) struct DescendAccess {
    /// Counter indexes defining the descent path.
    pub(crate) indexes: IntList,

    /// Optional prefix values restricting the descent path.
    pub(crate) prefix: Option<BStringList>,

    /// Identifier expressions selecting values from the descended counters.
    pub(crate) idents: BStrInputList,

    /// Optional hierarchy depths associated with the descent.
    pub(crate) depths: Option<IntList>,
}

impl Extraction<TokenStream> for DescendAccess {
    fn raw_extract(from: &TokenStream, _: &()) -> Result<Self, proc_macro2::TokenStream> {
        let k_info = match parse::<KeyInfo>(from.clone().into()) {
            Ok(v) => v,
            Err(e) => return Err(KeysError::ExpectedDescend { span: e.span() }.into()),
        };

        let args = DescendAccessSpec::parse(&k_info)?;

        let (indexes, prefix, idents, depths) = match args {
            DescendAccessSpec::Indexed(ints, idents) => (ints, None, idents, None),

            DescendAccessSpec::Leveled(ints, idents, depth) => {
                if depth.ints.len() != idents.exprs.len() {
                    return Err(ArgumentErrors::DepthArgsInconsistent {
                        lens: depth,
                        idents,
                    }
                    .into());
                }

                (ints, None, idents, Some(depth))
            }
            DescendAccessSpec::Prefixed(ints, prefix, idents) => (ints, Some(prefix), idents, None),
            DescendAccessSpec::PrefixLeveled(ints, prefix, idents, depth) => {
                if depth.ints.len() != idents.exprs.len() {
                    return Err(ArgumentErrors::DepthArgsInconsistent {
                        lens: depth,
                        idents,
                    }
                    .into());
                }

                (ints, Some(prefix), idents, Some(depth))
            }
        };

        indexes.duplicate_check(Some(ArgumentSpace::DuplicateCounterIndexes {}.into()))?;

        match &prefix {
            Some(prefix) => {
                let parent_len = prefix.bytes.len();
                let child_len = idents.exprs.len();
                let indexes_len = indexes.ints.len();
                if parent_len >= indexes_len {
                    return Err(ArgumentErrors::UnexpectedPrefixDepth {
                        span: prefix.bytes.span(),
                    }
                    .into());
                }
                if child_len >= indexes_len {
                    return Err(ArgumentErrors::UnexpectedPrefixIdentDepth {
                        span: idents.exprs.span(),
                    }
                    .into());
                }
                if parent_len + child_len != indexes_len {
                    return Err(
                        ArgumentErrors::UnexpectedPrefixPathDepth { span: from.span() }.into(),
                    );
                }
            }
            None => {
                if indexes.ints.len() != idents.exprs.len() {
                    return Err(
                        ArgumentErrors::BStrExprAndIndexesNotSameLen { indexes, idents }.into(),
                    );
                }
            }
        }

        Ok(Self {
            indexes,
            prefix,
            idents,
            depths,
        })
    }

    fn validate_extract(
        &self,
        _: &TokenStream,
        _: Option<&()>,
    ) -> Result<(), proc_macro2::TokenStream> {
        Ok(())
    }
}

// ===============================================================================
// ```````````````````````````` TOKEN ACCESS KEY-ARGS ````````````````````````````
// ===============================================================================

#[derive(Debug, Clone, Default)]
/// Represents unrecognized access arguments preserved as raw tokens.
pub(crate) struct TokenAccess {
    /// Original tokens preserved for deferred diagnostic reporting.
    pub(crate) tokens: TokenStream,
}

impl Extraction<TokenStream> for TokenAccess {
    fn raw_extract(from: &TokenStream, _: &()) -> Result<Self, proc_macro2::TokenStream> {
        Ok(Self {
            tokens: from.clone(),
        })
    }

    fn validate_extract(
        &self,
        _: &TokenStream,
        _: Option<&()>,
    ) -> Result<(), proc_macro2::TokenStream> {
        Ok(())
    }
}
