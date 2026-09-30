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
// ````````````````````````` PARSED PROC-MACRO ARGUMENTS `````````````````````````
// ===============================================================================

//! Parsers for inst proc-macros user inputs.
//!
//! This module defines the argument parsers used by the inst-proc-macro
//! Each parser converts an incoming [`TokenStream`] into a semantically
//! meaningful representation tailored to the macro being expanded.

// ===============================================================================
// ``````````````````````````````````` IMPORTS ```````````````````````````````````
// ===============================================================================

// --- Proc-Suite ---
use proc_suite::{BStringList, DuplicateCheck, ErrorInfo, IdentList, IntList, ParseDiagnostic};

// Proc Macro Crate ---
use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, format_ident, quote};
use syn::{
    Attribute, Block, Expr, ExprPath, Ident, Item, ItemImpl, ItemMod, ItemTrait, LitInt, Stmt, Token, Visibility, braced, bracketed, parenthesized, parse::{Parse, ParseStream, discouraged::Speculative}, parse_quote, punctuated::Punctuated, token::{self, Comma, PathSep},
};

// --- Local Crate ---
use crate::{
    Extraction, Inst, errors::{ParseError, ProcParseErr, TraitSpace}, node::{AccessArgsRef, NodeArgsRef},
};

// ===============================================================================
// ``````````````````````````````````` TRAITS ````````````````````````````````````
// ===============================================================================

/// Provides `span(&self)` for extended parsable syntax nodes.
pub trait SpanOf {
    /// Provides span of the syntax node.
    fn span(&self) -> Span;
}

// ===============================================================================
// ```````````````````````````````` COMMON IDENTS ````````````````````````````````
// ===============================================================================

impl Inst {
    pub(crate) fn feature() -> Attribute {
        parse_quote!(#[cfg(feature = "inst")])
    }

    pub(crate) fn not_feature() -> Attribute {
        parse_quote!(#[cfg(not(feature = "inst"))])
    }

    pub(crate) fn node() -> Ident {
        format_ident!("node")
    }

    pub(crate) fn tuple() -> Ident {
        format_ident!("tuple")
    }

    pub(crate) fn sum() -> Ident {
        format_ident!("sum")
    }

    pub(crate) fn subscriber() -> Ident {
        format_ident!("instance_sub")
    }

    pub(crate) fn publisher() -> Ident {
        format_ident!("instance_pub")
    }

    pub(crate) fn node_target() -> Ident {
        format_ident!("instance_node_explicit_target")
    }

    pub(crate) fn trait_target() -> Ident {
        format_ident!("instance_trait_explicit_target")
    }

    pub(crate) fn access_target() -> Ident {
        format_ident!("instance_access_explicit_target")
    }

    pub(crate) fn access_doc_target() -> Ident {
        format_ident!("instance_access_explicit_doc_target")
    }
}

// ===============================================================================
// ````````````````````````````````` PARSED ITEM `````````````````````````````````
// ===============================================================================

/// An item processed by the [`crate::inst`] attribute macro.
#[derive(Clone, Debug)]
pub(crate) enum InstItem {
    /// A trait item.
    Trait(ItemTrait),

    /// An implementation item.
    Impl(ItemImpl),

    /// A module item.
    Mod(ItemMod),
}

impl ToTokens for InstItem {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Trait(item) => item.to_tokens(tokens),
            Self::Impl(item) => item.to_tokens(tokens),
            Self::Mod(item) => item.to_tokens(tokens),
        }
    }
}

impl Extraction<TokenStream> for InstItem {
    fn raw_extract(from: &TokenStream, _: &()) -> Result<InstItem, TokenStream> {
        let item = match syn::parse::<Item>(from.clone().into()) {
            Ok(item) => item,
            Err(err) => return Err(err.into_compile_error()),
        };

        let item = match item {
            Item::Impl(item) => InstItem::Impl(item),
            Item::Mod(item) => InstItem::Mod(item),
            Item::Trait(item) => InstItem::Trait(item),
            item => return Err(ProcParseErr::TraitImplModConst { item }.into()),
        };

        Ok(item)
    }

    fn validate_extract(&self, _: &TokenStream, _: Option<&()>) -> Result<(), TokenStream> {
        Ok(())
    }
}

// ===============================================================================
// `````````````````````````````````` IDENT LIST `````````````````````````````````
// ===============================================================================

impl Extraction<TokenStream> for IdentList {
    fn raw_extract(from: &TokenStream, _: &()) -> Result<IdentList, TokenStream> {
        let idents = match syn::parse::<IdentList>(from.clone().into()) {
            Ok(item) => item,
            Err(err) => return Err(err.into_compile_error()),
        };
        idents.duplicate_check(Some(TraitSpace::DuplicateIdent {}.into()))?;

        Ok(idents)
    }

    fn validate_extract(&self, _: &TokenStream, _: Option<&()>) -> Result<(), TokenStream> {
        Ok(())
    }
}

impl Into<InstanceModel> for IdentList {
    fn into(self) -> InstanceModel {
        let mut collect: Punctuated<InstanceIdents, Comma> = Punctuated::new();
        for item in &self.idents {
            let mut inner: Punctuated<InstanceIdent, Comma> = Punctuated::new();
            inner.push(InstanceIdent::Compile(item.clone()));
            let idents = InstanceIdents { span: item.span(), paren_token: Default::default(), params: inner };
            collect.push(idents);
        }
        InstanceModel::Complex(ComplexInstanceRange::Terminated(TerminatedInstances { span: syn::spanned::Spanned::span(&self), bracket_token: Default::default(), params: collect }))
    }
}



// ===============================================================================
// ``````````````````````````````````` INT LIST ``````````````````````````````````
// ===============================================================================

impl Extraction<TokenStream> for IntList {
    fn raw_extract(from: &TokenStream, _: &()) -> Result<IntList, TokenStream> {
        let ints = match syn::parse::<IntList>(from.clone().into()) {
            Ok(item) => item,
            Err(err) => return Err(err.into_compile_error()),
        };
        ints.duplicate_check(Some(TraitSpace::DuplicateIndexes {}.into()))?;

        Ok(ints)
    }

    fn validate_extract(&self, _: &TokenStream, _: Option<&()>) -> Result<(), TokenStream> {
        Ok(())
    }
}

// ===============================================================================
// ```````````````````````````````` INSTANCE SPEC ````````````````````````````````
// ===============================================================================

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ```````````````````````````````` INST SPEC PUNCT ``````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// A separator between instance specifications.
#[derive(Clone, Debug)]
pub enum InstSpecPunct {
    /// Separates specifications with a comma.
    ///
    /// `Crypto[0], Sha256[1]`
    Comma(Token![,]),

