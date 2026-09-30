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
// ````````````````````````````` PROC-SUITE ERRORS ```````````````````````````````
// ===============================================================================

//! Provides shared crate-level utilities for diagnostics, error definitions,
//! proc-macro metadata, and common parsing and validation support.

// ===============================================================================
// ``````````````````````````````````` IMPORTS ```````````````````````````````````
// ===============================================================================

// --- Proc-suite ---
use proc_suite::{
    GitHost, GitHostInfo, bug_diagnostics, diagnostics, error_spaces, errors::ErrorMaintainers,
};

// --- Proc-Macro utils ---
use proc_macro2::{Span, TokenStream};
use syn::Ident;

// ===============================================================================
// `````````````````````````````````` CONSTANTS ``````````````````````````````````
// ===============================================================================

/// Remote Git repository metadata for this proc-macro crate.
///
/// Used by the diagnostic system to generate issue-reporting links for
/// internal macro errors and bug-class diagnostics.
///
/// This allows compiler diagnostics to direct users to the correct
/// repository and issue tracker when reporting proc-macro failures.
pub(crate) const GIT_HOST: GitHost = GitHost::Github(GitHostInfo {
    owner: "auguth",
    repo: "instance",
    bug_label: Some("bug"),
    feat_label: Some("feat"),
});

/// Maintainer contact information for this proc-macro crate.
///
/// Attached to structured diagnostics so users can identify where and how
/// to report issues when macro expansion fails due to internal errors.
pub(crate) const MAINTAINERS: ErrorMaintainers = ErrorMaintainers {
    mailto: None,
    git_host: Some(GIT_HOST),
};

/// Allowed primitive integer types for trait const-generic instance counters.
///
/// In this crate, "counters" refer to the const generics of a trait that
/// represent instance-selection positions.
///
/// Example:
///
/// ```ignore
/// trait Example<const A: usize, const B: usize> {}
/// ```
///
/// Here, `A` and `B` are instance counters.
///
/// Only unsigned integer primitives are supported because instance
/// selection relies on deterministic positional indexing and positive,
/// non-negative counter semantics during macro expansion.
pub(crate) const VALID_COUNTER_TYPES: &str = "`u8`, `u16`, `u32`";

// ===============================================================================
// ````````````````````````````````` ERROR SPACES ````````````````````````````````
// ===============================================================================

error_spaces! {
    space: "INSTANCE",
    maintain: MAINTAINERS,

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // ``````````````````````````````` PARSING ERRORS ````````````````````````````````
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

    pub(crate) enum ParseError {
        range: 0..=100,
        variants: {
            ItemTrait,
            IdentsOrInts,
            ItemImpl,
            ItemFn,
            ItemMacro,
            IntList,
            BStringList,
            BoundsList,
            JustTraitOrImpl,
            JustFnOrType,
            AvoidArguments,
            DelimBracket,
            TrailingTokens,
            DirectAccessExpected,
            SynParseInconsistent,
        }
    }

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // ```````````````````````````` MISCELLENEOUS ERRORS `````````````````````````````
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

    pub(crate) enum MiscSpace {
        range: 101..=200,
        variants: {
            DuplicateGenericParameter,
            MergedGenericParameterMissing,
        }

    }
}

// ===============================================================================
// ````````````````````````````````` DIAGNOSTICS `````````````````````````````````
// ===============================================================================

diagnostics!(
    space: ParseError,
    pub(crate) enum ProcParseErr {
        IdentsOrInts {
            fields: {
                args: TokenStream,
            },
            msg: "expected either identifier list or integer list",
            tags: [InvalidInput],
            span: { tokens: args },
            help: [
                "each identifier must correspond to a const generic of the trait",
                "each index must be of valid const generic of index declared by the trait",
                {format!("K")},
            ],
            note: [
                format!("allowed const generic types are: {}.", VALID_COUNTER_TYPES),
                "ensure these white-listed const-generics are all of same type",
            ]
        },
        JustTraitOrImpl {
            fields: {
                tokens: TokenStream,
            },
            msg: "expected either a trait or an impl block",
            tags: [InvalidInput],
            span: { tokens: tokens },
            help: [
                {
                    span: Span::call_site(),
                    msg: format!("do not provide arguments to this attribute, neglect this info if followed")
                },
            ]
        },
        JustFnOrType {
            fields: {
                tokens: TokenStream,
            },
            msg: "expected either a function or a type alias item",
            tags: [InvalidInput],
            span: { tokens: tokens },
        },
        AvoidArguments {
            fields: {
                args: TokenStream,
            },
            msg: "remove these arguments to the attribute",
            tags: [InvalidInput],
            span: { tokens: args },
            note: [
                "this attribute requires no arguments"
            ]
        },
        TrailingTokens {
            fields: {
                span: Span,
            },
            msg: "unexpected trailing tokens",
            tags: [InvalidInput],
            span: { span: span },
            help: [
                "remove these extra unnessary tokens",
            ]
        },
        DirectAccessExpected {
            fields: {
                span: Span,
            },
            msg: "direct access expects a leaf-access attribute and the instance item accessor expression",
            tags: [InvalidInput],
            span: { span: span },
            help: [
                "Expected: `#[access(leaf(index(..), idents(..)))] {expr}`",
            ],
            note: [
                "the expression must be accessing `<Type as InstanceTrait<...>>::Item`",
                "where Type can be a concrete or an associated type",
            ]
        },

    }
);

diagnostics!(
    space: MiscSpace,

    pub(super) enum MiscErrors {

        DuplicateGenericParameter {
            fields: {
                ident: Ident,
            },
            msg: "this generic parameter ident is already declared in this scope elsewhere",
            tags: [Unsupported],
            span: { tokens: ident },
            note: [
                "rename the generic parameter to avoid the duplicate",
            ]
        },
    }
);

// ===============================================================================
// ```````````````````````````````````` BUGS `````````````````````````````````````
// ===============================================================================

pub(super) const MISC_BUG: &'static str =
    "instance proc-macros misc phase didn't follow invariants";

pub(super) const PARSE_BUG: &'static str =
    "instance proc-macros parsing phase didn't follow invariants";

bug_diagnostics! {
    space: MiscSpace,
    bug: MISC_BUG.to_string(),

    pub(super) enum MiscBugs {
        MergedGenericParameterMissing => "a merged generic parameter missing",
    }
}

bug_diagnostics! {
    space: ParseError,
    bug: PARSE_BUG.to_string(),

    pub(super) enum ParseBug {
        SynParseInconsistent => "syn-crate's parsing inconsistency detected",
    }
}
