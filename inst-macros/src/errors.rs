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
use proc_macro2::Span;
use proc_suite::{
    GitHost, GitHostInfo, IdentList, bug_diagnostics, diagnostics, error_spaces,
    errors::ErrorMaintainers,
};
use quote::ToTokens;

// --- Proc-Macro utils ---
use syn::{
    Attribute, Expr, ExprPath, Generics, Ident, Item, ItemMod, Lifetime, 
    MetaNameValue, ParenthesizedGenericArguments, Path, PathSegment, 
    TraitBound, Type, TypeParamBound, WhereClause, punctuated::Punctuated, 
    token::{DotDot, PathSep, Plus, Underscore},
};

use crate::args::{DynamicIdent, InstanceIdent, InstanceIdents, InstanceModel};

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
    repo: "inst",
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

pub(crate) const PARSE_BUG: &'static str =
    "inst proc-macros parsing phase didn't follow invariants";

pub(crate) const TRAIT_BUG: &'static str = "inst proc-macros trait phase didn't follow invariants";

pub(crate) const IMPL_BUG: &'static str = "inst proc-macros impl phase didn't follow invariants";

pub(crate) const MOD_BUG: &'static str = "inst proc-macros mod phase didn't follow invariants";

pub(crate) const EXPR_BUG: &'static str = "inst proc-macros expr phase didn't follow invariants";

// ===============================================================================
// ````````````````````````````````` ERROR SPACES ````````````````````````````````
// ===============================================================================

error_spaces! {
    space: "INST",
    maintain: MAINTAINERS,

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // ``````````````````````````````` PARSING ERRORS ````````````````````````````````
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

    pub(crate) enum ParseError {
        range: 0..=100,
        variants: {
            InstSpecList,
            TraitImplModConst,
            TooManyItemsToTransform,
            LifetimeShouldBeDeclaredFront,
            TrailingTokens,
            ExpectedInstanceIdent,
            SynParseInconsistent,
            Max4NodeArgs,
            RequireNodeArgs,
            RangeMustBeFullInfer,
            PureTerminatedElseRange,
            MustHaveSameParamsLen,
            FullConcreteParamsExpected,
            PlainIdentAfterIndexed,
            RangeNotInAccess,
            InferNotInAccess,
            NoCompileIdentAfterRuntime,
            DepthLenParamsRequired,
            IndexesLengthInconsistent,
            RequiresIndexed,
            RequiresDepthArgument,
            UnsupportedAccessArgs,
            UnsupportedNodeArgs,
            ExpectedCommaOrSemiColon,
            ExpectedIndexed,
            RuntimeNotInNode,
            NoCompileIdentAfterInfer,
        }
    }

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // ````````````````````````````````` TRAIT INST ``````````````````````````````````
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

    pub(crate) enum TraitSpace {
        range: 101..=300,
        variants: {
            FileContainsNoTrait,
            DuplicateIdent,
            DuplicateIndexes,
            TransformedFileInconsistent,
            DelegateMacroNotInitialized,
            DelegateMacroNotFound,
            InstanceTraitNodeMacroNotFound,
            InstanceTraitNodeMacroArgsFound,
            InstanceTraitMacroNotFound,
            InstanceTraitMacroArgsNotFound,
            InstanceTraitMacroIndexesParseFailed,
            InstanceTraitMacroIndexesInconsistent,
            InstanceCounterIndexInvalid,
            InstanceCounterConstNotFound,
            InstanceCounterGenericNotConst,
            InstanceCounterGenericInvalidIdent,
            InstanceCounterGenericInvalidType,
            NodeRequiresArgs,
            NodeRequiresArgsNotValue,
            SumNoArgs,
            SumNotAllowed,
            DoubleNode,
            InconsistencyFindingNodeAttribute,
            TraitDocsNotFound,
            DocTargetInstTraitSumAttrExists,
            DocTargetInstTraitNodeAttrExists,
            NodeOnlyInTypeAssoc,
            DuplicateInstanceTraitBound,
            ExpectedInstTraitNodeBound,
            ParenthesizedInstTraitNodeBound,
            Max4Params,
            TransformedFileDoesNotHaveDocTarget,
            TransformedFileDoesNotHaveNotDocTarget,
            NotDocTargetDoesNotHaveDocs,
            DocTargetDoesNotHaveDocs,
        }
    }

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // `````````````````````````````````` IMPL INST ``````````````````````````````````
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

    pub(crate) enum ImplSpace {
        range: 301..=400,
        variants: {
            FileContainsNoImpl,
            InherentImplExpected,
            TraitImplExpected,
            NoParenTraitImpl,
            DelegateMacroNotInitialized,
            DelegateMacroNotFound,
            InstanceImplMacroNotFound,
            InstanceImplNodeMacroNotFound,
            InstanceImplNodeMacroArgsFound,
            InstanceImplMacroArgsNotFound,
            InstanceImplMacroIndexesParseFailed,
            InstanceLastImplMacroNotFound,
            InstanceLastImplMacroArgNotFound,
            InstanceLastImplMacroIndexParseFailed,
            InstanceLastImplDelegateMacroNotFound,
            InstanceLastImplMacroIndexInconsistent,
            InstanceImplIndexesArgsLenInconsistent,
            InstanceImplIndexesArgsWrong,
            TraitImplAngleArgsExpected,
            SumAttrNotRemoved,
            NodeAttrNotRemoved,
            IdentDuplicateFound,
            TransformedFileInconsistent,
            TransformedFileShouldNotHaveDocTarget,
            TransformedFileDoesNotHaveDocTarget,
            TransformedFileDoesNotHaveNotDocTarget,
            NotDocTargetDoesNotHaveDocs,
            NodeOnlyInTypeAssoc,
            InstanceImplCounterGenericArgMissing,
            InstanceImplCounterGenericArgNotConst,
            InstanceImplCounterGenericArgNotLit,
            InstanceImplCounterGenericArgNotLitInt,
            InstanceImplCounterGenericArgInvalid,
            InstanceImplCounterIdentConstItemMissing,
            InstanceImplCounterIdentConstNotLit,
            InstanceImplCounterIdentConstNotLitBStr,
            InstanceImplCounterIdentConstInvalid,
            InstanceImplCounterIdentConstTypeInvalid,
            InstanceImplCounterIdentConstCounterAttrMissing,
            InstanceImplCounterIdentConstCounterAttrInvalid,
            InstanceImplCounterIdentConstCounterAttrArgNotParsed,
            InstanceImplCounterIdentConstCounterAttrArgInvalid,
            InstanceImplInstanceCounterTypeMissing,
            InstanceImplInstanceCounterTypeInvalid,
            DoubleNode,
            InconsistencyFindingNodeAttribute,
            NodeRequiresArgs,
            NodeRequiresArgsNotValue,
            ExpectedNodeTypeImpl,
            QualifiedButNotTrait,
            GenericFoundExpectsTraitQualifier,
            ExpectedNodeTypeImplSub,
            ExpectedNodeTypeImplPub,
            ExpectedNodeTypeImplPubAttrList,
            ExpectedNodeTypeImplSubAttrPath,
            DocFileTooManyItemsToTransform,
            DocFileContainsNoImpl,
            DocImplExpectedToContainDocAttr,
            DocFileExpectedToBeEmpty,
        }
    }

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // `````````````````````````````````` MOD INST ```````````````````````````````````
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

    pub(crate) enum ModSpace {
        range: 401..=600,
        variants: {
            FileContainsNoMod,
            DuplicateIdent,
            DelegateMacroNotInitialized,
            DelegateMacroNotFound,
            InstanceAccessMacroNotFound,
            TransformedFileInconsistent,
            ModuleDocsNotAppended,
            RequiresInstAccessImpls,
            DoubleNode,
            InconsistencyFindingNodeAttribute,
            NodeRequiresArgs,
            NodeRequiresArgsNotValue,
            ExpectedWhereClauseForSelf,
            ExpectedSelfInWhereClause,
            InconsistentTranformedImplTargetsLength,
            ItemNotDocTargetAttrNotFound,
            ItemDocTargetAttrNotFound,
            ItemDocTargetHasNoDocs,
            ItemNotDocTargetHasNoDocs,
            OtherAttrsOnTopOfNode,
        }
    }

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // `````````````````````````````````` EXPR INST ``````````````````````````````````
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

    pub(crate) enum ExprSpace {
        range: 601..=800,
        variants: {
            InstNodeTraitAssociatedType,
            InstNodeQSelfButNotTraitAssoc,
            InstTypeInstTraitAssociatedAccess,
            InstTypeQSelfButNotInstTraitAssocAccess,
            InstTypeShouldBeConcrete,
            UnsupportedExprYet,
            ExpectedPathQualifier,
            PathQualifiedNotSelfQualified,
            ExpectedPathQualifiedItem,
            PathQualifierUnsupported,
            DirectExprFailed,
            AccessExprFailed,
        }
    }

}