    /// Terminates a specification group.
    ///
    /// `Crypto[0], Sha256[1];`
    SemiColon(Token![;]),
}

impl Parse for InstSpecPunct {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(Token![,]) {
            Ok(Self::Comma(input.parse()?))
        } else if input.peek(Token![;]) {
            Ok(Self::SemiColon(input.parse()?))
        } else {
            return Err(ProcParseErr::ExpectedCommaOrSemiColon { span: input.span() }.into());
        }
    }
}

impl ToTokens for InstSpecPunct {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Comma(token) => token.to_tokens(tokens),
            Self::SemiColon(token) => token.to_tokens(tokens),
        }
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ```````````````````````````````` INST SPEC LIST ```````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// A list of instance argument or parameter specifications.
///
/// An argument specification identifies an exact instance:
/// `Crypto[0], Sha256[1]`.
///
/// A parameter specification defines the available instance depth:
/// `Function[3]` represents three ordered instance positions.
#[derive(Clone, Debug)]
pub struct InstSpecList {
    /// Instance argument or parameter specifications.
    pub idents: Punctuated<InstSpec, InstSpecPunct>,
}

impl Parse for InstSpecList {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut idents = Punctuated::<InstSpec, InstSpecPunct>::new();

        while !input.is_empty() {
            idents.push_value(input.parse::<InstSpec>()?);

            if input.peek(Token![,]) {
                idents.push_punct(InstSpecPunct::Comma(input.parse()?));
            } else if input.peek(Token![;]) {
                idents.push_punct(InstSpecPunct::SemiColon(input.parse()?));
            } else {
                break;
            }
        }

        Ok(Self { idents })
    }
}

impl ToTokens for InstSpecList {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.idents.to_tokens(tokens);
    }
}

impl ParseDiagnostic for InstSpecList {
    const KIND: &'static str = "instance specification list";

    const EXPECTED: &'static str = "a comma-separated list of identifiers with a bracketed index or depth, optionally terminated by a semicolon";

    const EXAMPLE: &[&'static str] = &[
        "arguments: \"Crypto[0], Sha256[1];\"",
        "parameters: \"Module[3], Function[2];\"",
    ];

    const DEFAULT_ERROR_INFO: ErrorInfo = ParseError::InstSpecList.to_error_info();
}

impl Extraction<TokenStream> for InstSpecList {
    fn raw_extract(from: &TokenStream, _: &()) -> Result<InstSpecList, TokenStream> {
        syn::parse::<InstSpecList>(from.clone().into()).map_err(|err| err.into_compile_error())
    }

    fn validate_extract(&self, _: &TokenStream, _: Option<&()>) -> Result<(), TokenStream> {
        Ok(())
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ``````````````````````````````````` INST SPEC `````````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// An instance argument or parameter identifier with an index.
///
/// For an instance argument, the index identifies the exact instance.
///
/// `Crypto[2]` refers to instance `2`.
///
/// For an instance parameter, the index defines the available instance
/// depth and therefore the number of ordered instance positions.
///
/// `Module[3]` defines a parameter with three available instance positions.
#[derive(Clone, Debug)]
pub struct InstSpec {
    /// The span covering the specification.
    pub span: Span,

    /// The instance argument or parameter identifier.
    pub ident: Ident,

    /// The brackets enclosing the index.
    ///
    /// In `Crypto[2]`, this represents `[2]`.
    #[allow(unused)]
    pub bracket_token: token::Bracket,

    /// The exact instance index for an argument, or the instance depth
    /// for a parameter.
    ///
    /// For an argument, `Crypto[2]` selects instance index `2`.
    ///
    /// For a parameter, `Module[3]` defines three ordered instance
    /// positions.
    pub index: LitInt,
}

impl Parse for InstSpec {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let start = input.span();

        let ident = input.parse::<Ident>()?;

        let content;
        let bracket_token = bracketed!(content in input);

        let index = content.parse::<LitInt>()?;

        if !content.is_empty() {
            return Err(ProcParseErr::TrailingTokens {
                span: content.span(),
            }
            .into());
        }

        let span = start.join(bracket_token.span.close()).unwrap_or(start);

        Ok(Self {
            span,
            ident,
            bracket_token,
            index,
        })
    }
}

impl ToTokens for InstSpec {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.ident.to_tokens(tokens);

        let lit = syn::LitInt::new(&self.index.to_string(), self.ident.span());

        syn::token::Bracket::default().surround(tokens, |tokens| {
            lit.to_tokens(tokens);
        });
    }
}

impl SpanOf for InstSpec {
    fn span(&self) -> Span {
        self.span
    }
}

// ===============================================================================
// ````````````````````````````` INSTANCE SPEC MIXED `````````````````````````````
// ===============================================================================

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// `````````````````````````````````` MIXED LIST `````````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// A comma-separated list of instance identifiers and indexed specifications.
///
/// ```text
/// Module, Function
/// Module[0], Function[1]
/// Crypto, Sha256[1]
/// ```
#[derive(Clone, Debug)]
pub struct InstMixedList {
    /// The instance identifiers or indexed specifications.
    pub items: Punctuated<InstMixedItem, Comma>,
}

impl Parse for InstMixedList {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            items: Punctuated::parse_terminated(input)?,
        })
    }
}

impl ToTokens for InstMixedList {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.items.to_tokens(tokens);
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// `````````````````````````````````` MIXED ITEM `````````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// An instance identifier or indexed instance specification.
#[derive(Clone, Debug)]
pub enum InstMixedItem {
    /// An instance identifier without an explicit index.
    ///
    /// `Crypto`
    Ident(Ident),

    /// An instance identifier with an explicit index or depth.
    ///
    /// `Crypto[0]`
    Indexed(InstSpec),
}

impl Parse for InstMixedItem {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let fork = input.fork();

        let _ident = fork.parse::<Ident>()?;

        if fork.peek(token::Bracket) {
            return Ok(Self::Indexed(input.parse::<InstSpec>()?));
        }

        Ok(Self::Ident(input.parse()?))
    }
}

impl ToTokens for InstMixedItem {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Ident(ident) => {
                ident.to_tokens(tokens);
            }

            Self::Indexed(item) => {
                item.to_tokens(tokens);
            }
        }
    }
}

// ===============================================================================
// ````````````````````````````` INSTANCE ACCESS ARGS ````````````````````````````
// ===============================================================================

