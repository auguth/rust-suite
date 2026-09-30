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

//! Diagnostics emitted by the instance-node proc-macro pipeline.
//!
//! Defines error spaces, structured diagnostics, and internal invariant
//! violations covering subscriber and publisher models, etc.

// ===============================================================================
// ``````````````````````````````````` IMPORTS ```````````````````````````````````
// ===============================================================================

// --- Proc-Suite ---
use proc_suite::{BStringList, IntList, bug_diagnostics, diagnostics, error_spaces};

// --- Local Crate ---
use crate::errors::MAINTAINERS;

// --- Proc-Macro Utils ---
use proc_macro2::Span;
use syn::{
    Expr, GenericArgument, GenericParam, Ident, ImplItemType, Lit, LitInt, Path, PathSegment,
    TraitBound, TraitItemType, Type, TypeParamBound, TypePath, spanned::Spanned,
};

// ===============================================================================
// `````````````````````````````````` CONSTANTS ``````````````````````````````````
// ===============================================================================

pub(super) const NODE_BUG: &'static str =
    "instance node's proc macro phase didn't follow invariants";

pub(super) const POST_BUG: &'static str =
    "instance node's post proc macro phase didn't follow invariants";

pub(super) const SUB_BUG: &'static str =
    "instance subscriber node's proc macro phase didn't follow invariants";

pub(super) const ACCESS_BUG: &'static str =
    "instance node's access traits proc macro phase didn't follow invariants";

pub(super) const KEY_BUG: &'static str =
    "instance node key-value arguments proc macro phase didn't follow invariants";

pub(super) const LEAF_SUB_BUG: &'static str =
    "instance leaf subscriber node's proc macro phase didn't follow invariants";

pub(super) const BRANCH_SUB_BUG: &'static str =
    "instance branch subscriber node's proc macro phase didn't follow invariants";

pub(super) const PRUNE_SUB_BUG: &'static str =
    "instance prune subscriber node's proc macro phase didn't follow invariants";

pub(super) const TRAVERSE_SUB_BUG: &'static str =
    "instance traverse subscriber node's proc macro phase didn't follow invariants";

pub(super) const DESCEND_SUB_BUG: &'static str =
    "instance descend subscriber node's proc macro phase didn't follow invariants";

pub(super) const EXTEND_SUB_BUG: &'static str =
    "instance extend subscriber node's proc macro phase didn't follow invariants";

pub(super) const TRIM_SUB_BUG: &'static str =
    "instance trim subscriber node's proc macro phase didn't follow invariants";

pub(super) const SPREAD_SUB_BUG: &'static str =
    "instance spread subscriber node's proc macro phase didn't follow invariants";

pub(super) const ROOT_SUB_BUG: &'static str =
    "instance root subscriber node's proc macro phase didn't follow invariants";

pub(super) const PUB_BUG: &'static str =
    "instance publisher node's proc macro phase didn't follow invariants";

// ===============================================================================
// ````````````````````````````````` ERROR-SPACES ````````````````````````````````
// ===============================================================================