// ===============================================================================
// ````````````````````````````````` PARSE ERROR `````````````````````````````````
// ===============================================================================

diagnostics!(
    space: ParseError,
    pub(crate) enum ProcParseErr {
        TrailingTokens {
            fields: {
                span: Span,
            },
            msg: "unnessary trailing tokens",
            tags: [Unsupported],
            span: { span: span },
            help: [
                "remove these syntax tokens"
            ]
        },
        ExpectedInstanceIdent {
            fields: {
                span: Span,
            },
            msg: "expected an instance argument",
            tags: [Unsupported],
            span: { span: span },
            help: [
                "either direct identifier `ident`",
                "or a runtime identifier braced `{ident}`",
                "or a infer `_`",
            ]
        },
        ExpectedCommaOrSemiColon {
            fields: {
                span: Span,
            },
            msg: "expected a comma `,` or semicolon `;`",
            tags: [Unsupported],
            span: { span: span },
            note: [
                "a comma `,` continues the current instance argument",
                "a semicolon `;` terminates the current instance arguments and its right-side children if exists",
            ]
        },
        ExpectedIndexed {
            fields: {
                span: Span,
            },
            msg: "expected bracketed positive integer literal",
            tags: [Unsupported],
            span: { span: span },
            note: [
                "in module `#[inst(..)] mod _ {..}` it defines the depth",
                "in instance impl `#[inst(..)] impl InstanceTrait for T {..}` it defines the index",
                "depth should provide the maximum index it would have been defined in impls",
                "index should provide continuous index ordered where previous index (index-1) would have\
                already been utilized by another impl of the same instance trait and implementing type",
            ]
        },
        TraitImplModConst {
            fields: {
                item: Item,
            },
            msg: "`#[inst(..)]` attribute expects either of `trait` / `impl` / `mod` / `const` block",
            tags: [Unsupported],
            span: { tokens: item },
            note: [
                "each block serves different purposes",
                "`trait` block declares the inst params, or define inst bounds (assoc node)",
                "`impl` block defines the concrete inst args (with indexes) or provides concrete inst bounded types",
                "`mod` block declares scoped aliases via inherent impl for inst assoc nodes",
                "`const` block declares scoped inst trait impls without inst arg indexes",
            ]
        },
        LifetimeShouldBeDeclaredFront {
            fields: {
                lt: Lifetime,
            },
            msg: "this lifetime should be defined before before type/other params/arguments",
            tags: [InvalidInput],
            span: { tokens: lt },
        },
        Max4NodeArgs {
            fields: {
                params: InstanceIdents,
            },
            msg: "instance node arguments cannot have more than four params",
            tags: [Unsupported],
            span: { tokens: params },
            help: [
                "remove excessive arguments",
            ],
            note: [
                "an instance trait also is capped at four params"
            ]
        },
        RequireNodeArgs {
            fields: {
                params: InstanceIdents,
            },
            msg: "instance node arguments cannot be empty",
            tags: [Unsupported],
            span: { tokens: params },
            help: [
                "provide sufficient node arguments",
            ],
            note: [
                "check the given instance trait of the instance node for more information"
            ]
        },
        RangeMustBeFullInfer {
            fields: {
                dotdot: DotDot,
            },
            msg: "instead use simple parenthesized with only infer `_` arguments to denote a full range",
            tags: [Unsupported],
            span: { tokens: dotdot },
            help: [
                "use comma seperated infers `_` instead of [..] range",
            ],
            note: [
                "required to infer each instance argument via explicit infer",
                "example: for a 2 params (_, _)"
            ]
        },
        PureTerminatedElseRange {
            fields: {
                params: InstanceIdents,
            },
            msg: "comma seperated parenthesized instance arguments must have concrete idents",
            tags: [Unsupported],
            span: { tokens: params },
            help: [
                "use ranges [(<params>)..(<params>)] for non-concrete arguments",
            ],
            note: [
                "inferred or dynamic exprs are not allowed if its terminated",
            ]
        },
        MustHaveSameParamsLen {
            fields: {
                params: InstanceIdents,
                exp: usize,
                found: usize,
            },
            msg: format!("expected arguments length `{}`, but found {}", exp, found),
            tags: [Unsupported],
            span: { tokens: params },
            note: [
                "in case of terminated or ranges, all parenthesized arguments must be of same length",
            ]
        },
        FullConcreteParamsExpected {
            fields: {
                params: InstanceIdent,
            },
            msg: "expected all parenthesized arguments to be concrete identifiers, but found which is not",
            tags: [Unsupported],
            span: { tokens: params },
            help: [
                "use only concrete identifiers",
            ],
        },
        PlainIdentAfterIndexed {
            fields: {
                ident: Ident,
            },
            msg: "expected depth indexed, but found a plain ident",
            tags: [Unsupported],
            span: { tokens: ident },
            note: [
                "<idents> <indexed> is valid, but <indexed> <idents> is not allowed",
                "preliminary params before this is identified as indexed, hence"
            ],
        },
        RangeNotInAccess {
            fields: {
                model: InstanceModel,
            },
            msg: "range based `..` is only allowed in instance node associated type, not in instance access impls",
            tags: [Unsupported],
            span: { tokens: model },
            help: [
                "use parenthesized arguments only in instance access arguments"
            ],
            note: [
                "instance access args expects parenthesized arguments only `(<params>)`",
                "complex instance arguments such as range and comma seperated \
                parenthesized args are only available in instance node trait associated type arguments"
            ],
        },
        InferNotInAccess {
            fields: {
                infer: Underscore,
            },
            msg: "infer arg is only allowed in instance node associated type, not in instance access impls",
            tags: [Unsupported],
            span: { tokens: infer },
            help: [
                "instead use runtime argument to identify a dynamic argument"
            ],
            note: [
                "instance access args expects braced runtime arg for dynamic args `{ident}`",
                "the braced argument must be declared with depth in the module `#[inst(ident[i])]` \
                where i should be a integer literal"
            ],
        },
        NoCompileIdentAfterRuntime {
            fields: {
                ident: Ident,
            },
            msg: format!("expected runtime braced argument `{ident}` ?, but found a compile-time ident"),
            tags: [Unsupported],
            span: { tokens: ident },
            note: [
                "<compile> <runtime> is valid, but <runtime> <compile> is not allowed",
                "preliminary params before this is identified as runtime braced ident, hence"
            ],
        },
        NoCompileIdentAfterInfer {
            fields: {
                ident: Ident,
            },
            msg: format!("expected infered argument `_`, but found a compile-time ident"),
            tags: [Unsupported],
            span: { tokens: ident },
            note: [
                "<compile> <infer> is valid, but <infer> <compile> is not allowed",
                "preliminary params before this is identified as inferred argument, hence"
            ],
        },
        DepthLenParamsRequired {
            fields: {
                params: InstanceIdents,
                exp: usize,
                found: usize,
            },
            msg: format!("expected arguments of length {}, but found {}", exp, found),
            tags: [Unsupported],
            span: { tokens: params },
            note: [
                "for every depth param given in module's `#[inst(<params>)]`, \
                there should be an argument for access impl",
            ],
        },
        RequiresIndexed{
            fields: {
                ident: Ident,
            },
            msg: "requires depth index along with ident",
            tags: [Unsupported],
            span: { tokens: ident },
            help: [
                {format!("expected {}[i], where i should be a integer literal providing maximum depth of all available ordered instance arguments", ident)}
            ],
        },
        RequiresDepthArgument{
            fields: {
                ident: Ident,
            },
            msg: "requires this ident as an indexed argument in the depth argument list in module's `#[inst(<depth-args>)]`",
            tags: [Unsupported],
            span: { tokens: ident },
            help: [
                {format!("expected {}[i] in depth-argument list, where i should be a integer literal providing maximum depth of all available ordered instance arguments", ident)}
            ],
        },
        UnsupportedAccessArgs{
            fields: {
                model: InstanceModel,
            },
            msg: "unsupported instance access args",
            tags: [Unsupported],
            span: { tokens: model },
            help: [
                "either use all runtime arguments braced ({<ident>}, ...)",
                "or use all compile-time arguments (<ident>, ...)",
                "or use all mixed arguments where after runtime args braced, compile arguments are not supported (<ident>,.., {<ident>} ...)"
            ],
        },
        UnsupportedNodeArgs{
            fields: {
                model: InstanceModel,
            },
            msg: "an unsupported, could be valid instance node args",
            tags: [Future],
            span: { tokens: model },
        },
        RuntimeNotInNode {
            fields: {
                runtime: DynamicIdent,
            },
            msg: "braced runtime arg is only allowed in instance access impls, not in instance node associated type",
            tags: [Unsupported],
            span: { tokens: runtime },
            help: [
                "instead use infer argument `_` to identify a dynamic argument"
            ],
            note: [
                "instance node args expects infer arg for dynamic args `_` which params are declared in the instance trait",
            ],
        },
    }
);