/// An instance access specification for a trait path.
///
/// `Crypto::Hasher(Module, Function[0])`
#[derive(Clone, Debug)]
pub struct InstAccess {
    /// The span covering the complete instance access.
    pub span: Span,

    /// The trait path being accessed.
    ///
    /// `Crypto::Hasher`
    pub path: Punctuated<Ident, PathSep>,

    /// The parentheses enclosing the instance arguments.
    pub paren_token: token::Paren,

    /// The instance identifiers and indexed specifications.
    ///
    /// `Module, Function[0]`
    pub items: InstMixedList,
}

impl Parse for InstAccess {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let start = input.span();

        let mut path = Punctuated::new();

        loop {
            path.push_value(input.parse::<Ident>()?);

            if input.peek(PathSep) {
                path.push_punct(input.parse::<PathSep>()?);
            } else {
                break;
            }
        }

        let content;
        let paren_token = parenthesized!(content in input);

        let items = content.parse::<InstMixedList>()?;

        let span = start.join(paren_token.span.close()).unwrap_or(start);

        Ok(Self {
            span,
            path,
            paren_token,
            items,
        })
    }
}

impl ToTokens for InstAccess {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let path = &self.path;
        let paren_token = &self.paren_token;
        let items = &self.items;

        path.to_tokens(tokens);
        paren_token.surround(tokens, |tokens| {
            items.to_tokens(tokens);
        });
    }
}

impl SpanOf for InstAccess {
    fn span(&self) -> Span {
        self.span
    }
}

impl Extraction<TokenStream> for InstAccess {
    fn raw_extract(from: &TokenStream, _: &()) -> Result<InstAccess, TokenStream> {
        syn::parse::<InstAccess>(from.clone().into()).map_err(|err| err.into_compile_error())
    }

    fn validate_extract(&self, _: &TokenStream, _: Option<&()>) -> Result<(), TokenStream> {
        Ok(())
    }
}

// ===============================================================================
// ````````````````````````````` INSTANCE BOUND ARGS `````````````````````````````
// ===============================================================================

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ```````````````````````````````` BOUND INSTANCE ```````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// An instance bound associating a trait path with an [`InstanceModel`].
///
/// `Crypto::Hasher(Instance)`
#[derive(Debug, Clone)]
pub struct BoundInstance {
    /// The span covering the complete bound.
    pub span: Span,

    /// The trait path being bound.
    ///
    /// `Crypto::Hasher`
    pub path: Punctuated<Ident, PathSep>,

    /// The instance model applied to the trait path.
    pub bound: InstanceModel,
}

impl Parse for BoundInstance {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let start = input.span();

        let mut path = Punctuated::new();

        loop {
            path.push_value(input.parse::<Ident>()?);

            if input.peek(PathSep) {
                path.push_punct(input.parse::<PathSep>()?);
            } else {
                break;
            }
        }

        let bound = input.parse::<InstanceModel>()?;

        let span = start.join(bound.span()).unwrap_or(start);

        Ok(Self { span, path, bound })
    }
}

impl SpanOf for BoundInstance {
    fn span(&self) -> Span {
        self.span
    }
}

impl ToTokens for BoundInstance {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.path.to_tokens(tokens);
        self.bound.to_tokens(tokens);
    }
}

impl Extraction<TokenStream> for BoundInstance {
    fn raw_extract(from: &TokenStream, _: &()) -> Result<BoundInstance, TokenStream> {
        let bound = match syn::parse::<Self>(from.clone().into()) {
            Ok(item) => item,
            Err(err) => return Err(err.into_compile_error()),
        };
        Ok(bound)
    }

    fn validate_extract(&self, _: &TokenStream, _: Option<&()>) -> Result<(), TokenStream> {
        Ok(())
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ```````````````````````````````` INSTANCE MODEL ```````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// An instance model describing either simple or a complex
/// instance parameter/argument range.
///
/// Simple params/args:
/// `(Module, Function)`
///
/// Complex instance specification:
/// `[(Module, Function)]`
///
/// Instance range:
/// `[(Module, Function)..(Module, Other)]`
#[derive(Debug, Clone)]
pub enum InstanceModel {
    /// A single group of instance params/args.
    Simple(InstanceIdents),

    /// Multiple params/args groups or an instance range.
    Complex(ComplexInstanceRange),
}

impl Parse for InstanceModel {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let fork = input.fork();

        if let Ok(complex) = fork.parse::<ComplexInstanceRange>() {
            input.advance_to(&fork);
            return Ok(Self::Complex(complex));
        }

        Ok(Self::Simple(input.parse::<InstanceIdents>()?))
    }
}

impl SpanOf for InstanceModel {
    fn span(&self) -> Span {
        match self {
            Self::Simple(x) => x.span(),
            Self::Complex(x) => x.span(),
        }
    }
}

impl ToTokens for InstanceModel {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            Self::Simple(params) => params.to_tokens(tokens),
            Self::Complex(arguments) => arguments.to_tokens(tokens),
        }
    }
}

impl Extraction<TokenStream> for InstanceModel {
    fn raw_extract(from: &TokenStream, _: &()) -> Result<InstanceModel, TokenStream> {
        let bound = match syn::parse::<Self>(from.clone().into()) {
            Ok(item) => item,
            Err(err) => return Err(err.into_compile_error()),
        };
        Ok(bound)
    }

    fn validate_extract(&self, _: &TokenStream, _: Option<&()>) -> Result<(), TokenStream> {
        Ok(())
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ``````````````````````````````` INSTANCE IDENTS ```````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// A parenthesized group of instance params/args.
///
/// Each param/arg represents one component of an instance specification.
///
/// `(Module, Function, _)`
#[derive(Debug, Clone)]
pub struct InstanceIdents {
    /// The span covering the group.
    pub span: Span,

    /// The parentheses enclosing the parameters.
    pub paren_token: token::Paren,

    /// The params/args in declaration order.
    pub params: Punctuated<InstanceIdent, Token![,]>,
}

impl Parse for InstanceIdents {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let start = input.span();

        let content;
        let paren_token = parenthesized!(content in input);
        let params = Punctuated::parse_terminated(&content)?;

        let span = start.join(paren_token.span.close()).unwrap_or(start);

        Ok(Self {
            span,
            paren_token,
            params,
        })
    }
}

impl SpanOf for InstanceIdents {
    fn span(&self) -> Span {
        self.span
    }
}

impl ToTokens for InstanceIdents {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.paren_token.surround(tokens, |tokens| {
            self.params.to_tokens(tokens);
        });
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ``````````````````````````` COMPLEX INSTANCE ARGS `````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// A complex instance model consisting of either terminated parameter
/// or arguments groups or an instance range.
///
/// Multiple groups:
/// `[(Module, Function), (Crypto, Hash)]`
///
/// Range:
/// `[(Module, Function)..(Module, Hash)]`
#[derive(Debug, Clone)]
pub enum ComplexInstanceRange {
    /// Multiple explicitly terminated parameter groups.
    Terminated(TerminatedInstances),

