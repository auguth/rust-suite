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
// `````````````````````````````` PROC-MACRO ERRORS ``````````````````````````````
// ===============================================================================

//! Structured error diagnostics for proc-macros.
//!
//! ## Overview
//!
//! Provides structured, ID-based diagnostics for proc-macros.
//!
//! Diagnostics are emitted with stable IDs, spans, and optional
//! help/notes instead of ad-hoc string errors.
//!
//! ## Output
//!
//! ```text
//! error: [SPACE:ID] <message>
//!        help:
//!          • <help message>
//!          • ...
//!        note:
//!          • <note message>
//!          • ...
//!
//!  --> <file>:<line>:<col>
//!   |
//! <line> | <code>
//!   |     ^^^^^
//! ```
//!
//! Spanned helps are emitted as separate additional diagnostics and referenced
//! via `[SPACE:ID:i]`.
//!
//! ## Defining Diagnostics
//!
//! 1. Define an error space (assigns error IDs):
//!    [`crate::error_spaces!`] / [`crate::error_space!`]
//!
//! ```ignore
//! error_spaces! {
//!     space: "MY",
//!     maintain: MaintainInfo,
//!
//!     pub enum MyErrors {
//!         range: 100..=149,
//!         variants: {
//!             InvalidInput,
//!             Unsupported,
//!         }
//!     }
//!
//!     pub enum OtherErrors {
//!         range: 150..=199,
//!         variants: {
//!             Conflict,
//!             Missing,
//!         }
//!     }
//! }
//! ```
//!
//! 2. Define diagnostics using the same variants:
//!    [`crate::diagnostics!`]
//!
//! ```ignore
//! diagnostics! {
//!     space: MyErrors,
//!
//!     pub enum Error {
//!         InvalidInput {
//!             fields: { field: syn::Field },
//!             msg: "invalid input",
//!             tags: [InvalidInput],
//!             span: { spanned: field },
//!             help: ["try unsigned"],
//!         },
//!
//!         Unsupported {
//!             fields: { field: syn::Field },
//!             msg: format!("unsupported field {}", field),
//!             tags: [Unsupported],
//!             span: { spanned: field },
//!             note: ["try different name"],
//!         }
//!     }
//! }
//! ```
//!
//! Variants in `diagnostics!` must match those in the error space.
//!
//! ## Emitting Diagnostics
//!
//! Diagnostics convert directly into [`TokenStream`]:
//!
//! ```ignore
//! #[proc_macro_attribute]
//! pub fn my_attr(_attr: TokenStream, item: TokenStream) -> TokenStream {
//!     let field = ...;
//!
//!     if invalid {
//!         return Error::InvalidInput { field }.into();
//!     }
//!
//!     quote!(#item)
//! }
//! ```
//!
//! Produces:
//!
//! ```text
//! error: [MY:101] invalid input
//!        help:
//!          • try unsigned
//!
//!  --> src/main.rs:5:1
//!   |
//! 5 | <code>
//!   | ^^^^^
//! ```
//!
//! ## Classification
//!
//! Diagnostics can be classified using [`ErrorCatalog`] via [`ErrorTag`].
//!
//! This allows the system to apply specialized behavior for common cases.
//!
//! - [`ErrorCatalog::Bug`]
//!   Marks an internal macro error. Diagnostics automatically include
//!   maintainer contact and issue reporting information.
//!
//! ## Dev Mode
//!
//! When the `dev` feature is enabled, diagnostics include the
//! macro crate's internal location (`file:line:col`).
//!
//! ```text
//! // dev
//! error: [src/lib.rs:42:5] invalid input
//!
//! // normal
//! error: [INSTANCE:101] invalid input
//! ```
//!
//! Intended for macro development; users rely on stable IDs.
//!
//!
//! ## High-level Utilities
//!
//! Primary entry points for defining, parsing, and emitting diagnostics.
//!
//! These are directly used when writing proc-macros.
//!
//! - [`crate::diagnostics!`]: define diagnostic enums
//! - [`crate::error_spaces!`] / [`crate::error_space!`]: define error ID spaces
//! - [`crate::token_err!`] / [`crate::callsite_err!`]: emit simple errors
//! - [`ParseDiagnostic`]: convenient parsing diagnostics (implemented for common `syn` nodes)
//!
//! ## Common Concepts
//!
//! Core concepts used when defining diagnostics.
//!
//! These are configured externally and fed into macros,
//! rather than constructed as runtime values.
//!
//! - [`ErrorCatalog`] / [`ErrorTag`]: classify diagnostics (e.g. bug, invalid input)
//! - [`ErrorMaintainers`]: configure reporting (mail / repository)
//! - [`GitHost`] / [`GitHostInfo`]: repository metadata for reporting
//!
//! ## Low-level Diagnostic Types
//!
//! Underlying data structures of the diagnostic system.
//!
//! These are usually used implicitly via macros, but are available for
//! manual construction and advanced customization when needed.
//!
//! - [`Diagnostic`]: full diagnostic representation
//! - [`ErrorId`] / [`ErrorInfo`]: error identity and metadata
//! - [`ErrorDetails`]: static error description
//! - [`DiagSpan`]: span abstraction (span / tokens)
//! - [`Help`] / [`Note`]: additional diagnostic context
//! - [`MultiDiagnostic`]: aggregation of multiple diagnostics

// ===============================================================================
// ``````````````````````````````````` IMPORTS ```````````````````````````````````
// ===============================================================================

// --- Core ---
use core::fmt;

// --- Std Lib ---
use std::hash::Hash;

// --- Local crate ---
use crate::{
    misc::{BULLET, COLON, LINE_BREAK, LINE_SPACE},
    space::ParseError,
};

// --- Proc-macro2 ---
use proc_macro2::{Span, TokenStream};

// --- Syn/Quote ---
use quote::ToTokens;
use syn::{Error, parse::ParseStream, spanned::Spanned};

// --- URL crates ---
use urlencoding::encode;

// ===============================================================================
// ```````````````````````````````````` MACROS ```````````````````````````````````
// ===============================================================================

/// Emits a macro-expansion error using the given token as the span.
///
/// The provided `span` (any [`quote::ToTokens`]) is used to precisely
/// locate the error in user code, so the compiler highlights that token.
///
/// ## Example
///
/// ```ignore
/// // inside a proc-macro
/// if !is_supported(&field) {
///     // runs during macro expansion (not runtime)
///     return token_err!(field, "unsupported field type");
/// }
/// ```
///
/// ## Terminal output (example)
///
/// ```text
/// error: unsupported field type
///  --> src/main.rs:10:5
///   |
/// 10 |     field: i32,
///   |     ^^^^^
/// ```
///
/// ## See also
///
/// - [`crate::callsite_err!`]
#[macro_export]
macro_rules! token_err {
    ($span:expr, $msg:expr $(,)?) => {
        $crate::errors::token_err_impl($span, $msg)
    };
}

/// Emits a macro-expansion error at the macro call site.
///
/// The error is anchored to the invocation location (e.g., the attribute
/// or macro usage), so the compiler highlights where the macro is applied,
/// rather than a specific token inside the input.
///
/// ## Example
///
/// ```ignore
/// // inside a proc-macro
/// if invalid_context {
///     // runs during macro expansion (not runtime)
///     return callsite_err!("this macro cannot be used here");
/// }
/// ```
///
/// ## Terminal output (example)
///
/// ```text
/// error: this macro cannot be used here
///  --> src/main.rs:5:1
///   |
/// 5 | #[my_macro]
///   | ^^^^^^^^^^^
/// ```
///
/// ## See also
///
/// - [`crate::token_err!`]
#[macro_export]
macro_rules! callsite_err {
    ($msg:expr $(,)?) => {
        $crate::errors::callsite_err_impl($msg)
    };
}