bug_diagnostics! {
    space: ParseError,
    bug: PARSE_BUG.to_string(),

    pub(crate) enum ParseBug {
        TooManyItemsToTransform => "file contained too many (more than one) inst pre-item to transform",
        SynParseInconsistent => "syn-crate's parsing inconsistency detected",
        LifetimeShouldBeDeclaredFront => "lifetime allowed to be not declared upfront in generic params",
        IndexesLengthInconsistent => "the indexes generated for access arguments are invalid",
    }
}

// ===============================================================================
// ````````````````````````````````` TRAIT ERROR `````````````````````````````````
// ===============================================================================

diagnostics!(
    space: TraitSpace,
    pub(crate) enum TraitError {
        NodeRequiresArgs {
            fields: {
                attr: Attribute,
            },
            msg: "instance node requires arguments `<trait-path> <inst-args>`",
            tags: [Unsupported],
            span: { tokens: attr },
            note: [
                "<trait-path> is the instance trait path and must be a bound on the inst-node associated type",
                "<inst-args> defines the instance arguments associated with the trait",
                "simple: (<params>) defines one instance parameter group; parameters may be identifiers or `_` to infer a dynamic instance",
                "terminated: [(<params>), (<params>), ...] defines multiple instance parameter groups; terminated groups may contain only identifiers",
                "range: [(<params>)..(<params>)] defines an instance range between two parameter groups; range parameters may be identifiers or `_` to infer dynamic instances",
                "the instance arguments must provide a parameter for every instance declared by the <trait-path>",
                "an inferred `_` parameter may only be followed by another inferred `_` parameter",
                "examples for an instance trait with instance parameters `Module, Function`: \
                    `module::InstTrait(Crypto, _)`, \
                    `module::InstTrait[(Crypto, Sha256), (Crypto, Sha512)]`, \
                    `module::InstTrait[(Crypto, _)..(_, _)]`",
            ]
        },
        NodeRequiresArgsNotValue {
            fields: {
                meta: MetaNameValue,
            },
            msg: "instance node requires arguments `<trait-path> <inst-args>`, but found key-value",
            tags: [Unsupported],
            span: { tokens: meta },
            note: [
                "<trait-path> is the instance trait path and must be a bound on the inst-node associated type",
                "<inst-args> defines the instance arguments associated with the trait",
                "simple: (<params>) defines one instance parameter group; parameters may be identifiers or `_` to infer a dynamic instance",
                "terminated: [(<params>), (<params>), ...] defines multiple instance parameter groups; terminated groups may contain only identifiers",
                "range: [(<params>)..(<params>)] defines an instance range between two parameter groups; range parameters may be identifiers or `_` to infer dynamic instances",
                "the instance arguments must provide a parameter for every instance declared by the <trait-path>",
                "an inferred `_` parameter may only be followed by another inferred `_` parameter",
                "examples for an instance trait with instance parameters `Module, Function`: \
                    `module::InstTrait(Crypto, _)`, \
                    `module::InstTrait[(Crypto, Sha256), (Crypto, Sha512)]`, \
                    `module::InstTrait[(Crypto, _)..(_, _)]`",
            ]
        },
        Max4Params {
            fields: {
                idents: IdentList,
            },
            msg: "instance trait params are capped at maximum of four",
            tags: [Unsupported],
            span: { tokens: idents },
            help: [
                "remove excessive params",
            ],
        },
        SumNoArgs {
            fields: {
                attr: Attribute,
            },
            msg: "instance sum type attribute does not need arguments",
            tags: [Unsupported],
            span: { tokens: attr },
            help: [
                "remove arguments to avoid ambiguous contexts",
            ],
        },
        SumNotAllowed {
            fields: {
                attr: Attribute,
            },
            msg: "instance sum type attribute only allowed for instance traits",
            tags: [Unsupported],
            span: { tokens: attr },
            help: [
                "declare instance parameters as comma seperated identifiers on the trait attribute `#[inst(<params>)]` to enable inst-sum associated types",
            ],
            note: [
                "without instance parameter declarations, a trait's #[inst] is only used for inst-node associated types declared with #[node(..)] annotation",
            ],
        },
        DoubleNode {
            fields: {
                attr: Attribute,
            },
            msg: "only one instance node attribute annotation is allowed",
            tags: [Unsupported],
            span: { tokens: attr },
            help: [
                "found multiple node annotations, keep only one",
            ],
        },
        NodeOnlyInTypeAssoc {
            fields: {
                attr: Attribute,
            },
            msg: "instance node attribute annotation only given via a associated type, found else",
            tags: [Unsupported],
            span: { tokens: attr },
            help: [
                "use instance node declaration only in associated types",
            ],
        },
        DuplicateInstanceTraitBound {
            fields: {
                bound: TraitBound,
            },
            msg: "duplicate instance trait bound is found as instance node bound",
            tags: [Unsupported],
            span: { tokens: bound },
            help: [
                "remove this trait bound",
            ],
        },
        ExpectedInstTraitNodeBound {
            fields: {
                bounds: Punctuated<TypeParamBound, Plus>,
                exp: Punctuated<Ident, PathSep>,
            },
            msg: format!("expected instance trait bound `{}` for instance node is not found in the given bounds", exp.to_token_stream().to_string()),
            tags: [Unsupported],
            span: { tokens: bounds },
            help: [
                "provide expected trait bound as type param bound",
            ],
        },
        ParenthesizedInstTraitNodeBound {
            fields: {
                paren: ParenthesizedGenericArguments,
            },
            msg: "given instance trait bound for instance node has parenthesized generic arguments",
            tags: [Unsupported],
            span: { tokens: paren },
            note: [
                "instance traits can only have angle bracketed generic arguments",
            ],
        },
    }
);