    /// A range between two instance parameter groups.
    Range(InstanceRange),
}

impl Parse for ComplexInstanceRange {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let fork = input.fork();

        if let Ok(range) = fork.parse::<InstanceRange>() {
            input.advance_to(&fork);
            return Ok(Self::Range(range));
        }

        Ok(Self::Terminated(input.parse::<TerminatedInstances>()?))
    }
}

impl SpanOf for ComplexInstanceRange {
    fn span(&self) -> Span {
        match self {
            Self::Terminated(term) => term.span(),
            Self::Range(range) => range.span(),
        }
    }
}

impl ToTokens for ComplexInstanceRange {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            Self::Terminated(terminated) => terminated.to_tokens(tokens),
            Self::Range(range) => range.to_tokens(tokens),
        }
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ```````````````````````````` TERMINATED INSTANCES `````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// A bracketed sequence of instance parameter or arguments groups.
///
/// `[(Module, Function), (Crypto, Hash)]`
#[derive(Debug, Clone)]
pub struct TerminatedInstances {
    /// The span covering the complete group.
    pub span: Span,

    /// The brackets enclosing the parameter groups.
    pub bracket_token: token::Bracket,

    /// The parameter groups in order.
    pub params: Punctuated<InstanceIdents, Token![,]>,
}

impl Parse for TerminatedInstances {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let start = input.span();

        let content;
        let bracket_token = bracketed!(content in input);

        let params = Punctuated::parse_terminated(&content)?;

        let end = params
            .last()
            .map(|param: &InstanceIdents| param.span())
            .unwrap_or(start);

        let span = start.join(end).unwrap_or(start);

        Ok(Self {
            span,
            bracket_token,
            params,
        })
    }
}

impl SpanOf for TerminatedInstances {
    fn span(&self) -> Span {
        self.span
    }
}

impl ToTokens for TerminatedInstances {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.bracket_token.surround(tokens, |tokens| {
            self.params.to_tokens(tokens);
        });
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// `````````````````````````````` RANGED INSTANCES ```````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// A range between two instance parameter groups.
///
/// Either endpoint may be omitted.
///
/// `[(Module, Function)..(Module, Hash)]`
///
/// `[..(Module, Hash)]`
///
/// `[(Module, Function)..]`
#[derive(Debug, Clone)]
pub struct InstanceRange {
    /// The span covering the complete range.
    pub span: Span,

    /// The brackets enclosing the range.
    pub bracket_token: token::Bracket,

    /// The starting parameter/argument group, if present.
    pub start: Option<InstanceIdents>,

    /// The range operator.
    pub dotdot_token: Token![..],

    /// The ending parameter/argument group, if present.
    pub end: Option<InstanceIdents>,
}

impl Parse for InstanceRange {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let start = input.span();

        let content;
        let bracket_token = bracketed!(content in input);

        let start_params = if content.peek(token::Paren) {
            Some(content.parse::<InstanceIdents>()?)
        } else {
            None
        };

        let dotdot_token = content.parse::<Token![..]>()?;

        let end_params = if content.peek(token::Paren) {
            Some(content.parse::<InstanceIdents>()?)
        } else {
            None
        };

        let span = start
            .join(end_params.as_ref().map_or(
                syn::spanned::Spanned::span(&dotdot_token),
                syn::spanned::Spanned::span,
            ))
            .unwrap_or(start);

        Ok(Self {
            span,
            bracket_token,
            start: start_params,
            dotdot_token,
            end: end_params,
        })
    }
}

impl SpanOf for InstanceRange {
    fn span(&self) -> Span {
        self.span
    }
}

impl ToTokens for InstanceRange {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.bracket_token.surround(tokens, |tokens| {
            if let Some(start) = &self.start {
                start.to_tokens(tokens);
            }

            self.dotdot_token.to_tokens(tokens);

            if let Some(end) = &self.end {
                end.to_tokens(tokens);
            }
        });
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ``````````````````````````````` INSTANCE IDENT ````````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// A single instance parameter or argument.
///
/// An instance may be identified at compile time, supplied dynamically,
/// or inferred.
///
/// # Examples
///
/// Compile-time identifier:
/// `Module`
///
/// Runtime expression:
/// `{expr}`
///
/// Inferred parameter:
/// `_`
#[derive(Debug, Clone)]
pub enum InstanceIdent {
    /// A compile-time instance identifier.
    Compile(Ident),

    /// A runtime instance expression.
    Runtime(DynamicIdent),

    /// An inferred instance parameter.
    Infer(Token![_]),
}

impl Parse for InstanceIdent {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(Token![_]) {
            return Ok(Self::Infer(input.parse()?));
        }

        if input.peek(Ident) {
            return Ok(Self::Compile(input.parse()?));
        }

        if input.peek(token::Brace) {
            return Ok(Self::Runtime(input.parse()?));
        }

        return Err(ProcParseErr::ExpectedInstanceIdent { span: input.span() }.into());
    }
}

impl SpanOf for InstanceIdent {
    fn span(&self) -> Span {
        match self {
            Self::Compile(ident) => ident.span(),
            Self::Runtime(param) => param.span(),
            Self::Infer(infer) => syn::spanned::Spanned::span(infer),
        }
    }
}

impl ToTokens for InstanceIdent {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            Self::Compile(ident) => ident.to_tokens(tokens),
            Self::Runtime(param) => param.to_tokens(tokens),
            Self::Infer(infer) => infer.to_tokens(tokens),
        }
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// `````````````````````````````` DYNAMIC INSTANCE ```````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// A runtime instance expression enclosed in braces.
///
/// `{module_id}`
#[derive(Debug, Clone)]
pub struct DynamicIdent {
    /// The span covering the expression.
    pub span: Span,

    /// The braces enclosing the expression.
    pub brace: token::Brace,

    /// The ident-placeholder which will be supplied at runtime.
    pub ident: Ident,
}

impl Parse for DynamicIdent {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let span = input.span();

        let content;
        let brace = braced!(content in input);
        let ident = content.parse()?;

        Ok(Self { span, brace, ident })
    }
}