/// Defines a structured error-diagnostic enum for macro-systems.
///
/// This macro generates an enum whose variants represent compile-time
/// errors, along with automatic conversion into [`TokenStream`].
///
/// Each variant can carry fields and rich metadata such as message,
/// tags, span, help messages, and notes.
///
/// ### Basic Structure
///
/// ```ignore
/// diagnostics! {
///     space: ErrorSpace,              // Required: error space enum (provides error codes)
///
///     pub enum MyError {              // Required: diagnostic enum
///         VariantName {               // Required: diagnostic variants
///             // fields go here
///         }
///     }
/// }
/// ```
///
/// - `ErrorSpace` is defined separately using [`crate::error_spaces!`] and provides
///   error code semantics.
/// - All variants in this enum must correspond to variants in the given
///   error space (same names).
///
/// ### Variant (Required Fields)
///
/// ```ignore
/// VariantName {
///     fields: {                      // Required: variant data
///         field1: SynNode,
///         field2: Token,
///         field3: Span,
///     },
///     msg:  "error message",         // Required: primary message expression
///     tags: [InvalidInput],          // Required: at least one tag
///     span: { spanned: field1 },     // Required: what gets highlighted
/// }
/// ```
///
/// - `msg` is an expression, so you can construct messages using variant fields  
///   (e.g., `format!("expected {}, found {}", field1, field2)`).
/// - `tags` accept [`ErrorCatalog`] variants or custom string literals  
///   (e.g., `tags: [InvalidInput, "MyCustomError"]`).
/// - Attributes (`#[...]`) & doc-comments can be applied to the enum, variants, and fields
///
/// ### Span Kinds (Required)
///
/// Determines where the compiler points in the code.  
/// Variant fields can be used as span sources.
///
/// ```ignore
/// span: { spanned: field1 }    // span from syntax node
/// span: { tokens:  field2 }    // span derived from tokens
/// span: { span:    field3 }    // explicit proc_macro2::Span
/// ```
///
/// ### Help (Optional)
///
/// Supports plain suggestion messages or span-attached entries.
///
/// ```ignore
/// help: [
///     "try something else",
///     { format!("expected {}, found {}", a, b) },
///     {
///         span: field1,              // Required: target span
///         msg: "this part is wrong"  // Required: help message
///     }
/// ]
/// ```
///
/// Wrap expressions in `{ ... }` when they produce a message, to avoid
/// ambiguity with span-attached help entries.
///
/// ### Notes (Optional)
///
/// Additional unspanned contextual information.
///
/// ```ignore
/// note: [
///     "additional context",
///     SOME_CONST.to_string(),
/// ]
/// ```
///
/// ### Example
///
/// ```ignore
/// diagnostics! {
///     space: MyErrors,
///
///     pub enum Error {
///         UnsupportedField {
///             fields: { field: syn::Field },
///             msg: "unsupported field type",
///             tags: [InvalidInput],
///             span: { spanned: field },
///         }
///     }
/// }
///
/// // inside proc-macro (runs at macro expansion time)
/// return Error::UnsupportedField { field }.into();
/// ```
///
/// ### Advanced Example
///
/// ```ignore
/// diagnostics! {
///     space: MyErrors,
///
///     pub enum Error {
///         ComplexCase {
///             fields: {
///                 ident: syn::Ident,
///                 ty: syn::Type,
///                 span_src: proc_macro2::Span,
///             },
///
///             msg: format!(
///                 "invalid type `{}` for identifier `{}`",
///                 quote::quote!(#ty),
///                 ident
///             ),
///
///             tags: [InvalidInput, "TypeError"],
///
///             span: { spanned: ident },
///
///             help: [
///                 "ensure the type matches expected constraints",
///                 { format!("received type: {:?}", ty) },
///                 {
///                     span: ident,
///                     msg: "this identifier is problematic"
///                 },
///                 {
///                     span: span_src,
///                     msg: "explicit span override"
///                 }
///             ],
///
///             note: [
///                 "this error occurs during macro expansion",
///                 "only certain types are supported".to_string(),
///             ],
///         }
///     }
/// }
///
/// #[proc_macro]
/// pub fn my_macro(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
///     let item = syn::parse_macro_input!(input as Item);
///
///     let ident = ...;      // extracted from item
///     let ty = ...;         // inferred or parsed type
///     let span_src = ...;   // span source
///
///     if ... {
///         // diagnostic compile-time error
///         return Error::ComplexCase {
///             ident,
///             ty,
///             span_src,
///         }
///         .into();
///     }
///
///     quote!(#item).into()
/// }
/// ```
///
/// ## See also
///
/// - [`crate::error_spaces!`] / [`crate::error_space!`]
/// - [`crate::token_err!`]
/// - [`crate::callsite_err!`]
#[macro_export]
macro_rules! diagnostics {
    (
        space: $error_space:ident,

        $(#[$enum_meta:meta])*
        $vis:vis enum $Enum:ident {
            $(
                $(#[$variant_meta:meta])*
                $Variant:ident {
                    fields: { $( $(#[$field_meta:meta])* $field:ident : $ty:ty),* $(,)? },
                    msg: $msg:expr,
                    tags: [ $($sem:tt),+ $(,)? ],
                    span: { $($span:tt)* }
                    $(, help: [ $($help:tt),* $(,)? ])?
                    $(, note: [ $($note:expr),* $(,)? ])?
                    $(,)?
                }
            ),* $(,)?
        }
    ) => {

        $(#[$enum_meta])*
        #[derive(Clone, Debug)]
        $vis enum $Enum {
            $(
                $(#[$variant_meta])*
                $Variant {
                    $( $(#[$field_meta])* $field : $ty),*
                }
            ),*
        }

        impl $Enum {
            /// Converts the enum variant into a [`Diagnostic`] with all configured
            /// message, tags, span, help, and notes.
            #[track_caller]
            pub fn to_diagnostic(&self) -> $crate::errors::Diagnostic {
                match self {
                    $(
                        Self::$Variant { $($field),* } => {
                            $crate::errors::Diagnostic {
                                error: $error_space::$Variant.into(),
                                msg: diagnostics!(@msg $msg),
                                tags: vec![ $( diagnostics!(@sem $sem) ),* ],
                                span: diagnostics!(@span $($span)*),
                                helps: diagnostics!(@helps $( $($help),* )? ),
                                notes: diagnostics!(@notes $( $($note),* )? ),
                            }
                        }
                    ),*
                }
            }

            /// Renders the diagnostic as a plain string.
            ///
            /// Spanned helps are omitted since they rely on compiler diagnostics
            /// and are not meaningful in runtime panic messages.
            #[track_caller]
            pub fn to_string(&self) -> String {
                let mut diag = self.to_diagnostic();

                diag.helps.retain(|h| h.span.is_none());

                diag.to_syn_error().to_string()
            }
        }

        impl Into<proc_macro2::TokenStream> for $Enum {
            #[track_caller]
            fn into(self) -> proc_macro2::TokenStream {
                self.to_diagnostic().to_compile_error()
            }
        }

        impl Into<$crate::errors::Diagnostic> for $Enum {
            #[track_caller]
            fn into(self) -> $crate::errors::Diagnostic {
                self.to_diagnostic()
            }
        }

        impl Into<syn::Error> for $Enum {
            #[track_caller]
            fn into(self) -> syn::Error {
                self.to_diagnostic().to_syn_error()
            }
        }

        impl Into<String> for $Enum {
            #[track_caller]
            fn into(self) -> String {
                self.to_string()
            }
        }

    };

    (@sem $s:ident) => {
        $crate::errors::ErrorTag::Core($crate::errors::ErrorCatalog::$s)
    };

    (@sem $s:expr) => {
        $crate::errors::ErrorTag::Custom($s)
    };

    (@span spanned: $e:expr) => {
        $crate::errors::DiagSpan::spanned(&$e)
    };

    (@span tokens: $e:expr) => {
        $crate::errors::DiagSpan::tokens(&$e)
    };

    (@span span: $e:expr) => {
        $crate::errors::DiagSpan::span(&$e)
    };

    (@helps) => { Vec::new() };

    (@helps $($h:tt),+ $(,)?) => {
        vec![ $( diagnostics!(@help $h) ),* ]
    };

    (@help { $kind:ident : $e:expr, msg: $m:expr $(,)? }) => {
        $crate::errors::Help {
            msg: diagnostics!(@msg $m),
            span: Some(diagnostics!(@span $kind: $e)),
        }
    };

    (@help { msg: $m:expr }) => {
        $crate::errors::Help {
            msg: diagnostics!(@msg $m),
            span: None,
        }
    };

    (@help $m:tt) => {
        $crate::errors::Help {
            msg: diagnostics!(@msg $m),
            span: None,
        }
    };

    (@notes) => { Vec::new() };

    (@notes $($m:expr),+ $(,)?) => {
        vec![
            $(
                diagnostics!(@note $m)
            ),*
        ]
    };

    (@note $m:literal) => {
        $crate::errors::Note {
            msg: diagnostics!(@msg $m),
        }
    };

    (@note $m:expr) => {
        $crate::errors::Note {
            msg: diagnostics!(@msg $m),
        }
    };

    (@msg $m:literal) => {
        $m.to_string().into()
    };

    (@msg $m:expr) => {
        $m
    };
}

/// Defines simple internal bug diagnostics.
///
/// Intended for invariant failures and unreachable proc-macro states.
/// These diagnostics should normally never be visible unless the
/// macro system itself contains a bug.
///
/// Syntax:
///
/// ```ignore
/// bug_diagnostics! {
///     space: MyErrors, // declared via error_space!()/error_spaces!()
///     bug: "internal invariant violated", // first appended note
///
///     pub enum Bug {
///         InvalidState => "unexpected parser state",
///         MissingNode  => "missing generated node",
///     }
/// }
/// ```
///
/// Each variant:
/// - uses [`Bug`](ErrorCatalog::Bug) classification
/// - emits at call-site span
/// - appends standard bug-report notes
#[macro_export]
macro_rules! bug_diagnostics {
    (
        space: $space:ident,
        bug: $bug:expr,

        $vis:vis enum $name:ident {
            $(
                $variant:ident => $msg:literal
            ),* $(,)?
        } $(,)?
    ) => {
        diagnostics! {
            space: $space,

            $vis enum $name {
                $(
                    $variant {
                        fields: {},
                        msg: format!($msg),
                        tags: [Bug],
                        span: { span: proc_macro2::Span::call_site() },
                        note: [
                            $bug,
                            $crate::misc::LOGICAL_BUG.to_string(),
                            $crate::misc::REPORT_BUG.to_string()
                        ]
                    },
                )*
            }
        }
    };
}

/// Defines a single macro-system error space with a fixed range and metadata.
///
/// > **NOTE**: Use [`crate::error_spaces!`] for multiple enums with shared
/// `namespace` and `maintain` to avoid repetition and ensure globally valid ranges.
///
/// Generates an enum where each variant maps to a concrete error ID
/// (`namespace + offset`) and exposes compile-time constant utilities
/// via inherent methods.
///
/// - Each variant is assigned a sequential ID within the declared range.
/// - Ensures the range is sufficient for all variants (compile-time check).
/// - Provides compile-time introspection and conversion capabilities.
/// - Attaches maintainer + repository metadata via [`ErrorMaintainers`].
/// - `namespace` is a string literal used in formatted error output.
///
/// ## Syntax
///
/// ```ignore
/// error_space! {
///     /// Declared Error Space
///     pub enum MyErrors {                // Required: error space enum
///         namespace: "MY",               // Required: short identifier
///         maintain: MaintainInfo,        // Required: maintainer metadata
///         range: 1000..=1099,            // Required: fixed range
///         variants: {                    // Required: variants
///             A,                         // no description
///             B => "description",        // Optional: description
///         }
///     }
/// }
/// ```
///
/// ## Example
///
/// ```ignore
/// error_space! {
///     /// Declared Error Space
///     pub enum CounterErrors {
///         namespace: "CNT",
///         maintain: MaintainInfo,
///         range: 1000..=1002,
///         variants: {
///             /// Try New Input
///             InvalidInput => "invalid input",
///             Overflow,
///             Underflow,
///         }
///     }
/// }
///
/// let err = CounterErrors::InvalidInput;
/// let id = err.to_error_id();     // [CNT:1000]
/// let info = err.to_error_info(); // includes desc, location, maintainers
/// ```
///
/// ## See also
///
/// - [`ErrorMaintainers`]
/// - [`crate::error_spaces!`]
/// - [`crate::diagnostics!`]
/// - [`crate::token_err!`]
/// - [`crate::callsite_err!`]
#[macro_export]
macro_rules! error_space {
    (
        $(#[$enum_meta:meta])*
        $vis:vis enum $Enum:ident {
            namespace: $namespace:expr,
            maintain: $maintain:expr,
            range: $start:literal ..= $end:literal,
            variants: {
                $(
                    $(#[$variant_meta:meta])*
                    $Var:ident $(=> $desc:literal)?
                ),+ $(,)?
            }

        }
    ) => {
        $(#[$enum_meta])*
        #[repr(u32)]
        #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
        $vis enum $Enum {
            $(
                $(#[$variant_meta])*
                $Var
            ),+
        }

        #[allow(unused)]
        impl $Enum {

            #[doc(hidden)]
            /// Starting offset of this error space
            const _RANGE_START: u32 = $start;

            #[doc(hidden)]
            /// Total number of error-space variants
            const _VARIANT_COUNT: u32 = {
                let mut n = 0;
                $(let _ = stringify!($Var); n += 1;)*
                n
            };

            #[doc(hidden)]
            /// Ensures declared range can fit all error-space variants
            const _RANGE_CHECK: () = {
                if $start + ($Enum::_VARIANT_COUNT - 1) > $end {
                    panic!(concat!(
                        "error_space!: enum variants exceed declared range\n",
                        "  enum     = ", stringify!($Enum), "\n",
                        "  declared = ", stringify!($start), "..=", stringify!($end), "\n",
                        "  reason   = declared range is too small for the number of variants\n",
                        "  help     = increase the range or reduce variants\n",
                    ));
                }
            };

            #[doc(hidden)]
            /// Forces compile-time error-space range validation
            const _ASSERT_RANGE: () = {
                let _ = Self::_RANGE_CHECK;
            };

            // NOTE: as of current stable Rust, `const fn` in traits is not supported;
            // these utility functions will be refactored into a trait once available.

            // --- ID Conversion ---

            /// Converts error-space enum variant to its concrete error ID representation.
            $vis const fn to_error_id(self) -> $crate::errors::ErrorId {
                $crate::errors::ErrorId {
                    space: $namespace,
                    id: $start + self as u32,
                }
            }

            /// Attempts to construct error-space enum from raw ID.
            $vis const fn try_from_id(id: u32) -> Option<Self> {
                if id < Self::_RANGE_START {
                    return None;
                }

                let idx = id - Self::_RANGE_START;

                if idx < Self::_VARIANT_COUNT {
                    // SAFETY:
                    // - enum is #[repr(u32)]
                    // - variants are contiguous
                    // - idx < _VARIANT_COUNT is checked
                    return Some(unsafe { core::mem::transmute(idx as u32) })
                }
                None
            }

            /// Constructs error-space enum from raw ID (panics if invalid).
            $vis const fn from_id(id: u32) -> Self {
                match Self::try_from_id(id) {
                    Some(e) => e,
                    None => panic!("invalid error id"),
                }
            }

            /// Returns full error info (ID + metadata) of the error-space variant.
            $vis const fn to_error_info(self) -> $crate::errors::ErrorInfo {
                $crate::errors::ErrorInfo {
                    id: self.to_error_id(),
                    details: $crate::errors::ErrorDetails {
                        name: self.name(),
                        desc: self.desc(),
                        path: self.path(),
                        loc: self.loc(),
                        docs: self.docs(),
                    },
                    maintain: $maintain,
                }
            }

            // --- Metadata Access ---

            /// Returns error-space variant name.
            $vis const fn name(self) -> &'static str {
                match self {
                    $( Self::$Var => stringify!($Var), )+
                }
            }

            /// Returns error-space variant description (if provided, else empty).
            $vis const fn desc(self) -> &'static str {
                match self {
                    $( Self::$Var => $crate::error_space!(@unwrap_desc $( $desc )?), )+
                }
            }

            /// Returns fully-qualified error-space variant path.
            $vis const fn path(self) -> &'static str {
                match self {
                    $( Self::$Var => stringify!($Enum::$Var), )+
                }
            }

            /// Returns errors-space definition location (`file:line:col`).
            $vis const fn loc(self) -> &'static str {
                match self {
                    $( Self::$Var => concat!(file!(), ":", line!(), ":", column!()), )+
                }
            }

            // --- This is for Future Error-ID Doc-Webpage Generation ---

            $vis const fn docs(self) -> &'static str {
              ""
            }

            // --- Collections ---

            /// Returns all error-space variants.
            $vis const fn all() -> &'static [Self] {
                &[ $( Self::$Var ),+ ]
            }

            /// Returns all error-space variant names.
            $vis const fn all_names() -> &'static [&'static str] {
                &[ $( stringify!($Var) ),+ ]
            }

            /// Returns all error info entries (ID + metadata) of all error-space variants.
            $vis const fn all_info() -> &'static [$crate::errors::ErrorInfo] {
                &[
                    $(
                        $crate::errors::ErrorInfo {
                            id: $crate::errors::ErrorId {
                                space: $namespace,
                                id: $start + $Enum::$Var as u32,
                            },
                            details: $crate::errors::ErrorDetails {
                                name: stringify!($Var),
                                desc: $crate::error_space!(@unwrap_desc $( $desc )?),
                                path: stringify!($Enum::$Var),
                                loc: concat!(file!(), ":", line!(), ":", column!()),
                                docs: "",
                            },
                            maintain: $maintain
                        }
                    ),+
                ]
            }

            /// Returns all error-space variants IDs.
            $vis const fn all_ids() -> &'static [$crate::errors::ErrorId] {
                &[
                    $(
                        $crate::errors::ErrorId {
                            space: $namespace,
                            id: $start + $Enum::$Var as u32,
                        }
                    ),+
                ]
            }
        }

        impl Into<$crate::errors::ErrorId> for $Enum {
            fn into(self) -> $crate::errors::ErrorId {
                self.to_error_id()
            }
        }

        impl Into<$crate::errors::ErrorInfo> for $Enum {
            fn into(self) -> $crate::errors::ErrorInfo {
                self.to_error_info()
            }
        }
    };

    (@unwrap_desc $d:literal) => { $d };

    (@unwrap_desc) => { "" };

}

/// Defines multiple macro-system error spaces with contiguous, non-overlapping ranges.
///
/// Each error space provides concrete error IDs derived from enum variants,
/// enabling compile-time introspection, debugging, and issue resolution.
///
/// - Each variant is assigned a sequential error index within its range
///   (in declaration order).
/// - Attaches maintainer + hosted repository metadata via [`ErrorMaintainers`].
/// - `space` is a string literal used as a short identifier in error output.
///
/// ### Syntax
///
/// ```ignore
/// error_spaces! {
///     space: "CNT",                       // Required: short identifier
///     maintain: MaintainInfo,             // Required: maintainer metadata
///
///     pub enum CounterErrors {            // Required: error space enum
///         range: 1000..=1099,             // Required: unique range
///         variants: {                     // Required: variants
///             InvalidInput,
///             Overflow
///         }
///     }
///
///     pub enum ParseErrors {
///         range: 1100..=1199,             // Must be contiguous
///         variants: {
///             UnexpectedToken,
///             MissingField
///         }
///     }
/// }
/// ```
///
/// ### Example
///
/// ```ignore
/// let err = CounterErrors::InvalidInput;
///
/// let id = err.to_error_id();     // [CNT:1000]
/// let info = err.to_error_info(); // includes desc, location, maintainers
/// ```
///
/// ## See also
///
/// - [`ErrorMaintainers`]
/// - [`crate::diagnostics!`]
/// - [`crate::error_space!`]
/// - [`crate::token_err!`]
/// - [`crate::callsite_err!`]
#[macro_export]
macro_rules! error_spaces {
    (
        space: $namespace:expr,
        maintain: $maintain:expr,

        $(
            $(#[$enum_meta:meta])*
            $vis:vis enum $Enum:ident {
                range: $start:literal ..= $end:literal,
                variants: {
                    $(
                        $(#[$variant_meta:meta])*
                        $Var:ident $(=> $desc:literal)?
                    ),+ $(,)?
                }
            }
        )+ $(,)?
    ) => {
        $(
            $crate::error_space! {
                $(#[$enum_meta])*
                $vis enum $Enum {
                    namespace: $namespace,
                    maintain: $maintain,
                    range: $start ..= $end,
                    variants: {
                        $(
                            $(#[$variant_meta])*
                            $Var $(=> $desc)?
                        ),+
                    }
                }
            }
        )+

        #[doc(hidden)]
        /// Validates all ranges: ensures start <= end, no overlaps, and contiguous layout
        const _GLOBAL_RANGE_CHECK: () = {
            $(
                if $start > $end {
                    panic!(concat!(
                        "error_spaces!: invalid range for ",
                        stringify!($Enum), "\n",
                        "  range   : ", stringify!($start), "..=", stringify!($end), "\n",
                        "  reason  : start > end\n",
                    ));
                }
            )+

            error_spaces!(@check_overlap
                $( ($Enum, $start, $end) ),+
            );

            error_spaces!(@check_contiguous
                $( ($Enum, $start, $end) ),+
            );
        };

        #[doc(hidden)]
        /// Forces compile-time all error-spaces range validation
        const _FORCE_GLOBAL_RANGE_CHECK: () = {
            let _ = _GLOBAL_RANGE_CHECK;
        };
    };

    (@check_overlap
        ($E1:ident, $S1:literal, $E1End:literal),
        $( ($E2:ident, $S2:literal, $E2End:literal) ),+
    ) => {
        $(
            if !($E1End < $S2 || $E2End < $S1) {
                panic!(concat!(
                    "error_spaces!: overlapping ranges detected\n",
                    "  ", stringify!($E1), " : ",
                    stringify!($S1), "..=", stringify!($E1End), "\n",
                    "  ", stringify!($E2), " : ",
                    stringify!($S2), "..=", stringify!($E2End), "\n",
                    "  reason : ranges intersect\n",
                ));
            }
        )+

        error_spaces!(@check_overlap $( ($E2, $S2, $E2End) ),+);
    };

    (@check_overlap ($Enum:ident, $Start:literal, $End:literal)) => {};

    (@check_contiguous
        ($E1:ident, $S1:literal, $E1End:literal),
        ($E2:ident, $S2:literal, $E2End:literal)
        $(, $rest:tt)*
    ) => {
        if $E1End + 1 != $S2 {
            panic!(concat!(
                "error_spaces!: non-contiguous ranges detected\n",
                "  ", stringify!($E1), " : ",
                stringify!($S1), "..=", stringify!($E1End), "\n",
                "  ", stringify!($E2), " : ",
                stringify!($S2), "..=", stringify!($E2End), "\n",
                "  expected : ", stringify!($S2),
                " == ", stringify!($E1End), " + 1\n",
            ));
        }

        error_spaces!(@check_contiguous
            ($E2, $S2, $E2End)
            $(, $rest)*
        );
    };

    (@check_contiguous ($Enum:ident, $Start:literal, $End:literal)) => {};

    (@desc $d:literal) => { $d };

    (@desc) => { "" };

}

// ===============================================================================
// ``````````````````````````````` ESSENTIAL TYPES ```````````````````````````````
// ===============================================================================

/// Core semantic classification of errors emitted by the macro system.
///
/// This enum categorizes errors by *intent and origin* (user input,
/// macro logic, environment, etc.), enabling consistent diagnostics
/// and specialized handling across the ecosystem.
///
/// - Used for tagging diagnostics with meaningful categories.
/// - Enables utility logic to react differently based on error type.
///   (e.g., [`ErrorCatalog::Bug`] can guide users to report issues).
/// - Provides a stable, coarse-grained classification layer.
///
/// ## Notes
///
/// - This enum is `#[non_exhaustive]`; new categories may be added.
/// - Consumers should avoid exhaustive matching and handle unknown variants gracefully.
/// - Intended for semantic grouping, not fine-grained error details.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum ErrorCatalog {
    /// A bug in the proc macro itself.
    /// The user cannot fix this; it should be reported.
    Bug,

    /// The macro currently doesn't support this structure.
    /// But may support in the future if requested.
    Future,

    /// The expected structure/feature is unstable currently
    /// and actively being developed.
    Unstable(ActiveDevelopment),

    /// Input was syntactically valid Rust, but not supported
    /// by this macro for semantic reasons (wrong item kind, missing parts, etc.).
    Unsupported,

    /// Input was malformed or violated required structure.
    /// This is a user error.
    InvalidInput,

    /// The macro hit a known Rust limitation.
    Limitation,

    /// Anything else unexpected but not necessarily a bug.
    Unexpected,
}

#[derive(Debug, Clone)]
pub struct ActiveDevelopment {
    name: &'static str,
    tracker: &'static str,
    eta: &'static str,
    helps: Vec<Help>,
}

/// Tag attached to the macro-system error-diagnostics for classification.
#[derive(Debug, Clone)]
pub enum ErrorTag {
    /// Standard error-diagnotic category via [`ErrorCatalog`].
    Core(ErrorCatalog),

    /// User-defined classification via a string literal.
    Custom(&'static str),
}

/// Maintainer contact information attached to a macro-system's error space.
///
/// Provides optional details to help users reach maintainers
/// or locate the source repository for the error.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ErrorMaintainers {
    /// Contact email (if available).
    pub mailto: Option<&'static str>,

    /// Hosted repository information (if available).
    pub git_host: Option<GitHost>,
}

/// Supported Git hosting platforms for the macro-system maintainer metadata.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum GitHost {
    /// A GitHub remote-hosted repository.
    Github(GitHostInfo),

    /// A GitLab remote-hosted repository.
    GitLab(GitHostInfo),
}

/// Repository information for a Git hosting platform.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct GitHostInfo {
    /// Repository owner or organization.
    pub owner: &'static str,

    /// Repository name.
    pub repo: &'static str,

    /// Optional bug hint used for issue titles (and GitHub templates).
    ///
    /// - GitHub: template name (e.g., `bug_report` -> `bug_report.yml`)
    /// - Used for issue title identification (both GitHub & GitLab)
    pub bug_label: Option<&'static str>,

    /// Optional feature request used for issue titles (and GitHub templates).
    ///
    /// - GitHub: template name (e.g., `feature_req` -> `feature_req.yml`)
    /// - Used for issue title identification (both GitHub & GitLab)
    pub feat_label: Option<&'static str>,
}

// ===============================================================================
// `````````````````````````````` PARSING DIAGNOSTIC `````````````````````````````
// ===============================================================================

/// Helpers for generating structured parsing diagnostics.
///
/// Implement this trait for any parsed input type to describe:
/// - what kind of input it represents (`KIND`)
/// - what the macro expects (`EXPECTED`)
/// - examples of valid syntax (`EXAMPLE`)
///
/// Provides default methods to produce consistent diagnostics,
/// including a ready-to-return `TokenStream` via [`Self::syn_parse_error`]
/// and [`Self::span_parse_error`].
///
/// This is not tied to [`syn`], but integrates easily with it.
pub trait ParseDiagnostic {
    /// Human-readable description of this argument kind.
    const KIND: &'static str;

    /// What the macro expects to receive for this argument kind.
    const EXPECTED: &'static str;

    /// Example showing valid syntax for this argument format.
    const EXAMPLE: &[&'static str];

    /// Fallback parse error info.
    ///
    /// Uses the utility crate's error space (namespace + ID)
    /// when the user crate does not provide one.
    const DEFAULT_ERROR_INFO: ErrorInfo = ParseError::Default.to_error_info();

    /// Produces a parse-failed [`Diagnostic`] suitable for conversion
    /// into a compile-time error.
    ///
    /// - Uses [`Self::KIND`] for describing the input
    /// - Uses [`Self::EXPECTED`] for the expected form (help message)
    /// - Attaches [`Self::EXAMPLE`] entries as notes
    /// - Falls back to [`Self::DEFAULT_ERROR_INFO`] if no error info is provided
    #[track_caller]
    fn parse_diagnostic(span: &DiagSpan, err_site: Option<ErrorInfo>) -> Diagnostic {
        let mut notes = Vec::<Note>::new();
        for eg in Self::EXAMPLE {
            notes.push(Note {
                msg: format!("example: {}", eg),
            });
        }
        notes.push(Note {
            msg: format!("ensure if the input is indeed a `{}`", Self::KIND),
        });
        let diag = Diagnostic {
            error: match err_site {
                Some(i) => i,
                None => Self::DEFAULT_ERROR_INFO,
            },
            tags: vec![ErrorTag::Custom("parse_error")],
            msg: format!("cannot parse the given `{}`", Self::KIND,),
            span: span.clone(),
            helps: vec![Help {
                msg: format!("expected a valid `{}`", Self::EXPECTED),
                span: None,
            }],
            notes,
        };

        diag
    }

    /// Combines a [`syn::Error`] with a structured parsing diagnostic.
    ///
    /// Used when parsing via [`syn`] fails and you want to attach an additional
    /// structured diagnostic alongside the original error.
    ///
    /// Both diagnostics are emitted under the same span with separate messages,
    /// returning a [`TokenStream`]
    #[track_caller]
    fn syn_parse_error(err: &mut Error, err_site: Option<ErrorInfo>) -> TokenStream {
        let diag = Self::parse_diagnostic(&DiagSpan::Span(err.span()), err_site);
        err.combine(diag.to_syn_error());
        err.clone().into_compile_error()
    }

    /// Emits a parsing diagnostic from a [`Span`].
    ///
    /// Used when you want to attach a span directly to the generated parsing diagnostic.
    #[track_caller]
    fn span_parse_error(span: &Span, err_site: Option<ErrorInfo>) -> TokenStream {
        let diag = Self::parse_diagnostic(&DiagSpan::Span(*span), err_site);
        diag.to_compile_error()
    }
}

// ===============================================================================
// `````````````````````````````` PUBLIC UTILITIES ```````````````````````````````
// ===============================================================================

// Public diagnostic utilities.
//
// Usually used indirectly via macros; only needed for low-level
// or manual diagnostic construction.

/// Emits a macro-expansion compile-time error at the given span.
///
/// Used internally by [`crate::token_err!`].
#[track_caller]
pub fn token_err_impl(span: impl ToTokens, msg: impl AsRef<str>) -> TokenStream {
    Error::new_spanned(span, format_msg(msg)).to_compile_error()
}

/// Emits a macro-expansion compile-time error at the macro call site.
///
/// Used internally by [`crate::callsite_err!`].
#[track_caller]
pub fn callsite_err_impl(msg: impl AsRef<str>) -> TokenStream {
    Error::new(Span::call_site(), format_msg(msg)).to_compile_error()
}

/// Unique identifier for a macro-expansion error within an error space.
///
/// Combines a short namespace (`space`) with a numeric identifier (`id`),
/// typically formatted as `[SPACE:ID]` (e.g., `[CNT:45]`).
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ErrorId {
    /// Short namespace identifier.
    pub space: &'static str,

    /// Numeric error identifier within the space.
    pub id: u32,
}

/// Complete macro-expansion error information (ID, details, and maintainers).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ErrorInfo {
    /// Unique error identifier.
    pub id: ErrorId,

    /// Static metadata about the error.
    pub details: ErrorDetails,

    /// Maintainer contact and repository info.
    pub maintain: ErrorMaintainers,
}

/// Static metadata describing a macro-system's error.
///
/// Includes human-readable identifiers and source location.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ErrorDetails {
    /// Error Variant name.
    pub name: &'static str,

    /// Description (if provided, else empty).
    pub desc: &'static str,

    /// Fully-qualified variant path.
    pub path: &'static str,

    /// Definition location (`file:line:col`).
    pub loc: &'static str,

    /// Markdown Documentation.
    #[doc(hidden)]
    pub docs: &'static str,
}

/// Span representation used in the macro-system's error diagnostics.
///
/// Allows attaching either a precise source span or token-based span.
#[derive(Debug, Clone)]
pub enum DiagSpan {
    /// A concrete source span.
    Span(Span),

    /// Tokens used to derive a span (e.g., syn nodes, quoted tokens).
    Tokens(TokenStream),
}

/// Structured error-diagnostic emitted by the macro-system.
///
/// Combines error identity, message, span, and optional context.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// Error identity and metadata.
    pub error: ErrorInfo,

    /// Classification tags.
    pub tags: Vec<ErrorTag>,

    /// Primary error message.
    pub msg: String,

    /// Span where the error is attached.
    pub span: DiagSpan,

    /// Optional help messages (may include own spans).
    pub helps: Vec<Help>,

    /// Additional notes (unspanned context).
    pub notes: Vec<Note>,
}

/// Additional guidance attached to the macro-system's error diagnostic.
///
/// Can optionally point to a specific span.
#[derive(Debug, Clone)]
pub struct Help {
    /// Help message.
    pub msg: String,

    /// Optional span this help refers to.
    pub span: Option<DiagSpan>,
}

/// Additional context attached to the macro-system's error diagnostic.
///
/// Always spanned towards primary error-message.
#[derive(Debug, Clone)]
pub struct Note {
    /// Note message.
    pub msg: String,
}

/// Collection of multiple diagnostics.
///
/// Used to aggregate and emit several errors together,
/// convertible into [`TokenStream`] via `.into()`.
pub struct MultiDiagnostic(pub Vec<Diagnostic>);

// ===============================================================================
// ``````````````````````````````` INHERENT IMPLS ````````````````````````````````
// ===============================================================================

impl GitHost {
    /// Returns the base issue tracker URL for the repository.
    pub fn issue_url(&self) -> String {
        match self {
            GitHost::Github(hub) => {
                format!("https://github.com/{}/{}/issues/", hub.owner, hub.repo,)
            }

            GitHost::GitLab(lab) => {
                format!("https://gitlab.com/{}/{}/-/issues/", lab.owner, lab.repo,)
            }
        }
    }

    /// Returns a prefilled bug report URL (if supported).
    pub fn bug_issue_url(&self, title: &str, body: Option<&str>) -> Option<String> {
        let url = match self {
            GitHost::Github(hub) => {
                let template = hub.bug_label?;
                match body {
                    Some(body) => format!(
                        "https://github.com/{}/{}/issues/new\
                        ?template={}.yml&title={}&body={}",
                        hub.owner,
                        hub.repo,
                        template,
                        encode(title),
                        encode(body),
                    ),
                    None => format!(
                        "https://github.com/{}/{}/issues/new\
                        ?template={}.yml&title={}",
                        hub.owner,
                        hub.repo,
                        template,
                        encode(title),
                    ),
                }
            }

            GitHost::GitLab(lab) => match body {
                Some(body) => format!(
                    "https://gitlab.com/{}/{}/-/issues/new\
                        ?issue[title]={}&issue[description]={}",
                    lab.owner,
                    lab.repo,
                    encode(title),
                    encode(body),
                ),
                None => format!(
                    "https://gitlab.com/{}/{}/-/issues/new\
                        ?issue[title]={}",
                    lab.owner,
                    lab.repo,
                    encode(title),
                ),
            },
        };
        Some(url)
    }

    /// Returns a prefilled feature request URL (if supported).
    pub fn feat_issue_url(&self, title: &str, body: Option<&str>) -> Option<String> {
        let url = match self {
            GitHost::Github(hub) => {
                let template = hub.feat_label?;
                match body {
                    Some(body) => format!(
                        "https://github.com/{}/{}/issues/new\
                        ?template={}.yml&title={}&body={}",
                        hub.owner,
                        hub.repo,
                        template,
                        encode(title),
                        encode(body),
                    ),
                    None => format!(
                        "https://github.com/{}/{}/issues/new\
                        ?template={}.yml&title={}",
                        hub.owner,
                        hub.repo,
                        template,
                        encode(title),
                    ),
                }
            }

            GitHost::GitLab(lab) => match body {
                Some(body) => format!(
                    "https://gitlab.com/{}/{}/-/issues/new\
                        ?issue[title]={}&issue[description]={}",
                    lab.owner,
                    lab.repo,
                    encode(title),
                    encode(body),
                ),
                None => format!(
                    "https://gitlab.com/{}/{}/-/issues/new\
                        ?issue[title]={}",
                    lab.owner,
                    lab.repo,
                    encode(title),
                ),
            },
        };
        Some(url)
    }
}

impl ErrorInfo {
    /// Returns mail-based issue reporting hint.
    pub fn mail_report(&self) -> Option<String> {
        let mailto = self.maintain.mailto?;
        Some(format!("report issue via mail: ({})", mailto))
    }

    /// Returns issue tracker URL.
    pub fn issue_url(&self) -> Option<String> {
        let host = self.maintain.git_host?;
        Some(format!("report issue via tracker: {}", host.issue_url()))
    }

    /// Returns prefilled bug report URL.
    pub fn bug_report_url(&self) -> Option<String> {
        let host = self.maintain.git_host?;

        let label = match host {
            GitHost::Github(g) | GitHost::GitLab(g) => g.bug_label?,
        };

        let title = self.issue_title(label);
        let url = host.bug_issue_url(&title, None)?;

        Some(format!("report issue instantly: {}", url))
    }

    /// Returns prefilled feature request URL.
    pub fn feat_request_url(&self) -> Option<String> {
        let host = self.maintain.git_host?;

        let label = match host {
            GitHost::Github(g) | GitHost::GitLab(g) => g.feat_label?,
        };

        let title = self.issue_title(label);
        let url = host.feat_issue_url(&title, None)?;

        Some(format!("request feature instantly: {}", url))
    }

    /// Constructs a standard issue title including label, ID, crate and rustc version.
    pub fn issue_title(&self, label: &str) -> String {
        format!(
            "<{}>{}({})({})",
            label,
            self.id,
            env!("CARGO_PKG_VERSION"),
            rustc_version_runtime::version()
        )
    }
}

impl DiagSpan {
    /// Creates from a raw span.
    pub fn span(span: &proc_macro2::Span) -> Self {
        DiagSpan::Span(*span)
    }

    /// Creates from a value implementing [`ToTokens`].
    pub fn tokens<T: quote::ToTokens>(value: &T) -> Self {
        DiagSpan::Tokens(value.to_token_stream())
    }

    /// Creates from a value implementing [`Spanned`].
    pub fn from_spanned<T: Spanned>(value: &T) -> Self {
        DiagSpan::Span(value.span())
    }

    /// Converts into a [`syn::Error`].
    fn to_syn_error(&self, msg: impl std::fmt::Display) -> Error {
        match self {
            DiagSpan::Span(span) => Error::new(*span, msg),
            DiagSpan::Tokens(tokens) => Error::new_spanned(tokens.clone(), msg),
        }
    }
}

impl Diagnostic {
    /// Renders an error-diagnostic header with caller location (`dev` feature).
    ///
    /// ```text
    /// [src/lib.rs:42:5] unsupported field type
    /// ```
    ///
    /// Used during development to indicate where the diagnostic
    /// originated inside the macro crate.
    #[cfg(feature = "dev")]
    #[track_caller]
    fn header_msg(&self) -> String {
        let loc = std::panic::Location::caller();
        format!(
            "[{}:{}:{}]\n{}\n",
            loc.file(),
            loc.line(),
            loc.column(),
            self.msg,
        )
    }

    /// Renders a compact error-diagnostic header.
    ///
    /// ```text
    /// [INSTANCE:105] unsupported field type
    /// ```
    ///
    /// Used in normal builds `not(feature = "dev")` for stable, user-facing diagnostics.
    #[cfg(not(feature = "dev"))]
    fn header_msg(&self) -> String {
        format!(
            "[{}:{}] {}\n",
            self.error.id.space, self.error.id.id, self.msg
        )
    }

    /// Converts the macro-system's error diagnostic into a [`syn::Error`].
    ///
    /// - Emits the primary diagnostic at its span.
    /// - Spanned helps are emitted as separate diagnostics and linked via IDs.
    /// - Unspanned helps are included in the primary message.
    ///
    /// ## Format (non-bug)
    ///
    /// ```text
    /// error: [SPACE:ID] <message>
    ///        help:
    ///          • <inline help>
    ///          • ...
    ///          • see sub-diagnostics: [SPACE:ID:i], ...
    ///
    ///  --> <file>:<line>:<col>
    ///   |
    /// <line> | <code>
    ///   |     ^^^^^
    ///
    /// error: [SPACE:ID:i]
    ///        help: <spanned help message>
    ///
    ///  --> <file>:<line>:<col>
    ///   |
    /// <line> | <code>
    ///   |     ^^^^^
    /// ```
    ///
    /// ## Special handling
    ///
    /// - For [`ErrorCatalog::Bug`], additional diagnostics are generated:
    ///   - Maintainer contact (mail / issue tracker)
    ///   - Optional prefilled bug report link
    ///   - Drop-in issue title for quick reporting
    ///
    /// ## Format (bug)
    ///
    /// ```text
    /// error: [SPACE:ID] internal bug found, please report to maintainers
    ///        help:
    ///          • report issue via tracker: <url>
    ///          • report issue via mail: (<email>)
    ///          • drop-in issue title: <generated-title>
    ///          • report issue instantly: <prefilled-url>
    ///
    ///  --> <file>:<line>:<col>
    ///   |
    /// <line> | <code>
    ///   |     ^^^^^
    /// ```
    ///
    /// This enables structured diagnostics with optional bug-reporting integration.
    #[track_caller]
    pub fn to_syn_error(&self) -> Error {
        // Emit primary diagnostic at its span
        let mut err = self.span.to_syn_error(self.render_primary_msg());

        // Attach spanned helps as separate diagnostics linked by `[SPACE:ID:i]`
        for (i, Help { msg: help, span }) in self.helps.iter().enumerate() {
            if let Some(s) = span {
                err.combine(s.to_syn_error(format!(
                    "[{}:{}:{}]\nhelp: {help}",
                    self.error.id.space, self.error.id.id, i
                )));
            }
        }

        // If classified as a bug, attach reporting diagnostics
        if self
            .tags
            .iter()
            .any(|a| matches!(a, ErrorTag::Core(ErrorCatalog::Bug)))
        {
            // Collect reporting channels (tracker / mail)
            let githost = self.error.issue_url();
            let mailto = self.error.mail_report();

            // Build base help messages
            let mut helps = match (githost, mailto) {
                (None, None) => return err,
                (Some(g), None) => vec![Help { msg: g, span: None }],
                (None, Some(m)) => vec![Help { msg: m, span: None }],
                (Some(g), Some(m)) => {
                    vec![Help { msg: g, span: None }, Help { msg: m, span: None }]
                }
            };

            // Add prefilled bug report info if available
            if let Some(inst) = self.error.bug_report_url() {
                let title = match self.error.maintain.git_host.unwrap() {
                    GitHost::Github(g) | GitHost::GitLab(g) => g.bug_label.unwrap(),
                };

                helps.push(Help {
                    msg: format!("drop-in issue title: {}", self.error.issue_title(title)),
                    span: None,
                });

                helps.push(Help {
                    msg: inst,
                    span: None,
                });
            }

            // Emit secondary diagnostic for bug reporting
            let diag = Diagnostic {
                error: self.error,
                tags: vec![ErrorTag::Custom("bug_found")],
                msg: "internal bug found, please report to maintainers".into(),
                span: self.span.clone(),
                helps,
                notes: Vec::new(),
            };

            err.combine(diag.to_syn_error());
        }

        // If classified as a feature request (Future), attach reporting diagnostics
        if self
            .tags
            .iter()
            .any(|a| matches!(a, ErrorTag::Core(ErrorCatalog::Future)))
        {
            // Collect reporting channels (tracker / mail)
            let githost = self.error.issue_url();
            let mailto = self.error.mail_report();

            // Build base help messages
            let mut helps = match (githost, mailto) {
                (None, None) => return err,
                (Some(g), None) => vec![Help { msg: g, span: None }],
                (None, Some(m)) => vec![Help { msg: m, span: None }],
                (Some(g), Some(m)) => {
                    vec![Help { msg: g, span: None }, Help { msg: m, span: None }]
                }
            };

            // Add prefilled feature request info if available
            if let Some(inst) = self.error.feat_request_url() {
                let title = match self.error.maintain.git_host.unwrap() {
                    GitHost::Github(g) | GitHost::GitLab(g) => g.bug_label.unwrap(),
                };

                helps.push(Help {
                    msg: format!("drop-in issue title: {}", self.error.issue_title(title)),
                    span: None,
                });

                helps.push(Help {
                    msg: inst,
                    span: None,
                });
            }

            // Emit secondary diagnostic for feature request
            let diag = Diagnostic {
                error: self.error,
                tags: vec![ErrorTag::Custom("feat_request")],
                msg: "requested feature unavailable currently, request to maintainers".into(),
                span: self.span.clone(),
                helps,
                notes: Vec::new(),
            };

            err.combine(diag.to_syn_error());
        }

        // If classified as unstable active development, attach reference URL
        if let Some(dev) = self.tags.iter().find_map(|tag| match tag {
            ErrorTag::Core(ErrorCatalog::Unstable(dev)) => Some(dev),
            _ => None,
        }) {
            let tracker = dev.tracker;
            let feat = dev.name;
            let eta = dev.eta;
            // Emit secondary diagnostic for unstable feature information
            let diag = Diagnostic {
                error: self.error,
                tags: vec![ErrorTag::Custom("feat_request")],
                msg: format!("{feat} is unstable and under active development"),
                span: self.span.clone(),
                helps: dev.helps.clone(),
                notes: vec![
                    Note {
                        msg: format!("track development at : {tracker}"),
                    },
                    Note {
                        msg: format!("eta: {eta}"),
                    },
                ],
            };

            err.combine(diag.to_syn_error());
        }

        // Return combined diagnostic
        err
    }

    /// Appends multiple error-diagnostic helps and notes of the macro-system.
    pub fn append(&mut self, helps: Vec<Help>, notes: Vec<Note>) {
        let n = &mut self.notes;
        for note in notes {
            n.push(note);
        }
        let h = &mut self.helps;
        for help in helps {
            h.push(help);
        }
    }

    // Extends an error diagnostic of the macro-system with optional help and note.
    pub fn extend(&mut self, help: Option<Help>, note: Option<Note>) {
        if let Some(h) = help {
            self.helps.push(h);
        }

        if let Some(n) = note {
            self.notes.push(n);
        }
    }

    /// Converts an error diagnostic of the macro-system into a `TokenStream` ([`compile_error!`]).
    #[track_caller]
    pub fn to_compile_error(&self) -> TokenStream {
        self.to_syn_error().to_compile_error()
    }

    /// Renders the primary error diagnostic message of the macro system.
    ///
    /// - Includes header (`[SPACE:ID]`, or location when `dev` feature is enabled)
    /// - Groups unspanned helps under `help`
    /// - Appends notes under `note`
    /// - References spanned helps via sub-diagnostic IDs (`[SPACE:ID:i]`)
    ///
    /// Spanned helps are emitted as separate diagnostics and only
    /// referenced here.
    ///
    /// ## Format
    ///
    /// ```text
    /// [SPACE:ID] <message>
    /// help:
    ///   • <help message>
    ///   • ...
    ///   • see sub-diagnostics: [SPACE:ID:i], ...
    ///
    /// note:
    ///   • <note message>
    ///   • ...
    /// ```
    #[track_caller]
    fn render_primary_msg(&self) -> String {
        // Start with header (ID or dev location + message)
        let mut out = self.header_msg();

        // Helper to append labeled message blocks (help / note)
        fn push_msg(buffer: &mut String, label: &str, msgs: Vec<String>) {
            buffer.push_str(label);
            buffer.push(COLON);
            buffer.push(LINE_BREAK);
            for m in msgs {
                buffer.push(LINE_SPACE);
                buffer.push(LINE_SPACE);
                buffer.push_str(BULLET);
                buffer.push(LINE_SPACE);
                buffer.push_str(&m);
                buffer.push(LINE_BREAK);
            }
        }

        // Collect unspanned helps (inline help section)
        let mut h_msg: Vec<String> = Vec::new();
        for help in &self.helps {
            if help.span.is_none() {
                h_msg.push(help.msg.clone());
            }
        }

        // Collect IDs of spanned helps
        let mut ids = Vec::new();
        for (i, h) in self.helps.iter().enumerate() {
            if h.span.is_some() {
                ids.push(format!(
                    "[{}:{}:{}]",
                    self.error.id.space, self.error.id.id, i
                ));
            }
        }

        // Only reference them if any exist
        if !ids.is_empty() {
            let ids = ids.join(", ");
            h_msg.push(format!("see sub-diagnostics: \"{}\"", ids));
        }

        // Emit help section if any messages exist
        if !h_msg.is_empty() {
            if self.helps.iter().any(|h| h.span.is_some()) {
                // (reserved for future conditional formatting)
            }

            push_msg(&mut out, "help", h_msg);
        }

        // Collect and emit notes (always unspanned)
        if !self.notes.is_empty() {
            let mut n_msg: Vec<String> = Vec::new();

            for Note { msg } in &self.notes {
                n_msg.push(msg.clone());
            }

            push_msg(&mut out, "note", n_msg);
        }

        // Return fully rendered message
        out
    }

    /// Collects diagnostics into a [`MultiDiagnostic`].
    ///
    /// Returns `None` if the input is empty.
    ///
    /// The resulting collection can be converted into a [`TokenStream`]
    /// directly via `.into()`.
    pub fn collect(diags: Vec<Diagnostic>) -> Option<MultiDiagnostic> {
        if diags.is_empty() {
            return None;
        }
        Some(MultiDiagnostic(diags))
    }
}

// ===============================================================================
// ```````````````````````````````` DERIVE IMPLS `````````````````````````````````
// ===============================================================================

impl fmt::Display for ErrorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}:{}]", self.space, self.id)
    }
}

