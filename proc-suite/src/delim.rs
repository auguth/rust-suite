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
// ``````````````````````````` PARSE DEPRUNEED GROUPS ````````````````````````````
// ===============================================================================

//! Parse delimited groups using [`Delimited`] and its parsing methods.
//!
//! - Use [`Delimited::parse`] to accept any delimiter (`()`, `{}`, `[]`)
//! - Use [`Delimited<T>`] for stricter parsing of a specific delimiter
//! - Uses [`ParseDiagnostic`] to emit structured parsing diagnostics
//!
//! This module provides a lightweight parsing helper over [`syn`] streams.

// ===============================================================================
// ``````````````````````````````````` IMPORTS ```````````````````````````````````
// ===============================================================================

// --- Std Lib ---
use std::marker::PhantomData;

// --- Syn/Quote ---
use syn::{
    parse::{ParseBuffer, ParseStream},
    token::{Brace, Bracket, Paren},
};

// --- Local crate ---
use crate::{ParseDiagnostic, errors::ErrorInfo, space::ParseError};

// ===============================================================================
// ``````````````````````````````` DEPRUNEED GROUPS ``````````````````````````````
// ===============================================================================

/// Represents the contents of a delimited group following a key.
///
/// This enum is a *parsing helper*, not part of the final AST. It temporarily
/// holds the [`ParseBuffer`] corresponding to one of the supported delimiters:
/// `()`, `{}`, or `[]`.
///
/// The delimiter itself is consumed during parsing; only the inner token
/// stream is retained for further parsing.
///
/// The generic parameter `T` can be used to enforce a specific delimiter
/// at compile time (e.g. [`Paren`], [`Brace`], [`Bracket`] from [`syn::token`]).
pub enum Delimited<'a, T = ()> {
    /// Tokens inside `(...)`
    Paren(ParseBuffer<'a>),
    /// Tokens inside `{...}`
    Brace(ParseBuffer<'a>),
    /// Tokens inside `[...]`
    Bracket(ParseBuffer<'a>),
    /// Compile marker to enforce a specific delimiter via `T`
    _Marker(PhantomData<fn(&'a ()) -> T>),
}

impl<'a> Delimited<'a, ()> {
    /// Parses any supported delimiter group: `()`, `{}`, or `[]` and returns its inner buffer.
    pub fn parse(input: ParseStream<'a>) -> syn::Result<Delimited<'a>> {
        if let Ok(paren) = Delimited::<Paren>::parse(input) {
            return Ok(Delimited::Paren(paren));
        }
        if let Ok(brace) = Delimited::<Brace>::parse(input) {
            return Ok(Delimited::Brace(brace));
        }
        if let Ok(bracket) = Delimited::<Bracket>::parse(input) {
            return Ok(Delimited::Bracket(bracket));
        }

        Err(<Delimited as ParseDiagnostic>::parse_diagnostic(&input.into(), None).into())
    }
}

impl<'a> Delimited<'a, Paren> {
    /// Parses a parenthesized group `( ... )` and returns its inner buffer.
    pub fn parse(input: ParseStream<'a>) -> syn::Result<ParseBuffer<'a>> {
        if input.peek(Paren) {
            let content;
            syn::parenthesized!(content in input);
            return Ok(content);
        }

        Err(<Delimited<'a, Paren> as ParseDiagnostic>::parse_diagnostic(&input.into(), None).into())
    }
}

impl<'a> Delimited<'a, Brace> {
    /// Parses a braced group `{ ... }` and returns its inner buffer.
    pub fn parse(input: ParseStream<'a>) -> syn::Result<ParseBuffer<'a>> {
        if input.peek(Brace) {
            let content;
            syn::braced!(content in input);
            return Ok(content);
        }

        Err(<Delimited<'a, Brace> as ParseDiagnostic>::parse_diagnostic(&input.into(), None).into())
    }
}

impl<'a> Delimited<'a, Bracket> {
    /// Parses a bracketed group `[ ... ]` and returns its inner buffer.
    pub fn parse(input: ParseStream<'a>) -> syn::Result<ParseBuffer<'a>> {
        if input.peek(Bracket) {
            let content;
            syn::bracketed!(content in input);
            return Ok(content);
        }

        Err(
            <Delimited<'a, Bracket> as ParseDiagnostic>::parse_diagnostic(&input.into(), None)
                .into(),
        )
    }
}

// ===============================================================================
// `````````````````````````````` PARSE-DIAGNOSTICS ``````````````````````````````
// ===============================================================================

impl<'a> ParseDiagnostic for Delimited<'a> {
    const KIND: &'static str = "delimited group";
    const EXPECTED: &'static str = "group wrapped in either one of `()`, `{}`, or `[]`";
    const EXAMPLE: &'static [&'static str] =
        &["\"{ <group> }\"", "\"[ <group> ]\"", "\"( <group> )\""];
    const DEFAULT_ERROR_INFO: ErrorInfo = ParseError::Delim.to_error_info();
}

impl<'a> ParseDiagnostic for Delimited<'a, Paren> {
    const KIND: &'static str = "parenthesized group";
    const EXPECTED: &'static str = "group wrapped in parenthesis ( ... )";
    const EXAMPLE: &'static [&'static str] = &["\"( <group> )\""];
    const DEFAULT_ERROR_INFO: ErrorInfo = ParseError::ParenDelim.to_error_info();
}

impl<'a> ParseDiagnostic for Delimited<'a, Brace> {
    const KIND: &'static str = "braced group";
    const EXPECTED: &'static str = "group wrapped in braces { ... }";
    const EXAMPLE: &'static [&'static str] = &["\"{ <group> }\""];
    const DEFAULT_ERROR_INFO: ErrorInfo = ParseError::BraceDelim.to_error_info();
}

impl<'a> ParseDiagnostic for Delimited<'a, Bracket> {
    const KIND: &'static str = "bracketed group";
    const EXPECTED: &'static str = "group wrapped in brackets [ ... ]";
    const EXAMPLE: &'static [&'static str] = &["\"[ <group> ]\""];
    const DEFAULT_ERROR_INFO: ErrorInfo = ParseError::BracketDelim.to_error_info();
}
