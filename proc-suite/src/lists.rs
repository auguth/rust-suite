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
// ````````````````````````` LIST (PUNCTUATED) ARGUMENTS `````````````````````````
// ===============================================================================

//! Parse comma-separated lists with built-in diagnostics.
//!
//! - Use provided list types directly with [`Parse`]
//! - Parse failures automatically emit structured diagnostics via [`ParseDiagnostic`]
//! - Use [`DuplicateCheck::duplicate_check`] to validate and report duplicates

// ===============================================================================
// ``````````````````````````````````` IMPORTS ```````````````````````````````````
// ===============================================================================

// --- Std Lib ---
use std::{collections::HashSet, fmt::Debug};

// --- Local crate ---
use crate::{
    errors::*,
    keys::{KeyValue, ValueGroup},
    space::*,
};

// --- Proc-macro2 ---
use proc_macro2::TokenStream;

// --- Syn/Quote ---
use quote::ToTokens;
use syn::{
    Expr, Ident, Lit, LitByteStr, LitInt, Stmt, WherePredicate,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    token::Comma,
};

// ===============================================================================
// ``````````````````````````````` LOW-LEVEL LISTS ```````````````````````````````
// ===============================================================================

/// A wrapper around a `Punctuated<WherePredicate, Comma>`
/// representing a list of trait/lifetime bounds.
///
/// ## Example
/// ```ignore
/// T: Clone,
/// U: Send + Sync,
/// 'a: 'b,
/// for<'x> &'x T: Debug
/// ```
///
/// ---
///
/// ***Note**: If [`Debug`] is required then enable [`syn`] cfg feature "extra-traits"*
#[derive(Clone, Debug, Default)]
pub struct BoundsList {
    pub bounds: Punctuated<WherePredicate, Comma>,
}

/// A wrapper around a [`Punctuated<Expr, Comma>`] representing a
/// comma-separated list of expressions.
///
/// ## Example
/// ```ignore
/// foo, bar, b"hello", make_bytes(), 1 + 2
/// ```
///
/// In this case, `exprs` contains each expression as a [`syn::Expr`].
///
/// ---
///
/// ***Note****: If [`Debug`] is required then enable [`syn`] cfg feature "extra-traits"*
#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub struct ExprList {
    pub exprs: Punctuated<Expr, Comma>,
}

/// A wrapper around a [`Punctuated<Stmt, Comma>`] representing a
/// comma-separated list of Rust statements.
///
/// ## Example
/// ```ignore
/// let value = 8;, Tuple(value), foo();
/// ```
///
/// In this case, `stmts` contains each statement as a [`syn::Stmt`].
///
/// ---
///
/// ***Note**: If [`Debug`] is required then enable [`syn`] cfg feature "extra-traits"*
#[derive(Clone, Debug, Default)]
pub struct StmtList {
    pub stmts: Punctuated<Stmt, Comma>,
}

// ===============================================================================
// ``````````````````````````````` HIGH-LEVEL LISTS ``````````````````````````````
// ===============================================================================

/// A wrapper around a [`Punctuated<LitInt, Comma>`] representing a list
/// of integer literals provided to parse.
///
/// Uses [`LitInt`], which parses integer literal tokens (e.g. `1`, `42`, `0xff`).
///
/// Negative values (e.g. `-1`) are **not supported**, as the `-` is parsed
/// separately as a unary operator and not part of the literal.
///
/// ## Example
/// ```ignore
/// 1, 2, 0xff
/// ```
///
/// In this case, `ints` would contain the literals `1`, `2`, and `0xff`.
///
/// ---
///
/// ***Note**: If [`Debug`] is required then enable [`syn`] cfg feature "extra-traits"*
#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub struct IntList {
    pub ints: Punctuated<LitInt, Comma>,
}

/// A wrapper around a [`Punctuated<LitByteStr, Comma>`] representing a list
/// of byte string literals provided to parse.
///
/// Uses [`LitByteStr`], which parses byte string literals of the form `b"..."`.
/// These represent raw byte sequences and do not require UTF-8 validity.
///
/// ## Example
/// ```ignore
/// b"module", b"function"
/// ```
///
/// In this case, `bytes` would contain the literals `b"module"` and `b"function"`.
///
/// ---
///
/// ***Note**: If [`Debug`] is required then enable [`syn`] cfg feature "extra-traits"*
#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub struct BStringList {
    pub bytes: Punctuated<LitByteStr, Comma>,
}