impl fmt::Display for ErrorInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} <{}>", self.id, self.details.path)
    }
}

// ===============================================================================
// ````````````````````````````````` TRAIT IMPLS `````````````````````````````````
// ===============================================================================

impl From<proc_macro2::Span> for DiagSpan {
    fn from(span: proc_macro2::Span) -> Self {
        DiagSpan::Span(span)
    }
}

impl From<Error> for DiagSpan {
    fn from(value: Error) -> Self {
        DiagSpan::Span(value.span())
    }
}

impl From<ParseStream<'_>> for DiagSpan {
    fn from(value: ParseStream<'_>) -> Self {
        DiagSpan::Span(value.span())
    }
}

impl Into<TokenStream> for Diagnostic {
    #[track_caller]
    fn into(self) -> TokenStream {
        self.to_compile_error()
    }
}

impl Into<Error> for Diagnostic {
    #[track_caller]
    fn into(self) -> Error {
        self.to_syn_error()
    }
}

impl Into<TokenStream> for MultiDiagnostic {
    #[track_caller]
    fn into(self) -> TokenStream {
        Into::<Error>::into(self).into_compile_error()
    }
}

impl Into<Error> for MultiDiagnostic {
    #[track_caller]
    fn into(self) -> Error {
        let mut iter = self.0.into_iter();

        let first = iter.next().unwrap();

        let mut err = first.to_syn_error();

        for diag in iter {
            err.combine(diag.to_syn_error());
        }

        err
    }
}

