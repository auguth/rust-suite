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
// ```````````````````````````` INSTANCE ACCESS ERROR ````````````````````````````
// ===============================================================================

//! Diagnostics emitted by the instance-access proc-macro pipeline.
//!
//! Defines error spaces, structured diagnostics, and internal invariant
//! violations covering associated access expression extraction, counter idents and
//! generic indexes validation and generic placement.

// ===============================================================================
// ``````````````````````````````````` IMPORTS ```````````````````````````````````
// ===============================================================================

// --- Proc-Suite ---
use proc_suite::{BStringList, IntList, bug_diagnostics, diagnostics, error_spaces};

// --- Local Crate ---
use crate::{access::args::BStrInputList, errors::MAINTAINERS};

// --- Proc-Macro Utils ---
use proc_macro2::Span;
use quote::ToTokens;
use syn::{
    Attribute, BoundLifetimes, Expr, ExprPath, GenericArgument, GenericParam, Generics, Ident,
    ImplItem, Item, Lifetime, LitByteStr, LitInt, ParenthesizedGenericArguments, Path,
    PathArguments, PathSegment, PredicateType, TraitBound, Type, TypeParamBound, TypePath,
    WhereClause,
    punctuated::Punctuated,
    token::{Comma, SelfValue},
};

// ===============================================================================
// `````````````````````````````````` CONSTANTS ``````````````````````````````````
// ===============================================================================

pub(super) const KEY_BUG: &'static str =
    "instance access proc macro key-arguments eval-phase didn't follow invariants";

pub(super) const VALID_BUG: &'static str =
    "instance access proc macro validation phase didn't follow invariants";

pub(super) const EXTRACT_BUG: &'static str =
    "instance access proc macro extraction phase didn't follow invariants";

pub(super) const REPLACE_BUG: &'static str =
    "instance access proc macro replacement phase didn't follow invariants";

pub(super) const RESOLVE_BUG: &'static str =
    "instance access proc macro resolution phase didn't follow invariants";

pub(super) const GETTER_BUG: &'static str =
    "instance getter access proc macro phase didn't follow invariants";

// ===============================================================================
// ````````````````````````````````` ERROR-SPACES ````````````````````````````````
// ===============================================================================

error_spaces! {
    space: "INSTANCE_ACCESS",
    maintain: MAINTAINERS,

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // ``````````````````````````````` KEYS DIAGNOSTICS ``````````````````````````````
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

    pub(super) enum KeyExpectation {
        range: 0..=100,
        variants: {
            ExpectedKeys,
            ExpectedLeaf,
            ExpectedBranch,
            ExpectedPrune,
            ExpectedTrim,
            ExpectedExtend,
            ExpectedSpread,
            ExpectedRoot,
            ExpectedTraverse,
            ExpectedDescend,
            ExpectedDynKeyArgs,
        }
    }

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // ``````````````````````````` ACCESS ARGUMENTS SPACE ````````````````````````````
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

    pub(super) enum ArgumentSpace {
        range: 101..=200,
        variants: {
            BStrInputParseFail,
            BStrInputListParseFail,
            IdentsAndIndexesNotSameLen,
            BStrExprAndIndexesNotSameLen,
            DuplicateCounterIndexes,
            ExpectedSingleSegmentIdent,
            DepthArgsInconsistent,
            UnexpectedParentDepth,
            UnexpectedChildDepth,
            UnexpectedPathDepth,
            UnexpectedPrefixDepth,
            UnexpectedPrefixIdentDepth,
            UnexpectedPrefixPathDepth,
            AccessorExprUnsupported,
            MaxDepthReached,
            MaxTwoDepthsOnly,
            DepthIsZero,
            Maximum4Counters,
        }
    }

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // `````````````````````````` ACCESS VALIDATION SPACE ````````````````````````````
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

    pub(super) enum ValidationSpace {
        range: 201..=400,
        variants: {
            TraitImplNotAllowed,
            SelfTypeNotPath,
            QSelfTypeNotGeneric,
            NestedQSelfNotAllowed,
            GenericParameterHasArguments,
            QSelfGenericNotDeclared,
            QSelfGenericNotFirst,
            MultipleAssociatedSegments,
            AssociatedTypeHasArguments,
            AssociatedTypeMissing,
            FirstTypeNotGeneric,

            SelfNotFirstPathSegment,
            SelfHasMultipleFollowingSegments,
            SelfUsedInQualifiedSelfPath,
            SelfHasGenericArguments,

            DuplicateImplGenericParameter,
            NodeBoundGenericArgNotGenericParam,
            NodeBoundGenericArgNotExpectedGeneric,
            NodeBoundIntermediatePathHasArgs,

            ExtractedBoundLifetimeAfterGeneric,
            ExtractedBoundGenericOrderMismatch,

            SelfTypeNodeBoundMismatch,
            SelfTypeNodeBoundQualifierMismatch,

            SelfTypeNotValidated,
            InstanceAssocMustUseSelf,
            InstanceQAssocMustUseSelf,
            InstanceAssocTraitQualificationRequired,

            InstTraitUsedInImpl,
            InstTraitNotRemovedFromImpl,

            AssociatedItemRequiresQualifier,

            InstanceAssocTypeSelfReferenceNotAllowed,

            InvalidSelfFnArgument,

            SelfNotAllowedInConstItem,

            InstanceAssocTypePredicateNotAllowed,
            InstanceAssocTypePredicateNotRemoved,

            StandaloneSelfNotReplaced,

            SelfAssocFunctionMustBeCalled,
            SelfAssocFunctionPathRequired,

            ItemNotProvidedInFile,
            ImplProvidedInFile,
            ImplNotProvidedInFile,
            NotDocItemNotProvided,
            DocItemNotProvided,
            InvalidItemCfg,
            SelfFoundAfterTransformation,

            InnerSelfItem,
            SelfAssocNotAllowedInTraitBound,
        }
    }

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // `````````````````````````` ACCESS EXTRACTION SPACE ````````````````````````````
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

    pub(super) enum ExtractionSpace {
        range: 401..=500,
        variants: {
            QSelfGenericLifetimeBoundNotAllowed,
            QSelfGenericMultipleBounds,
            QSelfGenericTraitBoundNotFound,
            InstanceNodeExtractionInconsistent,
            NodeBoundHrtbNotSupported,
            NodeBoundSizedBoundInvalid,

            SelfTraitPredicateWhereClauseMissing,
            SelfTraitPredicateMissing,

            MultipleSelfTraitPredicates,
            SelfTraitBoundUnknownFirstBound,
            SelfTraitBoundNotTrait,
            InstanceTraitRepeated,
            HrtbInstanceTraitNotAllowed,
            SizedInstanceTraitNotAllowed,
            InstanceTraitNotRemoved,
        }
    }

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // ````````````````````````` ACCESS REPLACEMENT SPACE ````````````````````````````
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

    pub(super) enum ReplacementSpace {
        range: 501..=600,
        variants: {
            SelfAssocParenArgsNotSupported,
            ImplGenericsNotAppendedToSelfAssoc,
            ImplTyGenericsNotAppendedToSelfAssocSegment,

            SelfAssocSelfNotStripped,

            SumTypeSelfAssocNotReplaced,

            SelfAssocNotReplaced,

            SelfAssocCallNotReplaced,

            ReturnTyInferedErrorNotReplaced,

            NonLeafFnArgIdentConflict,

            ExpectedIdentNotRawBStr,

            RawBStrInputNotRemoved,

            DynFnArgNotAppended,

            DynFnCallArgNotAppended,

            SumAttrNotRemoved,

            DocPathNotTypePath,

            SumAttrOnlyOnType,
            AllTypesAreSum,
        }
    }

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // `````````````````````````` ACCESS RESOLUTION SPACE ````````````````````````````
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

    pub(super) enum ResolutionSpace {
        range: 601..=700,
        variants: {
            DuplicateCounterIndexes,
            AssocQSelfLastIdentUnavailable,
            IdentsAndIndexesNotSameLen,
            ExprPathRequiresQSelf,
            QSelfRequiresExprPath,
            InstanceTraitPathNotFound,
            QSelfNotTypePath,

            InstanceNonLeafPathResolutionYieldedNonPath,
            InstanceNonLeafPathResolvedQSelfMissing,
            InstanceNonLeafPathResolvedQSelfInvalid,
            InstanceNonLeafPathResolvedInconsistentAccess,

            ExprCallPathNotExprPath,
            MaxDynCounters,

            HangingBordersNotAvailable,

            IdentCheckerExprsEmpty,
            OnlyCounterHaveMultiDimensionExprs,
            MultiCounterHaveLessThanOneDimensionExprs,
            MultiCounterButIndexesInsufficient,
            TralingIdentCheckersUnnessary,

            DepthIsZero,
        }
    }

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // ````````````````````````````` ACCESS GETTER SPACE `````````````````````````````
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

    pub(super) enum GetterSpace {
        range: 701..=800,
        variants: {
            RemoveBStrInputs,
            NotRemovedBStrInputs,
            PathRequiresQSelf,
            PathRequiresQSelfTrait,
            TraitHasParenArgs,
            PathRequiresAssoc,
            AssocHasParenArgs,
            PathHasQSelf,
            PathHasNoAssoc,
            PathHasParenArgs,
            ArgsNotAppended,
        }
    }

}