/// A wrapper around a [`Punctuated<Ident, Comma>`] representing a
/// list of identifiers provided to parse.
///
/// ## Example
/// ```ignore
/// Module, function
/// ```
/// In this case, `idents` would contain `Module` and `function`.
///
/// ---
///
/// ***Note**: If [`Debug`] is required then enable [`syn`] cfg feature "extra-traits"*
#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub struct IdentList {
    pub idents: Punctuated<Ident, Comma>,
}

// ===============================================================================
// ``````````````````````````````` TO-TOKENS IMPL ````````````````````````````````
// ===============================================================================

impl ToTokens for BoundsList {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.bounds.to_tokens(tokens);
    }
}

impl ToTokens for ExprList {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.exprs.to_tokens(tokens);
    }
}

impl ToTokens for IntList {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.ints.to_tokens(tokens);
    }
}

impl ToTokens for BStringList {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.bytes.to_tokens(tokens);
    }
}

impl ToTokens for IdentList {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.idents.to_tokens(tokens);
    }
}

impl ToTokens for StmtList {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.stmts.to_tokens(tokens);
    }
}

// ===============================================================================
// ````````````````````````````````` CONVERSIONS `````````````````````````````````
// ===============================================================================

impl TryFrom<ExprList> for IntList {
    type Error = TokenStream;

    fn try_from(value: ExprList) -> Result<Self, Self::Error> {
        let err = <Self as ParseDiagnostic>::parse_diagnostic(
            &value.exprs.span().into(),
            Some(ParseError::IntList {}.into()),
        )
        .to_syn_error();
        let mut list = Punctuated::<LitInt, Comma>::new();
        for expr in value.exprs {
            let Expr::Lit(lit) = expr else {
                return Err(err.into_compile_error());
            };

            let Lit::Int(int) = lit.lit else {
                return Err(err.into_compile_error());
            };

            list.push(int);
        }
        Ok(Self { ints: list })
    }
}

impl TryFrom<ExprList> for BStringList {
    type Error = TokenStream;

    fn try_from(value: ExprList) -> Result<Self, Self::Error> {
        let err = <Self as ParseDiagnostic>::parse_diagnostic(
            &value.exprs.span().into(),
            Some(ParseError::BStrList {}.into()),
        )
        .to_syn_error();
        let mut list = Punctuated::<LitByteStr, Comma>::new();
        for expr in value.exprs {
            let Expr::Lit(lit) = expr else {
                return Err(err.into_compile_error());
            };

            let Lit::ByteStr(bstr) = lit.lit else {
                return Err(err.into_compile_error());
            };

            list.push(bstr);
        }
        Ok(Self { bytes: list })
    }
}

impl TryFrom<ExprList> for IdentList {
    type Error = TokenStream;

    fn try_from(value: ExprList) -> Result<Self, Self::Error> {
        let err = <Self as ParseDiagnostic>::parse_diagnostic(
            &value.exprs.span().into(),
            Some(ParseError::IdentList {}.into()),
        )
        .to_syn_error();
        let mut list = Punctuated::<Ident, Comma>::new();
        for expr in value.exprs {
            let Expr::Path(path) = expr else {
                return Err(err.into_compile_error());
            };
            if path.path.segments.len() > 1 {
                return Err(err.into_compile_error());
            }
            let ident = path.path.segments[0].ident.clone();
            list.push(ident);
        }
        Ok(Self { idents: list })
    }
}

impl<T: ParseDiagnostic> TryFrom<ValueGroup<ExprList>> for Punctuated<T, Comma>
where
    T: TryFrom<ExprList, Error = TokenStream>,
{
    type Error = TokenStream;

    fn try_from(value: ValueGroup<ExprList>) -> Result<Self, Self::Error> {
        let err = <ValueGroup<Self> as ParseDiagnostic>::parse_diagnostic(
            &DiagSpan::Tokens(value.to_token_stream()),
            Some(ParseError::ValueGroup {}.into()),
        )
        .to_syn_error();
        let group = value.values;
        let mut list = Punctuated::<T, Comma>::new();
        for item in group {
            let exprs = match item {
                KeyValue::Exprs(expr_list) => expr_list,
                _ => return Err(err.into_compile_error()),
            };
            let collect = match <T as TryFrom<ExprList>>::try_from(exprs) {
                Ok(c) => c,
                Err(e) => return Err(e),
            };
            list.push(collect)
        }
        Ok(list)
    }
}
// ===============================================================================
// ``````````````````````````````````` PARSERS ```````````````````````````````````
// ===============================================================================

