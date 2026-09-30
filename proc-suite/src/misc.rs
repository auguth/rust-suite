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
// ```````````````````````````````` MISCELLANEOUS ````````````````````````````````
// ===============================================================================

//! Concrete miscellaneous helpers used commonly across a proc-macro crate.
//!
//! Provides shared utility functions for parsing, identifier generation,
//! debugging, and generic parameter manipulation.

// ===============================================================================
// ``````````````````````````````````` IMPORTS ```````````````````````````````````
// ===============================================================================

// --- Local Crate ---
use crate::{Case, DocAttr, DocCodeBlock, InsertDocs, MarkerIdent, space::ProcParseErr};

// --- Proc-Macro Crates ---
use proc_macro2::TokenStream;
use syn::{Ident, LitInt};

// ===============================================================================
// `````````````````````````````````` CONSTANTS ``````````````````````````````````
// ===============================================================================

pub const COLON: char = ':';

pub const BULLET: &'static str = "\u{2022}";

pub const LINE_BREAK: char = '\n';

pub const LINE_SPACE: char = ' ';

/// Marks the failure as an internal logical bug in the proc-macro system.
pub const LOGICAL_BUG: &'static str = "indicates a logical bug in the proc-macro phase";

/// Suggests reporting the issue to the crate maintainers.
pub const REPORT_BUG: &'static str = "report this to the proc-macro crate's maintainers";

// ===============================================================================
// `````````````````````````````````` FUNCTIONS ``````````````````````````````````
// ===============================================================================

/// Parses a [`LitInt`] into a positive `usize`, emitting a diagnostic on failure.
pub fn parse_pos_usize(int: &LitInt) -> Result<usize, TokenStream> {
    let Ok(v) = int.base10_digits().parse() else {
        return Err(ProcParseErr::PositiveUsize { lit: int.clone() }.into());
    };
    Ok(v)
}

/// Parses a [`LitInt`] into a positive `u32`, emitting a diagnostic on failure.
pub fn parse_pos_u32(int: &LitInt) -> Result<u32, TokenStream> {
    let Ok(v) = int.base10_digits().parse() else {
        return Err(ProcParseErr::PositiveU32 { lit: int.clone() }.into());
    };
    Ok(v)
}

/// Parses a [`LitInt`] into a positive `u16`, emitting a diagnostic on failure.
pub fn parse_pos_u16(int: &LitInt) -> Result<u16, TokenStream> {
    let Ok(v) = int.base10_digits().parse() else {
        return Err(ProcParseErr::PositiveU16 { lit: int.clone() }.into());
    };
    Ok(v)
}

/// Parses a [`LitInt`] into a positive `u8`, emitting a diagnostic on failure.
pub fn parse_pos_u8(int: &LitInt) -> Result<u8, TokenStream> {
    let Ok(v) = int.base10_digits().parse() else {
        return Err(ProcParseErr::PositiveU8 { lit: int.clone() }.into());
    };
    Ok(v)
}

/// Generate a constant identifier from the marker type.
pub fn gen_const_ident<T: MarkerIdent>() -> Ident {
    <T as MarkerIdent>::from_type(
        Case::Upper,
        None,
        None,
        None,
    )
}

/// Generate a constant identifier from an existing identifier.
pub fn gen_const_ident_from_ident<T: MarkerIdent>(ident: &Ident) -> Ident {
    <T as MarkerIdent>::from_ident(
        ident,
        Case::Upper,
        None,
        None,
        None,
    )
}

/// Generate a constant identifier from an identifier with an optional suffix.
pub fn gen_const_ident_from_ident_with_suffix<T: MarkerIdent>(
    ident: &Ident,
    suffix: Option<&[u8]>,
) -> Ident {
    <T as MarkerIdent>::from_ident(
        ident,
        Case::Upper,
        None,
        suffix,
        None,
    )
}

/// Generate a constant identifier with an optional suffix.
pub fn gen_const_ident_with_suffix<T: MarkerIdent>(suffix: Option<&[u8]>) -> Ident {
    <T as MarkerIdent>::from_type(
        Case::Upper,
        None,
        suffix,
        None,
    )
}

/// Generate a type identifier from the marker type.
pub fn gen_type_ident<T: MarkerIdent>() -> Ident {
    <T as MarkerIdent>::from_type(
        Case::Pascal,
        None,
        None,
        None,
    )
}