// ===============================================================================
// ````````````````````````````````` DIAGNOSTICS `````````````````````````````````
// ===============================================================================

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ``````````````````````````````` KEY DIAGNOSTICS ```````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

diagnostics!(
    space: KeyExpectation,

    pub(crate) enum KeysError {
        ExpectedKeys {
            fields: {
                span: Span,
            },
            msg: "expected one of the following instance selectors \"leaf | branch | prune | trim | extend | spread | traverse | descend | root\"",
            tags: [Unsupported],
            span: { span: span },
            help: [
                "`leaf` : exact leaf. Example: #[_macro_(leaf { index(1,2), ident(b\"a\", b\"b\") })]",
                "`branch` : branch and children. Example: #[_macro_(branch { index(1,2,3), parent(b\"a\"), child(b\"b\") })]",
                "`prune` : root to instance. Example: #[_macro_(prune { index(1,2), ident(b\"a\", b\"b\") })]",
                "`trim` : root to branch. Example: #[_macro_(trim { index(1,2), ident(b\"a\", b\"b\") })]",
                "`extend` : instance to end. Example: #[_macro_(extend { index(1,2), ident(b\"a\", b\"b\") })]",
                "`spread` : branch to end. Example: #[_macro_(spread { index(1,2), ident(b\"a\", b\"b\") })]",
                "`traverse` : branch prefix to instance. Example: #[_macro_(traverse { index(1,2,3), prefix(b\"a\"), ident(b\"b\", b\"c\") })]",
                "`descend` : branch prefix to instance. Example: #[_macro_(descend { index(1,2,3), prefix(b\"a\"), ident(b\"b\", b\"c\") })]",
                "`root` : root hierarchy. Example: #[_macro_(root { index(1,2), ident(b\"a\", b\"b\") })]",
            ],
            note: [
                "`leaf` selects the specified leaf instances.",
                "`branch` selects instances below the specified branch; `child` can restrict the selection.",
                "`prune` selects from the root through the specified instance.",
                "`trim` selects from the root through the specified branch.",
                "`extend` selects from the specified instance through the end.",
                "`spread` selects from the specified branch through the end.",
                "`traverse` selects using a branch `prefix` and an instance path.",
                "`descend` selects using a branch `prefix` and an instance path.",
                "`root` selects instances in the root hierarchy.",
                "`depth` is optional and specifies the access level depth when provided.",
            ]
        },


        ExpectedLeaf {
            fields: {
                span: Span,
            },
            msg: "invalid leaf arguments",
            tags: [Unsupported],
            span: { span: span },
            help: [
                "`leaf` : accesses exact leaf instances",
                "Example: #[_macro_(leaf { index(1,2), ident(b\"a\", b\"b\") })]",
            ],
            note: [
                "`ident` must contain one expression for each `index`",
                "`ident` accepts raw byte string literals only",
                "`index` values must be unique",
            ]
        },

        ExpectedBranch {
            fields: {
                span: Span,
            },
            msg: "invalid branch arguments",
            tags: [Unsupported],
            span: { span: span },
            help: [
                "`branch` : accesses instances below a branch, optionally restricted to children",
                "Example: #[_macro_(branch { index(1,2,3), parent(b\"a\"), child(bytes, b\"c\") })]",
            ],
            note: [
                "`parent` identifies the branch using byte string literals",
                "`child` identifies the child instances using byte string literals or identifiers",
                "`parent` and `child` together must account for all `index` values",
                "`depth` is optional and specifies the access level when provided",
                "`index` values must be unique",
            ]
        },
        ExpectedPrune {
            fields: {
                span: Span,
            },
            msg: "invalid prune arguments",
            tags: [Unsupported],
            span: { span: span },
            help: [
                "`prune` : accesses instances from the root through an exact instance",
                "Example: #[_macro_(prune { index(1,2), ident(b\"a\", bytes) })]",
            ],
            note: [
                "`ident` must contain one expression for each `index`",
                "`ident` accepts byte string literals, identifiers, or a mixture of both",
                "`depth` is optional and specifies the access level when provided",
                "`index` values must be unique",
            ]
        },

        ExpectedTrim {
            fields: {
                span: Span,
            },
            msg: "invalid trim arguments",
            tags: [Unsupported],
            span: { span: span },
            help: [
                "`trim` : accesses instances from the root through a branch",
                "Example: #[_macro_(trim { index(1,2), ident(b\"a\", bytes) })]",
            ],
            note: [
                "`ident` must contain one expression for each `index`",
                "`ident` accepts byte string literals, identifiers, or a mixture of both",
                "`depth` is optional and specifies the access level when provided",
                "`index` values must be unique",
            ]
        },
        ExpectedExtend {
            fields: {
                span: Span,
            },
            msg: "invalid extend arguments",
            tags: [Unsupported],
            span: { span: span },
            help: [
                "`extend` : accesses instances from an exact instance through the end",
                "Example: #[_macro_(extend { index(1,2), ident(b\"a\", bytes) })]",
            ],
            note: [
                "`ident` must contain one expression for each `index`",
                "`ident` accepts byte string literals, identifiers, or a mixture of both",
                "`depth` is optional and specifies the access level when provided",
                "`index` values must be unique",
            ]
        },

        ExpectedSpread {
            fields: {
                span: Span,
            },
            msg: "invalid spread arguments",
            tags: [Unsupported],
            span: { span: span },
            help: [
                "`spread` : accesses instances from a branch through the end",
                "Example: #[_macro_(spread { index(1,2), ident(b\"a\", bytes) })]",
            ],
            note: [
                "`ident` must identify a branch, with fewer expressions than `index`",
                "`ident` accepts byte string literals, identifiers, or a mixture of both",
                "`depth` is optional and specifies the access level when provided",
                "`index` values must be unique",
            ]
        },
        ExpectedTraverse {
            fields: {
                span: Span,
            },
            msg: "invalid traverse arguments",
            tags: [Unsupported],
            span: { span: span },
            help: [
                "`traverse` : accesses instances from an exact instance through a branch",
                "Example: #[_macro_(traverse { index(1,2,3), prefix(b\"a\"), ident(bytes, b\"c\") })]",
            ],
            note: [
                "`prefix` (optional) identifies the branch using byte string literals",
                "`ident` identifies the remaining instance path using byte string literals or identifiers",
                "`prefix` and `ident` together must account for all `index` values",
                "without `prefix`, `ident` must contain one expression for each `index`",
                "`depth` is optional and specifies the access level when provided",
                "`index` values must be unique",
            ]
        },

        ExpectedDescend {
            fields: {
                span: Span,
            },
            msg: "invalid descend arguments",
            tags: [Unsupported],
            span: { span: span },
            help: [
                "`descend` : accesses instances from a branch through an exact instance",
                "Example: #[_macro_(descend { index(1,2,3), prefix(b\"a\"), ident(bytes, b\"c\") })]",
            ],
            note: [
                "`prefix` (optional) identifies the branch using byte string literals",
                "`ident` identifies the remaining instance path using byte string literals or identifiers",
                "`prefix` and `ident` together must account for all `index` values",
                "without `prefix`, `ident` must contain one expression for each `index`",
                "`depth` is optional and specifies the access level when provided",
                "`index` values must be unique",
            ]
        },

        ExpectedRoot {
            fields: {
                span: Span,
            },
            msg: "invalid root arguments",
            tags: [Unsupported],
            span: { span: span },
            help: [
                "`root` : accesses instances from the root hierarchy",
                "Example: #[_macro_(root { index(1,2), ident(b\"a\", bytes) })]",
            ],
            note: [
                "`ident` must contain one expression for each `index`",
                "`ident` accepts byte string literals, identifiers, or a mixture of both",
                "`depth` is optional and specifies the access level when provided",
                "`index` values must be unique",
            ]
        },
    }
);

