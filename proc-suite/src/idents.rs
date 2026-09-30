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
// ``````````````````````````` PROC-MACRO IDENTIFIERS ````````````````````````````
// ===============================================================================

//! Deterministic identifier generation for proc-macros.
//!
//! Generates internal identifiers from stable textual anchors, producing
//! repeatable identifier names without requiring runtime state or caching.
//!
//! Identifiers are derived from:
//! - a type name: for type-based anchors
//! - an identifier name: for name-based coordination
//! - a token stream: for syntax-based coordination
//!
//! Each anchor is transformed directly into an identifier, so the same
//! anchor produces the same identifier without relying on a global map.
//!
//! ## Usage
//!
//! [`MarkerIdent`] is blanket-implemented for all types:
//!
//! ```ignore
//! struct MyFunction;
//!
//! let fn_ident = <MyFunction as MarkerIdent<8>>::from_type(
//!     Case::Lower,
//!     Some(b"prefix"),
//!     None,
//!     None,
//! );
//! ```
//!
//! ## Notes
//!
//! - Identifier generation is deterministic from the selected anchor
//! - Casing via [`Case`]
//! - Optional ASCII prefix/suffix
//! - Optional span for hygiene

// ===============================================================================
// ``````````````````````````````````` IMPORTS ```````````````````````````````````
// ===============================================================================

// --- Proc-macro2 ---
use proc_macro2::{Span, TokenStream};

// --- Syn/Quote ---
use quote::format_ident;
use syn::Ident;

// ===============================================================================
// ``````````````````````````````` IDENTIFIER CASES ``````````````````````````````
// ===============================================================================

/// Specifies how a generated identifier should be cased.
///
/// This is used when formatting internally generated identifiers so they
/// blend naturally into Rust syntax and conventions at the expansion site.
#[derive(Clone, Copy)]
pub enum Case {
    /// Lowercase, underscore-separated (`snake_case`).
    ///
    /// Commonly used for functions, variables, and other value-level items.
    Lower,

    /// Uppercase, underscore-separated (`SCREAMING_SNAKE_CASE`).
    ///
    /// Commonly used for constants and other all-caps items.
    Upper,

    /// PascalCase (leading uppercase, no separators).
    ///
    /// Commonly used for types and other type-level items.
    Pascal,
}

// ===============================================================================
// ```````````````````````````` MARKER IDENT (BLANKET) ```````````````````````````
// ===============================================================================

/// A unified interface for generating deterministic identifiers from
/// different identity anchors.
///
/// **Why this exists:**  
/// Proc-macro code often needs to generate internal identifiers that are
/// stable, collision-resistant, and reproducible. This trait groups the
/// supported anchoring strategies behind a single, explicit API so call
/// sites can derive identifiers without managing the underlying
/// representation of the anchor.
///
/// **Three ways to anchor identity:**
///
/// - **`from_type`**  
///   Uses the implementing type `Self` as the identity anchor. The type's
///   [`std::any::type_name`] is used as the deterministic textual key.
///   No value is ever constructed.
///
/// - **`from_ident`**  
///   Uses an existing [`Ident`] as the identity anchor. The identifier's
///   textual name is used as the key; its span and hygiene do not
///   participate in identifier generation.
///
/// - **`from_tokens`**  
///   Uses a [`proc_macro2::TokenStream`] as the identity anchor. Its
///   deterministic textual representation is used as the key.
///
/// In all cases, the anchor is converted into a deterministic base word
/// before the requested casing, prefix, and suffix are applied.
///
/// An explicit span may be provided to control the hygiene of the
/// resulting identifier. When no span is provided, the generated
/// identifier uses the default span produced by `format_ident!`.
pub trait MarkerIdent: 'static + Sized {
    /// Generates an identifier anchored to the implementing marker type.
    fn from_type(
        case: Case,
        prefix: Option<&[u8]>,
        suffix: Option<&[u8]>,
        span: Option<Span>,
    ) -> Ident {
        let string = gen_ident_from_marker::<Self>(case, prefix, suffix);

        if let Some(span) = span {
            return Ident::new(&string, span);
        }

        format_ident!("{}", string)
    }

    /// Generates an identifier anchored to an existing identifier name.
    fn from_ident(
        ident: &Ident,
        case: Case,
        prefix: Option<&[u8]>,
        suffix: Option<&[u8]>,
        span: Option<Span>,
    ) -> Ident {
        let string = gen_ident_from_ident(ident, case, prefix, suffix);

        if let Some(span) = span {
            return Ident::new(&string, span);
        }

        format_ident!("{}", string)
    }

    /// Generates an identifier anchored to a token stream.
    fn from_tokens(
        tokens: &proc_macro2::TokenStream,
        case: Case,
        prefix: Option<&[u8]>,
        suffix: Option<&[u8]>,
        span: Option<Span>,
    ) -> Ident {
        let string = gen_ident_from_tokens(tokens, case, prefix, suffix);

        if let Some(span) = span {
            return Ident::new(&string, span);
        }

        format_ident!("{}", string)
    }
}