/// Generate a type identifier from an existing identifier.
pub fn gen_type_ident_from_ident<T: MarkerIdent>(ident: &Ident) -> Ident {
    <T as MarkerIdent>::from_ident(
        ident,
        Case::Pascal,
        None,
        None,
        None,
    )
}

/// Generate a type identifier from an identifier with an optional suffix.
pub fn gen_type_ident_from_ident_with_suffix<T: MarkerIdent>(
    ident: &Ident,
    suffix: Option<&[u8]>,
) -> Ident {
    <T as MarkerIdent>::from_ident(
        ident,
        Case::Pascal,
        None,
        suffix,
        None,
    )
}

/// Generate a type identifier with an optional suffix.
pub fn gen_type_ident_with_suffix<T: MarkerIdent>(suffix: Option<&[u8]>) -> Ident {
    <T as MarkerIdent>::from_type(
        Case::Pascal,
        None,
        suffix,
        None,
    )
}

/// Generate a function identifier from the marker type.
pub fn gen_fn_ident<T: MarkerIdent>() -> Ident {
    <T as MarkerIdent>::from_type(
        Case::Lower,
        None,
        None,
        None,
    )
}

/// Generate a function identifier with an optional suffix.
pub fn gen_fn_ident_with_suffix<T: MarkerIdent>(suffix: Option<&[u8]>) -> Ident {
    <T as MarkerIdent>::from_type(
        Case::Lower,
        None,
        suffix,
        None,
    )
}

/// Generate a constant identifier from a token stream.
pub fn gen_const_ident_from_tokens(tokens: &TokenStream) -> Ident {
    <() as MarkerIdent>::from_tokens(tokens, Case::Upper, None, None, None)
}

/// Generate a constant identifier from a token stream with an optional suffix.
pub fn gen_const_ident_from_tokens_with_suffix(
    tokens: &TokenStream,
    suffix: Option<&[u8]>,
) -> Ident {
    <() as MarkerIdent>::from_tokens(tokens, Case::Upper, None, suffix, None)
}

/// Generate a type identifier from a token stream.
pub fn gen_type_ident_from_tokens(tokens: &TokenStream) -> Ident {
    <() as MarkerIdent>::from_tokens(tokens, Case::Pascal, None, None, None)
}

/// Generate a type identifier from a token stream with an optional suffix.
pub fn gen_type_ident_from_tokens_with_suffix(
    tokens: &TokenStream,
    suffix: Option<&[u8]>,
) -> Ident {
    <() as MarkerIdent>::from_tokens(tokens, Case::Pascal, None, suffix, None)
}

/// Generate a function identifier from a token stream.
pub fn gen_fn_ident_from_tokens(tokens: &TokenStream) -> Ident {
    <() as MarkerIdent>::from_tokens(tokens, Case::Lower, None, None, None)
}

/// Generate a function identifier from a token stream with an optional suffix.
pub fn gen_fn_ident_from_tokens_with_suffix(tokens: &TokenStream, suffix: Option<&[u8]>) -> Ident {
    <() as MarkerIdent>::from_tokens(tokens, Case::Lower, None, suffix, None)
}

/// Prepends a documentation disclaimer explaining how to configure
/// rust-analyzer to analyze the `cfg(doc)` representation of the generated
/// trait.
///
/// The disclaimer also warns that rust-analyzer may report false-positive
/// diagnostics for the internal expanded form and recommends generated
/// rustdoc or docs.rs as the authoritative documentation.
pub fn cargo_not_doc_disclaimer<T: InsertDocs>(item: &mut T) {
    item.prepend_docs(vec![
        DocAttr::Raw(
            "*Note: If rust-analyzer shows this internal expanded form, enable `cfg(doc)` analysis:*"
                .to_string(),
        ),
        DocAttr::CodeBlock(DocCodeBlock::Manual {
            lang: "json".to_string(),
            block: vec![DocAttr::Raw(
                r#"{ "rust-analyzer.cargo.cfgs": ["doc"] }"#.to_string(),
            )],
        }),
        DocAttr::Bullet {
            level: 0,
            content: vec![DocAttr::Raw(
                "*This may produce false-positive diagnostics in some rust-analyzer versions.*"
                    .to_string(),
            )],
        },
        DocAttr::Bullet {
            level: 0,
            content: vec![DocAttr::Raw(
                "*For authoritative documentation, rely on generated rustdoc (`cargo doc`) or docs.rs output.*"
                    .to_string(),
            )],
        },
        DocAttr::LineBreak,
        DocAttr::LineBreak,
    ]);
}