// ===============================================================================
// `````````````````````````````` PRIVATE UTILITIES ``````````````````````````````
// ===============================================================================

/// Formats a diagnostic message.
///
/// - With `dev` feature: prefixes the message with caller location (`file:line:col`)
/// - Without `dev`: returns the message as-is
///
/// Used internally to provide additional debugging context during macro development.
#[cfg(feature = "dev")]
#[track_caller]
fn format_msg(msg: impl AsRef<str>) -> String {
    let loc = std::panic::Location::caller();
    format!(
        "[{}:{}:{}]\n{}\n",
        loc.file(),
        loc.line(),
        loc.column(),
        msg.as_ref()
    )
}

/// Formats a diagnostic message without additional context.
///
/// Returns the message unchanged (used in normal builds).
#[cfg(not(feature = "dev"))]
fn format_msg(msg: impl AsRef<str>) -> String {
    msg.as_ref().to_owned()
}

// ===============================================================================
// `````````````````````````` SYN-NODE PARSE-DIAGNOSTICS `````````````````````````
// ===============================================================================

/// Diagnostic implementations for [`syn`] structures.
///
/// This module is not used directly; it only provides trait implementations.
mod syn_diagnostics {

    use super::*;
    use syn::*;

    impl ParseDiagnostic for ItemTrait {
        const KIND: &'static str = "trait";
        const EXPECTED: &'static str = "trait declaration";
        const EXAMPLE: &[&'static str] = &[r#"
        trait MyTrait {
            /* ... */
        }"#];
    }