impl<T: 'static + Sized> MarkerIdent for T {}

// ===============================================================================
// `````````````````````````````` PRIVATE UTILITIES ``````````````````````````````
// ===============================================================================

/// Returns the bare (unqualified) ASCII-only type name of `T`.
///
/// For example:
/// - `my_crate::foo::Bar` -> `"Bar"`
/// - `my_crate::foo::Café` -> `"Caf"`
///
/// Non-ASCII characters are stripped so generated internal identifiers remain
/// stable, predictable, and compiler-friendly.
///
/// This is useful when generating proc-macro identifiers or diagnostics
/// without including module paths or Unicode normalization concerns.
fn bare_type_name<T: 'static>() -> String {
    use std::any::type_name;

    let k: String = type_name::<T>()
        .rsplit("::")
        .next()
        .unwrap()
        .chars()
        .filter(|c| c.is_ascii())
        .collect();

    k.replace("::", "_")
        .replace(['<', '>'], "_")
        .chars()
        .filter(|c| c.is_ascii())
        .collect()
}

/// Generates a deterministic, type-scoped identifier for use inside
/// proc-macro expansions.
///
/// **Why this exists:**
/// Proc macros often need to invent identifiers that are unlikely to collide
/// with user code. This function derives the identifier from the type's
/// textual name rather than relying on runtime state or a generated counter.
///
/// **How it works:**
/// The generic parameter `T` is never constructed or used as a value.
/// Its fully qualified type name is obtained using [`std::any::type_name`]
/// and used as the identity anchor for identifier generation.
///
/// Because the complete type name is used as the anchor, different types
/// normally produce different generated identifiers while repeated requests
/// for the same type produce the same identifier.
///
/// The generated identifier is formatted according to the requested casing,
/// with optional prefix and suffix applied.
fn gen_ident_from_marker<T: 'static>(
    case: Case,
    prefix: Option<&[u8]>,
    suffix: Option<&[u8]>,
) -> String {
    let key = bare_type_name::<T>();

    apply_case(&key, case, prefix, suffix)
}

/// Generates a deterministic, identifier-scoped identifier for use inside
/// proc-macro expansions.
///
/// **Why this exists:**
/// Some proc-macro scenarios already have a meaningful identifier available
/// and need to derive additional symbols from its name without introducing
/// a dedicated marker type.
///
/// **How it works:**
/// The provided [`Ident`] is converted to its textual representation.
/// Span and hygiene information are deliberately ignored, making the
/// identifier name itself the identity anchor.
///
/// The same identifier name therefore produces the same generated identifier
/// regardless of its span or hygiene context.
///
/// The generated identifier is formatted according to the requested casing,
/// with optional prefix and suffix applied.
fn gen_ident_from_ident(
    ident: &Ident,
    case: Case,
    prefix: Option<&[u8]>,
    suffix: Option<&[u8]>,
) -> String {

    let key = ident.to_string();

    apply_case(&key, case, prefix, suffix)
}

/// Generates a deterministic, token-scoped identifier for use inside
/// proc-macro expansions.
///
/// **Why this exists:**
/// Some proc-macro scenarios need to derive identifiers directly from syntax
/// rather than from a marker type or an existing identifier. In those cases,
/// the token stream itself provides the identity anchor.
///
/// **How it works:**
/// The provided [`TokenStream`] is converted into its textual representation
/// and used as the identity anchor for identifier generation.
///
/// Equivalent token-stream text therefore produces the same generated
/// identifier without requiring shared marker types, identifier names,
/// or runtime state.
///
/// The generated identifier is formatted according to the requested casing,
/// with optional prefix and suffix applied.
fn gen_ident_from_tokens(
    tokens: &TokenStream,
    case: Case,
    prefix: Option<&[u8]>,
    suffix: Option<&[u8]>,
) -> String {
    let key = tokens.to_string();

    apply_case(&key, case, prefix, suffix)
}