bug_diagnostics! {
    space: TraitSpace,
    bug: TRAIT_BUG.to_string(),

    pub(crate) enum TraitBug {
        FileContainsNoTrait =>
            "file contained no trait to proceed for inst trait phases",
        TransformedFileInconsistent =>
            "transformed file contains inconsistent trait items",
        DelegateMacroNotInitialized =>
            "delegate macro is not initialized in this proc-macro crate via pipeline declaration",
        DelegateMacroNotFound =>
            "delegate macro is not found for the proc-delegated trait to instances-macro crate",
        InstanceTraitMacroNotFound =>
            "instances-macro delegated proc-macro declaration for trait is not found",
        InstanceTraitNodeMacroNotFound =>
            "instances-macro delegated inst-node proc-macro declaration on the trait header is not found",
        InstanceTraitNodeMacroArgsFound =>
            "instances-macro delegated inst-node proc-macro declaration on the trait header has args",
        InstanceTraitMacroArgsNotFound =>
            "instances-macro trait declaration is missing its instance counter arguments",
        InstanceTraitMacroIndexesParseFailed =>
            "instances-macro trait declaration contains invalid instance counter indexes",
        InstanceTraitMacroIndexesInconsistent =>
            "instance counter indexes are inconsistent with the instance trait parameters",
        InstanceCounterIndexInvalid =>
            "instance counter index does not match its expected generic parameter position",
        InstanceCounterConstNotFound =>
            "instance counter generic parameter was not found at the expected position",
        InstanceCounterGenericNotConst =>
            "instance counter generic parameter is not a const parameter",
        InstanceCounterGenericInvalidIdent =>
            "instance counter const parameter has an unexpected identifier",
        InstanceCounterGenericInvalidType =>
            "instance counter const parameter must have type `u8`",
        NodeRequiresArgs =>
            "inst node requires arguments",
        NodeRequiresArgsNotValue =>
            "inst node arguments must not resolve to a key value",
        SumNoArgs =>
            "sum does not require argument",
        InconsistencyFindingNodeAttribute =>
            "inconsistency finding instance node attribute annotation in trait",
        TraitDocsNotFound =>
            "inst trait disclaimer docs are not appended",
        DocTargetInstTraitSumAttrExists =>
            "a doc-target-inst trait contains a sum type annotation",
        DocTargetInstTraitNodeAttrExists =>
            "a doc-target-inst trait contains a node type annotation",
        NodeOnlyInTypeAssoc =>
            "a non-assoc-type contains inst-node annotation",
        TransformedFileDoesNotHaveDocTarget =>
            "transformed file does not contain the required doc trait target",
        TransformedFileDoesNotHaveNotDocTarget =>
            "transformed file does not contain the required not-doc trait target",
        NotDocTargetDoesNotHaveDocs =>
            "not-doc trait target does not contain the required documentation disclaimer",
        DocTargetDoesNotHaveDocs =>
            "doc trait target does not contain the required instance documentation",
    }
}