impl Parse for IdentList {
    fn parse(input: ParseStream) -> syn::parse::Result<Self> {
        match Punctuated::<Ident, Comma>::parse_terminated(input) {
            Ok(v) => Ok(IdentList { idents: v }),
            Err(e) => Err(
                <IdentList as ParseDiagnostic>::parse_diagnostic(&e.span().into(), None)
                    .to_syn_error(),
            ),
        }
    }
}

impl Parse for IntList {
    fn parse(input: ParseStream) -> syn::parse::Result<Self> {
        match Punctuated::<LitInt, Comma>::parse_terminated(input) {
            Ok(v) => Ok(IntList { ints: v }),
            Err(e) => Err(
                <IntList as ParseDiagnostic>::parse_diagnostic(&e.span().into(), None)
                    .to_syn_error(),
            ),
        }
    }
}

impl Parse for BStringList {
    fn parse(input: ParseStream) -> syn::parse::Result<Self> {
        match Punctuated::<LitByteStr, Comma>::parse_terminated(input) {
            Ok(v) => Ok(BStringList { bytes: v }),
            Err(e) => Err(<BStringList as ParseDiagnostic>::parse_diagnostic(
                &e.span().into(),
                None,
            )
            .to_syn_error()),
        }
    }
}

impl Parse for BoundsList {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        match Punctuated::<WherePredicate, Comma>::parse_terminated(input) {
            Ok(v) => Ok(Self { bounds: v }),
            Err(e) => Err(<BoundsList as ParseDiagnostic>::parse_diagnostic(
                &e.span().into(),
                None,
            )
            .to_syn_error()),
        }
    }
}

impl Parse for ExprList {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        match Punctuated::<Expr, Comma>::parse_terminated(input) {
            Ok(v) => Ok(Self { exprs: v }),
            Err(e) => Err(
                <ExprList as ParseDiagnostic>::parse_diagnostic(&e.span().into(), None)
                    .to_syn_error(),
            ),
        }
    }
}

impl Parse for StmtList {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        match Punctuated::<Stmt, Comma>::parse_terminated(input) {
            Ok(v) => Ok(Self { stmts: v }),
            Err(e) => Err(
                <StmtList as ParseDiagnostic>::parse_diagnostic(&e.span().into(), None)
                    .to_syn_error(),
            ),
        }
    }
}

// ===============================================================================
// `````````````````````````````` PARSE-DIAGNOSTICS ``````````````````````````````
// ===============================================================================

impl ParseDiagnostic for IdentList {
    const KIND: &'static str = "identifier list";
    const EXPECTED: &'static str = "a comma-separated list of identifiers";
    const EXAMPLE: &[&'static str] = &["\"Ident0, ident_1, ..\""];
    const DEFAULT_ERROR_INFO: ErrorInfo = ParseError::IdentList.to_error_info();
}

impl ParseDiagnostic for IntList {
    const KIND: &'static str = "integer literal list";
    const EXPECTED: &'static str = "a comma-separated list of integer literals";
    const EXAMPLE: &[&'static str] = &["\"1, 2, 0xff\""];
    const DEFAULT_ERROR_INFO: ErrorInfo = ParseError::IntList.to_error_info();
}

impl ParseDiagnostic for BStringList {
    const KIND: &'static str = "byte string literal list";
    const EXPECTED: &'static str = "a comma-separated list of byte string literals";
    const EXAMPLE: &[&'static str] = &["\"b\"foo\", b\"bar\", ..\""];
    const DEFAULT_ERROR_INFO: ErrorInfo = ParseError::BStrList.to_error_info();
}

impl ParseDiagnostic for BoundsList {
    const KIND: &'static str = "bounds list";
    const EXPECTED: &'static str = "a comma-separated list of where predicates";
    const EXAMPLE: &[&'static str] = &["\"T: Clone, U: Send + Sync, 'a: 'b\""];
    const DEFAULT_ERROR_INFO: ErrorInfo = ParseError::BoundsList.to_error_info();
}

impl ParseDiagnostic for ExprList {
    const KIND: &'static str = "expression list";
    const EXPECTED: &'static str = "a comma-separated list of expressions";
    const EXAMPLE: &[&'static str] = &["\"foo, bar, b\\\"bytes\\\", make_bytes()\""];
    const DEFAULT_ERROR_INFO: ErrorInfo = ParseError::ExprList.to_error_info();
}

impl ParseDiagnostic for StmtList {
    const KIND: &'static str = "statement list";
    const EXPECTED: &'static str = "a comma-separated list of Rust statements";
    const EXAMPLE: &[&'static str] = &["\"let x = 1; , foo() , Tuple(8);\""];
    const DEFAULT_ERROR_INFO: ErrorInfo = ParseError::StmtList.to_error_info();
}

// ===============================================================================
// ``````````````````````````````` DUPLICATE CHECK ```````````````````````````````
// ===============================================================================

/// Validates duplicate entries in list-like parsed arguments.
///
/// Implement this for list wrappers to detect repeated items based on a
/// normalized string key. This check ignores spans and hygiene and compares
/// items purely by their string representation.
///
/// Use [`DuplicateCheck::duplicate_check`] to emit a diagnostic on the first
/// duplicate found.
pub trait DuplicateCheck: ParseDiagnostic {
    /// The element type stored in the list.
    type Item: ToTokens;