    impl ParseDiagnostic for ItemImpl {
        const KIND: &'static str = "impl block";
        const EXPECTED: &'static str = "well-defined impl block";
        const EXAMPLE: &[&'static str] = &[
            r#"
        impl MyType {
            /* ... */
        }"#,
            r#"
        impl MyTrait for MyType {
            /* ... */
        }"#,
        ];
    }

    impl ParseDiagnostic for TraitItemType {
        const KIND: &'static str = "associated type";
        const EXPECTED: &'static str = "trait's type declaration";
        const EXAMPLE: &[&'static str] = &[r#"
        trait MyTrait {
            type MyType;
        }"#];
    }

    impl ParseDiagnostic for ImplItemType {
        const KIND: &'static str = "associated type in an impl block";
        const EXPECTED: &'static str = "impl block's type definition";
        const EXAMPLE: &[&'static str] = &[r#"
        impl MyType {
            type MyType = ...;
        }"#];
    }

    impl ParseDiagnostic for Ident {
        const KIND: &'static str = "identifier";
        const EXPECTED: &'static str = "valid Rust identifier";
        const EXAMPLE: &[&'static str] = &["MyType", "my_function", "VALUE", "counter_0"];
    }

    impl ParseDiagnostic for ItemMacro {
        const KIND: &'static str = "macro_rules! definition";
        const EXPECTED: &'static str = "valid macro_rules! definition";
        const EXAMPLE: &[&'static str] = &[r#"
        macro_rules! my_macro {
            ($value:expr) => {
                /* ... */
            };
        }"#];
    }

    impl ParseDiagnostic for ItemFn {
        const KIND: &'static str = "function";
        const EXPECTED: &'static str = "valid Rust function declaration";
        const EXAMPLE: &[&'static str] = &[r#"
            fn my_function() {
                /* ... */
            }"#];
    }
}