impl ToTokens for DynamicIdent {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.brace.surround(tokens, |tokens| {
            self.ident.to_tokens(tokens);
        });
    }
}

impl SpanOf for DynamicIdent {
    fn span(&self) -> Span {
        self.span
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ````````````````````````````` INSTANCE MODEL LENGTH ```````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// The parameter arity of an [`InstanceModel`] utilized via [`Extraction::checked_extract`].
///
/// This is the number of instance positions defined by each parameter
/// group, not the number of possible instance indexes.
///
/// For a simple model, this is the number of parameters in its single
/// parameter group.
///
/// For a complex model, all parameter groups must have the same length,
/// and the first available group determines the model length.
///
/// `(Module, Function)` has length `2`.
///
/// `[(Module, Function), (Crypto, Hash)]` has length `2`.
///
/// `[(Module, Function)..(Crypto, Hash)]` has length `2`.
#[derive(Debug, Clone)]
pub struct InstModelLength(pub usize);

impl Extraction<InstanceModel> for InstModelLength {
    fn raw_extract(from: &InstanceModel, _: &()) -> Result<Self, proc_macro2::TokenStream> {
        from.atleast_one()?;
        from.no_range_roots()?;
        from.same_length_elems()?;
        match from {
            InstanceModel::Simple(simple) => Ok(Self(simple.params.len())),
            InstanceModel::Complex(complex) => match complex {
                ComplexInstanceRange::Terminated(term) => Ok(Self(term.params[0].params.len())),
                ComplexInstanceRange::Range(range) => match (&range.start, &range.end) {
                    (None, None) => unreachable!(),
                    (None, Some(end)) => Ok(Self(end.params.len())),
                    (Some(start), _) => Ok(Self(start.params.len())),
                },
            },
        }
    }

    fn validate_extract(
        &self,
        _: &InstanceModel,
        _: Option<&()>,
    ) -> Result<(), proc_macro2::TokenStream> {
        Ok(())
    }
}

// ===============================================================================
// ``````````````````````````````` BSTR-INPUT ARGS ```````````````````````````````
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

// ===============================================================================
// `````````````````````````````` INSTANCE NODE ARGS `````````````````````````````
// ===============================================================================

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ``````````````````````````````````` NODE ARGS `````````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// Lowered node arguments derived from an instance model and instance indexes.
///
/// The [`InstanceModel`] is resolved against the supplied [`IntList`] and
/// converted into a node-specific representation for code generation via
/// [`Extraction::checked_extract`].
///
/// # Terminology
///
/// - **Concrete instance** - A specific instance identified by an exact
///   instance index.
/// - **Parent instance** - An instance whose position represents a group of
///   all dynamic child instances rather than one concrete instance.
/// - **Concrete boundary** - A boundary that resolves to a specific concrete
///   instance.
/// - **Parent boundary** - A boundary expressed at a parent instance and
///   resolved to that parent's last concrete child instance.
/// - **Concrete source** - A source that resolves to a specific concrete
///   instance.
/// - **Parent source** - A source that resolves to a parent instance and
///   therefore applies across its dynamic children.
/// - **Last instance** - The final concrete instance available within the
///   relevant instance sequence.
/// - **Global last instance** - The final concrete instance available across
///   the complete instance model.
///
/// The node variants use these concepts to distinguish whether an operation
/// starts from or terminates at a concrete instance or a parent instance.
#[derive(Debug, Clone)]
pub enum NodeArgs {
    /// Represents concrete instances by their indexes and identifiers.
    ///
    /// ```ignore
    /// leaf {
    ///     index(indexes),
    ///     ident[(idents)],
    /// }
    /// ```
    Leaf(LeafNode),

    /// Represents movement from a parent to all of its dynamic child instances.
    ///
    /// ```ignore
    /// branch {
    ///     index(indexes),
    ///     parent(parents),
    /// }
    /// ```
    Branch(BranchNode),

    /// Represents pruning instances up to a concrete instance boundary.
    ///
    /// ```ignore
    /// prune {
    ///     index(indexes),
    ///     until(until),
    /// }
    /// ```
    Prune(PruneNode),

    /// Represents trimming instances up to a parent instance boundary,
    /// resolving the boundary to the parent's last concrete child instance.
    ///
    /// ```ignore
    /// trim {
    ///     index(indexes),
    ///     until(until),
    /// }
    /// ```
    Trim(TrimNode),

    /// Represents extending instances from a concrete instance source to
    /// the globally last available instance.
    ///
    /// ```ignore
    /// extend {
    ///     index(indexes),
    ///     from(from),
    /// }
    /// ```
    Extend(ExtendNode),

    /// Represents spreading instances from a parent instance source to
    /// the globally last available instance.
    ///
    /// ```ignore
    /// spread {
    ///     index(indexes),
    ///     from(from),
    /// }
    /// ```
    Spread(SpreadNode),

    /// Represents the root instance indexes, covering all available
    /// instances without requiring identifiers.
    ///
    /// ```ignore
    /// root {
    ///     index(indexes),
    /// }
    /// ```
    Root(RootNode),

    /// Represents traversal from a concrete instance source to a parent
    /// boundary, resolving the boundary to its last concrete child instance.
    ///
    /// ```ignore
    /// traverse {
    ///     index(indexes),
    ///     from(from),
    ///     until(until),
    /// }
    /// ```
    Traverse(TraverseNode),

    /// Represents descent from a parent instance source to a concrete
    /// instance boundary.
    ///
    /// ```ignore
    /// descend {
    ///     index(indexes),
    ///     from(from),
    ///     until(until),
    /// }
    /// ```
    Descend(DescendNode),
}

impl Extraction<InstanceModel, IntList> for NodeArgs {
    fn raw_extract(
        from: &InstanceModel,
        context: &IntList,
    ) -> Result<Self, proc_macro2::TokenStream> {
        let mut node = NodeArgsRef(from.clone());
        node.try_node(context)
    }

    fn validate_extract(
        &self,
        _: &InstanceModel,
        _: Option<&IntList>,
    ) -> Result<(), proc_macro2::TokenStream> {
        Ok(())
    }
}

impl ToTokens for NodeArgs {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Leaf(access) => access.to_tokens(tokens),
            Self::Branch(access) => access.to_tokens(tokens),
            Self::Prune(access) => access.to_tokens(tokens),
            Self::Trim(access) => access.to_tokens(tokens),
            Self::Extend(access) => access.to_tokens(tokens),
            Self::Spread(access) => access.to_tokens(tokens),
            Self::Root(access) => access.to_tokens(tokens),
            Self::Traverse(access) => access.to_tokens(tokens),
            Self::Descend(access) => access.to_tokens(tokens),
        }
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ``````````````````````````````````` LEAF NODE `````````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// Represents concrete instances selected by their indexes and identifiers.
///
/// `indexes` identifies the concrete instance positions, while `idents`
/// provides the corresponding instance identifiers used to resolve them.
///
/// ```ignore
/// leaf {
///     index(indexes),
///     ident[(idents)],
/// }
/// ```
#[derive(Debug, Clone, Default)]
pub(crate) struct LeafNode {
    /// Concrete instance indexes.
    pub(crate) indexes: IntList,

    /// Identifiers of the concrete instances at the corresponding indexes.
    pub(crate) idents: Vec<BStringList>,
}

impl ToTokens for LeafNode {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let indexes = &self.indexes;
        let idents = &self.idents;

        quote! {
            leaf {
                index(#indexes),
                ident [
                    #(
                        (#idents)
                    ),*
                ]
            }
        }
        .to_tokens(tokens);
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// `````````````````````````````````` BRANCH NODE ````````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// Represents movement from a parent instance to its dynamic child instances.
///
/// `indexes` identifies the parent positions, while `parents` identifies the
/// parent instances from which the child instances are resolved.
///
/// ```ignore
/// branch {
///     index(indexes),
///     parent(parents),
/// }
/// ```
#[derive(Debug, Clone, Default)]
pub(crate) struct BranchNode {
    /// Indexes of the parent instances.
    pub(crate) indexes: IntList,

    /// Parent instance identifiers.
    pub(crate) parents: BStringList,
}

impl ToTokens for BranchNode {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let indexes = &self.indexes;
        let parents = &self.parents;
        quote! {
            branch {
                index(#indexes),
                parent(#parents),
            }
        }
        .to_tokens(tokens);
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ``````````````````````````````````` PRUNE NODE ````````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// Represents pruning from an instance sequence up to a concrete boundary.
///
/// `indexes` identifies the instances being operated on, while `until`
/// identifies the concrete instance at which pruning terminates.
///
/// ```ignore
/// prune {
///     index(indexes),
///     until(until),
/// }
/// ```
#[derive(Debug, Clone, Default)]
pub(crate) struct PruneNode {
    /// Indexes of the instances being pruned.
    pub(crate) indexes: IntList,

    /// Concrete pruning boundaries.
    pub(crate) until: BStringList,
}

impl ToTokens for PruneNode {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let indexes = &self.indexes;
        let until = &self.until;
        quote! {
            prune {
                index(#indexes),
                until(#until),
            }
        }
        .to_tokens(tokens);
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// `````````````````````````````````` TRIM NODE ``````````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// Represents trimming from an instance sequence up to a parent boundary.
///
/// The parent boundary is resolved to the parent's last concrete child
/// instance.
///
/// ```ignore
/// trim {
///     index(indexes),
///     until(until),
/// }
/// ```
#[derive(Debug, Clone, Default)]
pub(crate) struct TrimNode {
    /// Indexes of the instances being trimmed.
    pub(crate) indexes: IntList,

    /// Parent boundaries resolved to their last concrete child instances.
    pub(crate) until: BStringList,
}

impl ToTokens for TrimNode {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let indexes = &self.indexes;
        let until = &self.until;
        quote! {
            trim {
                index(#indexes),
                until(#until),
            }
        }
        .to_tokens(tokens);
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ````````````````````````````````` EXTEND NODE `````````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// Represents extending from a concrete instance to the globally last
/// available instance.
///
/// `from` identifies the concrete source instance, and the global instance
/// model determines the final boundary.
///
/// ```ignore
/// extend {
///     index(indexes),
///     from(from),
/// }
/// ```
#[derive(Debug, Clone, Default)]
pub(crate) struct ExtendNode {
    /// Indexes of the instances being extended.
    pub(crate) indexes: IntList,

    /// Concrete source instance identifiers.
    pub(crate) from: BStringList,
}

impl ToTokens for ExtendNode {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let indexes = &self.indexes;
        let from = &self.from;

        quote! {
            extend {
                index(#indexes),
                from(#from),
            }
        }
        .to_tokens(tokens);
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ````````````````````````````````` SPREAD NODE `````````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// Represents spreading from a parent instance to the globally last
/// available instance.
///
/// `from` identifies the parent source whose dynamic children are included.
///
/// ```ignore
/// spread {
///     index(indexes),
///     from(from),
/// }
/// ```
#[derive(Debug, Clone, Default)]
pub(crate) struct SpreadNode {
    /// Indexes of the instances being spread.
    pub(crate) indexes: IntList,

    /// Parent source instance identifiers.
    pub(crate) from: BStringList,
}

impl ToTokens for SpreadNode {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let indexes = &self.indexes;
        let from = &self.from;

        quote! {
            spread {
                index(#indexes),
                from(#from),
            }
        }
        .to_tokens(tokens);
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// `````````````````````````````````` ROOT NODE ``````````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// Represents the root of the instance model.
///
/// The root covers all available instances, so no instance identifiers are
/// required.
///
/// ```ignore
/// root {
///     index(indexes),
/// }
/// ```
#[derive(Debug, Clone, Default)]
pub(crate) struct RootNode {
    /// Indexes covering the available root instances.
    pub(crate) indexes: IntList,
}

impl ToTokens for RootNode {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let indexes = &self.indexes;
        quote! {
            root {
                index(#indexes),
            }
        }
        .to_tokens(tokens);
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ```````````````````````````````` TRAVERSE NODE ````````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// Represents traversal from a concrete instance source to a parent boundary.
///
/// The parent boundary is resolved to its last concrete child instance.
///
/// ```ignore
/// traverse {
///     index(indexes),
///     from(from),
///     until(until),
/// }
/// ```
#[derive(Debug, Clone, Default)]
pub(crate) struct TraverseNode {
    /// Indexes of the instances being traversed.
    pub(crate) indexes: IntList,

    /// Concrete source instance identifiers.
    pub(crate) from: BStringList,

    /// Parent boundaries resolved to their last concrete child instances.
    pub(crate) until: BStringList,
}

impl ToTokens for TraverseNode {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let indexes = &self.indexes;
        let from = &self.from;
        let until = &self.until;

        quote! {
            traverse {
                index(#indexes),
                from(#from),
                until(#until),
            }
        }
        .to_tokens(tokens);
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ```````````````````````````````` DESCEND NODE `````````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// Represents descent from a parent instance source to a concrete boundary.
///
/// The parent source resolves through its dynamic child instances until the
/// specified concrete boundary.
///
/// ```ignore
/// descend {
///     index(indexes),
///     from(from),
///     until(until),
/// }
/// ```
#[derive(Debug, Clone, Default)]
pub(crate) struct DescendNode {
    /// Indexes of the instances being descended through.
    pub(crate) indexes: IntList,

    /// Parent source instance identifiers.
    pub(crate) from: BStringList,

    /// Concrete instance boundaries.
    pub(crate) until: BStringList,
}

impl ToTokens for DescendNode {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let indexes = &self.indexes;
        let from = &self.from;
        let until = &self.until;

        quote! {
            descend {
                index(#indexes),
                from(#from),
                until(#until),
            }
        }
        .to_tokens(tokens);
    }
}

// ===============================================================================
// `````````````````````````````` INSTANCE ACCESS ARGS ```````````````````````````
// ===============================================================================

/// Concrete access arguments derived from a resolved instance model.
///
/// Unlike [`NodeArgs`], these arguments contain the concrete identifiers
/// required by the generated access operation. The representation is reduced
/// to the access form needed by code generation.
///
/// - [`AccessArgs::Leaf`] accesses concrete instances directly.
/// - [`AccessArgs::Branch`] accesses concrete children through concrete
///   parent prefixes.
/// - [`AccessArgs::Root`] accesses dynamically resolved instances from the
///   root.
#[derive(Debug, Clone)]
pub enum AccessArgs {
    /// Direct access to concrete instances.
    ///
    /// ```ignore
    /// leaf {
    ///     index(indexes),
    ///     ident(idents)
    /// }
    /// ```
    Leaf(LeafAccess),

    /// Access to concrete children through parent prefixes.
    ///
    /// ```ignore
    /// branch {
    ///     index(indexes),
    ///     parent(parents),
    ///     child(childs),
    ///     depth(depths),
    /// }
    /// ```
    Branch(PrefixAccess),

    /// Full Dynamic access rooted at the complete instance model.
    ///
    /// ```ignore
    /// root {
    ///     index(indexes),
    ///     ident(idents),
    ///     depth(depths),
    /// }
    /// ```
    Root(DynAccess),
}

impl Extraction<InstanceModel, (&InstMixedList, &IntList)> for AccessArgs {
    fn raw_extract(
        from: &InstanceModel,
        context: &(&InstMixedList, &IntList),
    ) -> Result<Self, proc_macro2::TokenStream> {
        let mut node = AccessArgsRef(from.clone());
        node.try_access(context.0, context.1)
    }

    fn validate_extract(
        &self,
        _: &InstanceModel,
        _: Option<&(&InstMixedList, &IntList)>,
    ) -> Result<(), proc_macro2::TokenStream> {
        Ok(())
    }
}

impl ToTokens for AccessArgs {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Leaf(access) => access.to_tokens(tokens),
            Self::Branch(access) => access.to_tokens(tokens),
            Self::Root(access) => access.to_tokens(tokens),
        }
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ```````````````````````````````` LEAF ACCESS ``````````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// Direct access to concrete instances.
///
/// Each identifier corresponds to a concrete instance at the associated
/// index.
#[derive(Debug, Clone, Default)]
pub(crate) struct LeafAccess {
    /// Concrete instance indexes.
    pub(crate) indexes: IntList,

    /// Concrete instance identifiers.
    pub(crate) idents: BStringList,
}

impl ToTokens for LeafAccess {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let indexes = &self.indexes;
        let idents = &self.idents;

        quote! {
            leaf {
                index(#indexes),
                ident(#idents)
            }
        }
        .to_tokens(tokens);
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ```````````````````````````````` PREFIX ACCESS ````````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// Optimized access through an instance prefix.
///
/// Branch, traverse, and descend accesses are lowered into this common
/// prefix representation. The prefix captures the parent path and the child
/// resolved from it, eliminating the need to retain the original operation
/// kind.
///
/// The optional depth records the prefix depth when required.
#[derive(Debug, Clone, Default)]
pub(crate) struct PrefixAccess {
    /// Concrete instance indexes associated with the access.
    pub(crate) indexes: IntList,

    /// Parent identifiers forming the concrete access prefix.
    pub(crate) parents: BStringList,

    /// Child identifiers or expressions resolved from the prefix.
    pub(crate) childs: BStrInputList,

    /// Optional depth of the access prefix.
    pub(crate) depths: Option<IntList>,
}

impl ToTokens for PrefixAccess {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let indexes = &self.indexes;
        let parents = &self.parents;
        let childs = &self.childs;
        let depth = self.depths.as_ref().map(|depth| {
            quote! {
                depth(#depth),
            }
        });

        quote! {
            branch {
                index(#indexes),
                parent(#parents),
                child(#childs),
                #depth
            }
        }
        .to_tokens(tokens);
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ``````````````````````````````` DYNAMIC ACCESS ````````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// Dynamic access over the instance model.
///
/// Root, prune, trim, extend, spread, and other dynamic access forms are
/// lowered into this representation when their access can be expressed
/// through instance identifiers and optional depths.
///
/// Unlike [`LeafAccess`], the identifiers shall require dynamic resolution
/// against the instance model.
#[derive(Debug, Clone, Default)]
pub(crate) struct DynAccess {
    /// Instance indexes associated with the access.
    pub(crate) indexes: IntList,

    /// Instance identifiers or expressions used for dynamic resolution.
    pub(crate) idents: BStrInputList,

    /// Optional depths used to resolve the dynamic instances.
    pub(crate) depths: Option<IntList>,
}

impl ToTokens for DynAccess {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let indexes = &self.indexes;
        let idents = &self.idents;

        let depth = self.depths.as_ref().map(|depth| {
            quote! {
                depth(#depth),
            }
        });

        quote! {
            root {
                index(#indexes),
                ident(#idents),
                #depth
            }
        }
        .to_tokens(tokens);
    }
}

// ===============================================================================
// `````````````````````````````` INSTANCE DIRECT ARGS ```````````````````````````
// ===============================================================================

/// Arguments for directly supplying an instance concrete type access expression.
///
/// The expression provides the instance value directly, while the optional
/// identifiers specify the instance positions associated with the value.
///
/// ```ignore
/// <Concrete as InstTrait<..>>::Access, Crypto, Sha256
/// ```
#[derive(Debug, Clone)]
pub(crate) struct InstDirectArgs {
    /// The expression supplying the instance trait item access.
    pub(crate) expr: Expr,

    /// Instance arguments associated with the instance trait implementations.
    pub(crate) idents: IdentList,
}

impl Parse for InstDirectArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let expr = input.parse::<Expr>()?;

        let idents = if input.peek(Comma) {
            input.parse::<Comma>()?;
            input.parse::<IdentList>()?
        } else {
            IdentList::default()
        };

        if !input.is_empty() {
            return Err(ProcParseErr::TrailingTokens { span: input.span() }.into());
        }
        Ok(Self { expr, idents })
    }
}

impl Extraction<proc_macro2::TokenStream> for InstDirectArgs {
    fn raw_extract(
        from: &proc_macro2::TokenStream,
        _: &(),
    ) -> Result<Self, proc_macro2::TokenStream> {
        syn::parse2::<Self>(from.clone()).map_err(|e| e.to_compile_error())
    }

    fn validate_extract(
        &self,
        _: &proc_macro2::TokenStream,
        _: Option<&()>,
    ) -> Result<(), proc_macro2::TokenStream> {
        Ok(())
    }
}

// ===============================================================================
// `````````````````````````````` INSTANCE GET ARGS ``````````````````````````````
// ===============================================================================

/// Arguments for obtaining an instance through an associated type accessor path.
///
/// The path identifies the instance-node associated type, the expression
/// identifies the item in the special `inst` module to invoke, and the
/// optional identifiers or expressions specify the instance positions as
/// `&[u8]` values associated with the accessed value.
///
/// ```ignore
/// <T as NodeTrait<...>>::InstNode, inst_module::func(..), variable, {EXPR}
/// ```
#[derive(Debug, Clone)]
pub(crate) struct InstGetArgs {
    /// The associated type path identifying the instance node.
    pub(crate) path: ExprPath,

    /// The `inst` module item expression used to obtain the instance.
    pub(crate) expr: Expr,

    /// Instance identifiers or expressions specifying the accessed positions.
    pub(crate) inputs: BStrInputList,
}

impl Parse for InstGetArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let path = input.parse::<ExprPath>()?;

        input.parse::<Comma>()?;

        let expr = input.parse::<Expr>()?;

        let inputs = if input.peek(Comma) {
            input.parse::<Comma>()?;
            input.parse::<BStrInputList>()?
        } else {
            BStrInputList::default()
        };

        if !input.is_empty() {
            return Err(ProcParseErr::TrailingTokens { span: input.span() }.into());
        }
        Ok(Self { path, expr, inputs })
    }
}

impl Extraction<proc_macro2::TokenStream> for InstGetArgs {
    fn raw_extract(
        from: &proc_macro2::TokenStream,
        _: &(),
    ) -> Result<Self, proc_macro2::TokenStream> {
        syn::parse2::<Self>(from.clone()).map_err(|e| e.to_compile_error())
    }

    fn validate_extract(
        &self,
        _: &proc_macro2::TokenStream,
        _: Option<&()>,
    ) -> Result<(), proc_macro2::TokenStream> {
        Ok(())
    }
}

// ===============================================================================
// ````````````````````````````` INSTANCE TUPLE ARGS `````````````````````````````
// ===============================================================================

/// Arguments for declaring a tuple of instance identifiers.
///
/// Each entry consists of an optional visibility followed by the identifier
/// representing the instance position.
///
/// ```ignore
/// pub Foo, Bar, pub(crate) Baz
/// ```
#[derive(Debug, Clone)]
pub(crate) struct InstTupleArgs {
    pub(crate) span: Span,

    /// The instance tuple entries.
    pub(crate) list: Punctuated<InstTuple, Comma>,
}

impl InstTupleArgs {
    /// List all the tuple idents into a vector for documentation purposes
    pub fn list(&self) -> Vec<Ident> {
        self.list.iter().map(|tup| tup.ident.clone()).collect()
    }
}

/// A single instance tuple entry.
///
/// The optional visibility controls the visibility of the generated instance
/// item, while the identifier names the instance position.
///
/// ```ignore
/// pub Foo
/// Bar
/// pub(crate) Baz
/// ```
#[derive(Debug, Clone)]
pub(crate) struct InstTuple {
    pub(crate) span: Span,

    /// Visibility of the generated instance item.
    pub(crate) visibility: Visibility,

    /// Identifier naming the instance position.
    pub(crate) ident: Ident,
}

impl Parse for InstTupleArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let span = input.span();

        let list = Punctuated::<InstTuple, Comma>::parse_terminated(input)?;

        let span = span.join(input.span()).unwrap_or(span);

        Ok(Self { span, list })
    }
}

impl SpanOf for InstTupleArgs {
    fn span(&self) -> Span {
        self.span
    }
}

impl Parse for InstTuple {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let span = input.span();

        let visibility = input.parse::<Visibility>()?;
        let ident = input.parse::<Ident>()?;

        let span = span.join(ident.span()).unwrap_or(span);

        Ok(Self {
            span,
            visibility,
            ident,
        })
    }
}

impl SpanOf for InstTuple {
    fn span(&self) -> Span {
        self.span
    }
}

impl Into<InstanceModel> for InstTupleArgs {
    fn into(self) -> InstanceModel {
        let mut collect: Punctuated<InstanceIdents, Comma> = Punctuated::new();
        for item in &self.list {
            let mut inner: Punctuated<InstanceIdent, Comma> = Punctuated::new();
            inner.push(InstanceIdent::Compile(item.ident.clone()));
            let idents = InstanceIdents { span: item.span(), paren_token: Default::default(), params: inner };
            collect.push(idents);
        }
        InstanceModel::Complex(ComplexInstanceRange::Terminated(TerminatedInstances { span: self.span(), bracket_token: Default::default(), params: collect }))
    }
}



impl Extraction<proc_macro2::TokenStream> for InstTupleArgs {
    fn raw_extract(
        from: &proc_macro2::TokenStream,
        _: &(),
    ) -> Result<Self, proc_macro2::TokenStream> {
        let tuples = syn::parse2::<Self>(from.clone()).map_err(|e| e.to_compile_error())?;
        let mut idents = IdentList::default();
        for tuple in &tuples.list {
            let ident = tuple.ident.clone();
            idents.idents.push(ident);
        }
        idents.duplicate_check(Some(TraitSpace::DuplicateIdent {}.into()))?;
        Ok(tuples)
    }

    fn validate_extract(
        &self,
        _: &proc_macro2::TokenStream,
        _: Option<&()>,
    ) -> Result<(), proc_macro2::TokenStream> {
        Ok(())
    }
}