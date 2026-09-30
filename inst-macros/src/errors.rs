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
            TupleRequiresArgs,
            TupleRequiresArgsNotValue,
            EitherTupleOrNode,
            WhereClauseNotSupportInTuple,
            SumNoArgs,
            SumNotAllowed,
            DoubleNode,
            DoubleTuple,
            InconsistencyFindingNodeAttribute,
            InconsistencyFindingTupleAttribute,
            TraitDocsNotFound,
            DocTargetInstTraitSumAttrExists,
            DocTargetInstTraitNodeAttrExists,
            DocTargetInstTraitTupleAttrExists,
            NodeOnlyInTypeAssoc,
            TupleOnlyInTypeAssoc,
            DuplicateInstanceTraitBound,
            ExpectedInstTraitNodeBound,
            ParenthesizedInstTraitNodeBound,
            Max4Params,
            InstTraitOverflowReference,
            TransformedFileDoesNotHaveDocTarget,
            TransformedFileDoesNotHaveNotDocTarget,
            NotDocTargetDoesNotHaveDocs,
            DocTargetDoesNotHaveDocs,
            InstEitherTraitOrNodeOrTuple,
            TraitTupleTargetItemNonCamelCaseAttrMissing,
            TraitTupleTargetItemTypeAliasBoundsAttrMissing,
            TraitTupleTargetItemUnusedAttrMissing,
            TraitTupleExpectedAddonNotImplNorTypeAlias,
            TraitTupleExpectedAddonAccessImplNotFeatureGated,
            TraitTupleExpectedAddonAccessImplArgsNotFound,
            TraitTupleAddonAccessImplSelfNotTypePath,
            TraitTupleAddonAccessImplSelfNotAssoc,
            TraitTupleAddonAccessImplSelfAssocQSelfInvalid,
            TraitTupleAddonAccessImplSelfAssocTraitNotFound,
            TraitTupleAddonAccessImplSelfAssocTraitInvalid,
            TraitTupleAddonAccessImplSelfAssocNotFound,
            TraitTupleAddonAccessImplSelfInconsistent,
            TraitTupleAddonAccessImplForUnknownNode,
            TraitTupleAddonAccessImplWhereClauseMissing,
            TraitTupleAddonAccessImplInconsistentWhereClausePreds,
            TraitTupleAddonAccessImplNotTypePred,
            TraitTupleAddonAccessImplTypePredNotSelf,
            TraitTupleAddonAccessImplSelfPredBoundsInconsistent,
            TraitTupleAddonAccessImplInvalidSelfInstBound,
            TraitTupleAddonAccessImplGenericParamsLenInvalid,
            TraitTupleAddonAccessImplHolderGenericParamTypeMissing,
            TraitTupleAddonAccessImplHolderParamWrongIdent,
            TraitTupleAddonAccessImplHolderParamExtraBounds,
            TraitTupleAddonAccessImplHolderParamNotTypeBound,
            TraitTupleAddonAccessImplHolderParamBoundInvalid,
            TraitTupleAddonAccessImplLifetimeParamHasBounds,
            TraitTupleAddonAccessImplLifetimeIdentInvalid,
            TraitTupleAddonAccessImplTypeParamHasBounds,
            TraitTupleAddonAccessImplTypeParamIdentInvalid,
            TraitTupleAddonAccessImplConstParamInvalidDefault,
            TraitTupleAddonAccessImplConstParamInvalidType,
            TraitTupleAddonAccessImplConstParamInvalidIdent,
            TraitTupleAddonAccessImplInvalidGenericParam,
            TraitTupleAddonAccessImplExtraItemsFound,
            TraitTupleAddonAccessImplItemNotType,
            TraitTupleAddonAccessImplItemInvalidType,
            TraitTupleAddonExpectedDocTypeAliasNotFound,
            TraitTupleExpecteDocTypeAliasNotNotFeatureGated,
            TraitTupleExpecteDocTypeAliasInvalidType,
            TraitTupleExpecteDocTypeAliasInvalidVisibility,
            TraitTupleExpecteDocTypeAliasHasNoDocs,
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
            TupleAttrNotRemoved,
            IdentDuplicateFound,
            TransformedFileInconsistent,
            TransformedFileShouldNotHaveDocTarget,
            TransformedFileDoesNotHaveDocTarget,
            TransformedFileDoesNotHaveNotDocTarget,
            NotDocTargetDoesNotHaveDocs,
            NodeOnlyInTypeAssoc,
            TupleOnlyInTypeAssoc,
            InstEitherTraitOrNodeOrTuple,
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
            EitherTupleOrNode,
            WhereClauseNotSupportInTuple,
            DoubleTuple,
            InconsistencyFindingTupleAttribute,
            ExpectedTupleTypeImpl,
            TupleSubAssocQualifiedButNotTrait,
            TupleArgMustBeStruct,
            TupleArgMustNotBeQualified,
            TupleArgPathQualifyElsewhere,
            TupleArgGenericArgsNotSupported,
            TupleSubAssocMustBeSelfQualified,
            TupleArgsAreInfered,
            DocFileTooManyItemsToTransform,
            DocFileContainsNoImpl,
            DocImplExpectedToContainDocAttr,
            DocFileExpectedToBeEmpty,
            TupleNodeNotTupleType,
            ImplTupleAddonNotInstFeatureGated,
            ImplTupleAddonNonCamelCaseAttrMissing,
            TupleEnumAddonInstImplsInconsistent,
            TupleEnumAddonInstImplItemsExtraFound,
            ImplTupleAddonInstImplNotInstAttributed,
            ImplTupleAddonInstImplInstArgInconsistent,
            ImplTupleAddonInstImplNotTraitImpl,
            ImplTupleAddonInstTraitImplInvalidTrait,
            TupleEnumAddonInstImplItemNotType,
            TupleEnumAddonInstImplItemTypeInvalid,
            TupleEnumAddonInstImplItemIdentInvalid,
            TupleEnumAddonVariantIndexInvalid,
            TupleEnumAddonVariantIdentInvalid,
            TupleEnumAddonVariantFieldsNotUnamed,
            TupleEnumAddonVariantFieldInconsistent,
            TupleEnumAddonVariantTypeInvalid,
            TupleEnumInstNodeTypeNotFound,
            TupleEnumInstNodeTypePubNodeAttrsNotFound,
            TupleEnumAddonVariantLastInstPunctNotTerminated,
            TupleEnumAddonVariantInstPunctInvalid,
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
            TupleOnlyAllowedInInstTraitImpl,
            ExpectedWhereClauseForSelf,
            ExpectedSelfInWhereClause,
            InconsistentTranformedImplTargetsLength,
            ItemNotDocTargetAttrNotFound,
            ItemDocTargetAttrNotFound,
            ItemDocTargetHasNoDocs,
            ItemNotDocTargetHasNoDocs,
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
        TupleRequiresArgs {
            fields: {
                attr: Attribute,
            },
            msg: "instance tuple requires arguments `<visibility> <ident>, ...`",
            tags: [Unsupported],
            span: { tokens: attr },
            note: [
                "<visibility> is the visibility of the generated instance tuple item type",
                "<ident> is the identifier naming the instance tuple item",
                "each tuple item consists of an optional visibility followed by an identifier",
                "multiple tuple items are separated by commas",
                "examples: `Foo`, `pub Foo`, `Foo, Bar`, `pub(crate) Foo, Bar`",
            ]
        },
        TupleRequiresArgsNotValue {
            fields: {
                meta: MetaNameValue,
            },
            msg: "instance tuple requires arguments `<visibility> <ident>, ...`, but found key-value",
            tags: [Unsupported],
            span: { tokens: meta },
            note: [
                "<visibility> is the visibility of the generated instance tuple item",
                "<ident> is the identifier naming the instance tuple item",
                "each tuple item consists of an optional visibility followed by an identifier",
                "multiple tuple items are separated by commas",
                "examples: `Foo`, `pub Foo`, `Foo, Bar`, `pub(crate) Foo, Bar`",
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
        InstTraitOverflowReference  {
            fields: {
                ident: Ident,
            },
            msg: "instance node or tuple cannot self reference its encompassing instance trait itself",
            tags: [Unsupported],
            span: { tokens: ident },
            help: [
                "remove the trait overflowing reference",
                "use aliasing if intended to be different trait with same ident",
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
        DoubleTuple {
            fields: {
                attr: Attribute,
            },
            msg: "only one instance tuple attribute annotation is allowed",
            tags: [Unsupported],
            span: { tokens: attr },
            help: [
                "found multiple tuple annotations, keep only one",
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
        TupleOnlyInTypeAssoc {
            fields: {
                attr: Attribute,
            },
            msg: "instance tuple attribute annotation only given via a associated type, found else",
            tags: [Unsupported],
            span: { tokens: attr },
            help: [
                "use instance tuple declaration only in associated types",
            ],
        },
        InstEitherTraitOrNodeOrTuple {
            fields: {
            },
            msg: "inst attribute is provided either for declaring trait instance or associated types as instance node or instance tuple",
            tags: [Unsupported],
            span: { span: &Span::call_site() },
            help: [
                "use #[inst] to declare this as instance trait or declare associated types as instance node or tuple, found none",
                "can remove this attribute to avoid ambiguity if none-such requirement exists",
            ],
            note: [
                "either declare idents as params to declare an instance trait `#[inst(<params>)]`",
                "where <params> are comma seperated ident list",
                "or declare a instance node via `#[node(..)]` in an associated type with bound to an instance trait",
                "or declare a instance tuple via `#[tuple(..)]` in an associated type",
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
        EitherTupleOrNode {
            fields: {
                attr: Attribute,
            },
            msg: "either provide instance tuple or instance node attribute annotation, found both",
            tags: [Unsupported],
            span: { tokens: attr },
            note: [
                "keep any one annotation, in case of lower level control use instance node",
            ],
        },
        WhereClauseNotSupportInTuple {
            fields: {
                where_: WhereClause,
            },
            msg: "where clauses are not yet supported in instance tuple assoc types",
            tags: [Unsupported],
            span: { tokens: where_ },
            help : [
                "remove the where clause"
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
        TupleRequiresArgs =>
            "inst tuple requires arguments",
        TupleRequiresArgsNotValue =>
            "inst tuple arguments must not resolve to a key value",
        SumNoArgs =>
            "sum does not require argument",
        InconsistencyFindingNodeAttribute =>
            "inconsistency finding instance node attribute annotation in trait",
        InconsistencyFindingTupleAttribute =>
            "inconsistency finding instance tuple attribute annotation in trait",
        TraitDocsNotFound =>
            "inst trait disclaimer docs are not appended",
        DocTargetInstTraitSumAttrExists =>
            "a doc-target-inst trait contains a sum type annotation",
        DocTargetInstTraitNodeAttrExists =>
            "a doc-target-inst trait contains a node type annotation",
        DocTargetInstTraitTupleAttrExists =>
            "a doc-target-inst trait contains a tuple type annotation",
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
        EitherTupleOrNode =>
            "allowed both tuple and node instance in a single assoc type",

        TraitTupleTargetItemNonCamelCaseAttrMissing =>
            "a tuple target item is missing the required non-camel-case-types allow attribute",
        TraitTupleTargetItemTypeAliasBoundsAttrMissing =>
            "a tuple target item is missing the required type-alias-bounds allow attribute",
        TraitTupleTargetItemUnusedAttrMissing =>
            "a tuple target item is missing the required unused allow attribute",
        TraitTupleExpectedAddonNotImplNorTypeAlias =>
            "an expected tuple addon is neither an impl nor a type alias",
        TraitTupleExpectedAddonAccessImplNotFeatureGated =>
            "an expected tuple addon access impl is not feature gated",
        TraitTupleExpectedAddonAccessImplArgsNotFound =>
            "an expected tuple addon access impl does not contain the required access arguments",
        TraitTupleAddonAccessImplSelfNotTypePath =>
            "a tuple addon access impl self type is not a type path",
        TraitTupleAddonAccessImplSelfNotAssoc =>
            "a tuple addon access impl self type is not an associated type",
        TraitTupleAddonAccessImplSelfAssocQSelfInvalid =>
            "a tuple addon access impl self associated type has an invalid qualified self type",
        TraitTupleAddonAccessImplSelfAssocTraitNotFound =>
            "a tuple addon access impl self associated type does not contain the expected trait",
        TraitTupleAddonAccessImplSelfAssocTraitInvalid =>
            "a tuple addon access impl self associated type contains an invalid trait",
        TraitTupleAddonAccessImplSelfAssocNotFound =>
            "a tuple addon access impl self associated type does not contain the expected associated type",
        TraitTupleAddonAccessImplSelfInconsistent =>
            "a tuple addon access impl self associated type path contains inconsistent path segments",
        TraitTupleAddonAccessImplForUnknownNode =>
            "a tuple addon access impl targets an unknown node type",
        TraitTupleAddonAccessImplWhereClauseMissing =>
            "a tuple addon access impl is missing its required where clause",
        TraitTupleAddonAccessImplInconsistentWhereClausePreds =>
            "a tuple addon access impl contains inconsistent where clause predicates",
        TraitTupleAddonAccessImplNotTypePred =>
            "a tuple addon access impl where clause does not contain a type predicate",
        TraitTupleAddonAccessImplTypePredNotSelf =>
            "a tuple addon access impl where clause type predicate is not bounded on Self",
        TraitTupleAddonAccessImplSelfPredBoundsInconsistent =>
            "a tuple addon access impl Self predicate contains inconsistent bounds",
        TraitTupleAddonAccessImplInvalidSelfInstBound =>
            "a tuple addon access impl Self predicate does not have the required instance tuple bound",
        TraitTupleAddonAccessImplGenericParamsLenInvalid =>
            "a tuple addon access impl has an invalid number of generic parameters",
        TraitTupleAddonAccessImplHolderGenericParamTypeMissing =>
            "a tuple addon access impl is missing its holder type parameter",
        TraitTupleAddonAccessImplHolderParamWrongIdent =>
            "a tuple addon access impl holder parameter has the wrong identifier",
        TraitTupleAddonAccessImplHolderParamExtraBounds =>
            "a tuple addon access impl holder parameter contains extra bounds",
        TraitTupleAddonAccessImplHolderParamNotTypeBound =>
            "a tuple addon access impl holder parameter does not have the required trait bound",
        TraitTupleAddonAccessImplHolderParamBoundInvalid =>
            "a tuple addon access impl holder parameter has an invalid trait bound",
        TraitTupleAddonAccessImplLifetimeParamHasBounds =>
            "a tuple addon access impl lifetime parameter has bounds",
        TraitTupleAddonAccessImplLifetimeIdentInvalid =>
            "a tuple addon access impl lifetime parameter has an invalid identifier",
        TraitTupleAddonAccessImplTypeParamHasBounds =>
            "a tuple addon access impl type parameter has bounds",
        TraitTupleAddonAccessImplTypeParamIdentInvalid =>
            "a tuple addon access impl type parameter has an invalid identifier",
        TraitTupleAddonAccessImplConstParamInvalidDefault =>
            "a tuple addon access impl const parameter has an invalid default",
        TraitTupleAddonAccessImplConstParamInvalidType =>
            "a tuple addon access impl const parameter has an invalid type",
        TraitTupleAddonAccessImplConstParamInvalidIdent =>
            "a tuple addon access impl const parameter has an invalid identifier",
        TraitTupleAddonAccessImplInvalidGenericParam =>
            "a tuple addon access impl contains an invalid generic parameter",
        TraitTupleAddonAccessImplExtraItemsFound =>
            "a tuple addon access impl contains unexpected extra items",
        TraitTupleAddonAccessImplItemNotType =>
            "a tuple addon access impl item is not a type",
        TraitTupleAddonAccessImplItemInvalidType =>
            "a tuple addon access impl item has an invalid type",
        TraitTupleAddonExpectedDocTypeAliasNotFound =>
            "an expected tuple documentation type alias was not found",
        TraitTupleExpecteDocTypeAliasNotNotFeatureGated =>
            "an expected tuple documentation type alias is not gated with not(feature = \"inst\")",
        TraitTupleExpecteDocTypeAliasInvalidType =>
            "an expected tuple documentation type alias has an invalid type",
        TraitTupleExpecteDocTypeAliasInvalidVisibility =>
            "an expected tuple documentation type alias has an invalid visibility",
        TraitTupleExpecteDocTypeAliasHasNoDocs =>
            "an expected tuple documentation type alias has no documentation",
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
        ExpectedTupleTypeImpl {
            fields: {
                ty: Type,
            },
            msg: "expected a tuple type or an associated type",
            tags: [Unsupported],
            span: { tokens: ty },
            help: [
                "either give a associated type `<T as Trait>::Assoc`",
                "or a concrete tuple type `(A, B, C)` or `(A,)` if single tuple",
            ],
            note: [
                "if associated type must be trait qualified",
                "if concrete tuple type it must be a well-declared structs",
                "concrete tuple types must not be path qualified i.e., (module::A, module::B) not allowed, qualify elsewhere"
            ]
        },
        TupleArgsAreInfered {
            fields: {
                attr: Attribute,
            },
            msg: "instance tuple does not require arguments here",
            tags: [Unsupported],
            span: { tokens: attr },
            help: [
                "as arguments are inferred remove these arguments",
            ],
        },
        TupleArgMustBeStruct {
            fields: {
                ty: Type,
            },
            msg: "expected a concrete struct type, found else",
            tags: [Unsupported],
            span: { tokens: ty },
            help: [
                "use a non-path qualified struct identifier only",
                "example: `Foo`",
            ],
            note: [
                "must not be path qualified i.e., module::A not allowed, qualify elsewhere"
            ]
        },
        TupleArgMustNotBeQualified {
            fields: {
                qself: Type,
            },
            msg: "expected a concrete non qualified struct type, found qualifier",
            tags: [Unsupported],
            span: { tokens: qself },
            help: [
                "use a non-path qualified struct identifier only",
                "example: `Foo`",
            ],
            note: [
                "must not be Self or Trait qualified"
            ]
        },
        TupleArgPathQualifyElsewhere {
            fields: {
                seg: PathSegment,
            },
            msg: "qualify this path elsewhere",
            tags: [Unsupported],
            span: { tokens: seg },
            help: [
                "expected a non-path qualified struct identifier only",
                "example: `Foo`",
            ],
            note: [
                "use import statements to qualify this elsewhere"
            ]
        },
        TupleArgGenericArgsNotSupported  {
            fields: {
                path_args: syn::PathArguments,
            },
            msg: "path arguments to concrete tuple structs not allowed",
            tags: [Future],
            span: { tokens: path_args },
            help: [
                "expected a non-path qualified, non-generic argumented struct identifier only",
                "example: `Foo`",
            ],
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
        TupleSubAssocQualifiedButNotTrait {
            fields: {
                qself: Type,
            },
            msg: "qualifier found but not trait qualified, `as` not found",
            tags: [Unsupported],
            span: { tokens: qself },
            help: [
                "give a direct associated type `<T as Trait>::Assoc` which is trait qualified",
            ],
        },
        TupleSubAssocMustBeSelfQualified {
            fields: {
                qself: Type,
            },
            msg: "qualifier found but not trait qualified, `as` not found",
            tags: [Unsupported],
            span: { tokens: qself },
            help: [
                "give a direct associated type `<T as Trait>::Assoc` which is trait qualified",
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
        TupleOnlyInTypeAssoc {
            fields: {
                attr: Attribute,
            },
            msg: "instance tuple attribute annotation only given via a associated type, found else",
            tags: [Unsupported],
            span: { tokens: attr },
            help: [
                "use instance tuple definition only in associated types",
            ],
        },
        InstEitherTraitOrNodeOrTuple {
            fields: {
            },
            msg: "inst attribute is provided either for defining the trait's instance impl or defining instance node or instance tuple associated",
            tags: [Unsupported],
            span: { span: &Span::call_site() },
            help: [
                "use #[inst] to define this instance trait with instance arguments or define instance node or instance tuple associated types, found none",
                "can remove this attribute to avoid ambiguity if none-such requirement exists",
            ],
            note: [
                "either declare idents as params to define the instance trait's impl `#[inst(<params>)]`",
                "where <params> are comma seperated ident or indexed ident list `ident, ident[i]` where i is the index counter",
                "or define an instance node via `#[node(..)]` in an associated type with instance arguments",
                "or define an instance tuple via `#[tuple]` in an associated type",
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
        EitherTupleOrNode {
            fields: {
                attr: Attribute,
            },
            msg: "either provide instance tuple or instance node attribute annotation, found both",
            tags: [Unsupported],
            span: { tokens: attr },
            note: [
                "keep any one annotation, in case of lower level control use instance node",
            ],
        },
        WhereClauseNotSupportInTuple {
            fields: {
                where_: WhereClause,
            },
            msg: "where clauses are not yet supported in instance tuple assoc types",
            tags: [Unsupported],
            span: { tokens: where_ },
            help : [
                "remove the where clause"
            ],
        },
        DoubleTuple {
            fields: {
                attr: Attribute,
            },
            msg: "only one instance tuple attribute annotation is allowed",
            tags: [Unsupported],
            span: { tokens: attr },
            help: [
                "found multiple tuple annotations, keep only one",
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
        TupleAttrNotRemoved =>
            "tuple attribute annotation on type-assoc not removed",
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
        EitherTupleOrNode =>
            "allowed both tuple and node instance in a single assoc type",
        InconsistencyFindingTupleAttribute =>
            "inconsistency finding instance tuple attribute annotation in impl",
        ExpectedTupleTypeImpl => 
            "expected a tuple type or an associated type for instance tuple impl type resolution",
        TupleSubAssocQualifiedButNotTrait =>
            "tuple node associated type is not qualified via a trait",
        DocFileTooManyItemsToTransform =>
            "a doc impl file contains too many items to transform",
        DocFileContainsNoImpl =>
            "a doc impl file contains no impl item",
        DocImplExpectedToContainDocAttr =>
            "a doc impl is missing the required not-inst (doc-only) feature attribute",
        DocFileExpectedToBeEmpty =>
            "a doc impl file is expected to be empty",

        ImplTupleAddonNotInstFeatureGated =>
            "a tuple addon is not gated with feature `inst`",
        ImplTupleAddonNonCamelCaseAttrMissing =>
            "a tuple addon is missing the required non-camel-case-types allow attribute",
        TupleEnumAddonInstImplsInconsistent =>
            "a tuple addon enum does not have a corresponding instance impl for each variant",
        TupleEnumAddonInstImplItemsExtraFound =>
            "a tuple addon instance impl contains unexpected extra items",
        ImplTupleAddonInstImplNotInstAttributed =>
            "a tuple addon instance impl is missing the required inst attribute",
        ImplTupleAddonInstImplInstArgInconsistent =>
            "a tuple addon instance impl contains an inconsistent inst argument",
        ImplTupleAddonInstImplNotTraitImpl =>
            "a tuple addon instance impl does not implement a trait",
        ImplTupleAddonInstTraitImplInvalidTrait =>
            "a tuple addon instance impl implements an invalid trait",
        TupleEnumAddonInstImplItemNotType =>
            "a tuple addon instance impl item is not a type",
        TupleEnumAddonInstImplItemTypeInvalid =>
            "a tuple addon instance impl Variant type is invalid",
        TupleEnumAddonInstImplItemIdentInvalid =>
            "a tuple addon instance impl item has an invalid identifier",
        TupleEnumAddonVariantIndexInvalid =>
            "a tuple addon enum variant has an invalid instance index",
        TupleEnumAddonVariantIdentInvalid =>
            "a tuple addon enum variant has an invalid identifier",
        TupleEnumAddonVariantFieldsNotUnamed =>
            "a tuple addon enum variant does not have unnamed fields",
        TupleEnumAddonVariantFieldInconsistent =>
            "a tuple addon enum variant does not contain exactly one field",
        TupleEnumAddonVariantTypeInvalid =>
            "a tuple addon enum variant field has an invalid type",
        TupleEnumInstNodeTypeNotFound =>
            "a tuple addon instance node type was not found",
        TupleEnumInstNodeTypePubNodeAttrsNotFound =>
            "a tuple addon instance node type is missing the required publisher node attribute",        
        TupleEnumAddonVariantLastInstPunctNotTerminated =>
            "the last tuple enum variant instance specification is not terminated with a semicolon",
        TupleEnumAddonVariantInstPunctInvalid =>
            "a non-last tuple enum variant instance specification is incorrectly terminated with a semicolon",

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
        TupleOnlyAllowedInInstTraitImpl {
            fields: {
                attr: Attribute,
            },
            msg: "instance tuple node is only allowed in traits and impls, not in instance accessor module containing impls",
            tags: [Unsupported],
            span: { tokens: attr },
            note: [
                "remove this instance tuple attribute annotation",
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
        TupleOnlyAllowedInInstTraitImpl =>
            "instance tuple node only allowed in instance traits and impls",
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