// ===============================================================================
// `````````````````````````````````` IMPL ERROR `````````````````````````````````
// ===============================================================================

diagnostics!(
    space: ImplSpace,
    pub(crate) enum ImplError {
        NodeRequiresArgs {
            fields: {
                attr: Attribute,
            },
            msg: "instance node requires arguments `<inst-args>`",
            tags: [Unsupported],
            span: { tokens: attr },
            note: [
                "`<inst-args>` defines the instance arguments associated with the instance trait bound on the declaration",
                "the trait and impl node arguments must match; visit the trait declaration to determine the required arguments",
                "examples for an instance impl node with instance parameters `Module, Function`: \
                    `(Crypto, _)`, \
                    `[(Crypto, Sha256), (Crypto, Sha512)]`, \
                    `[(Crypto, _)..(_, _)]`",
            ]
        },
        NodeRequiresArgsNotValue {
            fields: {
                meta: MetaNameValue,
            },
            msg: "instance node requires arguments `<inst-args>`, but found key-value",
            tags: [Unsupported],
            span: { tokens: meta },
            note: [
                "`<inst-args>` defines the instance arguments associated with the instance trait bound on the declaration",
                "the trait and impl node arguments must match; visit the trait declaration to determine the required arguments",
                "examples for an instance impl node with instance parameters `Module, Function`: \
                    `(Crypto, _)`, \
                    `[(Crypto, Sha256), (Crypto, Sha512)]`, \
                    `[(Crypto, _)..(_, _)]`",
            ]
        },
        ExpectedNodeTypeImpl {
            fields: {
                ty: Type,
            },
            msg: "expected a concrete or an associated type",
            tags: [Unsupported],
            span: { tokens: ty },
            help: [
                "either give a associated type `<T as Trait>::Assoc`",
                "or a concrete type `T` or `module::T`",
            ],
            note: [
                "if associated type must be trait qualified",
                "if concrete type it must be a well-declared struct",
            ]
        },
        QualifiedButNotTrait {
            fields: {
                qself: Type,
            },
            msg: "qualifier found but not trait qualified, `as` not found",
            tags: [Unsupported],
            span: { tokens: qself },
            help: [
                "give a direct associated type `<T as Trait>::Assoc` which is trait qualified",
                "or use a concrete struct path `T` or `module::T`"
            ],
        },
        GenericFoundExpectsTraitQualifier {
            fields: {
                seg: PathSegment,
            },
            msg: "identified as generic param but not trait qualified, `as` not found",
            tags: [Unsupported],
            span: { tokens: seg },
            help: [
                "give a direct associated type `<GenericParam as Trait>::Assoc` which is trait qualified",
                "or use a concrete struct path `T` or `module::T`"
            ],
        },
        InherentImplExpected {
            fields: {
                trait_: Path,
            },
            msg: "expected an inherent impl block, found a trait impl",
            tags: [Unsupported],
            span: { tokens: trait_ },
        },
        TraitImplExpected {
            fields: {
                self_ty: Type,
            },
            msg: "expected a trait impl block, found an inherent impl",
            tags: [Unsupported],
            span: { tokens: self_ty },
        },
        NoParenTraitImpl {
            fields: {
                seg: PathSegment,
            },
            msg: "expected a angle-bracketed trait impl block, found paren-trait impl",
            tags: [Unsupported],
            span: { tokens: seg },
        },
        NodeOnlyInTypeAssoc {
            fields: {
                attr: Attribute,
            },
            msg: "instance node attribute annotation only given via a associated type, found else",
            tags: [Unsupported],
            span: { tokens: attr },
            help: [
                "use instance node definition only in associated types",
            ],
        },
        DoubleNode {
            fields: {
                attr: Attribute,
            },
            msg: "only one instance node attribute annotation is allowed",
            tags: [Unsupported],
            span: { tokens: attr },
            help: [
                "found multiple node annotations, keep only one",
            ],
        },
    }
);