    /// Human-readable name for the item type.
    const KIND: &'static str;

    const DEFAULT_ERROR_INFO: ErrorInfo = DupError::Default.to_error_info();

    /// Returns the list being checked.
    fn items(&self) -> &Punctuated<Self::Item, Comma>;

    /// Returns a string key used for duplicate detection.
    ///
    /// This deliberately ignores spans and hygiene.
    fn key(item: &Self::Item) -> String;

    /// Constructs a diagnostic for a given duplicate item in the list.
    ///
    /// The error is emitted at the duplicate item's span and includes
    /// guidance and examples for correction.
    #[track_caller]
    fn duplicate_error(
        item: &Self::Item,
        dup_key: &str,
        err_site: Option<ErrorInfo>,
    ) -> Diagnostic {
        let mut notes = Vec::<Note>::new();
        for eg in Self::EXAMPLE {
            notes.push(Note {
                msg: format!("example: {}", eg),
            });
        }
        let diag = Diagnostic {
            error: match err_site {
                Some(i) => i,
                None => <Self as DuplicateCheck>::DEFAULT_ERROR_INFO,
            },
            tags: vec![ErrorTag::Custom("duplicate")],
            msg: format!(
                "duplicate {} in argument list",
                <Self as DuplicateCheck>::KIND,
            ),
            span: DiagSpan::tokens(item),
            helps: vec![
                Help {
                    msg: "remove this duplicate item".into(),
                    span: None,
                },
                Help {
                    msg: format!(
                        "the {} `{}` appears more than once",
                        <Self as DuplicateCheck>::KIND,
                        dup_key,
                    ),
                    span: None,
                },
            ],
            notes,
        };

        diag
    }

    /// Checks for duplicates and returns a [`TokenStream`] error if found.
    ///
    /// Convenience wrapper over [`DuplicateCheck::duplicate_diagnostic`].
    #[track_caller]
    fn duplicate_check(&self, err_site: Option<ErrorInfo>) -> Result<(), TokenStream> {
        if let Err(e) = self.duplicate_diagnostic(err_site) {
            return Err(e.into());
        };
        Ok(())
    }

    /// Performs duplicate detection and returns a [`Diagnostic`] on failure.
    ///
    /// Iterates through items, and reports the first duplicate encountered.
    #[track_caller]
    fn duplicate_diagnostic(&self, err_site: Option<ErrorInfo>) -> Result<(), Diagnostic> {
        let mut seen = HashSet::<String>::new();

        for item in self.items().iter() {
            let key = Self::key(item);

            if !seen.insert(key.clone()) {
                return Err(Self::duplicate_error(item, &key, err_site));
            }
        }

        Ok(())
    }
}

impl DuplicateCheck for IdentList {
    type Item = Ident;

    const KIND: &'static str = "identifier";

    const DEFAULT_ERROR_INFO: ErrorInfo = DupError::IdentList.to_error_info();

    fn items(&self) -> &Punctuated<Self::Item, Comma> {
        &self.idents
    }

    fn key(item: &Ident) -> String {
        item.to_string()
    }
}

impl DuplicateCheck for IntList {
    type Item = LitInt;

    const KIND: &'static str = "integer literal";

    const DEFAULT_ERROR_INFO: ErrorInfo = DupError::IntList.to_error_info();

    fn items(&self) -> &Punctuated<Self::Item, Comma> {
        &self.ints
    }

    fn key(item: &LitInt) -> String {
        item.base10_digits().to_string()
    }
}