error_spaces! {
    space: "INSTANCE_NODE",
    maintain: MAINTAINERS,

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // ````````````````````````````````` NODE SPACE ``````````````````````````````````
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

    pub(super) enum NodeSpace {
        range: 0..=100,
        variants: {
            PubNotAllowed,
            PubSubOnSameNode,
            MultiSubNode,
            MultiPubNode,
            DuplicateSubNode,
            DuplicateIndexes,
            NodeNotFound,
            NodeAttrNotFound,
            UnexpectedMetaValueAttr,
        }
    }

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // ``````````````````````````````` POST NODE SPACE ```````````````````````````````
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

    pub(super) enum PostSpace {
        range: 101..=200,
        variants: {
            SubNodeExists,
            PubNodeExists,
        }
    }

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // ``````````````````````````````` NODE STATE SPACE ``````````````````````````````
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

    pub(super) enum StateSpace {
        range: 201..=300,
        variants: {

            TraitItemIdxFetchFailed,
            FetchedTraitItemNotTypeAssoc,
            ImplItemIdxFetchFailed,
            FetchedImplItemNotTypeAssoc,

            FoundNoBoundForInstance,
            RequiresBoundAsInstanceTraitBound,
            ExpectedInstanceTraitBoundInvalid,
            HrtbInstanceTraitsUnavailable,
            InstanceTraitBoundLastSegUnavailable,
            GATInstanceAssocTyUnavailable,

            NonAngleInstanceTraitBound,
            FirstCounterNotFound,
            InvalidFirstCounter,

            DelegateAssocPathSegNotFound,

            RemoveNodeArgs,

            InstanceTraitBoundNotAngleArgs,
            InstanceTraitBoundInstanceTypeNumArgUnavailable,
            InstanceTraitBoundInstanceTypeNumArgNotType,
            GlobalNodeInvalidGenerics,
            GlobalNodeNotFound,
            TwinNodeInvalidGenerics,
            TwinNodeNotFound,
            CheckerNodeInvalidGenerics,
            CheckerNodeNotFound,

            InitialNodeInvalidGenerics,
            InitialNodeNotFound,
            FinalNodeInvalidGenerics,
            FinalNodeNotFound,

            TerminalGlobalNodeInvalidGenerics,
            TerminalGlobalNodeInvalidType,
            TerminalGlobalNodeNotFound,
            TerminalTwinNodeInvalidGenerics,
            TerminalTwinNodeInvalidType,
            TerminalTwinNodeNotFound,
            TerminalCheckerNodeInvalidGenerics,
            TerminalCheckerNodeNotFound,

            ValidateContextFirstOfIndexesUnavailable,
            StateInstanceNodeUnavailable,
            StateInstanceNodeBoundUnavailable,
            StateInstanceNodeReplacementBoundIdentInvalid,
            StateInstanceNodeReplacementBoundLastSegUnavailable,
            StateInstanceNodeReplacementBoundNotAngleArgs,
            StateInstanceOldNodeBoundNotTraitBound,

            TerminalTwinBoundsMoreThanSingleTerminalBound,
            TerminalTwinBoundNotTraitBound,
            NonReplacedInstanceBoundLastPathSegNotFound,


            TerminalGlobalBoundsMoreThanSingleTerminalBound,
            TerminalGlobalBoundNotTraitBound,

            TerminalCheckerDefaultExprNotFound,
            TerminalCheckerDefaultNotExprPath,
            TerminalCheckerDefaultExprPathQSelfNotFound,
            TerminalCheckerDefaultExprPathQSelfInvalid,
            TerminalCheckerDefaultExprPathLastSegNotFound,
            TerminalCheckerDefaultExprPathLastSegInvalid,
            TerminalCheckerDefaultExprPathQSelfTraitBoundInvalid,

            ImplInstanceTyLastSegNotFound,
            ImplTerminalTwinTyNotTypePath,
            ImplTerminalTwinTyQSelfNotFound,
            ImplTerminalTwinTyQSelfInvalid,
            ImplTerminalTwinTyLastSegNotFound,
            ImplTerminalTwinTyLastSegInvalid,
            ImplTerminalTwinTyLastQSelfBoundInvalid,

            ImplTerminalGlobalTyNotTypePath,
            ImplTerminalGlobalTyQSelfNotFound,
            ImplTerminalGlobalTyQSelfInvalid,
            ImplTerminalGlobalTyLastSegNotFound,
            ImplTerminalGlobalTyLastSegInvalid,
            ImplTerminalGlobalTyLastQSelfBoundInvalid,
        }

    }

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // ````````````````````````````` PUBLISHER NODE SPACE ````````````````````````````
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

    pub(super) enum PublisherSpace {
        range: 301..=400,
        variants: {
            ImplOrAssocMayHaveGenerics,
            ExpectedKeys,

            PublisherCheckerConstInvalidIdent,
            PublisherCheckerConstIsGeneric,
            PublisherCheckerConstInvalidExpr,
            PublisherCheckerConstNotFound,

            ImplItemIdxFetchFailed,
            FetchedImplItemNotTypeAssoc,
        }
    }

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // ```````````````````````````` SUBSCRIBER NODE SPACE ````````````````````````````
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

    pub(super) enum SubscriberSpace {
        range: 401..=600,
        variants: {
            KeySpecBStrListFailed,
            DuplicateIdentInstance,
            OneInstanceBoundAllowed,
            IndexIdentsLenMismatch,

            ImplSubscriberQTypePath,
            ImplSubscriberTypePathQSelfNotFound,

            ImplSubCheckerAvailable,
            ImplSubBaseInstanceTypeNotTypePath,
            ImplSubBaseInstanceTypeLastSegmentNotFound,
            ImplSubBaseInstanceTypeQSelfNotFound,
            ImplSubGlobalTyNotTypePath,
            ImplSubGlobalTyQSelfNotFound,
            ImplSubGlobalTyQSelfInvalid,
            ImplSubGlobalTyLastPathSegNotFound,
            ImplSubGlobalTyLastSegInvalid,
            ImplSubGlobalTyQSelfBoundInvalid,

            ImplSubTwinTyNotTypePath,
            ImplSubTwinTyQSelfNotFound,
            ImplSubTwinTyQSelfInvalid,
            ImplSubTwinTyLastSegNotFound,
            ImplSubTwinTyLastSegInvalid,
            ImplSubTwinTyLastQSelfBoundInvalid,

            ImplSubInitialTyNotTypePath,
            ImplSubInitialTyQSelfNotFound,
            ImplSubInitialTyQSelfInvalid,
            ImplSubInitialTyLastSegNotFound,
            ImplSubInitialTyLastSegInvalid,
            ImplSubInitialTyLastQSelfBoundInvalid,

            ImplSubFinalTyNotTypePath,
            ImplSubFinalTyQSelfNotFound,
            ImplSubFinalTyQSelfInvalid,
            ImplSubFinalTyLastSegNotFound,
            ImplSubFinalTyLastSegInvalid,
            ImplSubFinalTyLastQSelfBoundInvalid,
        }
    }

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // ```````````````````````````` LEAF SUBSCRIBER SPACE ````````````````````````````
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

    pub(super) enum LeafSubscriber {
        range: 601..=700,
        variants: {
            ReplacedBoundsAreEmpty,
            ReplacedBoundsUnavailableForEveryIdentList,
            TerminalEquivalenceBoundNotFound,
            TerminalEquivalenceBoundInvalid,

            FirstCounterGenericIndexNotFound,
            ReplacedBoundLastPathSegNotFound,
            ReplacedBoundNotAngleArgs,
            ReplacedBoundTypeNumArgNotFound,
            ReplacedBoundTypeNumArgInvalid,

            GlobalAccessBoundsLenInconsistentWithIdentList,
            GlobalAllBoundsLenInconsistentWithIdentList,
            GlobalCounterAccessBoundFirstSegNotFound,
            GlobalCounterAccessBoundFirstSegNotSupportCrate,
            GlobalCounterAccessBoundLastSegNotFound,
            GlobalCounterAccessBoundNotAngleArgs,
            GlobalCounterAccessBoundLastAngleArgNotFound,
            GlobalCounterAccessBoundLastAngleArgNotTypeGeneric,
            GlobalCounterAccessBoundLastAngleArgInvalid,
            GlobalCounterAccessBoundAngleArgsInconsistentWithIdentList,
            GlobalCounterAccessBoundIdentHashAngleArgNotConstGeneric,
            GlobalCounterAccessBoundIdentHashAngleArgInvalid,
            GlobalHaveNonTraitBound,


        }
    }

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // ``````````````````````````` BRANCH SUBSCRIBER SPACE ```````````````````````````
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

    pub(super) enum BranchSubscriber {
        range: 701..=800,
        variants: {
            ParentNotLeafIdent,
            ParentMustBeOfLesserLen,
            ReplacedBoundsAreEmpty,
            ReplacedBoundsInconsistent,
            ReplacedBoundNeitherOnSetNorBoundary,
            ReplacedArgNeitherInitialNorFinal,
            GlobalAccessBoundsInconsistent,
            ReplacedBoundLastPathSegNotFound,
            TerminalEquivalenceBoundNotFound,
            TerminalEquivalenceBoundInvalid,

            GlobalHaveNonTraitBound,

            InitialBoundsInconsistent,
            InitialBoundNotTraitBound,
            FinalBoundsInconsistent,
            FinalBoundNotTraitBound,
        }
    }

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // ````````````````````````````` ACCESS TRAITS SPACE `````````````````````````````
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

    pub(super) enum AccessSpace {
        range: 801..=900,
        variants: {
            TraitPathNotAngleArgs,
            CountersGenericsIndexesEmpty,
            InvalidFirstCounter,
            AccessTyGenericArgNotFound,
            AccessGenericArgNotType,
            UnexpectedAccessTyGenericArg,
            UnexpectedAccessTraitBound,

            CounterGenericArgNotConstGeneric,
            CounterGenericArgNotConstLit,
            CounterGenericArgNotConstLitInt,

            ExactAccessGenBoundQualifierNotFound,
            ExactAccessGenBoundQualifierNotSupportCrate,
            ExactAccessGenBoundIdentNotFound,
            ExactAccessGenBoundIdentInvalid,
            ExactAccessGenBoundArgsNotAngle,
            ExactAccessGenBoundArgsInconsistent,
            ExactAccessGenBoundArgNotType,
            ExactAccessGenBoundArgNotTypePath,
            ExactAccessGenBoundArgTypeQSelfNotFound,
            ExactAccessGenBoundArgTypeQSelfInvalid,
            ExactAccessGenBoundArgTypePathProjectionNotFound,
            ExactAccessGenBoundArgTypePathProjectionInvalid,
            ExactAccessBoundQualifierNotFound,
            ExactAccessBoundQualifierInvalid,
            ExactAccessGenBoundPathNotFound,
            ExactAccessBoundPathNotFound,
            ExactAccessGenBoundPathInconsistent,
        }
    }

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // ```````````````````````````` PRUNE SUBSCRIBER SPACE ```````````````````````````
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

    pub(super) enum PruneSubscriber {
        range: 901..=1000,
        variants: {
            ReplacedBoundsAreEmpty,
            ReplacedBoundsInconsistent,
            ReplacedBoundNeitherOnSetNorCounter,
            ReplacedArgNeitherInitialNorFinal,
            GlobalAccessBoundsInconsistent,
            ReplacedBoundLastPathSegNotFound,
            TerminalEquivalenceBoundNotFound,
            TerminalEquivalenceBoundInvalid,

            GlobalHaveNonTraitBound,

            InitialBoundsInconsistent,
            InitialBoundNotTraitBound,
            FinalBoundsInconsistent,
            FinalBoundNotTraitBound,
        }
    }

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // ``````````````````````````` EXTEND SUBSCRIBER SPACE ```````````````````````````
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

    pub(super) enum ExtendSubscriber {
        range: 1001..=1100,
        variants: {
            ReplacedBoundsAreEmpty,
            ReplacedBoundsInconsistent,
            ReplacedBoundNeitherCounterNorBoundary,
            ReplacedArgNeitherInitialNorFinal,
            GlobalAccessBoundsInconsistent,
            ReplacedBoundLastPathSegNotFound,
            TerminalEquivalenceBoundNotFound,
            TerminalEquivalenceBoundInvalid,

            GlobalHaveNonTraitBound,

            InitialBoundsInconsistent,
            InitialBoundNotTraitBound,
            FinalBoundsInconsistent,
            FinalBoundNotTraitBound,
        }
    }

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // `````````````````````````` TRAVERSE SUBSCRIBER SPACE ``````````````````````````
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

    pub(super) enum TraverseSubscriber {
        range: 1101..=1200,
        variants: {
            ParentNotDescendIdent,
            ParentMustBeOfLesserLen,

            ReplacedBoundsAreEmpty,
            ReplacedBoundsInconsistent,
            ReplacedBoundNeitherOnSetNorCounter,
            ReplacedArgNeitherInitialNorFinal,
            GlobalAccessBoundsInconsistent,
            ReplacedBoundLastPathSegNotFound,
            TerminalEquivalenceBoundNotFound,
            TerminalEquivalenceBoundInvalid,

            GlobalHaveNonTraitBound,

            InitialBoundsInconsistent,
            InitialBoundNotTraitBound,
            PrefixBoundNotTraitBound,
            FinalBoundsInconsistent,
            FinalBoundNotTraitBound,

        }
    }

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // ```````````````````````````` TRIM SUBSCRIBER SPACE ````````````````````````````
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

    pub(super) enum TrimSubscriber {
        range: 1201..=1300,
        variants: {
            ParentNotPruneIdent,
            ParentMustBeOfLesserLen,

            ReplacedBoundsAreEmpty,
            ReplacedBoundsInconsistent,
            ReplacedBoundNeitherOnSetNorBoundary,
            ReplacedArgNeitherInitialNorFinal,
            GlobalAccessBoundsInconsistent,
            ReplacedBoundLastPathSegNotFound,
            TerminalEquivalenceBoundNotFound,
            TerminalEquivalenceBoundInvalid,

            GlobalHaveNonTraitBound,

            InitialBoundsInconsistent,
            InitialBoundNotTraitBound,
            FinalBoundsInconsistent,
            FinalBoundNotTraitBound,
        }
    }

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // ``````````````````````````` SPREAD SUBSCRIBER SPACE ```````````````````````````
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

    pub(super) enum SpreadSubscriber {
        range: 1301..=1400,
        variants: {
            ParentNotExtendIdent,
            ParentMustBeOfLesserLen,

            ReplacedBoundsAreEmpty,
            ReplacedBoundsInconsistent,
            ReplacedBoundNeitherOnSetNorBoundary,
            ReplacedArgNeitherInitialNorFinal,
            GlobalAccessBoundsInconsistent,
            ReplacedBoundLastPathSegNotFound,
            TerminalEquivalenceBoundNotFound,
            TerminalEquivalenceBoundInvalid,

            GlobalHaveNonTraitBound,

            InitialBoundsInconsistent,
            InitialBoundNotTraitBound,
            FinalBoundsInconsistent,
            FinalBoundNotTraitBound,
        }
    }

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // `````````````````````````` DESCEND SUBSCRIBER SPACE ```````````````````````````
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

    pub(super) enum DescendSubscriber {
        range: 1401..=1500,
        variants: {
            ParentNotTraverseIdent,
            ParentMustBeOfLesserLen,

            ReplacedBoundsAreEmpty,
            ReplacedBoundsInconsistent,
            ReplacedBoundNeitherOnSetNorCounter,
            ReplacedArgNeitherInitialNorFinal,
            GlobalAccessBoundsInconsistent,
            ReplacedBoundLastPathSegNotFound,
            TerminalEquivalenceBoundNotFound,
            TerminalEquivalenceBoundInvalid,

            GlobalHaveNonTraitBound,

            InitialBoundsInconsistent,
            InitialBoundNotTraitBound,
            PrefixBoundNotTraitBound,
            FinalBoundsInconsistent,
            FinalBoundNotTraitBound,
        }
    }

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // ```````````````````````````` ROOT SUBSCRIBER SPACE ````````````````````````````
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

    pub(super) enum RootSubscriber {
        range: 1501..=1600,
        variants: {
            ReplacedBoundsAreEmpty,
            ReplacedBoundsInconsistent,
            ReplacedBoundNeitherOnSetNorBoundary,
            ReplacedArgNeitherInitialNorFinal,
            GlobalAccessBoundsInconsistent,
            ReplacedBoundLastPathSegNotFound,
            TerminalEquivalenceBoundNotFound,
            TerminalEquivalenceBoundInvalid,

            GlobalHaveNonTraitBound,

            InitialBoundsInconsistent,
            InitialBoundNotTraitBound,
            FinalBoundsInconsistent,
            FinalBoundNotTraitBound,
        }
    }

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // ``````````````````````````````` KEYS DIAGNOSTICS ``````````````````````````````
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

    pub(super) enum KeyExpectation {
        range: 1601..=1700,
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
            UnexpectedKeys,
        }
    }
}