bug_diagnostics! {
    space: ImplSpace,
    bug: IMPL_BUG.to_string(),

    pub(crate) enum ImplBug {
        FileContainsNoImpl => "file contained no impl to proceed for inst impl phases",
        DelegateMacroNotInitialized =>
            "delegate macro is not initialized in this proc-macro crate via pipeline declaration",
        DelegateMacroNotFound =>
            "delegate macro is not found for the proc-delegated impl to instances-macro crate",
        InstanceImplMacroNotFound =>
            "instances-macro delegated proc-macro declaration for impl is not found",
        InstanceImplNodeMacroNotFound =>
            "instances-macro delegated inst-node proc-macro declaration on the impl header is not found",
        InstanceImplNodeMacroArgsFound =>
            "instances-macro delegated inst-node proc-macro declaration on the impl header has args",
        InstanceImplMacroArgsNotFound =>
            "instances-macro impl declaration is missing its instance counter arguments",
        InstanceImplMacroIndexesParseFailed =>
            "instances-macro impl declaration contains invalid instance counter indexes",
        InstanceLastImplMacroNotFound =>
            "instances-macro delegated proc-macro declaration for impl is not found",
        InstanceLastImplMacroArgNotFound =>
            "instances-macro last-instance impl declaration is missing its instance counter",
        InstanceLastImplMacroIndexParseFailed =>
            "instances-macro last-instance impl declaration contains invalid instance counter",
        InstanceLastImplDelegateMacroNotFound =>
            "delegate macro is not found for the proc-delegated last instance impl to instances-macro crate",
        InstanceLastImplMacroIndexInconsistent =>
            "instances-macro last-instance impl declaration contains inconsistent instance counter",
        InstanceImplIndexesArgsLenInconsistent =>
            "instances-macro impl declaration contains invalid length instance counter indexes",
        InstanceImplIndexesArgsWrong =>
            "instances-macro impl declaration contains unexpected instance counter indexes",
        TraitImplExpected =>
            "expected a trait impl for instance impl macro, found inherent",
        TraitImplAngleArgsExpected =>
            "expected a trait impl with angle generic arguments for instance impl macro",
        NodeAttrNotRemoved =>
            "node attribute annotation on type-assoc not removed",
        TransformedFileInconsistent =>
            "transformed file contains an inconsistent number of impl items",
        TransformedFileShouldNotHaveDocTarget =>
            "transformed file contains an unexpected doc impl target",
        TransformedFileDoesNotHaveDocTarget =>
            "transformed file does not contain the required doc impl target",
        TransformedFileDoesNotHaveNotDocTarget =>
            "transformed file does not contain the required not-doc impl target",
        NotDocTargetDoesNotHaveDocs =>
            "not-doc impl target does not contain the required documentation disclaimer",
        InstanceImplCounterGenericArgMissing =>
            "instances-macro impl declaration is missing an instance counter generic argument",
        InstanceImplCounterGenericArgNotConst =>
            "instances-macro impl declaration contains a non-const instance counter generic argument",
        InstanceImplCounterGenericArgNotLit =>
            "instances-macro impl declaration contains a non-literal instance counter generic argument",
        InstanceImplCounterGenericArgNotLitInt =>
            "instances-macro impl declaration contains a non-integer-literal instance counter generic argument",
        InstanceImplCounterGenericArgInvalid =>
            "instances-macro impl declaration contains an invalid instance counter generic argument",
        InstanceImplCounterIdentConstItemMissing =>
            "instances-macro impl declaration is missing the instance identifier constant item",
        InstanceImplCounterIdentConstNotLit =>
            "instances-macro impl declaration contains a non-literal instance identifier constant",
        InstanceImplCounterIdentConstNotLitBStr =>
            "instances-macro impl declaration contains a non-byte-string instance identifier constant",
        InstanceImplCounterIdentConstInvalid =>
            "instances-macro impl declaration contains an invalid instance identifier constant",
        InstanceImplCounterIdentConstTypeInvalid =>
            "instances-macro impl declaration contains an invalid instance identifier constant type",
        InstanceImplCounterIdentConstCounterAttrMissing =>
            "instances-macro impl declaration is missing the instance identifier counter attribute",
        InstanceImplCounterIdentConstCounterAttrInvalid =>
            "instances-macro impl declaration contains an invalid instance identifier counter attribute",
        InstanceImplCounterIdentConstCounterAttrArgNotParsed =>
            "instances-macro impl declaration contains an unparseable instance identifier counter attribute argument",
        InstanceImplCounterIdentConstCounterAttrArgInvalid =>
            "instances-macro impl declaration contains an invalid instance identifier counter attribute argument",
        InstanceImplInstanceCounterTypeMissing =>
            "instances-macro impl declaration is missing the `InstanceCounter` associated type",
        InstanceImplInstanceCounterTypeInvalid =>
            "instances-macro impl declaration contains an invalid `InstanceCounter` associated type",
        InconsistencyFindingNodeAttribute =>
            "inconsistency finding instance node attribute annotation in impl",
        ExpectedNodeTypeImpl =>
            "expected inst-node impl type to be a type-path, found else",
        ExpectedNodeTypeImplSub =>
            "expected subscriber inst-node impl type to be a trait qualified associated type path, found else",
        ExpectedNodeTypeImplPub =>
            "expected publisher inst-node impl type to be concrete struct type path, found else",
        ExpectedNodeTypeImplPubAttrList =>
            "expected publisher inst-node impl type attribute to have args",
        ExpectedNodeTypeImplSubAttrPath =>
            "expected subscriber inst-node impl type attribute to not have args",
        DocFileTooManyItemsToTransform =>
            "a doc impl file contains too many items to transform",
        DocFileContainsNoImpl =>
            "a doc impl file contains no impl item",
        DocImplExpectedToContainDocAttr =>
            "a doc impl is missing the required not-inst (doc-only) feature attribute",
        DocFileExpectedToBeEmpty =>
            "a doc impl file is expected to be empty",

    }
}