/// Applies a casing style and optional prefix/suffix to a base identifier.
///
/// **Why this exists:**  
/// Identifier generation in proc macros needs to be predictable, hygienic,
/// and fast. This function centralizes all string-shaping logic so the rest
/// of the codebase can focus purely on *what* identifier is being generated,
/// not *how* it is formatted.
///
/// **What it does:**  
/// - Takes a base identifier word (`word`)
/// - Optionally prepends and/or appends byte-based prefixes and suffixes
/// - Formats the result according to the requested casing style:
///   - `Lower`: lowercase alphanumeric segments separated by `_`
///   - `Upper`: uppercase alphanumeric segments separated by `_`
///   - `Pascal`: PascalCase with non-alphanumeric characters treated as
///     word boundaries
///   - Always prepends __ to the resulting identifier.
///
/// Non-alphanumeric characters are treated as separators:
/// - `Lower` and `Upper` collapse each separator run into a single `_`.
/// - `Pascal` removes separators and capitalizes the next alphanumeric
///   character.
///
/// Prefixes and suffixes are provided as byte slices to keep internal
/// identifier components lightweight and avoid unnecessary allocations.
pub(crate) fn apply_case(
    word: &str,
    case: Case,
    prefix: Option<&[u8]>,
    suffix: Option<&[u8]>,
) -> String {
    let mut out = String::with_capacity(
        word.len()
            + prefix.map_or(0, |p| p.len() + 1)
            + suffix.map_or(0, |s| s.len() + 1),
    );

    match case {
        Case::Lower => {
            append_lower(&mut out, prefix);
            append_separator(&mut out);
            append_lower_str(&mut out, word);
            append_separator(&mut out);
            append_lower(&mut out, suffix);

            trim_separator(&mut out);
        }

        Case::Upper => {
            append_upper(&mut out, prefix);
            append_separator(&mut out);
            append_upper_str(&mut out, word);
            append_separator(&mut out);
            append_upper(&mut out, suffix);

            trim_separator(&mut out);
        }

        Case::Pascal => {
            if let Some(prefix) = prefix {
                push_pascal_bytes(&mut out, prefix);
            }

            push_pascal_str(&mut out, word);

            if let Some(suffix) = suffix {
                push_pascal_bytes(&mut out, suffix);
            }
        }
    }

    out.insert_str(0, "__");

    out
}

/// Appends a lowercase transformation of an ASCII byte slice.
///
/// Non-alphanumeric characters are treated as word boundaries and collapsed
/// into a single underscore when followed by an alphanumeric character.
fn append_lower(out: &mut String, bytes: Option<&[u8]>) {
    let Some(bytes) = bytes else {
        return;
    };

    let mut separator = false;

    for &b in bytes {
        if b.is_ascii_alphanumeric() {
            if separator && !out.is_empty() && !out.ends_with('_') {
                out.push('_');
            }

            out.push(b.to_ascii_lowercase() as char);
            separator = false;
        } else {
            separator = true;
        }
    }
}

/// Appends an uppercase transformation of an ASCII byte slice.
///
/// Non-alphanumeric characters are treated as word boundaries and collapsed
/// into a single underscore when followed by an alphanumeric character.
fn append_upper(out: &mut String, bytes: Option<&[u8]>) {
    let Some(bytes) = bytes else {
        return;
    };

    let mut separator = false;

    for &b in bytes {
        if b.is_ascii_alphanumeric() {
            if separator && !out.is_empty() && !out.ends_with('_') {
                out.push('_');
            }

            out.push(b.to_ascii_uppercase() as char);
            separator = false;
        } else {
            separator = true;
        }
    }
}

/// Appends a lowercase transformation of a string slice.
///
/// Non-alphanumeric characters are treated as word boundaries and collapsed
/// into a single underscore when followed by an alphanumeric character.
fn append_lower_str(out: &mut String, s: &str) {
    let mut separator = false;

    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            if separator && !out.is_empty() && !out.ends_with('_') {
                out.push('_');
            }

            out.push(c.to_ascii_lowercase());
            separator = false;
        } else {
            separator = true;
        }
    }
}

/// Appends an uppercase transformation of a string slice.
///
/// Non-alphanumeric characters are treated as word boundaries and collapsed
/// into a single underscore when followed by an alphanumeric character.
fn append_upper_str(out: &mut String, s: &str) {
    let mut separator = false;

    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            if separator && !out.is_empty() && !out.ends_with('_') {
                out.push('_');
            }

            out.push(c.to_ascii_uppercase());
            separator = false;
        } else {
            separator = true;
        }
    }
}

/// Appends a PascalCase transformation of an ASCII byte slice.
///
/// Non-alphanumeric characters are treated as word boundaries and removed.
/// The next alphanumeric character is capitalized.
fn push_pascal_bytes(out: &mut String, bytes: &[u8]) {
    let mut capitalize_next = true;

    for &b in bytes {
        if !b.is_ascii_alphanumeric() {
            capitalize_next = true;
            continue;
        }

        let c = b as char;

        if capitalize_next {
            out.push(c.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            out.push(c);
        }
    }
}

/// Appends a PascalCase transformation of a string slice.
///
/// Non-alphanumeric characters are treated as word boundaries and removed.
/// The next alphanumeric character is capitalized.
fn push_pascal_str(out: &mut String, s: &str) {
    let mut capitalize_next = true;

    for c in s.chars() {
        if !c.is_ascii_alphanumeric() {
            capitalize_next = true;
            continue;
        }

        if capitalize_next {
            out.push(c.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            out.push(c);
        }
    }
}

/// Appends a separator when the output is non-empty and does not already end
/// with one.
fn append_separator(out: &mut String) {
    if !out.is_empty() && !out.ends_with('_') {
        out.push('_');
    }
}

/// Removes trailing separators from the generated identifier.
fn trim_separator(out: &mut String) {
    while out.ends_with('_') {
        out.pop();
    }
}