bug_diagnostics! {
    space: KeyExpectation,
    bug: KEY_BUG.to_string(),

    pub(super) enum KeysBugs {
        ExpectedDynKeyArgs =>
            "expected dyn key args not leaf",
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// `````````````````````````` INSTANCE ARGUMENTS ACCESS ``````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

diagnostics!(
    space: ArgumentSpace,

    pub(super) enum ArgumentErrors {
        IdentsAndIndexesNotSameLen {
            fields: {
                indexes: IntList,
                idents: BStringList,
            },
            msg: format!("according to given counter indexes expected {} idents, found {}", indexes.ints.len().to_string(), idents.bytes.len().to_string()),
            tags: [Unsupported],
            span: { tokens: &idents.bytes },
            note: [
                "for each counter index the corresponding ident must be given",
                "hence both indexes and idents should have same length of elements",
                "example: (0,1) (b\"ident0\", b\"ident1\"), )"
            ]
        },
        BStrExprAndIndexesNotSameLen {
            fields: {
                indexes: IntList,
                idents: BStrInputList,
            },
            msg: format!("according to given counter indexes expected {} byte string literals or identifiers or mixed, found {}", indexes.ints.len().to_string(), idents.exprs.len().to_string()),
            tags: [Unsupported],
            span: { tokens: &idents.exprs },
            note: [
                "for each counter index the corresponding ident expression must be given",
                "hence both indexes and idents should have same length of elements",
                "example: (0,1) (b\"ident0\", bytes), )"
            ]
        },
        ExpectedSingleSegmentIdent {
            fields: {
                span: Span,
            },
            msg: "expected a single-segment identifier",
            tags: [Unsupported],
            span: { span: span },
            help: [
                "provide a single-segment identifier",
                "if its an expression wrap in braces `{ .. }`",
                "Example: `bytes` or if expression then `{ bytes }` ",
            ],
            note: [
                "qualified paths (<Q as T>::T) and multi-segment (mod::T) paths are not allowed",
            ]
        },
        DepthArgsInconsistent {
            fields: {
                lens: IntList,
                idents: BStrInputList,
            },
            msg: "one depth argument (unsigned literal) is allowed per accessing ident",
            tags: [Unsupported],
            span: { tokens: &lens.ints },
            help: [
                {format!("depth list has `{}` elements, where as should provide `{}` elements", lens.ints.len(), idents.exprs.len())},
            ],
            note: [
                "either depths should be provided for all dynamic idents or not"
            ]
        },
        UnexpectedParentDepth {
            fields: {
                span: Span,
            },
            msg: "invalid parent length",
            tags: [Unsupported],
            span: { span: span },
            help: [
                "`parent` must contain fewer identifiers than `index`",
                "Example: { index(1,2,3), parent(b\"a\"), child(b\"b\", bytes) }",
            ],
            note: [
                "`parent` identifies the branch",
            ]
        },

        UnexpectedChildDepth {
            fields: {
                span: Span,
            },
            msg: "invalid child length",
            tags: [Unsupported],
            span: { span: span },
            help: [
                "`child` must contain fewer identifiers than `index`",
                "Example: { index(1,2,3), parent(b\"a\"), child(b\"b\", bytes) }",
            ],
            note: [
                "`child` identifies instances below the branch",
            ]
        },

        UnexpectedPathDepth {
            fields: {
                span: Span,
            },
            msg: "invalid path length",
            tags: [Unsupported],
            span: { span: span },
            help: [
                "`parent` and `child` must together account for all `index` values",
                "Example:{ index(1,2,3), parent(b\"a\"), child(b\"b\", bytes) }",
            ],
            note: [
                "`parent` identifies the branch and `child` identifies the remaining hierarchy",
            ]
        },
        UnexpectedPrefixDepth {
            fields: {
                span: Span,
            },
            msg: "invalid prefix length",
            tags: [Unsupported],
            span: { span: span },
            help: [
                "`prefix` must contain fewer identifiers than `index`",
                "Example: { index(1,2,3), prefix(b\"a\"), ident(b\"b\", bytes) }",
            ],
            note: [
                "`prefix` identifies the branch",
            ]
        },

        UnexpectedPrefixIdentDepth {
            fields: {
                span: Span,
            },
            msg: "invalid ident length",
            tags: [Unsupported],
            span: { span: span },
            help: [
                "`ident` must contain fewer identifiers than `index`",
                "Example: { index(1,2,3), prefix(b\"a\"), ident(b\"b\", bytes) }",
            ],
            note: [
                "`ident` identifies instances below the branch",
            ]
        },

        UnexpectedPrefixPathDepth {
            fields: {
                span: Span,
            },
            msg: "invalid path length",
            tags: [Unsupported],
            span: { span: span },
            help: [
                "`prefix` and `ident` must together account for all `index` values",
                "Example:{ index(1,2,3), prefix(b\"a\"), ident(b\"b\", bytes) }",
            ],
            note: [
                "`prefix` identifies the branch and `ident` identifies the remaining hierarchy",
            ]
        },

        AccessorExprUnsupported {
            fields: {
                expr: Expr,
            },
            msg: "unsupported instance accessor expression kind",
            tags: [Unsupported],
            span: { tokens: expr },
            help: [
                "internally `Self::..` is expanded to the following,",
                "an associated-item path: `<Self as InstanceTrait>::ITEM`",
                "or an associated-item call: `<Type as InstanceTrait>::method(args)`"
            ],
            note: [
                "also supports nested expression kinds of: path, call, reference, return, try?, and tuple",
                "any of the above supported expression kinds are not satisfied",
            ]
        },

        MaxDepthReached {
            fields: {
                lit: LitInt,
                max: u8,
            },
            msg: format!("expected at-max {} depth, but found else", max),
            tags: [Future],
            span: { tokens: lit },
        },
        MaxTwoDepthsOnly {
            fields: {
                depths: IntList,
            },
            msg: "expected at-max only 2 depth elements, found more",
            tags: [Future],
            span: { tokens: depths },
            note: [
                "use branch arguments for deep counters",
                "reduce explosion by having lesser depths",
                "when depths are deep use static/prefix/parent access over purely dynamic"
            ]
        },
        DepthIsZero {
            fields: {
                lit: LitInt,
            },
            msg: "depth indexes are length like, hence zero depth is not accepted",
            tags: [Unsupported],
            span: { tokens: lit },
            note: [
                "use non-zero depth values, or non-dynamic arguments",
            ]
        },
        Maximum4Counters {
            fields: {
                ints: IntList,
            },
            msg:
                "maximum instance access given for 4 instance counter params only",
            tags: [Unsupported],
            span: { tokens: ints },
            note: [
                "access attempted for more than four instance counter params"
            ]
        },

    }
);

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ````````````````````````````` VALIDATION ERRORS ```````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

diagnostics!(
    space: ValidationSpace,

    pub(super) enum ValidErrors {
        TraitImplNotAllowed {
            fields: {
                item: Path,
            },
            msg: "instance accessor requires an inherent impl",
            tags: [Unsupported],
            span: { tokens: item },
            note: [
                "use an inherent impl instead of implementing the instance trait",
                "add `Self: InstanceTrait` as the first bound in the where clause",
            ]
        },

        SelfTypeNotPath {
            fields: {
                ty: Type,
            },
            msg: "impl self type must be a type path",
            tags: [Unsupported],
            span: { tokens: ty },
            help: [
                "use a type such as `T::Associated` or `<T as Trait>::Associated`",
            ],
            note: [
                "the impl self type must be a generic associated type path",
            ]
        },
        QSelfTypeNotGeneric {
            fields: {
                ty: Type,
            },
            msg: "qualified self type must be a generic parameter",
            tags: [Unsupported],
            span: { tokens: ty },
            note: [
                "use a generic parameter such as `T` in `<T as Trait>::Associated`",
            ]
        },
        NestedQSelfNotAllowed {
            fields: {
                ty: TypePath,
            },
            msg: "nested qualified self types are not supported",
            tags: [Unsupported],
            span: { tokens: ty },
            note: [
                "the qualified self type must be a simple generic parameter",
                "use a generic parameter such as `T` in `<T as Trait>::Associated`",
            ]
        },
        GenericParameterHasArguments {
            fields: {
                segment: PathSegment,
            },
            msg: "generic parameter cannot have generic arguments",
            tags: [Unsupported],
            span: { tokens: segment },
            note: [
                "the generic parameter must be used without generic arguments",
                "its mimicking HKTs (higher kinded types) which is not available in rust",
                "use a generic parameter such as `T` in `<T as Trait>::Associated`",
            ]
        },
        QSelfGenericNotDeclared {
            fields: {
                segment: PathSegment,
            },
            msg: "qualified self type must be a declared generic parameter",
            tags: [Unsupported],
            span: { tokens: segment },
            note: [
                "the generic parameter must be declared by the impl",
                "use the first type generic parameter of the impl as the QSelf type",
            ]
        },
        QSelfGenericNotFirst {
            fields: {
                segment: PathSegment,
            },
            msg: "the QSelf generic parameter must be the first type generic parameter declared by the impl",
            tags: [InvalidInput],
            span: { tokens: segment },
            help: [
                "use the first type generic parameter of the impl as the QSelf type",
            ],
            note: [
                "lifetimes may appear before the first type generic parameter",
                "for example, `impl<'a, T, K>` may use `T`, but `impl<'a, K, T>` cannot use `K` or `T` as the QSelf type",
            ]
        },
        MultipleAssociatedSegments {
            fields: {
                ty: TypePath,
            },
            msg: "self type must contain exactly one direct associated type",
            tags: [Unsupported],
            span: { tokens: ty },
            note: [
                "nested associated paths are not supported",
                "use a direct self type such as `T` in `<T as Trait>::Associated`",
            ]
        },
        AssociatedTypeHasArguments {
            fields: {
                segment: PathSegment,
            },
            msg: "associated types cannot have generic arguments",
            tags: [Future],
            span: { tokens: segment },
            note: [
                "generic associated types on instance nodes are not supported yet",
            ]
        },
        AssociatedTypeMissing {
            fields: {
                ty: TypePath,
            },
            msg: "self type must contain exactly one associated type",
            tags: [Unsupported],
            span: { tokens: ty },
            note: [
                "use `T::Associated` or `<T as Trait>::Associated`",
            ]
        },
        FirstTypeNotGeneric {
            fields: {
                segment: PathSegment,
            },
            msg: "first path segment must be a declared generic parameter",
            tags: [Unsupported],
            span: { tokens: segment },
            note: [
                "use `T::Associated` where `T` is a generic parameter of the impl",
                "use the first type generic parameter of the impl as the first path segment",
            ]
        },

        SelfNotFirstPathSegment {
            fields: {
                seg: PathSegment,
            },
            msg: "`Self` must be the first path segment",
            tags: [Unsupported],
            span: { tokens: seg },
            note: [
                "`Self` may only appear as `Self` or `Self::X`",
                "paths such as `Foo::Self` and `Foo::Bar::Self` are not supported",
            ]
        },
        SelfHasMultipleFollowingSegments {
            fields: {
                seg: PathSegment,
            },
            msg: "only one path segment may follow `Self`",
            tags: [Unsupported],
            span: { tokens: seg },
            note: [
                "`Self::X` / `Self::X<A>` is supported",
                "`Self::X::Y` is not supported",
            ]
        },
        SelfUsedInQualifiedSelfPath {
            fields: {
                seg: PathSegment,
            },
            msg: "`Self` cannot be used in a trait or path component of a qualified self path",
            tags: [Unsupported],
            span: { tokens: seg },
            note: [
                "qualified self paths such as `<Foo as Self>::Assoc` or `<Foo as Trait>::Self` are not supported",
            ]
        },
        SelfHasGenericArguments {
            fields: {
                seg: PathSegment,
            },
            msg: "`Self` must not have generic arguments",
            tags: [Future],
            span: { tokens: seg },
            note: [
                "remove the generic arguments from `Self`",
                "generic arguments may only be specified on the associated item",
                "expected: `Self` / `Self::Assoc` / `Self::Assoc<X>`",
                "`Self<dyn Trait>` is expected in future",
            ]
        },

        DuplicateImplGenericParameter {
            fields: {
                ident: Ident,
            },
            msg: format!("generic parameter `{ident}` already utilized in impl"),
            tags: [Unsupported],
            span: { tokens: ident },
            help: [
                {format!("if not hidden, declare `{ident}` only once in the impl generics")},
                "if hidden, rename this generic parameter"
            ],
        },
        NodeBoundGenericArgNotGenericParam {
            fields: {
                arg: GenericArgument,
            },
            msg: "expected a generic parameter",
            tags: [InvalidInput],
            span: { tokens: arg },
            help: [
                "use a generic parameter declared by the impl",
            ],
            note: [
                "instance node trait arguments may only be type parameters, const parameters, or lifetime parameters declared by the impl",
            ]
        },
        NodeBoundGenericArgNotExpectedGeneric {
            fields: {
                ident: Ident,
            },
            msg: "expected a generic parameter",
            tags: [InvalidInput],
            span: { tokens: ident },
            help: [
                "use a generic parameter declared by the impl",
            ],
            note: [
                "instance node trait arguments may only be type parameters, const parameters, or lifetime parameters declared by the impl",
            ]
        },
        NodeBoundIntermediatePathHasArgs {
            fields: {
                seg: PathSegment,
            },
            msg: "instance node trait intermediate path segment has arguments",
            tags: [Future],
            span: { tokens: seg },
            help: [
                "remove these path arguments",
            ],
            note: [
                "in future if the semantics are valid it will be supported",
            ]
        },

        ExtractedBoundLifetimeAfterGeneric {
            fields: {
                lifetime: Lifetime,
            },
            msg: format!("lifetime `{lifetime}` appears after a non-lifetime generic argument"),
            tags: [InvalidInput],
            span: { tokens: lifetime },
            help: [
                "place all lifetime arguments before type and const arguments",
            ],
            note: [
                "the lifetime parameters must appear in the same order in the impl as they appear in the instance trait bound",
            ]
        },
        ExtractedBoundGenericOrderMismatch {
            fields: {
                expected: String,
                params: Punctuated<GenericParam, Comma>,
            },
            msg: "impl generic parameters are not in the required order",
            tags: [InvalidInput],
            span: { tokens: params },
            help: [
                {format!("order the impl generic parameters as `<{expected}>`")},
            ],
            note: [
                "remove unnessary generics if not required",
            ]
        },

        SelfTypeNodeBoundMismatch {
            fields: {
                expected: Path,
                span: Span,
            },
            msg: "the associated trait in the impl self type does not match the instance node trait",
            tags: [InvalidInput],
            span: { span: *span },
            help: [
                {format!("expected: `{}`", expected.to_token_stream())},
            ],
            note: [
                "use the instance node trait in the qualified self type",
                "the instance node trait is the inline trait bound of the first generic type parameter of the impl"
            ],
        },
        SelfTypeNodeBoundQualifierMismatch {
            fields: {
                expected: Path,
                span: Span,
            },
            msg: "the associated trait in the impl self type's qualifier paths does not match the instance node trait",
            tags: [InvalidInput],
            span: { span: *span },
            help: [
                {format!("expected: `{}`", expected.to_token_stream())},
            ],
            note: [
                "use the exact path-qualified instance node trait in the qualified self type",
                "the instance node trait is the inline trait bound of the first generic type parameter of the impl"
            ],
        },

        InstanceAssocMustUseSelf {
            fields: {
                path: Path,
            },
            msg: "instance self-type associated item must be accessed through `Self`",
            tags: [InvalidInput],
            span: { tokens: path },
            help: [
                "replace this entire associated-item with `Self`",
                "expected: `Self`",
            ],
            note: [
                "the instance associated item defining the impl self type cannot be accessed through its direct raw generic UFCS",
            ],
        },
        InstanceQAssocMustUseSelf {
            fields: {
                t_path: TypePath,
            },
            msg: "instance self-type associated item must be accessed through `Self`",
            tags: [InvalidInput],
            span: { tokens: t_path },
            help: [
                "replace this entire associated-item with `Self`",
                "expected: `Self`",
            ],
            note: [
                "the instance associated item defining the impl self type cannot be accessed through its direct raw generic UFCS",
            ],
        },
        InstanceAssocTraitQualificationRequired {
            fields: {
                path: Path,
                may_be_expected: TypePath,
            },
            msg: "trait qualification is required for this associated item access",
            tags: [InvalidInput],
            span: { tokens: path },
            help: [
                "qualify the associated item with the trait that defines it",
                {format!("is it expected to be: `{}` ?", may_be_expected.to_token_stream())},
            ],
            note: [
                "associated items (other than the one defining the impl self type) cannot be accessed through the bare generic type",
            ],
        },

        InstTraitUsedInImpl {
            fields: {
                seg: PathSegment,
            },
            msg: "the instance trait cannot be used directly in the impl",
            tags: [Unsupported],
            span: { tokens: seg },
            help: [
                "usage of instance trait associated items via `Self::..` is allowed though"
            ],
            note: [
                "the instance trait is reserved for the instance accessor bound only",
                "use other concrete type, aliases instead or use qualifiers to disambiguate",
            ]
        },

        AssociatedItemRequiresQualifier {
            fields: {
                ident: Ident,
            },
            msg: "the associated item requires a path qualifier to disambiguate",
            tags: [InvalidInput],
            span: { tokens: ident },
            help: [
                "use `Self::...` or another appropriate qualifier to disambiguate the associated type",
            ],
            note: [
                "unqualified associated type references are ambiguous in this context",
            ],
        },

        InstanceAssocTypeSelfReferenceNotAllowed {
            fields: {
                path: Path,
            },
            msg: "cannot reference same scope associated types through `Self::...`",
            tags: [Future],
            span: { tokens: path },
            help: [
                "remove the `Self::...` reference or use a concrete or appropriately qualified type instead",
            ],
            note: [
                "associated type declarations cannot self/nested reference associated types \
                declared by the same implementation",
            ],
        },

        InvalidSelfFnArgument {
            fields: {
                self_value: SelfValue,
            },
            msg: "`self` function argument unavailable",
            tags: [Future],
            span: { tokens: self_value },
            help: [
                "use typed argument `arg: Self` instead of receiver short-hand `self`",
                "use an explicit argument instead of the receiver syntax",
            ]
        },

        SelfNotAllowedInConstItem {
            fields: {
                seg: PathSegment,
            },
            msg: "`Self` references are not supported in const items of this impl",
            tags: [Unsupported],
            span: { tokens: seg },
            note: [
                "this is primariliy due to const-generics unstability for instances",
                "use an explicitly qualified type instead",
            ]
        },

        InstanceAssocTypePredicateNotAllowed {
            fields: {
                pred: PredicateType,
            },
            msg: "instance trait associated types cannot be used as bounded types in generic predicates",
            tags: [InvalidInput],
            span: { tokens: pred },
            help: [
                "remove the `Self::...` predicate or use a concrete or appropriately qualified type instead",
            ],
            note: [
                "instance trait associated types cannot be used as the bounded type of generic predicates",
                "make use of its implicitly available bounds already declared in the instance trait"
            ],
        },

        SelfAssocFunctionMustBeCalled {
            fields: {
                seg: PathSegment,
            },
            msg: "instance associated function must be called",
            tags: [Unsupported],
            span: { tokens: seg },
            help: [
                {format!("expected: `{}()`", seg.to_token_stream())},
            ],
            note: [
                "use a function call synatax",
            ]
        },
        SelfAssocFunctionPathRequired {
            fields: {
                expr: Expr,
            },
            msg: "associated function must be accessed through a path",
            tags: [Unsupported],
            span: { tokens: expr },
            note: [
                "use a path such as `Self::call()`",
            ]
        },
        SelfAssocNotAllowedInTraitBound  {
            fields: {
                path: TypePath,
            },
            msg: "`Self::..` path not allowed in instance trait bound",
            tags: [Unsupported],
            span: { tokens: path },
            note: [
                "since it involves trait overflow",
                "whereas standalone `Self` can be utilized",
            ]
        },
        InnerSelfItem {
            fields: {
                item: Item,
            },
            msg: "inner scoped items only allowed which doesn't use `Self`",
            tags: [Unsupported],
            span: { tokens: item },
            help: [
                "either remove `Self` references, else declare it as a associated item outside",
            ],
            note: [
                "`Self` is only allowed on direct associated items",
            ]
        },
    }
);

bug_diagnostics! {
    space: ValidationSpace,
    bug: VALID_BUG.to_string(),

    pub(super) enum ValidBugs {
        TraitImplNotAllowed =>
            "failed to reject a non-inherent impl",
        SelfTypeNotPath =>
            "failed to reject a non-path impl self type",
        QSelfTypeNotGeneric =>
            "failed to reject a qualified self type that is not a generic parameter",
        NestedQSelfNotAllowed =>
            "failed to reject a nested qualified self type",
        GenericParameterHasArguments =>
            "failed to reject generic arguments on a generic parameter",
        QSelfGenericNotDeclared =>
            "failed to reject an undeclared qualified self generic parameter",
        QSelfGenericNotFirst =>
            "failed to enforce that the QSelf generic parameter is the first \
            type generic parameter of the impl",
        MultipleAssociatedSegments =>
            "failed to reject multiple associated type segments",
        AssociatedTypeHasArguments =>
            "failed to reject generic arguments on an associated type",
        AssociatedTypeMissing =>
            "failed to reject a self type without an associated type",
        FirstTypeNotGeneric =>
            "failed to reject a non-generic first path segment",

        SelfNotFirstPathSegment =>
            "failed to reject Self after a previous path segment",
        SelfHasMultipleFollowingSegments =>
            "failed to reject multiple path segments following Self",
        SelfUsedInQualifiedSelfPath =>
            "failed to reject Self inside a qualified self path",
        SelfHasGenericArguments =>
            "failed to reject a generic argumented self key",

        DuplicateImplGenericParameter =>
            "duplicate generic parameter violates the impl generic parameter uniqueness invariant",
        NodeBoundGenericArgNotGenericParam =>
            "instance node trait argument was expected to be a generic parameter",
        NodeBoundGenericArgNotExpectedGeneric =>
            "instance node trait argument references a generic parameter that is not declared by the impl",
        NodeBoundIntermediatePathHasArgs =>
            "instance node trait intermediate path segment unexpectedly has generic arguments",

        ExtractedBoundLifetimeAfterGeneric =>
            "allowed lifetimes after generics in extracted instance node bound",
        ExtractedBoundGenericOrderMismatch =>
            "ordered generic arguments of extracted instance node bound and impl generic params mismatch allowed",

        SelfTypeNodeBoundMismatch =>
            "impl self type does not match the extracted node bound",
        SelfTypeNodeBoundQualifierMismatch =>
            "impl self type does not contain the complete extracted node bound",

        SelfTypeNotValidated =>
            "failed to reject an invalid impl self type",
        InstanceAssocMustUseSelf =>
            "the instance associated item must be accessed through `Self`",
        InstanceQAssocMustUseSelf =>
            "the instance qualified associated item must be accessed through `Self`",
        InstanceAssocTraitQualificationRequired =>
            "the associated item requires explicit trait qualification",

        InstTraitNotRemovedFromImpl =>
            "failed to remove the extracted instance trait from the impl",

        AssociatedItemRequiresQualifier =>
            "allowed an ambiguous associated item without path or `Self::` qualifier",

        InstanceAssocTypeSelfReferenceNotAllowed =>
            "allowed a cross referenced `Self::..` type path in an associated type item",

        InvalidSelfFnArgument =>
            "allowed a `self` receiver argument in an associated fn",

        SelfNotAllowedInConstItem =>
            "`Self` is not allowed in an impl const item",

        InstanceAssocTypePredicateNotRemoved =>
            "failed to reject `Self::...` associated type predicates \
            in the implementation",

        StandaloneSelfNotReplaced =>
            "failed to replace standalone `Self` with the impl self type",

        ItemNotProvidedInFile =>
            "No items was found in the final transformed file.",
        ImplProvidedInFile =>
            "An implementation item (impl) remained in the transformed file.",
        ImplNotProvidedInFile =>
            "An implementation item (impl) is not found in to be transformed file.",
        NotDocItemNotProvided =>
            "A `#[cfg(doc)]` item has no corresponding `#[cfg(not(doc))]` item.",
        DocItemNotProvided =>
            "A `#[cfg(not(doc))]` item has no corresponding `#[cfg(doc)]` item.",
        InvalidItemCfg =>
            "An item is not consistent with either or not cfg target rule",
        SelfFoundAfterTransformation =>
            "Self keyword is found after transformation passes",

        InnerSelfItem =>
            "allowed an inner item which uses Self keyword",
        SelfAssocNotAllowedInTraitBound =>
            "a `Self::..` is allowed in instance trait bound leading to overflow",
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ````````````````````````````` EXTRACTION ERRORS ```````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

diagnostics!(
    space: ExtractionSpace,

    pub(super) enum ExtractErrors {
        QSelfGenericLifetimeBoundNotAllowed {
            fields: {
                segment: Ident,
                lifetime: Lifetime,
            },
            msg: format!("the first type generic parameter `{segment}` cannot yet have a lifetime bound"),
            tags: [Future],
            span: { tokens: lifetime },
            help: [
                {format!("remove the lifetime bound and specify exactly one trait bound on `{segment}`")},
            ],
            note: [
                "the instance node trait is extracted from the only trait bound of the first type generic parameter",
                format!("use where clauses for other post predicate bound on {segment}"),
            ],
        },
        QSelfGenericMultipleBounds {
            fields: {
                segment: Ident,
                bound: TypeParamBound,
            },
            msg: format!("the first type generic parameter `{segment}` has multiple inline bounds"),
            tags: [InvalidInput],
            span: { tokens: bound },
            help: [
                {format!("remove this bound and specify exactly one inline trait bound on `{segment}`")},
            ],
            note: [
                "the instance node trait is extracted from the only trait bound of the first type generic parameter",
                format!("use where clauses for other post predicate bound on {segment}"),
            ],
        },
        QSelfGenericTraitBoundNotFound {
            fields: {
                segment: Ident,
            },
            msg: "the first type generic parameter requires an inline trait bound",
            tags: [InvalidInput],
            span: { tokens: segment },
            help: [
                "add a trait bound directly to the first type generic parameter",
                "for example, use `T: Trait` instead of placing the bound only in the `where` clause",
            ],
            note: [
                "the instance node bound is extracted only from the inline bounds of the first type generic parameter",
            ],
        },
        NodeBoundHrtbNotSupported {
            fields: {
                lt: BoundLifetimes,
            },
            msg: "hrtb bounds are not yet supported for instance node trait bound",
            tags: [Future],
            span: { tokens: lt },
            help: [
                "use trait bound without bounded lifetimes",
            ],
        },
        NodeBoundSizedBoundInvalid {
            fields: {
                bound: TraitBound,
            },
            msg: "optional bounds (`?` modifier) instance node trait bound is not allowed",
            tags: [Future],
            span: { tokens: bound },
            help: [
                "use a valid instance node trait bound",
            ],
            note: [
                "currently rust only supports modifier `?` for `Sized` trait bounds"
            ]
        },

        SelfTraitPredicateWhereClauseMissing {
            fields: {
                generics: Generics,
            },
            msg: "impl requires a `Self: InstanceTrait` where predicate",
            tags: [Unexpected],
            span: { tokens: generics },
            note: [
                "add a where predicate such as `where Self: InstanceTrait`",
            ]
        },
        SelfTraitPredicateMissing {
            fields: {
                where_: WhereClause,
            },
            msg: "impl requires a instance trait bound for `Self` where predicate",
            tags: [Unexpected],
            span: { tokens: where_ },
            note: [
                "add a trait bound where predicate for `Self`",
                "example: `Self: InstanceTrait`"
            ]
        },
        MultipleSelfTraitPredicates {
            fields: {
                pred: PredicateType,
            },
            msg: "only one `Self` where predicate is allowed",
            tags: [Unsupported],
            span: { tokens: pred },
            note: [
                "combine all type bounds of multiple `Self` predicate into a single `Self` predicate",
            ]
        },
        SelfTraitBoundUnknownFirstBound {
            fields: {
                pred: PredicateType,
            },
            msg: "required first bound of `Self` must be an instance trait bound",
            tags: [Unexpected],
            span: { tokens: pred },
            note: [
                "the required form is `Self: InstanceTrait + ...`",
            ]
        },
        SizedInstanceTraitNotAllowed {
            fields: {
                bound: TraitBound,
            },
            msg: "instance trait bound with optional trait modifier (?) is not allowed",
            tags: [Unexpected],
            span: { tokens: bound },
            help: [
                "the required form is `Self: InstanceTrait + ...`",
            ],
            note: [
                "currently rust only supports modifier `?` for `Sized` trait bounds"
            ]
        },
        SelfTraitBoundNotTrait {
            fields: {
                bound: TypeParamBound,
            },
            msg: "the first bound of `Self` must be an instance trait bound, found else",
            tags: [Unexpected],
            span: { tokens: bound },
            note: [
                "the required form is `Self: InstanceTrait + ...`",
            ]
        },
        InstanceTraitRepeated {
            fields: {
                bound: TraitBound,
            },
            msg: "instance trait bound is specified more than once",
            tags: [Unsupported],
            span: { tokens: bound },
            note: [
                "the instance trait must appear only once (at first) in the `Self` where predicate bounds",
                "use aliases to differentiate similar identifier non-instance trait, compiler disambiguation not yet supported"
            ]
        },
        HrtbInstanceTraitNotAllowed {
            fields: {
                lt: BoundLifetimes,
            },
            msg: "invalid hrtb (lifetime bounded) instance trait",
            tags: [Future],
            span: { tokens: lt },
            note: [
                "instance trait currently does not support HRTB instances",
                "remove the `for<'_>` lifetime bound",
            ]
        },
    }
);

bug_diagnostics! {
    space: ExtractionSpace,
    bug: EXTRACT_BUG.to_string(),

    pub(super) enum ExtractBugs {
        QSelfGenericLifetimeBoundNotAllowed =>
            "the impl first type generic parameter has an inline lifetime bound`",
        QSelfGenericMultipleBounds =>
            "the impl first type generic parameter has multiple inline bounds",
        QSelfGenericTraitBoundNotFound =>
            "the impl first type generic parameter has no inline trait bound",
        InstanceNodeExtractionInconsistent =>
            "the impl first type generic parameter instance node trait bound extraction is inconsistent",
        NodeBoundHrtbNotSupported =>
            "the instance node extracted allowed to have hrtb bounds",
        NodeBoundSizedBoundInvalid =>
            "the instance node extracted allowed to be a optional (modifier) bound",

        SelfTraitBoundNotTrait =>
            "extracted instance bound is not a trait bound",
        InstanceTraitNotRemoved =>
            "the instance trait in Self predicate is not removed",
        HrtbInstanceTraitNotAllowed =>
            "hrtb instance trait is not rejected",
        SizedInstanceTraitNotAllowed =>
            "trait modifier included instance trait is not rejected",
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ```````````````````````````` REPLACEMENT ERRORS ```````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

diagnostics!(
    space: ReplacementSpace,

    pub(super) enum ReplaceErrors {
        SelfAssocParenArgsNotSupported {
            fields: {
                paren: ParenthesizedGenericArguments,
            },
            msg: "parenthesized generic arguments are not supported here",
            tags: [InvalidInput],
            span: { tokens: paren },
            help: [
                "use angle bracketed generic arguments instead on associated types",
            ],
        },

        NonLeafFnArgIdentConflict {
            fields: {
                ident: Ident,
            },
            msg: "change the function argument identifier, conflicting internally",
            tags: [Unsupported],
            span: { tokens: ident },
            note: [
                "change the existing function argument identifier",
            ]
        },

        ExpectedIdentNotRawBStr {
            fields: {
                lit: LitByteStr,
            },
            msg: "expected a identifier, not a raw byte string",
            tags: [Unsupported],
            span: { tokens: lit },
            note: [
                "non-leaf arguments expect a identifier as its dynamic access",
                "use prefix, leaf, or parent based arguments if static raw byte string is required"
            ]
        },

        SumAttrOnlyOnType {
            fields: {
                assoc: ImplItem,
            },
            msg: "sum types are only allowed in associated types only, found else",
            tags: [Unsupported],
            span: { tokens: assoc },
        },
        AllTypesAreSum {
            fields: {
                attr: Attribute,
            },
            msg: "for non-leaf (dynamic) instance trait accessor impls all types are sum types only",
            tags: [Unsupported],
            span: { tokens: attr },
            help: [
                "remove this attribute to avoid ambiguity",
            ],
        },

    }
);

bug_diagnostics! {
    space: ReplacementSpace,
    bug: REPLACE_BUG.to_string(),

    pub(super) enum ReplaceBugs {
        ImplGenericsNotAppendedToSelfAssoc =>
            "the impl generics are not appended to a associated item that involves `Self`",
        ImplTyGenericsNotAppendedToSelfAssocSegment =>
            "the impl's type generics are not appended to a `Self::..` associated item's path segment",

        SelfAssocSelfNotStripped =>
            "Self::Assoc (associated impl item) where the Self keyword has not been stripped",

        SumTypeSelfAssocNotReplaced =>
            "associated types marked with #[sum], Self::Assoc type paths not replaced",

        SelfAssocNotReplaced =>
            "Self::Assoc type paths in impl not replaced",

        SelfAssocCallNotReplaced =>
            "unqualified `Self` associated function call was not replaced",

        ReturnTyInferedErrorNotReplaced =>
            "inferred error type in associated fn return type is not replaced",

        RawBStrInputNotRemoved =>
            "raw bstr input in a non-leaf argument is not rejected",

        DynFnArgNotAppended =>
            "dynamic ident fn arg is not appended to a function args",

        DynFnCallArgNotAppended =>
            "dynamic ident fn arg is not appended to a function call args",

        SumAttrNotRemoved =>
            "sum attribute is not removed post-op",

        DocPathNotTypePath =>
            "self assoc path provided for doc target is not a type path with segments",

        AllTypesAreSum =>
            "all non-leaf arguments are sum but ambiguity not removed",

    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// `````````````````````````````` RESOLUTION ERRORS ``````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

diagnostics!(
    space: ResolutionSpace,

    pub(super) enum ResolveErrors {

        IdentsAndIndexesNotSameLen {
            fields: {
                indexes: IntList,
                idents: BStringList,
            },
            msg: format!("according to given counter indexes expected {} idents, found {}", indexes.ints.len().to_string(), idents.bytes.len().to_string()),
            tags: [Unsupported],
            span: { tokens: &idents.bytes },
            note: [
                "for each counter index the corresponding ident must be given",
                "hence both indexes and idents should have same length of elements",
                "example: (.. [b\"ident0\", b\"ident1\"], [0,1] )"
            ]
        },
        ExprPathRequiresQSelf {
            fields: {
                expr_path: ExprPath,
            },
            msg: "expected a qualified path with `Self` type/assoc access",
            tags: [InvalidInput],
            span: { tokens: expr_path },
            help: [
                "expected: `<QSelf as InstanceTrait>::Assoc`, found no QSelf i.e., a qualified type-path",
                "QSelf must be an expression path",
            ],
            note: [
                "the qualified trait must be an instance trait",
                "the qualified `Self` type must be the type implementing or \
                bounds associated with the instance trait"
            ]
        },
        QSelfRequiresExprPath {
            fields: {
                qself: Type,
            },
            msg: "qualified path with `Self` type/assoc access should be an expr-path",
            tags: [InvalidInput],
            span: { tokens: qself },
            help: [
                "in `<QSelf as InstanceTrait>::Assoc`, expected QSelf to be an expression path",
            ],
            note: [
                "the qualified trait must be an instance trait",
                "the qualified `Self` type must be the type implementing or \
                bounds associated with the instance trait"
            ]
        },

        InstanceTraitPathNotFound {
            fields: {
                path: Path,
            },
            msg: "failed to retrieve the instance trait path segment",
            tags: [Unexpected],
            span: { tokens: path },
            help: [
                "expected to retrieve `InstanceTrait` failed in `<QSelf as InstanceTrait>::Assoc`"
            ],
            note: [
                "its assumed to be allocating the instance trait as the final path segment",
            ]
        },
        AssocQSelfLastIdentUnavailable {
            fields: {
                path: Path,
            },
            msg: "failed to retrieve the instance trait implementing associated type",
            tags: [Unexpected],
            span: { tokens: path },
            help: [
                "expected to retrieve `Assoc` failed in `< <QSelf as Trait>::Assoc as InstanceTrait >`"
            ],
            note: [
                "its found that this is the type accessing the instance trait as an associated item",
            ]
        },
        QSelfNotTypePath {
            fields: {
                q_self: Type,
            },
            msg: "expected a type path, found else type variant",
            tags: [InvalidInput],
            span: { tokens: q_self },
            help: [
                "expected to ensure `QSelf` as a type-path failed in `<QSelf as InstanceTrait>::Assoc`"
            ],
            note: [
                "its found that this is the type accessing the instance trait",
                "QSelf could be itself an associated type (qualified in itself)"
            ]
        },

        ExprCallPathNotExprPath {
            fields: {
                expr: Expr,
            },
            msg: "expected a expr path in the instance access function expr-call",
            tags: [InvalidInput],
            span: { tokens: expr },
            help: [
                "expexted to retrieve function as expr-path from `function(args)` but found `{expr}(args)`"
            ],
            note: [
                "the function path must be an expr-path",
            ]
        },

        MaxDynCounters {
            fields: {
                list: BStrInputList,
            },
            msg: "maximum 2 counters are allowed, found more",
            tags: [Future],
            span: { tokens: list },
            note: [
                "use branch arguments for deep counters",
                "reduce explosion by having lesser depths",
                "when depths are deep use static/prefix/parent access over purely dynamic"
            ]
        },

    }
);

bug_diagnostics! {
    space: ResolutionSpace,
    bug: RESOLVE_BUG.to_string(),

    pub(super) enum ResolveBugs {
        IdentsAndIndexesNotSameLen =>
            "counter identifier and counter index collections unexpectedly differ in length \
            while accessing instance trait associated item",
        InstanceTraitPathNotFound =>
            "instance-trait path could not be recovered from the qualified accessor \
            while accessing instance trait associated item",
        AssocQSelfLastIdentUnavailable =>
            "accessor global lookup path unexpectedly has no terminal path segment (sub-global-neighbour)\
            while accessing instance trait implementing associated item",
        ExprPathRequiresQSelf =>
            "qualified instance-trait accessor unexpectedly has no QSelf as type-path \
            while accessing instance trait associated item",
        QSelfRequiresExprPath =>
            "the qualified QSelf of the instance-trait access is not an expr-path",
        ExprCallPathNotExprPath =>
            "the instance non-leaf expr call path function path is not an expr-path",
        QSelfNotTypePath =>
            "the qself path of the accessor resolution path is not a simple type path",

        InstanceNonLeafPathResolutionYieldedNonPath =>
            "instance non-leaf type/expr path resolution yielded an inconsistent result",
        InstanceNonLeafPathResolvedQSelfMissing =>
            "instance non-leaf type/expr path resolution does not have a qself",
        InstanceNonLeafPathResolvedQSelfInvalid =>
            "instance non-leaf type/expr path resolution yielded an non-type path qself",
        InstanceNonLeafPathResolvedInconsistentAccess =>
            "instance non-leaf type/expr path resolution yielded an inconsistent type-access inside ",

        HangingBordersNotAvailable =>
            "during instance assoc call derivation hanging border type aliases not generated",

        IdentCheckerExprsEmpty =>
            "during instance assoc call derivation checker exprs for each dyn ident is found empty",
        OnlyCounterHaveMultiDimensionExprs =>
            "during instance assoc call final exprs are multi-dimension for a single dyn ident",
        MultiCounterHaveLessThanOneDimensionExprs =>
            "during instance assoc call final exprs are one or less dimension for multi dyn idents",
        MultiCounterButIndexesInsufficient =>
            "during instance assoc call indexes insufficient for multi dyn idents",
        TralingIdentCheckersUnnessary =>
            "during instance assoc call extra derivation checkers are found",

        DepthIsZero =>
            "zero depth is accepted for non-leaf resolution",
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ```````````````````````````````` GETTER ERRORS ````````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

diagnostics!(
    space: GetterSpace,

    pub(super) enum GetterErrors {

        RemoveBStrInputs {
            fields: {
                list: BStrInputList,
            },
            msg: "remove these inputs, as its not required",
            tags: [Unsupported],
            span: { tokens: list },
        },

        PathRequiresQSelf {
            fields: {
                expr_path: ExprPath,
            },
            msg: "expected the path to contain qualified self",
            tags: [Unsupported],
            span: { tokens: &expr_path },
            help: [
                "provide the path using qualified self syntax",
                "Example: `<Type as InstanceNode>::Item::Assoc`",
                "Example: `<Type as InstanceNode>::Item::assoc()`",
            ],
            note: [
                "the getter uses the qualified self type to construct the generated getter expression/resolution",
            ]
        },

        PathRequiresQSelfTrait {
            fields: {
                path: Path,
            },
            msg: "expected an explicit trait path before the nested associated item",
            tags: [Unsupported],
            span: { tokens: &path },
            help: [
                "provide an associated item through an explicit trait and its assoc access",
                "Example: `<Type as InstanceNode>::Item::Assoc`",
                "Example: `<Type as InstanceNode>::Item::assoc()`",
            ],
            note: [
                "the trait path is required to obtain the trait generic arguments used by the getter expression/resolution",
            ]
        },

        TraitHasParenArgs {
            fields: {
                args: PathArguments,
            },
            msg: "trait item path does not support parenthesized generic arguments",
            tags: [Unsupported],
            span: { tokens: args },
            help: [
                "use angle-bracketed generic arguments for the trait (instance node) item path",
                "Example: `<Type as InstanceNode<T>>::Item::Assoc`",
                "Example: `<Type as InstanceNode>::Item<T>::assoc()`",
            ],
            note: [
                "parenthesized path arguments are not supported when extracting trait generic arguments for the getter",
            ]
        },

        PathRequiresAssoc {
            fields: {
                path: Path,
            },
            msg: "expected an associated item after the trait-item path",
            tags: [Unsupported],
            span: { tokens: &path },
            help: [
                "provide an associated item after the trait-item path",
                "Example: `<Type as InstanceNode>::Item::Assoc`",
                "Example: `<Type as InstanceNode>::Item::assoc()`",
            ],
            note: [
                "the final path segment identifies the getter associated item",
            ]
        },

        AssocHasParenArgs {
            fields: {
                args: PathArguments,
            },
            msg: "associated item does not support parenthesized generic arguments",
            tags: [Unsupported],
            span: { tokens: args },
            help: [
                "use angle-bracketed generic arguments for the associated item",
                "Example: `<Type as InstanceNode>::Item::Assoc<T>`",
                "Example: `<Type as InstanceNode>::Item::assoc::<T>()`",
            ],
            note: [
                "parenthesized path arguments cannot be merged with the trait and qualified self generic arguments",
            ]
        },
    }
);

bug_diagnostics! {
    space: GetterSpace,
    bug: GETTER_BUG.to_string(),

    pub(super) enum GetterBugs {
        NotRemovedBStrInputs =>
            "unnessary instance dynamic inputs is not removed",
        PathHasQSelf =>
            "the transformed path has a qself type in the expr path",
        PathHasNoAssoc =>
            "the transformed path has no assoc (as last segment) in the expr path",
        PathHasParenArgs =>
            "the transformed path has parenthesized arguments in the last segment of the expr path",
        ArgsNotAppended =>
            "the transformed call hasnot appended dynamic ident expressions to its arguments",
    }
}