// ===============================================================================
// `````````````````````````````````` MOD ERROR ``````````````````````````````````
// ===============================================================================

diagnostics!(
    space: ModSpace,
    pub(crate) enum ModError {
        RequiresInstAccessImpls {
            fields: {
                module: ItemMod,
            },
            msg: "instance module requires instance access impls (inherent impls for instance-bounded associated types), found none",
            tags: [Unsupported],
            span: { tokens: module },
            note: [
                "this module must contain at least one inst-node's inherent impl",
                "each impl must include the attribute annotation `#[node(<inst-args>)]`",
                "<inst-args> defines the instance arguments corresponding to the declared instance arguments associated with the inherent associated type's instance node bound declaration",
                "(<params>) parameters may be identifiers or runtime braced `{ident}` for a dynamic instance",
                "the instance arguments must provide a parameter for every instance declared by the trait-side instance node declaration",
                "a runtime dynamic parameter `{<ident>}` may only be followed by another runtime parameter",
                "examples for an instance access module with instance parameters `Module[l], Function[m]`: \
                    `(Crypto, {Function})`, \
                    `({Module}, {Function})`",
            ]
        },
        NodeRequiresArgs {
            fields: {
                attr: Attribute,
            },
            msg: "instance node requires arguments `<inst-args>`",
            tags: [Unsupported],
            span: { tokens: attr },
            note: [
                "<inst-args> defines the instance arguments corresponding to the declared instance arguments associated with the inherent associated type's instance node bound declaration",
                "(<params>) parameters may be identifiers or runtime braced `{ident}` for a dynamic instance",
                "the instance arguments must provide a parameter for every instance declared by the trait-side instance node declaration",
                "a runtime dynamic parameter `{<ident>}` may only be followed by another runtime parameter",
                "examples for an instance access module with instance parameters `Module[l], Function[m]`: \
                    `(Crypto, {Function})`, \
                    `({Module}, {Function})`",
            ]
        },
        NodeRequiresArgsNotValue {
            fields: {
                meta: MetaNameValue,
            },
            msg: "instance node requires arguments `<inst-args>`, but found key-value",
            tags: [Unsupported],
            span: { tokens: meta },
            note: [
                "<inst-args> defines the instance arguments corresponding to the declared instance arguments associated with the inherent associated type's instance node bound declaration",
                "(<params>) parameters may be identifiers or runtime braced `{ident}` for a dynamic instance",
                "the instance arguments must provide a parameter for every instance declared by the trait-side instance node declaration",
                "a runtime dynamic parameter `{<ident>}` may only be followed by another runtime parameter",
                "examples for an instance access module with instance parameters `Module[l], Function[m]`: \
                    `(Crypto, {Function})`, \
                    `({Module}, {Function})`",
            ]
        },
        DoubleNode {
            fields: {
                attr: Attribute,
            },
            msg: "only one instance node attribute annotation is allowed",
            tags: [Unsupported],
            span: { tokens: attr },
            help: [
                "found multiple node annotations, keep only one",
            ],
        },
        ExpectedWhereClauseForSelf {
            fields: {
                generics: Generics,
            },
            msg: "every instance access impl must have post predicate `Self: <inst-trait>`",
            tags: [Unsupported],
            span: { tokens: generics },
            help: [
                "provide a `where` clause for `Self` with the implementing associated type's instance bound",
            ],
        },
        ExpectedSelfInWhereClause  {
            fields: {
                where_: WhereClause,
                exp: Punctuated<Ident, PathSep>,
            },
            msg: "every instance access impl must have post predicate `Self: <inst-trait>`",
            tags: [Unsupported],
            span: { tokens: where_ },
            help: [
                "provide a `where` clause for `Self` with the implementing associated type's instance bound",
                {format!("expected `Self: {}`", exp.to_token_stream().to_string())},
            ],
        },
        OtherAttrsOnTopOfNode  {
            fields: {
                attr: Attribute,
            },
            msg: "other attributes should be top of inst node attribute",
            tags: [Unsupported],
            span: { tokens: attr },
            help: [
                "instance proc macro phase removes the impl header, \
                so other attribute transformations should occur earlier",
            ],
        },
    }
);