// ===============================================================================
// ````````````````````````````````` DIAGNOSTICS `````````````````````````````````
// ===============================================================================

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ````````````````````````````````` NODE SPACE ``````````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

diagnostics!(
    space: NodeSpace,

    pub(super) enum NodeError {

        PubNotAllowed {
            fields: {
                path: Path,
            },
            msg: "instance publisher node incompatible in instance trait node's context",
            tags: [Unsupported],
            span: { tokens: path },
            note: [
                "publisher semantics require type resolution (definition)",
                "trait node declares assoc-types as subscribers (later resolved via publishers)",
            ]
        },

        PubSubOnSameNode {
            fields: {
                ty : ImplItemType,
            },
            msg: "instance publisher and subscriber node conflicting in a single node",
            tags: [Unsupported],
            span: { tokens: ty },
            note: [
                "expected any one node in a impl associated type",
            ]
        },

        MultiSubNode {
            fields: {
                path: Path,
            },
            msg: "multiple instance subscriber node attribute found",
            tags: [Unsupported],
            span: { tokens: path },
            note: [
                "expected one subscriber node per associated type",
            ]
        },
        MultiPubNode {
            fields: {
                path: Path,
            },
            msg: "multiple instance publisher node attribute found",
            tags: [Unsupported],
            span: { tokens: path },
            note: [
                "expected one publisher node per associated type",
            ]
        },

        DuplicateSubNode {
            fields: {
                ident: Ident,
            },
            msg: "duplicate instance subscriber node with conflicting identifier found",
            tags: [Unsupported],
            span: { tokens: ident },
            note: [
                "cannot allow conflicting subscriber node identifiers in proc-macro phase itself",
            ]
        },
        UnexpectedMetaValueAttr {
            fields: {
                span: Span,
            },
            msg: "instance node invalid value attribute found",
            tags: [Unsupported],
            span: { span: span },
            help: [
                "expected a meta-list attribute such as `#[_macro_(content)]`",
            ],
            note: [
                "name value attributes such as `#[_macro_ = \"content\"]` are invalid",
            ]
        },

    }
);