bug_diagnostics! {
    space: ModSpace,
    bug: MOD_BUG.to_string(),

    pub(crate) enum ModBug {
        FileContainsNoMod => "file contained no mod item to proceed for inst inherent impls phases",
        DelegateMacroNotInitialized =>
            "delegate macro is not initialized in this proc-macro crate via pipeline declaration",
        DelegateMacroNotFound =>
            "delegate macro is not found for the proc-delegated impl to instances-macro crate",
        InstanceAccessMacroNotFound =>
            "instances-macro delegated proc-macro declaration for access impl is not found",
        TransformedFileInconsistent =>
            "transformed file contains an inconsistent number of mod items",
        ModuleDocsNotAppended =>
            "transformed instance access impls holding module instance docs not found",
        InconsistencyFindingNodeAttribute =>
            "inconsistency finding instance node attribute annotation in impls in instance mod",
        RequiresInstAccessImpls =>
            "instance module requires inst access impls, found none",
        InconsistentTranformedImplTargetsLength =>
            "transformed instance access impl targets have inconsistent lengths",
        ItemNotDocTargetAttrNotFound =>
            "instance access impl item is missing the required `#[cfg(not(doc))]` attribute",
        ItemDocTargetAttrNotFound =>
            "instance access impl documentation item is missing the required `#[cfg(doc)]` attribute",
        ItemDocTargetHasNoDocs =>
            "instance access impl documentation item contains no documentation",
        ItemNotDocTargetHasNoDocs =>
            "instance access not-doc target impl item contains no documentation",
    }
}

// ===============================================================================
// `````````````````````````````````` EXPR ERROR `````````````````````````````````
// ===============================================================================

diagnostics!(
    space: ExprSpace,
    pub(crate) enum ExprError {
        InstNodeTraitAssociatedType {
            fields: {
                expr: ExprPath,
            },
            msg: "expected a inst-node associated type",
            tags: [Unsupported],
            span: { tokens: expr },
            help: [
                "example `<T as Trait>::Node`"
            ],
            note: [
                "instance node are associated types bounded using instance traits",
                "expects a trait qualified associated type i.e., instance node"
            ]
        },
        InstNodeQSelfButNotTraitAssoc {
            fields: {
                ty: Type,
            },
            msg: "identfied a qualified type, but not a trait qualified inst-node associated type",
            tags: [Unsupported],
            span: { tokens: ty },
            help: [
                "found `T` in `<T as Trait>::Node` but `as Trait` missing ",
            ],
            note: [
                "instance node are associated types bounded using instance traits",
                "expects a trait qualified associated type i.e., instance node"
            ]
        },
        UnsupportedExprYet {
            fields: {
                expr: Expr,
            },
            msg: "unsupported expression kind",
            tags: [Future],
            span: { tokens: expr },
            note: [
                "supports nested expression kinds of: path, call, reference, return, try?, and tuple only",
                "other complex expressions maybe not yet supported"
            ]
        },
        ExpectedPathQualifier {
            fields: {
                seg: PathSegment,
            },
            msg: "expected a path qualifier, found a argumented segment",
            tags: [Unsupported],
            span: { tokens: seg },
            help: [
                "only allowed to use simple path qualifiers only"
            ]
        },
        PathQualifiedNotSelfQualified {
            fields: {
                qself: Type,
            },
            msg: "expected a path qualifier, found a self qualifier",
            tags: [Unsupported],
            span: { tokens: qself },
            help: [
                "only allowed to use simple path qualifiers only",
                "`<T as Trait>::..` syntax or any Type qualified access is not allowed, use direct objects"
            ]
        },
        ExpectedPathQualifiedItem {
            fields: {
                path: Path,
            },
            msg: "expected a path qualified item",
            tags: [Unsupported],
            span: { tokens: path },
            help: [
                "only allowed to use simple path qualifiers only",
                "`<T as Trait>::..` syntax is not allowed, use direct objects"
            ]
        },
        PathQualifierUnsupported {
            fields: {
                seg: PathSegment,
            },
            msg: "import this path using seperate import statements outside",
            tags: [Unsupported],
            span: { tokens: seg },
            help: [
                "direct path qualification is not allowed here",
                "only the direct object can be referenced without additional path qualifiers"
            ]
        },

        InstTypeInstTraitAssociatedAccess {
            fields: {
                expr: ExprPath,
            },
            msg: "expected fully qualified access of a instance implementing type's instance trait item access",
            tags: [Unsupported],
            span: { tokens: expr },
            help: [
                "example `<InstType as InstTrait>::Item` or `<InstType as InstTrait>::func(..)`"
            ],
            note: [
                "to directly access an instance trait implementation from the concrete instance type itself",
            ]
        },
        InstTypeQSelfButNotInstTraitAssocAccess {
            fields: {
                ty: Type,
            },
            msg: "identfied a qualified type, but not a instance trait qualified instance type direct access",
            tags: [Unsupported],
            span: { tokens: ty },
            help: [
                "found `InstType` in `<InstType as InstTrait>::Item` but `as InstTrait` missing ",
            ],
            note: [
                "to directly access an instance trait implementation from the concrete instance type itself",
            ]
        },
        InstTypeShouldBeConcrete {
            fields: {
                ty: Type,
            },
            msg: "instance trait implemented type should be a concrete struct",
            tags: [Unsupported],
            span: { tokens: ty },
            help: [
                "found a qualified type, but expected a type-path",
            ],
            note: [
                "to directly access an instance trait implementation from the concrete instance type itself",
            ]
        },

    }
);

bug_diagnostics! {
    space: ExprSpace,
    bug: EXPR_BUG.to_string(),

    pub(crate) enum ExprBug {
        DirectExprFailed => "instance direct access expr validation failed",
        AccessExprFailed => "instance node access expr validation failed",
    }
}