bug_diagnostics! {
    space: NodeSpace,
    bug: NODE_BUG.to_string(),

    pub(super) enum NodeBugs {
        PubNotAllowed =>
            "instance publisher node is allowed in instance trait node context, \
            evaluation didn't followed invariants",

        PubSubOnSameNode =>
            "instance publisher and subscriber node is allowed in single instance node , \
            evaluation didn't followed invariants",

        NodeNotFound =>
            "the expected instance node is not found",

        NodeAttrNotFound =>
            "the expected instance node attribute is not found",


    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ``````````````````````````````` POST NODE SPACE ```````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

bug_diagnostics! {
    space: PostSpace,
    bug: POST_BUG.to_string(),

    pub(super) enum PostBugs {
        SubNodeExists =>
            "instance subscriber node attribute is not post-cleaned",

        PubNodeExists =>
            "instance publisher node attribute is not post-cleaned",
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ``````````````````````````````` NODE STATE SPACE ``````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

diagnostics!(
    space: StateSpace,

    pub(super) enum StateError {


        FoundNoBoundForInstance {
            fields: {
                assoc: TraitItemType,
            },
            msg: "expected an instance trait's bound, found no bounds",
            tags: [Unsupported],
            span: { tokens: assoc },
            help: [
                "bound the associated type with instance trait's bound",
                {format!("example: type {} : __InstanceTrait<..>", &assoc.ident)},
            ],
        },

        RequiresBoundAsInstanceTraitBound {
            fields: {
                assoc: TraitItemType,
                bound: TypeParamBound,
            },
            msg: "expected an instance trait's bound, found an invalid associated type bound",
            tags: [Unsupported],
            span: { tokens: bound },
            help: [
                "bound the associated type's first bound with an instance trait's bound",
                {format!("example: type {} : __InstanceTrait<..>", &assoc.ident)},
            ],
        },
        ExpectedInstanceTraitBoundInvalid {
            fields: {
                assoc: TraitItemType,
                trait_bound: TraitBound,
            },
            msg: "expected an instance trait's bound, found an invalid trait bound",
            tags: [Unsupported],
            span: { tokens: trait_bound },
            help: [
                "bound the associated type's first bound with an instance trait's bound of angle bracketed generics",
                {format!("example: type {} : __InstanceTrait<..>", &assoc.ident)},
            ],
        },

        HrtbInstanceTraitsUnavailable {
            fields: {
                assoc: TraitItemType,
                trait_bound: TraitBound,
            },
            msg: "expected a non hrtb (lifetime bounded) instance trait's bound, found an hrtb bound",
            tags: [Future],
            span: {
                span: match &trait_bound.lifetimes  {
                    Some(lt) => lt.span(),
                    None => Span::call_site(),
                }
            },
            help: [
                "bound the associated type's first bound with a non-hrtb instance trait's bound",
                {format!("example: type {} : __InstanceTrait<..>", &assoc.ident)},
            ],
        },
        InstanceTraitBoundLastSegUnavailable {
            fields: {
                assoc: TraitItemType,
                trait_bound: TraitBound,
            },
            msg: "trait bound's last path segment unavailable, ensure valid a instance trait path",
            tags: [InvalidInput],
            span: { tokens : trait_bound },
            help: [
                "bound the associated type's first bound with a valid identified instance trait's bound",
                {format!("example: type {} : __InstanceTrait<..>", &assoc.ident)},
            ],
        },
        GATInstanceAssocTyUnavailable {
            fields: {
                assoc: TraitItemType,
            },
            msg: "expected a non GAT associated type to carry instance trait's bound, found else",
            tags: [Future],
            span: {
                span: match &assoc.generics.params.first()  {
                    Some(p) => p.span(),
                    None => Span::call_site(),
                }
            },
            help: [
                "associated type with a instance trait's bound should not have generic parameters",
                {format!("example: type {} : __InstanceTrait<..>", &assoc.ident)},
            ],
        },
        NonAngleInstanceTraitBound {
            fields: {
                segment: PathSegment,
            },
            msg: "expected an angle bracketed instance trait bound, found else",
            tags: [Unsupported],
            span: { tokens: segment },
            help: [
                "associated type with a instance trait's bound should not have bracketed generic parameters",
            ],
        },
        InvalidFirstCounter {
            fields: {
                lit: LitInt,
                index: usize,
                len: usize,
            },
            msg: "first counter index creates out of bounds",
            tags: [Unsupported],
            span: { tokens: lit },
            help: [
                {format!("try providing correct counter generic of index less than {}", len)},
            ],
            note: [
                format!("current counter generic index is {}, expected less than (<) {}", index, len),
                "when removing the existing counter constant generic args, \
                the first counter must be inside available slots"
            ]
        },

        DelegateAssocPathSegNotFound {
            fields: {
                ty: TypePath,
            },
            msg: "expected a fully qualified associated type's type-path",
            tags: [Unsupported],
            span: { tokens: ty },
            help: [
                "example: <T as Trait<..>>::Assoc",

            ],
            note: [
                "type-path found but the qualifier `Assoc` identfier segment in <Self as Trait>::Assoc is not found",
            ]
        },
        RemoveNodeArgs {
            fields: {
                span: Span,
            },
            msg: "arguments to this instance node is not required",
            tags: [Unsupported],
            span: { span: span },
            help: [
                "remove the arguments to avoid ambiguity",
            ],

        },
    }
);

bug_diagnostics! {
    space: StateSpace,
    bug: NODE_BUG.to_string(),

    pub(super) enum StateBugs {

        FetchedImplItemNotTypeAssoc =>
            "instance node's item fetched via index is not a impl associated type",

        TraitItemIdxFetchFailed =>
            "instance node's item index invalid to fetch via trait node",
        ImplItemIdxFetchFailed =>
            "instance node's item index invalid to fetch via impl node",
        FetchedTraitItemNotTypeAssoc =>
            "instance node's item fetched via index is not a trait associated type",

        FirstCounterNotFound =>
            "instance trait bound's counter generic indexes are not provided to get atleast the first counter",

        FoundNoBoundForInstance =>
            "instance node doesn't posess the instance bound/s",
        RequiresBoundAsInstanceTraitBound =>
            "instance node's instance bound is not a trait bound",
        ExpectedInstanceTraitBoundInvalid =>
            "the given instance node's instance bound is invalid",
        HrtbInstanceTraitsUnavailable =>
            "instance not yet supports hrtb instance trait bounds",
        InstanceTraitBoundLastSegUnavailable =>
            "instance node's instance bound's last segment of path unavailable",
        InstanceTraitBoundNotAngleArgs =>
            "instance node's instance bound does not have angle bracketed generic args",
        InstanceTraitBoundInstanceTypeNumArgUnavailable =>
            "instance node's instance bound's instance type-num generic argument unavailable",
        InstanceTraitBoundInstanceTypeNumArgNotType =>
            "instance node's instance bound's instance type-num generic argument not type generic",
        GlobalNodeInvalidGenerics =>
            "instance node's neighbour global node's generics are invalid",
        GlobalNodeNotFound =>
            "instance node's neighbour global node is unavailable",
        TwinNodeInvalidGenerics =>
            "instance node's neighbour twin node's generics are invalid",
        TwinNodeNotFound =>
            "instance node's neighbour twin node is unavailable",
        CheckerNodeInvalidGenerics =>
            "instance node's neighbour checker node's generics are invalid",
        CheckerNodeNotFound =>
            "instance node's neighbour checker node is unavailable",
        InitialNodeInvalidGenerics =>
            "instance node's neighbour initial node's generics are invalid",
        InitialNodeNotFound =>
            "instance node's neighbour initial node is unavailable",
        FinalNodeInvalidGenerics =>
            "instance node's neighbour final node's generics are invalid",
        FinalNodeNotFound =>
            "instance node's neighbour final node is unavailable",

        TerminalGlobalNodeInvalidGenerics =>
            "instance node's terminal global node's generics are invalid",
        TerminalGlobalNodeInvalidType =>
            "instance node's terminal global node's resolved type is invalid",
        TerminalGlobalNodeNotFound =>
            "instance node's terminal global node is unavailable",
        TerminalTwinNodeInvalidGenerics =>
            "instance node's terminal twin node's generics are invalid",
        TerminalTwinNodeInvalidType =>
            "instance node's terminal twin node's resolved type is invalid",
        TerminalTwinNodeNotFound =>
            "instance node's terminal twin node is unavailable",
        TerminalCheckerNodeInvalidGenerics =>
            "instance node's terminal checker node's generics are invalid",
        TerminalCheckerNodeNotFound =>
            "instance node's terminal checker node is unavailable",

        ValidateContextFirstOfIndexesUnavailable =>
            "instance node's validation phase given context of indexes \
            doesn't contain first counter generic index",
        StateInstanceNodeUnavailable =>
            "instance node is not found",
        StateInstanceNodeBoundUnavailable =>
            "instance node's instance bound (first-instance) is not found",
        StateInstanceNodeReplacementBoundIdentInvalid =>
            "instance node replacements instance bound's ident is invalid \
            as all replacement bounds shall be of the same instance trait bound ident",
        StateInstanceNodeReplacementBoundLastSegUnavailable =>
            "instance node replacement instance bounds doesn't have last \
            segment of its path",
        StateInstanceNodeReplacementBoundNotAngleArgs =>
            "instance node replacement instance bounds are not of angle
            bracketed generic arguments",
        StateInstanceOldNodeBoundNotTraitBound =>
            "instance node old bound (going to be replaced) is not a trait bound",

        TerminalTwinBoundsMoreThanSingleTerminalBound =>
            "instance terminal twin node contains more than one terminal bound",
        TerminalTwinBoundNotTraitBound =>
            "instance terminal twin bound is not a trait bound",
        NonReplacedInstanceBoundLastPathSegNotFound =>
            "instance terminal instance bound last path segment not found",

        TerminalGlobalBoundsMoreThanSingleTerminalBound =>
            "instance terminal global node contains more than one terminal bound",
        TerminalGlobalBoundNotTraitBound =>
            "instance terminal global bound is not a trait bound",

        TerminalCheckerDefaultExprNotFound =>
            "instance checker default expression not found",
        TerminalCheckerDefaultNotExprPath =>
            "instance checker default expression is not a path expression",
        TerminalCheckerDefaultExprPathQSelfNotFound =>
            "instance checker default path expression qself not found",
        TerminalCheckerDefaultExprPathQSelfInvalid =>
            "instance checker default path expression qself is invalid",
        TerminalCheckerDefaultExprPathLastSegNotFound =>
            "instance checker default path expression last path segment not found",
        TerminalCheckerDefaultExprPathLastSegInvalid =>
            "instance checker default path expression last path segment is invalid",
        TerminalCheckerDefaultExprPathQSelfTraitBoundInvalid =>
            "instance checker default path expression trait bound is invalid",

        ImplInstanceTyLastSegNotFound =>
            "instance impl type path node last segment not found",
        ImplTerminalTwinTyNotTypePath =>
            "instance impl node terminal twin associated type is not a type path",
        ImplTerminalTwinTyQSelfNotFound =>
            "instance impl node terminal twin associated type qself not found",
        ImplTerminalTwinTyQSelfInvalid =>
            "instance impl node terminal twin associated type qself is invalid",
        ImplTerminalTwinTyLastSegNotFound =>
            "instance impl node terminal twin associated type last path segment not found",
        ImplTerminalTwinTyLastSegInvalid =>
            "instance impl node terminal twin associated type last path segment is invalid",
        ImplTerminalTwinTyLastQSelfBoundInvalid =>
            "instance impl node terminal twin associated type qself path is invalid",

        ImplTerminalGlobalTyNotTypePath =>
            "instance impl node terminal global associated type is not a type path",
        ImplTerminalGlobalTyQSelfNotFound =>
            "instance impl node terminal global associated type qself not found",
        ImplTerminalGlobalTyQSelfInvalid =>
            "instance impl node terminal global associated type qself is invalid",
        ImplTerminalGlobalTyLastSegNotFound =>
            "instance impl node terminal global associated type last path segment not found",
        ImplTerminalGlobalTyLastSegInvalid =>
            "instance impl node terminal global associated type last path segment is invalid",
        ImplTerminalGlobalTyLastQSelfBoundInvalid =>
            "instance impl node terminal global associated type qself path is invalid",
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ```````````````````````````` SUBSCRIBER NODE SPACE ````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

diagnostics!(
    space: SubscriberSpace,

    pub(super) enum SubscriberError {
        DuplicateIdentInstance {
            fields: {
                idents: BStringList,
            },
            msg: "the identifiers are found to be duplicate of a previous identifiers list group",
            tags: [Unsupported],
            span: { tokens: &idents.bytes },
            help: [
                "remove this identifier list as it already exists",
            ],
            note: [
                "duplicate identifier list creates ambiguity",
            ]
        },
        OneInstanceBoundAllowed {
            fields: {
                bound: TraitBound,
            },
            msg: "only one instance trait bound is allowed",
            tags: [Unsupported],
            span: { tokens: bound },
            help: [
                "remove this bound and resolve multiple subscriber instances via its node arguments",
            ],
            note: [
                "an instance subscriber node will only allow one trait instance bound"
            ],

        },
        IndexIdentsLenMismatch {
            fields: {
                idents: BStringList,
                exp_len: usize,
            },
            msg: format!("counter identifiers expected len is {}, but found length of {}", exp_len, idents.bytes.len()),
            tags: [Unsupported],
            span: { tokens: &idents.bytes },
            note: [
                "given counter generic indexes and idents length must be same, mapped to indexes",
                "ensure if the given indexes if idents are given correctly"
            ]
        },

        ImplSubscriberQTypePath {
            fields: {
                ty: Type,
            },
            msg: "expected a fully qualified associated type's type-path",
            tags: [Unsupported],
            span: { tokens: ty },
            help: [
                "example: <T as Trait<..>>::Assoc",
            ],
        },
        ImplSubscriberTypePathQSelfNotFound  {
            fields: {
                ty: TypePath,
            },
            msg: "expected a fully qualified associated type's type-path",
            tags: [Unsupported],
            span: { tokens: ty },
            help: [
                "example: <T as Trait<..>>::Assoc",

            ],
            note: [
                "type-path found but the qualifier `Self` in <Self as Trait>::Assoc is not found",
            ]
        },

    }
);

bug_diagnostics! {
    space: SubscriberSpace,
    bug: SUB_BUG.to_string(),

    pub(super) enum SubscriberBugs {
        KeySpecBStrListFailed =>
            "keys-spec parsing failed to fetch byte string list for instance subscriber node",

        ImplSubCheckerAvailable =>
            "impl subscriber checker node is still present",
        ImplSubBaseInstanceTypeNotTypePath =>
            "impl subscriber base associated type is not a type path",
        ImplSubBaseInstanceTypeLastSegmentNotFound =>
            "impl subscriber base associated type last path segment not found",
        ImplSubBaseInstanceTypeQSelfNotFound =>
            "impl subscriber base associated type qself not found",
        ImplSubGlobalTyNotTypePath =>
            "impl subscriber global associated type is not a type path",
        ImplSubGlobalTyQSelfNotFound =>
            "impl subscriber global associated type qself not found",
        ImplSubGlobalTyQSelfInvalid =>
            "impl subscriber global associated type qself is invalid",
        ImplSubGlobalTyLastPathSegNotFound =>
            "impl subscriber global associated type last path segment not found",
        ImplSubGlobalTyLastSegInvalid =>
            "impl subscriber global associated type last path segment is invalid",
        ImplSubGlobalTyQSelfBoundInvalid =>
            "impl subscriber global associated type qself path is invalid",

        ImplSubTwinTyNotTypePath =>
            "impl subscriber twin associated type is not a type path",
        ImplSubTwinTyQSelfNotFound =>
            "impl subscriber twin associated type qself not found",
        ImplSubTwinTyQSelfInvalid =>
            "impl subscriber twin associated type qself is invalid",
        ImplSubTwinTyLastSegNotFound =>
            "impl subscriber twin associated type last path segment not found",
        ImplSubTwinTyLastSegInvalid =>
            "impl subscriber twin associated type last path segment is invalid",
        ImplSubTwinTyLastQSelfBoundInvalid =>
            "impl subscriber twin associated type qself path is invalid",

        ImplSubInitialTyNotTypePath =>
            "impl subscriber initial associated type is not a type path",
        ImplSubInitialTyQSelfNotFound =>
            "impl subscriber initial associated type qself not found",
        ImplSubInitialTyQSelfInvalid =>
            "impl subscriber initial associated type qself is invalid",
        ImplSubInitialTyLastSegNotFound =>
            "impl subscriber initial associated type last path segment not found",
        ImplSubInitialTyLastSegInvalid =>
            "impl subscriber initial associated type last path segment is invalid",
        ImplSubInitialTyLastQSelfBoundInvalid =>
            "impl subscriber initial associated type qself path is invalid",

        ImplSubFinalTyNotTypePath =>
            "impl subscriber final associated type is not a type path",
        ImplSubFinalTyQSelfNotFound =>
            "impl subscriber final associated type qself not found",
        ImplSubFinalTyQSelfInvalid =>
            "impl subscriber final associated type qself is invalid",
        ImplSubFinalTyLastSegNotFound =>
            "impl subscriber final associated type last path segment not found",
        ImplSubFinalTyLastSegInvalid =>
            "impl subscriber final associated type last path segment is invalid",
        ImplSubFinalTyLastQSelfBoundInvalid =>
            "impl subscriber final associated type qself path is invalid",
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ````````````````````````````` PUBLISHER NODE SPACE ````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

diagnostics!(
    space: PublisherSpace,

    pub(super) enum PublisherError {

        ImplOrAssocMayHaveGenerics {
            fields: {
                generic: GenericParam,
            },
            msg: "instance publisher node expects no generic parameter (fully resolved)",
            tags: [Future],
            span: { tokens: generic },
            note: [
                "surrounding impl and the instance publisher node assoc type should not declare generic params",
            ]
        },
        ExpectedKeys {
            fields: {
                span: Span,
            },
            msg: "expected any one of the following equivalent subscriber keys",
            tags: [Unsupported],
            span: { span: span },
            help: [
                "leaf | branch | prune | trim | extend | spread | traverse | descend | root",
            ],
            note: [
                "should be equivalent to the trait side declaration",
            ]
        },
    }
);

bug_diagnostics! {
    space: PublisherSpace,
    bug: PUB_BUG.to_string(),

    pub(super) enum PublisherBugs {

        PublisherCheckerConstInvalidIdent =>
            "instance publisher node's instance cumulated checker assertion constant has invalid ident",
        PublisherCheckerConstIsGeneric =>
            "instance publisher node's instance cumulated checker assertion constant has generics",
        PublisherCheckerConstInvalidExpr =>
            "instance publisher node's instance cumulated checker assertion constant has invalid expression",
        PublisherCheckerConstNotFound =>
            "instance publisher node's instance cumulated checker assertion constant not found",

        ImplItemIdxFetchFailed =>
            "instance publisher node's item index invalid to fetch via impl node",
        FetchedImplItemNotTypeAssoc =>
            "instance publisher node's item fetched via index is not a trait associated type",
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ```````````````````````````` LEAF SUBSCRIBER SPACE ````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

bug_diagnostics! {
    space: LeafSubscriber,
    bug: LEAF_SUB_BUG.to_string(),

    pub(super) enum ResolveBugs {
        ReplacedBoundsAreEmpty =>
            "leaf subscriber replaced instance bounds are empty",
        ReplacedBoundsUnavailableForEveryIdentList =>
            "leaf subscriber replaced instance bounds do not match every identifier list",
        TerminalEquivalenceBoundNotFound =>
            "leaf subscriber replaced instance bound doesn't include terminal instance equivalence bound",
        TerminalEquivalenceBoundInvalid =>
            "leaf subscriber replaced instance bound's terminal instance equivalence bound is invalid",

        FirstCounterGenericIndexNotFound =>
            "leaf subscriber first counter generic index not found",
        ReplacedBoundLastPathSegNotFound =>
            "leaf subscriber replaced bound last path segment not found",
        ReplacedBoundNotAngleArgs =>
            "leaf subscriber replaced bound does not use angle bracketed generic arguments",
        ReplacedBoundTypeNumArgNotFound =>
            "leaf subscriber replaced bound typenum generic argument not found",
        ReplacedBoundTypeNumArgInvalid =>
            "leaf subscriber replaced bound typenum generic argument is invalid",

        GlobalAccessBoundsLenInconsistentWithIdentList =>
            "leaf subscriber global counter access bound count does not match the identifier list count",
        GlobalAllBoundsLenInconsistentWithIdentList =>
            "leaf subscriber global bound count does not match the expected counter access bounds count",
        GlobalCounterAccessBoundFirstSegNotFound =>
            "leaf subscriber global counter access bound first path segment not found",
        GlobalCounterAccessBoundFirstSegNotSupportCrate =>
            "leaf subscriber global counter access bound does not begin with the support crate path",
        GlobalCounterAccessBoundLastSegNotFound =>
            "leaf subscriber global counter access bound last path segment not found",
        GlobalCounterAccessBoundNotAngleArgs =>
            "leaf subscriber global counter access bound does not use angle bracketed generic arguments",
        GlobalCounterAccessBoundLastAngleArgNotFound =>
            "leaf subscriber global counter access bound last generic argument not found",
        GlobalCounterAccessBoundLastAngleArgNotTypeGeneric =>
            "leaf subscriber global counter access bound last generic argument is not a type",
        GlobalCounterAccessBoundLastAngleArgInvalid =>
            "leaf subscriber global counter access bound last generic argument is invalid",
        GlobalCounterAccessBoundAngleArgsInconsistentWithIdentList =>
            "leaf subscriber global counter access bound generic arguments do not match the identifier list",
        GlobalCounterAccessBoundIdentHashAngleArgNotConstGeneric =>
            "leaf subscriber global counter access bound identifier hash generic argument is not a const",
        GlobalCounterAccessBoundIdentHashAngleArgInvalid =>
            "leaf subscriber global counter access bound identifier hash generic argument is invalid",
        GlobalHaveNonTraitBound =>
            "leaf subscriber global bound is not trait bound",
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ``````````````````````````` BRANCH SUBSCRIBER SPACE ```````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

diagnostics!(
    space: BranchSubscriber,

    pub(super) enum BranchError {

        ParentNotLeafIdent {
            fields: {
                idents: BStringList,
                less_than_len: usize,
            },
            msg: format!("counter identifiers expected len is less than {}, but found length of {}, an exact mapping of given indexes", less_than_len, idents.bytes.len()),
            tags: [Unsupported],
            span: { tokens: &idents.bytes },
            note: [
                "branch instance subscriber expects parent identifiers, not exact",
                "for exact instances use leaf subscriber instead"
            ]
        },

        ParentMustBeOfLesserLen {
            fields: {
                idents: BStringList,
                less_than_len: usize,
            },
            msg: format!("counter identifiers expected len is less than {}, but found length of {}", less_than_len, idents.bytes.len()),
            tags: [Unsupported],
            span: { tokens: &idents.bytes },
            note: [
                "branch instance subscriber expects parent identifiers",
                "parent idents length should be less than indexes length"
            ]
        },

    }
);

bug_diagnostics! {
    space: BranchSubscriber,
    bug: BRANCH_SUB_BUG.to_string(),

    pub(super) enum BranchBugs {
        ReplacedBoundsAreEmpty =>
            "branch subscriber replaced instance bounds are empty",
        ReplacedBoundsInconsistent =>
            "branch subscriber replaced instance bounds are invalid",
        ReplacedBoundNeitherOnSetNorBoundary =>
            "branch subscriber global bound is neither onset nor a boundary bound",
        ReplacedArgNeitherInitialNorFinal =>
            "branch subscriber replaced instance bound doesn't contain neither initial node or final node as generic argument",
        ReplacedBoundLastPathSegNotFound =>
            "branch subscriber replaced bound last path segment not found",

        TerminalEquivalenceBoundNotFound =>
            "branch subscriber replaced instance bound doesn't include terminal instance equivalence bound",
        TerminalEquivalenceBoundInvalid =>
            "branch subscriber replaced instance bound's terminal instance equivalence bound is invalid",

        GlobalAccessBoundsInconsistent =>
            "branch subscriber global bounds are invalid",
        GlobalHaveNonTraitBound =>
            "branch subscriber global bound is not trait bound",

        InitialBoundsInconsistent =>
            "branch subscriber initial neighbour node bounds are invalid",
        InitialBoundNotTraitBound =>
            "branch subscriber initial neighbour node's bound is not trait bound",
        FinalBoundsInconsistent =>
            "branch subscriber final neighbour node bounds are invalid",
        FinalBoundNotTraitBound =>
            "branch subscriber final neighbour node's bound is not trait bound",
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ```````````````````````````` PRUNE SUBSCRIBER SPACE ```````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

bug_diagnostics! {
    space: PruneSubscriber,
    bug: PRUNE_SUB_BUG.to_string(),

    pub(super) enum PruneBugs {
        ReplacedBoundsAreEmpty =>
            "prune subscriber replaced instance bounds are empty",
        ReplacedBoundsInconsistent =>
            "prune subscriber replaced instance bounds are invalid",
        ReplacedBoundNeitherOnSetNorCounter =>
            "prune subscriber global bound is neither onset nor counter access bound",
        ReplacedArgNeitherInitialNorFinal =>
            "prune subscriber replaced instance bound doesn't contain neither initial node or final node as generic argument",
        ReplacedBoundLastPathSegNotFound =>
            "prune subscriber replaced bound last path segment not found",

        TerminalEquivalenceBoundNotFound =>
            "prune subscriber replaced instance bound doesn't include terminal instance equivalence bound",
        TerminalEquivalenceBoundInvalid =>
            "prune subscriber replaced instance bound's terminal instance equivalence bound is invalid",

        GlobalAccessBoundsInconsistent =>
            "prune subscriber global bounds are invalid",
        GlobalHaveNonTraitBound =>
            "prune subscriber global bound is not trait bound",

        InitialBoundsInconsistent =>
            "prune subscriber initial neighbour node bounds are invalid",
        InitialBoundNotTraitBound =>
            "prune subscriber initial neighbour node's bound is not trait bound",
        FinalBoundsInconsistent =>
            "prune subscriber final neighbour node bounds are invalid",
        FinalBoundNotTraitBound =>
            "prune subscriber final neighbour node's bound is not trait bound",
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ```````````````````````````` TRIM SUBSCRIBER SPACE ````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

diagnostics!(
    space: TrimSubscriber,

    pub(super) enum TrimError {

        ParentNotPruneIdent {
            fields: {
                idents: BStringList,
                less_than_len: usize,
            },
            msg: format!("counter identifiers expected len is less than {}, but found length of {}, an exact mapping of given indexes", less_than_len, idents.bytes.len()),
            tags: [Unsupported],
            span: { tokens: &idents.bytes },
            note: [
                "trim instance subscriber expects parent identifiers, not prune",
                "for exact instances use prune subscriber instead"
            ]
        },

        ParentMustBeOfLesserLen {
            fields: {
                idents: BStringList,
                less_than_len: usize,
            },
            msg: format!("counter identifiers expected len is less than {}, but found length of {}", less_than_len, idents.bytes.len()),
            tags: [Unsupported],
            span: { tokens: &idents.bytes },
            note: [
                "trim instance subscriber expects parent identifiers",
                "parent idents length should be less than indexes length"
            ]
        },

    }
);

bug_diagnostics! {
    space: TrimSubscriber,
    bug: TRIM_SUB_BUG.to_string(),

    pub(super) enum TrimBugs {
        ReplacedBoundsAreEmpty =>
            "trim subscriber replaced instance bounds are empty",
        ReplacedBoundsInconsistent =>
            "trim subscriber replaced instance bounds are invalid",
        ReplacedBoundNeitherOnSetNorBoundary =>
            "trim subscriber global bound is neither onset nor counter access bound",
        ReplacedArgNeitherInitialNorFinal =>
            "trim subscriber replaced instance bound doesn't contain neither initial node or final node as generic argument",
        ReplacedBoundLastPathSegNotFound =>
            "trim subscriber replaced bound last path segment not found",

        TerminalEquivalenceBoundNotFound =>
            "trim subscriber replaced instance bound doesn't include terminal instance equivalence bound",
        TerminalEquivalenceBoundInvalid =>
            "trim subscriber replaced instance bound's terminal instance equivalence bound is invalid",

        GlobalAccessBoundsInconsistent =>
            "trim subscriber global bounds are invalid",
        GlobalHaveNonTraitBound =>
            "trim subscriber global bound is not trait bound",

        InitialBoundsInconsistent =>
            "trim subscriber initial neighbour node bounds are invalid",
        InitialBoundNotTraitBound =>
            "trim subscriber initial neighbour node's bound is not trait bound",
        FinalBoundsInconsistent =>
            "trim subscriber final neighbour node bounds are invalid",
        FinalBoundNotTraitBound =>
            "trim subscriber final neighbour node's bound is not trait bound",
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ``````````````````````````` EXTEND SUBSCRIBER SPACE ```````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

bug_diagnostics! {
    space: ExtendSubscriber,
    bug: EXTEND_SUB_BUG.to_string(),

    pub(super) enum ExtendBugs {
        ReplacedBoundsAreEmpty =>
            "extend subscriber replaced instance bounds are empty",
        ReplacedBoundsInconsistent =>
            "extend subscriber replaced instance bounds are invalid",
        ReplacedBoundNeitherCounterNorBoundary =>
            "extend subscriber global bound is neither boundary nor counter access bound",
        ReplacedArgNeitherInitialNorFinal =>
            "extend subscriber replaced instance bound doesn't contain neither initial node or final node as generic argument",
        ReplacedBoundLastPathSegNotFound =>
            "extend subscriber replaced bound last path segment not found",

        TerminalEquivalenceBoundNotFound =>
            "extend subscriber replaced instance bound doesn't include terminal instance equivalence bound",
        TerminalEquivalenceBoundInvalid =>
            "extend subscriber replaced instance bound's terminal instance equivalence bound is invalid",

        GlobalAccessBoundsInconsistent =>
            "extend subscriber global bounds are invalid",
        GlobalHaveNonTraitBound =>
            "extend subscriber global bound is not trait bound",

        InitialBoundsInconsistent =>
            "extend subscriber initial neighbour node bounds are invalid",
        InitialBoundNotTraitBound =>
            "extend subscriber initial neighbour node's bound is not trait bound",
        FinalBoundsInconsistent =>
            "extend subscriber final neighbour node bounds are invalid",
        FinalBoundNotTraitBound =>
            "extend subscriber final neighbour node's bound is not trait bound",
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ``````````````````````````` SPREAD SUBSCRIBER SPACE ```````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

diagnostics!(
    space: SpreadSubscriber,

    pub(super) enum SpreadError {

        ParentNotExtendIdent {
            fields: {
                idents: BStringList,
                less_than_len: usize,
            },
            msg: format!("counter identifiers expected len is less than {}, but found length of {}, an exact mapping of given indexes", less_than_len, idents.bytes.len()),
            tags: [Unsupported],
            span: { tokens: &idents.bytes },
            note: [
                "spread instance subscriber expects parent identifiers, not extend",
                "for exact instances use extend subscriber instead"
            ]
        },

        ParentMustBeOfLesserLen {
            fields: {
                idents: BStringList,
                less_than_len: usize,
            },
            msg: format!("counter identifiers expected len is less than {}, but found length of {}", less_than_len, idents.bytes.len()),
            tags: [Unsupported],
            span: { tokens: &idents.bytes },
            note: [
                "spread instance subscriber expects parent identifiers",
                "parent idents length should be less than indexes length"
            ]
        },

    }
);

bug_diagnostics! {
    space: SpreadSubscriber,
    bug: SPREAD_SUB_BUG.to_string(),

    pub(super) enum SpreadBugs {
        ReplacedBoundsAreEmpty =>
            "spread subscriber replaced instance bounds are empty",
        ReplacedBoundsInconsistent =>
            "spread subscriber replaced instance bounds are invalid",
        ReplacedBoundNeitherOnSetNorBoundary =>
            "spread subscriber global bound is neither boundary nor onset access bound",
        ReplacedArgNeitherInitialNorFinal =>
            "spread subscriber replaced instance bound doesn't contain neither initial node or final node as generic argument",
        ReplacedBoundLastPathSegNotFound =>
            "spread subscriber replaced bound last path segment not found",

        TerminalEquivalenceBoundNotFound =>
            "spread subscriber replaced instance bound doesn't include terminal instance equivalence bound",
        TerminalEquivalenceBoundInvalid =>
            "spread subscriber replaced instance bound's terminal instance equivalence bound is invalid",

        GlobalAccessBoundsInconsistent =>
            "spread subscriber global bounds are invalid",
        GlobalHaveNonTraitBound =>
            "spread subscriber global bound is not trait bound",

        InitialBoundsInconsistent =>
            "spread subscriber initial neighbour node bounds are invalid",
        InitialBoundNotTraitBound =>
            "spread subscriber initial neighbour node's bound is not trait bound",
        FinalBoundsInconsistent =>
            "spread subscriber final neighbour node bounds are invalid",
        FinalBoundNotTraitBound =>
            "spread subscriber final neighbour node's bound is not trait bound",
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ```````````````````````````` ROOT SUBSCRIBER SPACE ````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

bug_diagnostics! {
    space: RootSubscriber,
    bug: ROOT_SUB_BUG.to_string(),

    pub(super) enum RootBugs {
        ReplacedBoundsAreEmpty =>
            "root subscriber replaced instance bounds are empty",
        ReplacedBoundsInconsistent =>
            "root subscriber replaced instance bounds are invalid",
        ReplacedBoundNeitherOnSetNorBoundary =>
            "root subscriber global bound is neither boundary nor onset access bound",
        ReplacedArgNeitherInitialNorFinal =>
            "root subscriber replaced instance bound doesn't contain neither initial node or final node as generic argument",
        ReplacedBoundLastPathSegNotFound =>
            "root subscriber replaced bound last path segment not found",

        TerminalEquivalenceBoundNotFound =>
            "root subscriber replaced instance bound doesn't include terminal instance equivalence bound",
        TerminalEquivalenceBoundInvalid =>
            "root subscriber replaced instance bound's terminal instance equivalence bound is invalid",

        GlobalAccessBoundsInconsistent =>
            "root subscriber global bounds are invalid",
        GlobalHaveNonTraitBound =>
            "root subscriber global bound is not trait bound",

        InitialBoundsInconsistent =>
            "root subscriber initial neighbour node bounds are invalid",
        InitialBoundNotTraitBound =>
            "root subscriber initial neighbour node's bound is not trait bound",
        FinalBoundsInconsistent =>
            "root subscriber final neighbour node bounds are invalid",
        FinalBoundNotTraitBound =>
            "root subscriber final neighbour node's bound is not trait bound",
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// `````````````````````````` TRAVERSE SUBSCRIBER SPACE ``````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

diagnostics!(
    space: TraverseSubscriber,

    pub(super) enum TraverseError {
        ParentNotDescendIdent {
            fields: {
                idents: BStringList,
                less_than_len: usize,
            },
            msg: format!("counter identifiers expected len is less than {}, but found length of {}, an exact mapping of given indexes", less_than_len, idents.bytes.len()),
            tags: [Unsupported],
            span: { tokens: &idents.bytes },
            note: [
                "traverse instance subscriber expects parent identifiers, not descend",
                "for exact instances use descend subscriber instead"
            ]
        },

        ParentMustBeOfLesserLen {
            fields: {
                idents: BStringList,
                less_than_len: usize,
            },
            msg: format!("counter identifiers expected len is less than {}, but found length of {}", less_than_len, idents.bytes.len()),
            tags: [Unsupported],
            span: { tokens: &idents.bytes },
            note: [
                "traverse instance subscriber expects parent identifiers",
                "parent idents length should be less than indexes length"
            ]
        },
    }
);

bug_diagnostics! {
    space: TraverseSubscriber,
    bug: TRAVERSE_SUB_BUG.to_string(),

    pub(super) enum TraverseBugs {
        ReplacedBoundsAreEmpty =>
            "traverse subscriber replaced instance bounds are empty",
        ReplacedBoundsInconsistent =>
            "traverse subscriber replaced instance bounds are invalid",
        ReplacedBoundNeitherOnSetNorCounter =>
            "traverse subscriber global bound is neither onset nor counter access bound",
        ReplacedArgNeitherInitialNorFinal =>
            "traverse subscriber replaced instance bound doesn't contain neither initial node or final node as generic argument",
        ReplacedBoundLastPathSegNotFound =>
            "traverse subscriber replaced bound last path segment not found",

        TerminalEquivalenceBoundNotFound =>
            "traverse subscriber replaced instance bound doesn't include terminal instance equivalence bound",
        TerminalEquivalenceBoundInvalid =>
            "traverse subscriber replaced instance bound's terminal instance equivalence bound is invalid",

        GlobalAccessBoundsInconsistent =>
            "traverse subscriber global bounds are invalid",
        GlobalHaveNonTraitBound =>
            "traverse subscriber global bound is not trait bound",

        InitialBoundsInconsistent =>
            "traverse subscriber initial neighbour node bounds are invalid",
        InitialBoundNotTraitBound =>
            "traverse subscriber initial neighbour node's bound is not trait bound",
        PrefixBoundNotTraitBound =>
            "traverse subscriber initial neighbour node's prefix bound is not trait bound",
        FinalBoundsInconsistent =>
            "traverse subscriber final neighbour node bounds are invalid",
        FinalBoundNotTraitBound =>
            "traverse subscriber final neighbour node's bound is not trait bound",
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// `````````````````````````` DESCEND SUBSCRIBER SPACE ```````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

diagnostics!(
    space: DescendSubscriber,

    pub(super) enum DescendError {

        ParentNotTraverseIdent {
            fields: {
                idents: BStringList,
                less_than_len: usize,
            },
            msg: format!("counter identifiers expected len is less than {}, but found length of {}, an exact mapping of given indexes", less_than_len, idents.bytes.len()),
            tags: [Unsupported],
            span: { tokens: &idents.bytes },
            note: [
                "descend instance subscriber expects parent identifiers, not traverse",
                "for exact instances use traverse subscriber instead"
            ]
        },

        ParentMustBeOfLesserLen {
            fields: {
                idents: BStringList,
                less_than_len: usize,
            },
            msg: format!("counter identifiers expected len is less than {}, but found length of {}", less_than_len, idents.bytes.len()),
            tags: [Unsupported],
            span: { tokens: &idents.bytes },
            note: [
                "descend instance subscriber expects parent identifiers",
                "parent idents length should be less than indexes length"
            ]
        },

    }
);

bug_diagnostics! {
    space: DescendSubscriber,
    bug: DESCEND_SUB_BUG.to_string(),

    pub(super) enum DescendBugs {
        ReplacedBoundsAreEmpty =>
            "descend subscriber replaced instance bounds are empty",
        ReplacedBoundsInconsistent =>
            "descend subscriber replaced instance bounds are invalid",
        ReplacedBoundNeitherOnSetNorCounter =>
            "descend subscriber global bound is neither onset nor counter access bound",
        ReplacedArgNeitherInitialNorFinal =>
            "descend subscriber replaced instance bound doesn't contain neither initial node or final node as generic argument",
        ReplacedBoundLastPathSegNotFound =>
            "descend subscriber replaced bound last path segment not found",

        TerminalEquivalenceBoundNotFound =>
            "descend subscriber replaced instance bound doesn't include terminal instance equivalence bound",
        TerminalEquivalenceBoundInvalid =>
            "descend subscriber replaced instance bound's terminal instance equivalence bound is invalid",

        GlobalAccessBoundsInconsistent =>
            "descend subscriber global bounds are invalid",
        GlobalHaveNonTraitBound =>
            "descend subscriber global bound is not trait bound",

        InitialBoundsInconsistent =>
            "descend subscriber initial neighbour node bounds are invalid",
        InitialBoundNotTraitBound =>
            "descend subscriber initial neighbour node's bound is not trait bound",
        PrefixBoundNotTraitBound =>
            "descend subscriber initial neighbour node's prefix bound is not trait bound",
        FinalBoundsInconsistent =>
            "descend subscriber final neighbour node bounds are invalid",
        FinalBoundNotTraitBound =>
            "descend subscriber final neighbour node's bound is not trait bound",
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ````````````````````````````` ACCESS TRAITS SPACE `````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

diagnostics!(
    space: AccessSpace,

    pub(super) enum AccessError {

        TraitPathNotAngleArgs {
            fields: {
                segment: PathSegment,
            },
            msg: "instance trait expected to have generic angle arguments",
            tags: [Unexpected],
            span: { tokens: segment.arguments },
            note: [
                "its where the counter const generics of index provided reside",
            ]
        },

        CountersGenericsIndexesEmpty {
            fields: {
                indexes: IntList,
            },
            msg: "requires instance counters generic indexes as list, found empty",
            tags: [Unsupported],
            span: { tokens: &indexes.ints },
            note: [
                "each counter index represents its corresponding instance ident",
                "example: b\"ident0\", b\"ident1\" "
            ]
        },

        InvalidFirstCounter {
            fields: {
                lit: LitInt,
                index: usize,
                len: usize,
            },
            msg: "first counter index creates out of bounds",
            tags: [Unsupported],
            span: { tokens: lit },
            help: [
                {format!("try providing correct counter generic of index less than {}", len)},
            ],
            note: [
                format!("current counter generic index is {}, expected less than (<) {}", index, len),
                "when removing the existing counter constant generic args, \
                the first counter must be inside available slots"
            ]
        },

        CounterGenericArgNotConstGeneric {
            fields: {
                arg: GenericArgument,
                index: usize,
                counter: LitInt,
            },
            msg: format!("expected generic arg of index {} to be a const-generic argument", index),
            tags: [Unsupported],
            span: { tokens: arg },
            help: [
                {
                    tokens: counter,
                    msg: "ensure if the correct counter generic of index is provided \
                    as its not a const generic"
                }
            ],
            note: [
                "required a positive integer literal const expression argument"
            ]
        },
        CounterGenericArgNotConstLit {
            fields: {
                expr: Expr,
                index: usize,
                counter: LitInt,
            },
            msg: format!("expected generic arg of index {} to be a integer literal const-generic argument", index),
            tags: [Unsupported],
            span: { tokens: expr },
            help: [
                {
                    tokens: counter,
                    msg: "ensure if the correct counter generic of index is provided \
                    as its not a literal const expression"
                }
            ],
            note: [
                "required a positive integer literal const expression argument, found non-literal expression"
            ]
        },
        CounterGenericArgNotConstLitInt {
            fields: {
                lit: Lit,
                index: usize,
                counter: LitInt,
            },
            msg: format!("expected generic arg of index {} to be a integer literal const-generic argument", index),
            tags: [Unsupported],
            span: { tokens: lit },
            help: [
                {
                    tokens: counter,
                    msg: "ensure if the correct counter generic of index is provided \
                    as its not an positive integer const expression"
                }
            ],
            note: [
                "required a positive integer literal const expression argument, found other-literal expression"
            ]
        },

    }
);

bug_diagnostics! {
    space: AccessSpace,
    bug: ACCESS_BUG.to_string(),

    pub(super) enum AccessBugs {
        CountersGenericsIndexesEmpty =>
            "counter generic index collection is unexpectedly empty \
            while statically accessing instance trait associated item",

        TraitPathNotAngleArgs =>
            "instance-trait path unexpectedly has no angle-bracketed generic arguments \
            while statically accessing instance trait associated item",

        AccessTyGenericArgNotFound =>
            "expected replaced access trait counter type arg is not found \
            while statically accessing instance trait associated item",

        AccessGenericArgNotType =>
            "generated access trait generic argument is not a type generic arg\
            while statically accessing instance trait associated item",

        UnexpectedAccessTyGenericArg =>
            "generated access trait counter type generic is invalid \
            while statically accessing instance trait associated item",

        UnexpectedAccessTraitBound =>
            "generated access trait bound is invalid",

        ExactAccessGenBoundQualifierNotFound =>
            "generated exact access trait bound's qualifier path (support crate) not found",
        ExactAccessGenBoundQualifierNotSupportCrate =>
            "generated exact access trait bound's qualifier path is not the support crate",
        ExactAccessGenBoundIdentNotFound =>
            "generated exact access trait bound's ident is not found",
        ExactAccessGenBoundIdentInvalid =>
            "generated exact access trait bound's ident is invalid",
        ExactAccessGenBoundArgsNotAngle =>
            "generated exact access trait bound's generic arguments are not angle args",
        ExactAccessGenBoundArgsInconsistent =>
            "generated exact access trait bound's angle arguments are inconsistent",
        ExactAccessGenBoundArgNotType =>
            "generated exact access trait bound's angle arg is not type generic",
        ExactAccessGenBoundArgNotTypePath =>
            "generated exact access trait bound's type generic angle arg is not type path",
        ExactAccessGenBoundArgTypeQSelfNotFound =>
            "generated exact access trait bound's type path angle arg qself not found",
        ExactAccessGenBoundArgTypeQSelfInvalid =>
            "generated exact access trait bound's type path angle arg qself invalid",
        ExactAccessGenBoundArgTypePathProjectionNotFound =>
            "generated exact access trait bound's projection access type not found",
        ExactAccessGenBoundArgTypePathProjectionInvalid =>
            "generated exact access trait bound's projection access type is invalid",
        ExactAccessBoundQualifierNotFound =>
            "generated exact access trait inside bound's qualifier is not found",
        ExactAccessBoundQualifierInvalid =>
            "generated exact access trait inside bound's qualifier is invalid",
        ExactAccessGenBoundPathNotFound =>
            "generated exact access trait bound's path is not found",
        ExactAccessBoundPathNotFound =>
            "generated exact access trait inside bound's path is not found",
        ExactAccessGenBoundPathInconsistent =>
            "generated exact access trait inside bound's path is invalid",

    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ``````````````````````````````` KEY DIAGNOSTICS ```````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

diagnostics!(
    space: KeyExpectation,

    pub(super) enum KeysError {
        ExpectedKeys {
            fields: {
                span: Span,
            },
            msg: "expected one of the following instance selectors \"leaf | branch | prune | trim | extend | spread | traverse | descend | root\"",
            tags: [Unsupported],
            span: { span: span },
            help: [
                "`leaf` : exact leaf. Example: #[_macro_(leaf { index(1,2), ident{ (b\"a\", b\"b\"), (b\"x\", b\"y\") } })]",
                "`branch` : branch and children. Example: #[_macro_(branch { index(1,2), parent(b\"a\") })]",
                "`prune` : root to leaf. Example: #[_macro_(prune { index(1,2), until(b\"a\", b\"b\") })]",
                "`trim` : root to branch. Example: #[_macro_(trim { index(1,2), until(b\"a\") })]",
                "`extend` : leaf to end. Example: #[_macro_(extend { index(1,2), from(b\"a\", b\"b\") })]",
                "`spread` : branch to end. Example: #[_macro_(spread { index(1,2), from(b\"a\") })]",
                "`traverse` : leaf to branch. Example: #[_macro_(traverse { index(1,2,3), from(b\"a\", b\"b\", b\"c\"), until(b\"a\", b\"b\") })]",
                "`descend` : branch to leaf. Example: #[_macro_(descend { index(1,2,3), from(b\"a\", b\"b\"), until(b\"a\", b\"b\", b\"c\") })]",
                "`root` : entire hierarchy. Example: #[_macro_(root { index(1,2) })]",
            ],
            note: [
                "`leaf` selects only the specified leaf instances.",
                "`branch` selects the specified branch and all instances beneath it.",
                "`prune` selects the path from the root through the specified leaf.",
                "`trim` selects the path from the root through the specified branch, including its subtree.",
                "`extend` selects from the specified leaf through the end of the hierarchy.",
                "`spread` selects from the specified branch through the end of the hierarchy, including its subtree.",
                "`traverse` selects from the specified leaf through the specified branch.",
                "`descend` selects from the specified branch through the specified leaf.",
                "`root` selects all instances in the hierarchy.",
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
                "`leaf` : exact leaf instances",
                "Example: #[_macro_(leaf { index(1,2), ident{ (b\"a\", b\"b\"), (b\"x\", b\"y\") } })]",
            ],
            note: [
                "each `ident` group must have the same length as `index`",
                "duplicate `ident` paths are not allowed",
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
                "`branch` : branch and its subtree",
                "Example: #[_macro_(branch { index(1,2), parent(b\"a\") })]",
            ],
            note: [
                "`parent` must have fewer identifiers than `index`",
                "`parent` must identify a branch, not a leaf",
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
                "`prune` : root through an exact leaf",
                "Example: #[_macro_(prune { index(1,2), until(b\"a\", b\"b\") })]",
            ],
            note: [
                "`until` must have the same length as `index`",
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
                "`trim` : root through a branch",
                "Example: #[_macro_(trim { index(1,2), until(b\"a\") })]",
            ],
            note: [
                "`until` must have fewer identifiers than `index`",
                "`until` must identify a branch, not a leaf",
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
                "`extend` : exact leaf through the end",
                "Example: #[_macro_(extend { index(1,2), from(b\"a\", b\"b\") })]",
            ],
            note: [
                "`from` must have the same length as `index`",
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
                "`spread` : branch through the end",
                "Example: #[_macro_(spread { index(1,2), from(b\"a\") })]",
            ],
            note: [
                "`from` must have fewer identifiers than `index`",
                "`from` must identify a branch, not a leaf",
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
                "`traverse` : exact leaf through a branch",
                "Example: #[_macro_(traverse { index(1,2,3), from(b\"a\", b\"b\", b\"c\"), until(b\"a\", b\"b\") })]",
            ],
            note: [
                "`from` must have the same length as `index`",
                "`until` must have fewer identifiers than `index`",
                "`until` must identify a branch, not a leaf",
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
                "`descend` : branch through an exact leaf",
                "Example: #[_macro_(descend { index(1,2,3), from(b\"a\", b\"b\"), until(b\"a\", b\"b\", b\"c\") })]",
            ],
            note: [
                "`from` must have fewer identifiers than `index`",
                "`from` must identify a branch, not a leaf",
                "`until` must have the same length as `index`",
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
                "`root` : entire instance hierarchy",
                "Example: #[_macro_(root { index(1,2) })]",
            ],
            note: [
                "`index` values must be unique",
                "`root` does not accept instance identifiers",
            ]
        },
    }
);

bug_diagnostics! {
    space: KeyExpectation,
    bug: KEY_BUG.to_string(),

    pub(super) enum KeyBugs {
        ExpectedKeys =>
            "expected node argument keys are not parsed",
        UnexpectedKeys =>
            "unexpected node argument keys are parsed and provided - requires none",
    }
}
