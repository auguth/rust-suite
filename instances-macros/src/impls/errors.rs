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
// ````````````````````````````` INSTANCE IMPL ERROR `````````````````````````````
// ===============================================================================

//! Diagnostics emitted by the instance-impl proc-macro pipeline.
//!
//! Defines error spaces, structured diagnostics, and internal invariant
//! violations covering counter arguments extraction, metadata generation,
//! affiliate generation, and identifier collections generation.

// ===============================================================================
// ``````````````````````````````````` IMPORTS ```````````````````````````````````
// ===============================================================================

// --- Proc-Suite ---
use proc_suite::{IntList, LOGICAL_BUG, REPORT_BUG, bug_diagnostics, diagnostics, error_spaces};

// --- Local Crate ---
use crate::{LAST_INSTANCE_MACRO_NAME, TRAIT_IMPL_MACRO_NAME, errors::MAINTAINERS};

// --- Proc-Macro Utils ---
use proc_macro2::Span;
use quote::ToTokens;
use syn::{
    Expr, GenericArgument, Ident, ItemImpl, Lit, LitInt, Path, PathSegment, Type,
    punctuated::Punctuated, token::Comma,
};

// ===============================================================================
// `````````````````````````````````` CONSTANTS ``````````````````````````````````
// ===============================================================================

pub(super) const EXTRACTION_BUG: &'static str =
    "instance impl's counter arguments extraction-phase didn't follow invariants";

pub(super) const TYPENUM_BUG: &'static str = "instance impls's counter arguments as a typenum on generic args \
    transformation-phase didn't follow invariants";

pub(super) const META_BUG: &'static str = "instance impl's meta-items for trait-side compliance reflection-phase \
    didn't follow invariants";

pub(super) const AFFILIATE_BUG: &'static str = "instance impl's affiliate-items for neighbouring instance access \
    extension-phase didn't follow invariants";

pub(super) const IDENT_BUG: &'static str = "instance impl's ident collection-items extension-phase \
    didn't follow invariants";

pub(super) const POST_BUG: &'static str = "instance impl's post-extension-phase for attributes cleanup \
    didn't follow invariants";

pub(super) const ADDON_BUG: &'static str = "instance impl's addon-phase for additional supplementary tokens \
    didn't follow invariants";

pub(super) const UTILITY_BUG: &'static str = "instance impl's utility functions to extract semantic constructs \
    didn't follow invariants";

pub(super) const LAST_BUG: &'static str = "last instancce impl's invariants were violated";

pub(super) const VALID_CONST_ARGS: &'static str =
    "provide valid instance declared const-generic positional index/es.";

pub(super) const CHECK_TRAIT_GEN: &'static str = "check the trait's generics and dpcumentation for identifying valid \
     instance counter const-generics indexes";

pub(super) const READ_TRAIT_DOC: &'static str = "read the trait's documentation if it requires macro-arguments for identifying \
    instance counter generic indexes";

// ===============================================================================
// ````````````````````````````````` ERROR-SPACES ````````````````````````````````
// ===============================================================================

error_spaces! {
    space: "INSTANCE_IMPL",
    maintain: MAINTAINERS,

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // ``````````````````````` IMPL & INSTANCE UTILITY ERRORS ````````````````````````
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

    pub(super) enum ImplInstanceUtilityError {
        range: 0..=100,
        variants: {
           NotTraitImpl,
           TraitPathLastSegmentNotFound,
           NegativeTraitsNotApplicable,
           FunctionTraitsNotApplicable,
           CountersTyNotFound,
           TypeNumGenericCountersNotReplacedYet,
           CountersGenericIsNotATypeGeneric,
           ExpectedConstAssocItemNotFound,
           CounterArgsEmptyForFirstCounterAccess,
        }
    }

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // ```````````````````````` COUNTER ARGUMENTS EXTRACTION `````````````````````````
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

    pub(super) enum CounterArgError {
        range: 101..=200,
        variants: {
            DuplicateCounterIndexes,
            EmptyCountersIndexes,
            OutOfBoundsGenericIndex,
            ArgumentNotConstGeneric,
            ArgumentNotConstGenericExpr,
            ArgumentNotConstGenericExprLitInt,
            EmptyUtilizedCounterArgsFromList,
            TraitArgsNeedsLeadConstGenerics,
            CannotDeriveCounterArgsImplicitly,

            CounterIndexesExtractionViaGenArgsInconsistent,
            CounterIndexesExtractionInconsistent,
            ExtractedCounterArgsLenAboveActualGenArgs,
            ExtractedArgumentOutOfBoundsGenericIndex,
            ExtractedArgumentNotConstGeneric,
            ExtractedArgumentNotConstGenericExpr,
            ExtractedArgumentNotConstGenericExprLitInt,
            ExtractedCounterArgumentIsNotSame,
            ExtractionViaLeadConstGenericsInconsistent,
        }
    }

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // ```````````````````````` TYPENUM COUNTERS REPLACEMENT `````````````````````````
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

    pub(super) enum TypeNumErrors {
        range: 201..=300,
        variants: {
            TraitPathLastSegmentNotFound,
            FunctionTraitsNotApplicable,
            CounterArgExtractedTraitInvalidOnGenericsLen,
            PostTypeNumNoGenericsAvailable,
            PostTypeNumFirstGenericNotType,
            PostTypeNumInvalidTypeArg,
            InvalidFirstCounter,
        }
    }

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // ```````````````````````````` META ITEMS REFLECTION ````````````````````````````
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

    pub(super) enum MetaItemsError {
        range: 301..=400,
        variants: {
            UnexpectedCountersTy,
            InvalidCounterIndexes,

            ImplCountersTyCheckerNotFound,
            ImplCountersTyCheckerWrongIdent,
            ImplCountersTyCheckerInvalidType,
            ImplCountersTyCheckerInvalidExpr,
            ImplCountersTyCheckerHasGenerics,

            ImplCountersGenericsIndexesMetaNotFound,
            ImplCountersGenericsIndexesMetaWrongIdent,
            ImplCountersGenericsIndexesMetaInvalidType,
            ImplCountersGenericsIndexesMetaInvalidExpr,
            ImplCountersGenericsIndexesMetaHasGenerics,

            ImplCountersLenMetaNotFound,
            ImplCountersLenMetaWrongIdent,
            ImplCountersLenMetaInvalidType,
            ImplCountersLenMetaInvalidExpr,
            ImplCountersLenMetaHasGenerics,

            ImplCountersGenericsCheckerNotFound,
            ImplCountersGenericsCheckerWrongIdent,
            ImplCountersGenericsCheckerInvalidType,
            ImplCountersGenericsCheckerInvalidExpr,
            ImplCountersGenericsCheckerHasGenerics,

            ImplOriginalCounterConstNotFound,
            ImplOriginalCounterConstWrongIdent,
            ImplOriginalCounterConstInvalidType,
            ImplOriginalCounterConstInvalidExpr,
            ImplOriginalCounterConstHasGenerics,

            CounterArgsAreEmptyToProvideForGenericsIndexesMeta,

            ImplCumulatedConstCheckerNotFound,
            ImplCumulatedConstCheckerWrongIdent,
            ImplCumulatedConstCheckerNotUnitType,
            ImplCumulatedConstCheckerInvalidExpr,
            ImplCumulatedConstCheckerHasGenerics,


            GlobalAssocNotFound,
            GlobalAssocWrongType,

            GlobalTerminalExactNotFound,
            GlobalTerminalExactWrongType,

            SelfTerminalAssocNotFound,
            SelfTerminalAssocWrongType,

        }
    }

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // ``````````````````````` AFFILIATE COUNTERS & INSTANCES ````````````````````````
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

    pub(super) enum AffiliateItemsError {
        range: 401..=600,
        variants: {
            ImplAffiliateInstanceNotFound,
            ImplAffiliateInstanceWrongIdent,
            ImplAffiliateInstanceInvalidType,
            ImplAffiliateInstanceHasGenerics,

            ImplReverseAffiliateCounterNotFound,
            ImplReverseAffiliateCounterWrongIdent,
            ImplReverseAffiliateCounterInvalidType,
            ImplReverseAffiliateCounterHasGenerics,

            ImplCeilAffiliateCounterNotFound,
            ImplCeilAffiliateCounterWrongIdent,
            ImplCeilAffiliateCounterInvalidType,
            ImplCeilAffiliateCounterHasGenerics,

            ImplBackAffiliateCounterNotFound,
            ImplBackAffiliateCounterWrongIdent,
            ImplBackAffiliateCounterInvalidType,
            ImplBackAffiliateCounterHasGenerics,

            ImplFloorAffiliatesCountersNotFound,
            ImplFloorAffiliatesCountersWrongIdent,
            ImplFloorAffiliatesCountersInvalidType,
            ImplFloorAffiliatesCountersHasGenerics,

            ImplNextAffiliateCounterNotFound,
            ImplNextAffiliateCounterWrongIdent,
            ImplNextAffiliateCounterInvalidType,
            ImplNextAffiliateCounterHasGenerics,

            ImplAffiliateCountersCheckerNotFound,
            ImplAffiliateCountersCheckerWrongIdent,
            ImplAffiliateCountersCheckerInvalidType,
            ImplAffiliateCountersCheckerHasGenerics,
            ImplAffiliateCountersCheckerInvalidExpr,

            ExtractedReverseAffiliateCountersAreEmpty,
            ExtractedCeilAffiliateCountersAreEmpty,
            CountersAreEmptyToFindFirstCounter,
            CountersAreEmptyToFindLastCounter,
            ExtractedNextAffiliateCountersAreEmpty,

            RougueInstanceFoundBackNextFailed,
            RougueInstanceFoundNextBackFailed,
        }
    }

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // `````````````````````` IDENTIFIERS, HASHES & COLLECTIONS ``````````````````````
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

    pub(super) enum IdentItemsErrors {
        range: 601..=800,
        variants: {
            ImplCounterIdentCollectionLenTypeNumNotFound,
            ImplCounterIdentCollectionLenTypeNumWrongIdent,
            ImplCounterIdentCollectionLenTypeNumInvalidType,
            ImplCounterIdentCollectionLenTypeNumHasGenerics,

            ImplCounterIdentHistoricalCollectionLenTypeNumNotFound,
            ImplCounterIdentHistoricalCollectionLenTypeNumWrongIdent,
            ImplCounterIdentHistoricalCollectionLenTypeNumInvalidType,
            ImplCounterIdentHistoricalCollectionLenTypeNumHasGenerics,

            ImplCounterIdentCollectionGenArrayNotFound,
            ImplCounterIdentCollectionGenArrayWrongIdent,
            ImplCounterIdentCollectionGenArrayInvalidType,
            ImplCounterIdentCollectionGenArrayHasGenerics,
            ImplCounterIdentCollectionGenArrayInvalidExpr,

            ImplCounterIdentHashCollectionGenArrayNotFound,
            ImplCounterIdentHashCollectionGenArrayWrongIdent,
            ImplCounterIdentHashCollectionGenArrayInvalidType,
            ImplCounterIdentHashCollectionGenArrayHasGenerics,
            ImplCounterIdentHashCollectionGenArrayInvalidExpr,

            ImplCounterIdentHistoricalCollectionGenArrayNotFound,
            ImplCounterIdentHistoricalCollectionGenArrayWrongIdent,
            ImplCounterIdentHistoricalCollectionGenArrayInvalidType,
            ImplCounterIdentHistoricalCollectionGenArrayHasGenerics,
            ImplCounterIdentHistoricalCollectionGenArrayInvalidExpr,

            ImplCounterIdentHashHistoricalCollectionGenArrayNotFound,
            ImplCounterIdentHashHistoricalCollectionGenArrayWrongIdent,
            ImplCounterIdentHashHistoricalCollectionGenArrayInvalidType,
            ImplCounterIdentHashHistoricalCollectionGenArrayHasGenerics,
            ImplCounterIdentHashHistoricalCollectionGenArrayInvalidExpr,

            ImplCounterIdentExpectedHashCheckerNotFound,
            ImplCounterIdentExpectedHashCheckerWrongIdent,
            ImplCounterIdentExpectedHashCheckerNotUnitType,
            ImplCounterIdentExpectedHashCheckerHasGenerics,
            ImplCounterIdentExpectedHashCheckerInvalidExpr,

            DuplicateCounterIdentFound,

            ParentCountersIdentsNotConsistent,
        }
    }

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // ````````````````````````````` LAST INSTANCE IMPL ``````````````````````````````
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

    pub(super) enum LastInstanceErrors {
        range: 801..=1000,
        variants: {
            DuplicateCounterIndexes,
            InstanceImplMacroNotApplied,

            CountersIndexesMetaNotARefExpr,
            CountersIndexesMetaNotArrayExpr,
            CountersIndexesMetaNotLitArrayExpr,
            CountersIndexesMetaNotLitIntArrayExpr,

            OriginalCounterAssocNotLitExpr,
            OriginalCounterAssocNotLitIntExpr,
            CollectedCounterArgsAssocIsEmpty,

            CounterArgsPostValidateOnEmptyArgs,
            RegainingOriginalCounterFailedOnPostValidate,
            RegainedOriginalCounterExprIsNotLit,
            RegainedOriginalCounterExprIsNotLitInt,
            RegainedOriginalCounterExprNotMatch,

            CounterArgsAssocIsEmptyForFirstCounterAccess,

            NotACounterGenericIndex,

            LastInstanceArgNotFoundInCountersMeta,
            LastInstanceArgFromDuplicateIndexes,
            LastInstanceArgFromNoIndexesButNotFirstCounter,
            LastInstanceArgNotFromCounterIndexes,
            InvalidLastInstanceArgGiven,

            BestLastInstanceArgNotFound,

            FloorAffiliatesCountersReplacementNotFound,
            FloorAffiliatesCountersReplacementWrongIdent,
            FloorAffiliatesCountersReplacementInvalidType,
            FloorAffiliatesCountersReplacementHasGenerics,

            AnExistingAffiliatesCountersTypeUnavailableToReplace,

            NextAffiliatesCountersReplacementNotFound,
            NextAffiliatesCountersReplacementWrongIdent,
            NextAffiliatesCountersReplacementInvalidType,
            NextAffiliatesCountersReplacementHasGenerics,

            LastInstanceArgIndexCannotFetchItsConstLit,

            BackAffiliatesCountersReplacementNotFound,
            BackAffiliatesCountersReplacementWrongIdent,
            BackAffiliatesCountersReplacementInvalidType,
            BackAffiliatesCountersReplacementHasGenerics,

            AnExistingAssocConstUnavailableToReplace,

            AffiliateCountersCheckerReplacementNotFound,
            AffiliateCountersCheckerReplacementWrongIdent,
            AffiliateCountersCheckerReplacementInvalidType,
            AffiliateCountersCheckerReplacementInvalidExpr,
            AffiliateCountersCheckerReplacementHasGenerics,
        }
    }

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // ````````````````````````````` POST IMPL INSTANCE ``````````````````````````````
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

    pub(super) enum PostInstanceImplErrors {
        range: 1001..=1100,
        variants: {
            CounterAttrNotRemoved,
            SelfBoundsAttrNotRemoved,
            LastInstanceAttrNotRemoved,
        }
    }

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // ```````````````````````````` IMPL INSTANCE ADDONS `````````````````````````````
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

    pub(super) enum InstanceImplAddonErrors {
        range: 1101..=1300,
        variants: {
            IdentifierUnavailableForCounterIndex,
            ExtractedIdentifierRawExprsLengthMismatch,
            ExtractedIdentifierRawExprIsInconsistent,

            InvalidCounterAccessImplIdent,
            CounterAccessImplAddonNotFound,
            InvalidGlobalTypePathForCounterAccess,
            InvalidCounterAccessImplPathSegmentsLen,
            InvalidCounterAccessImplPathSegment,
            InvalidCounterAccessImplTraitSegment,
            CounterAccessImplHasInvalidGenericArgs,
            CounterAccessImplExpectedGenericArgsLenInvalid,
            CounterAccessImplOriginalTypeGenericArgUnavailable,
            CounterAccessImplOriginalTypeNotTypeGenericArg,
            CounterAccessImplOriginalTypeGenericArgInvalid,
            CounterAccessImplExpectedConstGenericArgsLenInvalid,
            CounterAccessImplExpectedConstGenericFoundElse,
            CounterAccessImplHashExprInvalid,
            CounterAccessImplOriginalTypeGenericWhereClausesInvalid,
            CounterAccessImplOriginalTypeGenericParamsInvalid,
            CounterAccessImplAddonWithNoAssocItems,
            CounterAccessImplCounterAssocNotType,
            CounterAccessImplAddonTypeAssocIdentInvalid,
            CounterAccessImplInvalidCounterTy,
            CounterAccessImplHasExcessAssocs,

            InvalidIdentHashLenAccessImplIdent,
            IdentHashLenAccessImplAddonNotFound,
            IdentHashLenAccessImplAddonMissing,
            InvalidGlobalTypePathForIdentHashLenAccess,
            InvalidIdentHashLenAccessImplPathSegmentsLen,
            InvalidIdentHashLenAccessImplPathSegment,
            InvalidIdentHashLenAccessImplTraitSegment,
            IdentHashLenAccessImplHasInvalidGenericArgs,
            IdentHashLenAccessImplExpectedGenericArgsLenInvalid,
            IdentHashLenAccessImplOriginalTypeGenericArgUnavailable,
            IdentHashLenAccessImplOriginalTypeNotTypeGenericArg,
            IdentHashLenAccessImplOriginalTypeGenericArgInvalid,
            IdentHashLenAccessImplExpectedConstGenericArgsLenInvalid,
            IdentHashLenAccessImplExpectedConstGenericFoundElse,
            IdentHashLenAccessImplHashExprInvalid,
            IdentHashLenAccessImplOriginalTypeGenericWhereClausesInvalid,
            IdentHashLenAccessImplOriginalTypeGenericParamsInvalid,
            IdentHashLenAccessImplAddonWithNoAssocItems,
            IdentHashLenAccessImplLengthAssocNotType,
            IdentHashLenAccessImplAddonTypeAssocIdentInvalid,
            IdentHashLenAccessGivenCounterIndexInvalid,
            IdentHashLenAccessImplInvalidLengthTy,
            IdentHashLenAccessImplHasExcessAssocs,
            IdentHashLenAccessCounterIndexGenericArgUnavailable,
            IdentHashLenAccessCounterIndexNotConstGenericArg,
            IdentHashLenAccessCounterIndexNotConstExprLit,
            IdentHashLenAccessCounterIndexNotConstLitInt,

            InvalidTerminalAccessImplIdent,
            InvalidGlobalTypePathForTerminalAccess,
            InvalidTerminalAccessImplPathSegmentsLen,
            InvalidTerminalAccessImplPathSegment,
            InvalidTerminalAccessImplTraitSegment,
            TerminalAccessImplHasInvalidGenericArgs,
            TerminalAccessImplExpectedGenericArgsLenInvalid,
            TerminalAccessImplOriginalTypeGenericArgUnavailable,
            TerminalAccessImplOriginalTypeNotTypeGenericArg,
            TerminalAccessImplOriginalTypeGenericArgInvalid,
            TerminalAccessImplOriginalTypeGenericWhereClausesInvalid,
            TerminalAccessImplOriginalTypeGenericParamsInvalid,
            TerminalAccessImplAddonWithNoAssocItems,
            TerminalAccessImplCounterAssocNotType,
            TerminalAccessImplAddonTypeAssocIdentInvalid,
            TerminalAccessImplInvalidCounterTy,
            TerminalAccessImplHasExcessAssocs,
            TerminalAccessImplNotFound,
            TerminalAccessImplFoundOtherItem,
            TerminalAccessImplAttemptedForNonTerminalCounter,

            TerminalCheckerConstAttemptedForNonTerminalCounter,
            TerminalCheckerConstInvalidIdent,
            TerminalCheckerConstAttemptedForGenericImpl,
            TerminalCheckerConstInvalidExpr,
            TerminalCheckerConstNotFound,

            LastInstanceCounterInvalid,
            BoundaryAccessHashOfIndexFromHashExprsUnavailable,
            InvalidGlobalTypePathForBoundaryAccess,
            InvalidBoundaryAccessImplPathSegmentsLen,
            InvalidBoundaryAccessImplPathSegment,
            InvalidBoundaryAccessImplTraitSegment,
            BoundaryAccessImplHasInvalidGenericArgs,
            BoundaryAccessImplOriginalTypeGenericArgUnavailable,
            BoundaryAccessImplOriginalTypeNotTypeGenericArg,
            BoundaryAccessImplOriginalTypeGenericArgInvalid,
            BoundaryAccessHashExprArgsLenInvalid,
            BoundaryAccessHashExprArgNotConst,
            BoundaryAccessHashExprArgConstInvalid,
            BoundaryAccessImplOriginalTypeGenericWhereClausesInvalid,
            BoundaryAccessImplOriginalTypeGenericParamsInvalid,
            BoundaryAccessImplAddonWithNoAssocItems,
            BoundaryAccessImplHasExcessAssocs,
            BoundaryAccessImplCounterAssocTypeNotFound,
            BoundaryAccessImplAddonTypeAssocIdentInvalid,
            BoundaryAccessImplInvalidCounterTy,
            BoundaryAccessImplHashAssocConstNotFound,
            BoundaryAccessImplAddonConstAssocIdentInvalid,
            BoundaryAccessImplInvalidHashConstTy,
            BoundaryAccessImplInvalidConstAssocExpr,
            BoundaryAccessImplNotFound,

            OnSetAccessHashOfIndexFromHashExprsUnavailable,
            InvalidGlobalTypePathForOnSetAccess,
            InvalidOnSetAccessImplPathSegmentsLen,
            InvalidOnSetAccessImplPathSegment,
            InvalidOnSetAccessImplTraitSegment,
            OnSetAccessImplHasInvalidGenericArgs,
            OnSetAccessImplOriginalTypeGenericArgUnavailable,
            OnSetAccessImplOriginalTypeNotTypeGenericArg,
            OnSetAccessImplOriginalTypeGenericArgInvalid,
            OnSetAccessHashExprArgsLenInvalid,
            OnSetAccessHashExprArgNotConst,
            OnSetAccessHashExprArgConstInvalid,
            OnSetAccessImplOriginalTypeGenericWhereClausesInvalid,
            OnSetAccessImplOriginalTypeGenericParamsInvalid,
            OnSetAccessImplAddonWithNoAssocItems,
            OnSetAccessImplHasExcessAssocs,
            OnSetAccessImplCounterAssocTypeNotFound,
            OnSetAccessImplAddonTypeAssocIdentInvalid,
            OnSetAccessImplInvalidCounterTy,
            OnSetAccessImplHashAssocConstNotFound,
            OnSetAccessImplAddonConstAssocIdentInvalid,
            OnSetAccessImplInvalidHashConstTy,
            OnSetAccessImplInvalidConstAssocExpr,
            OnSetAccessImplNotFound,
        }
    }

}

// ===============================================================================
// ````````````````````````````````` DIAGNOSTICS `````````````````````````````````
// ===============================================================================

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ````````````````````````` COUNTER ARGUMENTS EXTRACTION ````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

diagnostics!(
    space: CounterArgError,

    pub(super) enum CounterArgExtractionError {

        EmptyCountersIndexes {
            fields: {
                ints: IntList,
            },
            msg:
                "macro-arguments should be providing non-empty const-generic \
                indexes list, but haven't",
            tags: [Bug],
            span: { tokens: ints.ints },
            note: [
                EXTRACTION_BUG.to_string(),
                LOGICAL_BUG.to_string(),
                REPORT_BUG.to_string()
            ]
        },


        OutOfBoundsGenericIndex {
            fields: {
                gen_idx: usize,
                given_idx: LitInt,
                trait_ident: Ident,
                trait_generics : Punctuated<GenericArgument, Comma>,
            },
            msg: format!(
                "given generic's index `{}` of trait `{}` via impl is out of bounds \
                to access", gen_idx, trait_ident
            ),
            tags: [Unsupported],
            span: { tokens: given_idx },
            help: [
                {CHECK_TRAIT_GEN.to_string()},
                {
                    tokens: trait_generics,
                    msg: format!("generic of index `{}` is not available in trait's generic arguments", gen_idx),
                }
            ],
            note: [
                VALID_CONST_ARGS.to_string(),
            ],
        },

        ArgumentNotConstGeneric {
            fields: {
                gen_idx: usize,
                given_idx: LitInt,
                trait_ident: Ident,
                found: GenericArgument,
            },
            msg: format!(
                "given generic's index `{}` of trait `{}` via impl is not a const-generic", gen_idx, trait_ident
            ),
            tags: [Unsupported],
            span: { tokens: given_idx },
            help: [
                {format!("expected a const-generic generic argument, found `{}`", found.to_token_stream())},
                {CHECK_TRAIT_GEN.to_string()},
                {
                    tokens: found,
                    msg: "expected a const-generic argument instead of this",
                }
            ],
            note: [
                VALID_CONST_ARGS.to_string(),
            ],
        },

        ArgumentNotConstGenericExpr {
            fields: {
                gen_idx: usize,
                given_idx: LitInt,
                trait_ident: Ident,
                found: Expr,
            },
            msg: format!(
                "given generic's index `{}` of trait `{}` via impl is not a unsigned integer's const-generic", gen_idx, trait_ident
            ),
            tags: [Unsupported],
            span: { tokens: given_idx },
            help: [
                {format!("expected a unsigned integer numeric literal, found `{}`", found.to_token_stream())},
                {CHECK_TRAIT_GEN.to_string()},
                {
                    tokens: found,
                    msg: "expected a unsigned integer const-generic argument instead of this",
                }
            ],
            note: [
                VALID_CONST_ARGS.to_string(),
            ],
        },

        ArgumentNotConstGenericExprLitInt {
            fields: {
                gen_idx: usize,
                given_idx: LitInt,
                trait_ident: Ident,
                found: Lit,
            },
            msg: format!(
                "given generic's index `{}` of trait `{}` via impl is not a unsigned integer's const-generic", gen_idx, trait_ident
            ),
            tags: [Unsupported],
            span: { tokens: given_idx },
            help: [
                {format!("expected a unsigned integer numeric literal, found `{}`", found.to_token_stream())},
                {CHECK_TRAIT_GEN.to_string()},
                {
                    tokens: found,
                    msg: "expected a unsigned integer numeric literal argument instead of this",
                }
            ],
            note: [
                VALID_CONST_ARGS.to_string(),
            ],
        },

        EmptyUtilizedCounterArgsFromList {
            fields: {
                ints: IntList,
            },
            msg:
                "should be able to extract/utilize atleast one const-generic \
                argument from non-empty macro-arguments indexes list, but couldn't",
            tags: [Bug],
            span: { tokens: ints.ints },
            note: [
                EXTRACTION_BUG.to_string(),
                LOGICAL_BUG.to_string(),
                REPORT_BUG.to_string()
            ]
        },

        TraitArgsNeedsLeadConstGenerics {
            fields: {
                trait_generics : Punctuated<GenericArgument, Comma>,
                trait_ident: Ident,
            },
            msg: format!(
                "impl of trait `{}` has no leading unsigned const generics to qualify \
                as a instance trait's implementation", trait_ident
            ),
            tags : [Unsupported],
            span: {tokens: trait_generics},
            help: [
                "provide suitable leading unsigned const-generics as instance counter/s",
                {
                        format!("ensure if the trait `{}` that is implemented is actually an \
                        instance trait", trait_ident)
                },
            ],
            note: [
                READ_TRAIT_DOC.to_string()
            ],
        },

        CannotDeriveCounterArgsImplicitly {
            fields: {
                trait_ident: Ident,
                trait_path : Path,
            },
            msg: format!(
                "impl of trait `{}` has no valid instance counters generics to qualify \
                as a instance trait's implementation", trait_ident
            ),
            tags : [Unsupported],
            span: {tokens: trait_path},
            help: [
                {
                        format!("ensure if the trait `{}` that is implemented is actually an \
                        instance trait", trait_ident)
                },
                "provide suitable leading unsigned const-generic indexes as instance counter/s \
                via macro-arguments (if required)",
            ],
            note: [
                READ_TRAIT_DOC.to_string()
            ],
        },
    }
);

bug_diagnostics! {
    space: CounterArgError,
    bug: EXTRACTION_BUG.to_string(),

    pub(super) enum CounterArgBugs {
        CounterIndexesExtractionViaGenArgsInconsistent =>
                "macro-arguments of counter generic indexes is inconsistent \
                with actual instance trait's generic arguments",
        CounterIndexesExtractionInconsistent =>
                "macro-arguments of counter generic indexes is inconsistent \
                with actual counter arguments extracted",
        ExtractedCounterArgsLenAboveActualGenArgs =>
                "extracted counter arguments from instance trait has indexes
                higher than actual trait's generic arguments",
        ExtractedArgumentOutOfBoundsGenericIndex =>
                "extracted counter argument via its index position does not exist \
                in actual trait's generic arguments (index out of bounds)",
        ExtractedArgumentNotConstGeneric =>
                "extracted counter argument from instance trait generic arguments \
                is not a const-generic",
        ExtractedArgumentNotConstGenericExpr =>
                "extracted counter argument from instance trait generic arguments \
                is does not have a const-generic expression",
        ExtractedArgumentNotConstGenericExprLitInt =>
                "extracted counter argument from instance trait generic arguments \
                is does not have a unsigned integer const-generic expression",
        ExtractedCounterArgumentIsNotSame =>
                "extracted counter argument from instance trait generic arguments \
                is not the same from actual trait's generic arguments",
        ExtractionViaLeadConstGenericsInconsistent =>
                "extracted leading const-generic counter argument from \
                actual instance trait generic arguments is not sequentially consistent",
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ```````````````````````` TYPENUM COUNTERS REPLACEMENT `````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

diagnostics!(
    space: TypeNumErrors,

    pub(super) enum TypeNumError {

        InvalidFirstCounter {
            fields: {
                lit: LitInt,
                index: usize,
                len: usize,
            },
            msg: "first counter placement creates out of bounds",
            tags: [Unsupported],
            span: { tokens: lit },
            help: [
                {format!("try placing counter in generic of index less than {}", len)},
            ],
            note: [
                format!("current counter placement is index {}, expected less than (<) {}", index, len),
                "when removing the existing counter constant generic args, \
                the first counter must be inside available slots",
            ]
        },
    }
);

bug_diagnostics! {
    space: TypeNumErrors,
    bug: TYPENUM_BUG.to_string(),

    pub(super) enum TypeNumBugs {
        TraitPathLastSegmentNotFound =>
                "unexpected for a trait-impl to not have a last path segment (syn-bug) \
                , typenum counter arguments replacement phase violated",
        FunctionTraitsNotApplicable =>
                "cannot apply instance impl on a function trait (call-trait) impl \
                , typenum counter arguments replacement phase violated",
        CounterArgExtractedTraitInvalidOnGenericsLen =>
                "extracted counter arguments is inconsistent with actual trait generics args \
                , typenum counter arguments replacement phase violated",
        PostTypeNumNoGenericsAvailable =>
                "post-type num trait generic arguments - no args found \
                , typenum counter arguments replacement phase violated",
        PostTypeNumFirstGenericNotType =>
                "post-type num trait generic arguments - first argument is not a type generic \
                , typenum counter arguments replacement phase violated",
        PostTypeNumInvalidTypeArg =>
                "post-type num trait generic arguments - unexpected type-num argument \
                , typenum counter arguments replacement phase violated",
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ``````````````````````` IMPL & INSTANCE UTILITY ERRORS ````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

diagnostics!(
    space: ImplInstanceUtilityError,

    pub(super) enum UtilityErrors {

        NotTraitImpl {
            fields: {
                impl_of: ItemImpl,
            },
            msg:
                "impl is not a trait implementation",
            tags: [Unexpected],
            span: { tokens: impl_of },
            help: [
                {
                    tokens: impl_of,
                    msg: "expected a trait-impl"
                }
            ]

        },

        TraitPathLastSegmentNotFound {
            fields: {
                trait_path: Path,
            },
            msg:
                "unexpected for a trait-impl to not have a last path segment (syn-bug)",
            tags: [Bug],
            span: { tokens: trait_path },
        },

        NegativeTraitsNotApplicable {
            fields: {
                trait_path: Path,
            },
            msg:
                "unexpected negative trait impl",
            tags: [Unexpected],
            span: { tokens: trait_path },
            help: [
                "expected a normal (angle bracketed) trait-impl",
            ]

        },



        FunctionTraitsNotApplicable {
            fields: {
                segment: PathSegment,
            },
            msg:
                "unexpected function trait (call-trait) impl \
                which have paranthesized generic arguments",
            tags: [Unexpected],
            span: { tokens: segment },
            help: [
                {
                    tokens: segment,
                    msg: "expected a normal (angle bracketed) trait-impl"
                }
            ]

        },

        CountersTyNotFound {
            fields: {
                exp: Ident,
                trait_ident: Ident,
                impl_of: ItemImpl,
            },
            msg:
                format!("expected assoc-type `{}` in trait `{}` implementation, but not found", exp, trait_ident),
            tags: [Unexpected],
            span: { tokens: impl_of },
            help: [
                {
                    format!("ensure if the trait `{}` that is implemented is actually an \
                    instance trait", trait_ident)
                },
            ]

        },

        ExpectedConstAssocItemNotFound {
            fields: {
                exp: Ident,
            },
            msg:
                format!("an expected const-assoc-type `{}` is not found in the instance trait impl", exp),
            tags: [Bug],
            span: { span: Span::call_site() },
            note: [
                UTILITY_BUG.to_string(),
                LOGICAL_BUG.to_string(),
                REPORT_BUG.to_string(),
            ]

        },
    }
);

bug_diagnostics! {
    space: ImplInstanceUtilityError,
    bug: UTILITY_BUG.to_string(),

    pub(super) enum UtilityBugs {

        TypeNumGenericCountersNotReplacedYet =>
                "trait generics transformation to replace/inject typenum counters has not done \
                and expecting otherwise",
        CountersGenericIsNotATypeGeneric =>
                "found non-type generic in place of typenum counter type generic at index 0 of \
                trait's generics",
        CounterArgsEmptyForFirstCounterAccess =>
                "provided counters arguments are empty when accessing first counter \
                counter argument extraction is violated",
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ````````````````````````` IMPL META ITEMS (EXTENSION) `````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

diagnostics!(
    space: MetaItemsError,

    pub(super) enum MetaErrors {

        UnexpectedCountersTy {
            fields: {
                original: Ident,
                trait_of: Ident,
            },
            msg:
                format!("unexpected trait instance counter type provided via `{}` associate type of trait `{}` ", original, trait_of),
            tags: [Unexpected],
            span: { span: &Span::call_site() },
            help: [
                {
                    format!("ensure if the trait `{}` that is implemented is actually an \
                    instance trait", trait_of)
                },
                "provide the valid trait instance counter type declared on trait-side"
            ]

        },
        InvalidCounterIndexes {
            fields: {
                trait_of: Ident,
            },
            msg:
                format!("invalid instance counters generic indexes for instance trait `{}`", trait_of),
            tags: [Unexpected],
            span: { span: &Span::call_site()},
            help: [
                {
                    format!("ensure if the trait `{}` that is implemented is actually an \
                    instance trait", trait_of)
                },
                "the trait's documentation header may list the expected const generic indices, \
                else no indices required."
            ],
            note: [
                "possible if the impl instance macro arguments are invalid"
            ]

        },
    }
);

bug_diagnostics! {
    space: MetaItemsError,
    bug: META_BUG.to_string(),

    pub(super) enum MetaImplItemsBugs {

        ImplCountersTyCheckerNotFound =>
                "hidden impl const-assoc-checker (unit-type) to assert restated instance counter type \
                is not found",
        ImplCountersTyCheckerWrongIdent =>
                "hidden impl const-assoc-checker (unit-type) to assert restated instance counter type \
                does not have a determinisitic random name",
        ImplCountersTyCheckerInvalidExpr =>
                "hidden impl const-assoc-checker (unit-type) to assert restated instance counter type \
                has an invalid expression",
        ImplCountersTyCheckerInvalidType =>
                "hidden impl const-assoc-checker (unit-type) to assert restated instance counter type \
                has an invalid const-type",
        ImplCountersTyCheckerHasGenerics =>
                "hidden impl const-assoc-checker (unit-type) to assert restated instance counter type \
                has unnecessary generics",

        ImplCountersGenericsIndexesMetaNotFound =>
                "hidden impl const-assoc to take impl-side counters generic indexes for trait-side compliance \
                is not found",
        ImplCountersGenericsIndexesMetaWrongIdent =>
                "hidden impl const-assoc to take impl-side counters generic indexes for trait-side compliance \
                does not have a determinisitic random name",
        ImplCountersGenericsIndexesMetaInvalidExpr =>
                "hidden impl const-assoc to take impl-side counters generic indexes for trait-side compliance \
                has an invalid expression",
        ImplCountersGenericsIndexesMetaInvalidType =>
                "hidden impl const-assoc to take impl-side counters generic indexes for trait-side compliance \
                has an invalid const-type",
        ImplCountersGenericsIndexesMetaHasGenerics =>
                "hidden impl const-assoc to take impl-side counters generic indexes for trait-side compliance \
                has unnecessary generics",

        ImplCountersLenMetaNotFound =>
                "hidden impl const-assoc to take impl-side counters length for trait-side compliance \
                is not found",
        ImplCountersLenMetaWrongIdent =>
                "hidden impl const-assoc to take impl-side counters length for trait-side compliance \
                does not have a determinisitic random name",
        ImplCountersLenMetaInvalidExpr =>
                "hidden impl const-assoc to take impl-side counters length for trait-side compliance \
                has an invalid expression",
        ImplCountersLenMetaInvalidType =>
                "hidden impl const-assoc to take impl-side counters length for trait-side compliance \
                has an invalid const-type",
        ImplCountersLenMetaHasGenerics =>
                "hidden impl const-assoc to take impl-side counters length for trait-side compliance \
                has unnecessary generics",

        ImplCountersGenericsCheckerNotFound =>
                "hidden impl const-assoc-checker (unit-type) to assert trait and impl side counters meta information \
                is not found",
        ImplCountersGenericsCheckerWrongIdent =>
                "hidden impl const-assoc-checker (unit-type) to assert trait and impl side counters meta information \
                does not have a determinisitic random name",
        ImplCountersGenericsCheckerInvalidExpr =>
                "hidden impl const-assoc-checker (unit-type) to assert trait and impl side counters meta information \
                has an invalid expression",
        ImplCountersGenericsCheckerInvalidType =>
                "hidden impl const-assoc-checker (unit-type) to assert trait and impl side counters meta information \
                has an invalid const-type",
        ImplCountersGenericsCheckerHasGenerics =>
                "hidden impl const-assoc-checker (unit-type) to assert trait and impl side counters meta information \
                has unnecessary generics",

        ImplOriginalCounterConstNotFound =>
                "hidden impl const-assoc to provide original counter const since generics are replaced by typenum \
                is not found",
        ImplOriginalCounterConstWrongIdent =>
                "hidden impl const-assoc to provide original counter const since generics are replaced by typenum \
                does not have a determinisitic random name",
        ImplOriginalCounterConstInvalidExpr =>
                "hidden impl const-assoc to provide original counter const since generics are replaced by typenum \
                has an invalid expression",
        ImplOriginalCounterConstInvalidType =>
                "hidden impl const-assoc to provide original counter const since generics are replaced by typenum \
                has an invalid const-type",
        ImplOriginalCounterConstHasGenerics =>
                "hidden impl const-assoc to provide original counter const since generics are replaced by typenum \
                has unnecessary generics",

        CounterArgsAreEmptyToProvideForGenericsIndexesMeta =>
                "provided counter arguments are empty to provide for instance counters \
                generic indexes meta hidden const-assoc",

        ImplCumulatedConstCheckerNotFound =>
                "hidden impl const-assoc to do a cumulated recursive check of all instance related \
                is not found",
        ImplCumulatedConstCheckerWrongIdent =>
                "hidden impl const-assoc to do a cumulated recursive check of all instance related \
                does not have a determinisitic random name",
        ImplCumulatedConstCheckerInvalidExpr =>
                "hidden impl const-assoc to do a cumulated recursive check of all instance related \
                has an invalid expression",
        ImplCumulatedConstCheckerNotUnitType =>
                "hidden impl const-assoc to do a cumulated recursive check of all instance related \
                does not have a unit type",
        ImplCumulatedConstCheckerHasGenerics =>
                "hidden impl const-assoc to do a cumulated recursive check of all instance related \
                has unnecessary generics",

        GlobalAssocNotFound =>
            "hidden impl assoc-type providing global access is not found",
        GlobalAssocWrongType =>
            "hidden impl assoc-type providing global access has an invalid type",

        GlobalTerminalExactNotFound =>
            "hidden impl assoc-type providing the exact global terminal counter is not found",
        GlobalTerminalExactWrongType =>
            "hidden impl assoc-type providing the exact global terminal counter has an invalid type",

        SelfTerminalAssocNotFound =>
            "hidden impl assoc-type providing the concrete terminal-associated instance is not found",
        SelfTerminalAssocWrongType =>
            "hidden impl assoc-type providing the concrete terminal-associated instance has an invalid type",
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ``````````````````````` IMPL AFFILIATE ITEMS (EXTENSION) ``````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

diagnostics!(
    space: AffiliateItemsError,

    pub(super) enum AffiliateErrors {

        RougueInstanceFoundBackNextFailed {
            fields: {
                trait_of: Ident,
                self_ty: Type,
                impl_of: ItemImpl,
            },
            msg:
                format!("rougue instance implemention found for trait `{}` for type `{}`", trait_of, self_ty.to_token_stream()),
            tags: [Unexpected],
            span: { tokens: impl_of },
            help: [
                "ensure if this impl stays on the instance trait impl chain",
                {format!("ensure if the border macro `#[{}(...)]` is required for this impl", LAST_INSTANCE_MACRO_NAME)}
            ],
            note: [
                "it is found to be rougue since its back instance did't chained it to be its respective next instance"
            ]

        },
        RougueInstanceFoundNextBackFailed {
            fields: {
                trait_of: Ident,
                self_ty: Type,
                impl_of: ItemImpl,
            },
            msg:
                format!("rougue instance implemention found for trait `{}` for type `{}`", trait_of, self_ty.to_token_stream()),
            tags: [Unexpected],
            span: { tokens: impl_of },
            help: [
                "ensure if this impl stays on the instance trait impl chain",
                {format!("ensure if the border macro `#[{}(...)]` is required for this impl", LAST_INSTANCE_MACRO_NAME)}
            ],
            note: [
                "it is found to be rougue since its next instance did't chained it to be its respective back instance"
            ]

        },
    }
);

bug_diagnostics! {
    space: AffiliateItemsError,
    bug: AFFILIATE_BUG.to_string(),

    pub(super) enum AffiliateBugs {

        ImplAffiliateInstanceNotFound =>
                "hidden impl assoc-type binded to `Self` for accesing the affiliate instance \
                is not found",
        ImplAffiliateInstanceWrongIdent =>
                "hidden impl assoc-type binded to `Self` for accesing the affiliate instance \
                does not have a determinisitic random name",
        ImplAffiliateInstanceInvalidType =>
                "hidden impl assoc-type binded to `Self` for accesing the affiliate instance \
                has an invalid type",
        ImplAffiliateInstanceHasGenerics =>
                "hidden impl assoc-type binded to `Self` for accesing the affiliate instance \
                has unnecessary generics",

        ImplReverseAffiliateCounterNotFound =>
                "hidden impl assoc-type for taking reverse affiliate typenum counters \
                is not found",
        ImplReverseAffiliateCounterWrongIdent =>
                "hidden impl assoc-type for taking reverse affiliate typenum counters \
                does not have a determinisitic random name",
        ImplReverseAffiliateCounterInvalidType =>
                "hidden impl assoc-type for taking reverse affiliate typenum counters \
                has an invalid type",
        ImplReverseAffiliateCounterHasGenerics =>
                "hidden impl assoc-type for taking reverse affiliate typenum counters \
                has unnecessary generics",

        ImplCeilAffiliateCounterNotFound =>
                "hidden impl assoc-type for taking ceil affiliate typenum counters \
                is not found",
        ImplCeilAffiliateCounterWrongIdent =>
                "hidden impl assoc-type for taking ceil affiliate typenum counters \
                does not have a determinisitic random name",
        ImplCeilAffiliateCounterInvalidType =>
                "hidden impl assoc-type for taking ceil affiliate typenum counters \
                has an invalid type",
        ImplCeilAffiliateCounterHasGenerics =>
                "hidden impl assoc-type for taking ceil affiliate typenum counters \
                has unnecessary generics",

        ImplBackAffiliateCounterNotFound =>
                "hidden impl assoc-type for taking back affiliate typenum counters \
                is not found",
        ImplBackAffiliateCounterWrongIdent =>
                "hidden impl assoc-type for taking back affiliate typenum counters \
                does not have a determinisitic random name",
        ImplBackAffiliateCounterInvalidType =>
                "hidden impl assoc-type for taking back affiliate typenum counters \
                has an invalid type",
        ImplBackAffiliateCounterHasGenerics =>
                "hidden impl assoc-type for taking back affiliate typenum counters \
                has unnecessary generics",

         ImplFloorAffiliatesCountersNotFound =>
                "hidden impl assoc-type for taking floor affiliate typenum counters \
                is not found",
        ImplFloorAffiliatesCountersWrongIdent =>
                "hidden impl assoc-type for taking floor affiliate typenum counters \
                does not have a determinisitic random name",
        ImplFloorAffiliatesCountersInvalidType =>
                "hidden impl assoc-type for taking floor affiliate typenum counters \
                has an invalid type",
        ImplFloorAffiliatesCountersHasGenerics =>
                "hidden impl assoc-type for taking floor affiliate typenum counters \
                has unnecessary generics",

        ImplNextAffiliateCounterNotFound =>
                "hidden impl assoc-type for taking next affiliate typenum counters \
                is not found",
        ImplNextAffiliateCounterWrongIdent =>
                "hidden impl assoc-type for taking next affiliate typenum counters \
                does not have a determinisitic random name",
        ImplNextAffiliateCounterInvalidType =>
                "hidden impl assoc-type for taking next affiliate typenum counters \
                has an invalid type",
        ImplNextAffiliateCounterHasGenerics =>
                "hidden impl assoc-type for taking next affiliate typenum counters \
                has unnecessary generics",

        ImplAffiliateCountersCheckerNotFound =>
                "hidden impl assoc-const checker for evaluating and finding rougue instances\
                is not found",
        ImplAffiliateCountersCheckerWrongIdent =>
                "hidden impl assoc-const checker for evaluating and finding rougue instances\
                does not have a determinisitic random name",
        ImplAffiliateCountersCheckerInvalidType =>
                "hidden impl assoc-const checker for evaluating and finding rougue instances\
                has an invalid type",
        ImplAffiliateCountersCheckerHasGenerics =>
                "hidden impl assoc-const checker for evaluating and finding rougue instances\
                has unnecessary generics",
        ImplAffiliateCountersCheckerInvalidExpr =>
                "hidden impl assoc-const checker for evaluating and finding rougue instances\
                has an invalid expression",

        ExtractedReverseAffiliateCountersAreEmpty =>
                "extracted counter arguments for reverse affilaite counter extension \
                is found empty",
        ExtractedCeilAffiliateCountersAreEmpty =>
                "extracted counter arguments for reverse affilaite counter extension \
                is found empty",
        CountersAreEmptyToFindFirstCounter =>
                "provided counter arguments are empty to find the instance's first counter",
        CountersAreEmptyToFindLastCounter =>
                "provided counter arguments are empty to find the instance's last counter",
        ExtractedNextAffiliateCountersAreEmpty =>
                "extracted counter arguments for reverse affilaite counter extension \
                is found empty",
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ````````````````````` IMPL IDENT & COLLECTIONS (EXTENSION) ````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

diagnostics!(
    space: IdentItemsErrors,

    pub(super) enum IdentErrors {

        DuplicateCounterIdentFound {
            fields: {
                lit: LitInt,
            },
            msg:
                "the identifier for the given instance counter is found conflicting with one of its previous counters",
            tags: [Unsupported],
            span: { tokens: lit },
            help: [
                "try changing the ascii identifier or fix the conflicting previous instances counter idents"
            ]

        },

        ParentCountersIdentsNotConsistent {
            fields: { },
            msg:
                "this instance's parent counters identifiers must be consistent with the previous instances, but found otherwise",
            tags: [Unsupported],
            span: { span: &Span::call_site() },
            help: [
                "provide consistent ascii identifiers for its parent counters as its required"
            ]

        },
    }
);

bug_diagnostics! {
    space: IdentItemsErrors,
    bug: IDENT_BUG.to_string(),

    pub(super) enum IdentBugs {

        ImplCounterIdentCollectionLenTypeNumNotFound =>
                "hidden impl assoc-type to provide length of current counter collection for generic arrays \
                is not found",
        ImplCounterIdentCollectionLenTypeNumWrongIdent =>
                "hidden impl assoc-type to provide length of current counter collection for generic arrays \
                does not have a determinisitic random name",
        ImplCounterIdentCollectionLenTypeNumInvalidType =>
                "hidden impl assoc-type to provide length of current counter collection for generic arrays \
                has an invalid type",
        ImplCounterIdentCollectionLenTypeNumHasGenerics =>
                "hidden impl assoc-type to provide length of current counter collection for generic arrays \
                has unnecessary generics",

        ImplCounterIdentHistoricalCollectionLenTypeNumNotFound =>
                "hidden impl assoc-type to provide length of historical counter collection for generic arrays \
                is not found",
        ImplCounterIdentHistoricalCollectionLenTypeNumWrongIdent =>
                "hidden impl assoc-type to provide length of historical counter collection for generic arrays \
                does not have a determinisitic random name",
        ImplCounterIdentHistoricalCollectionLenTypeNumInvalidType =>
                "hidden impl assoc-type to provide length of historical counter collection for generic arrays \
                has an invalid type",
        ImplCounterIdentHistoricalCollectionLenTypeNumHasGenerics =>
                "hidden impl assoc-type to provide length of historical counter collection for generic arrays \
                has unnecessary generics",

        ImplCounterIdentCollectionGenArrayNotFound =>
                "hidden impl assoc-const providing current counter identifiers generic array collection \
                is not found",
        ImplCounterIdentCollectionGenArrayWrongIdent =>
                "hidden impl assoc-const providing current counter identifiers generic array collection \
                does not have a determinisitic random name",
        ImplCounterIdentCollectionGenArrayInvalidType =>
                "hidden impl assoc-const providing current counter identifiers generic array collection \
                has an invalid type",
        ImplCounterIdentCollectionGenArrayHasGenerics =>
                "hidden impl assoc-const providing current counter identifiers generic array collection \
                has unnecessary generics",
        ImplCounterIdentCollectionGenArrayInvalidExpr =>
                "hidden impl assoc-const providing current counter identifiers generic array collection \
                has an invalid expression",

        ImplCounterIdentHashCollectionGenArrayNotFound =>
                "hidden impl assoc-const providing current counter identifier's hash generic array collection \
                is not found",
        ImplCounterIdentHashCollectionGenArrayWrongIdent =>
                "hidden impl assoc-const providing current counter identifier's hash generic array collection \
                does not have a determinisitic random name",
        ImplCounterIdentHashCollectionGenArrayInvalidType =>
                "hidden impl assoc-const providing current counter identifier's hash generic array collection \
                has an invalid type",
        ImplCounterIdentHashCollectionGenArrayHasGenerics =>
                "hidden impl assoc-const providing current counter identifier's hash generic array collection \
                has unnecessary generics",
        ImplCounterIdentHashCollectionGenArrayInvalidExpr =>
                "hidden impl assoc-const providing current counter identifier's hash generic array collection \
                has an invalid expression",

        ImplCounterIdentHistoricalCollectionGenArrayNotFound =>
                "hidden impl assoc-const providing historical counter identifiers generic array collection \
                is not found",
        ImplCounterIdentHistoricalCollectionGenArrayWrongIdent =>
                "hidden impl assoc-const providing historical counter identifiers generic array collection \
                does not have a determinisitic random name",
        ImplCounterIdentHistoricalCollectionGenArrayInvalidType =>
                "hidden impl assoc-const providing historical counter identifiers generic array collection \
                has an invalid type",
        ImplCounterIdentHistoricalCollectionGenArrayHasGenerics =>
                "hidden impl assoc-const providing historical counter identifiers generic array collection \
                has unnecessary generics",
        ImplCounterIdentHistoricalCollectionGenArrayInvalidExpr =>
                "hidden impl assoc-const providing historical counter identifiers generic array collection \
                has an invalid expression",

        ImplCounterIdentHashHistoricalCollectionGenArrayNotFound =>
                "hidden impl assoc-const providing historical counter identifier's hash generic array collection \
                is not found",
        ImplCounterIdentHashHistoricalCollectionGenArrayWrongIdent =>
                "hidden impl assoc-const providing historical counter identifier's hash generic array collection \
                does not have a determinisitic random name",
        ImplCounterIdentHashHistoricalCollectionGenArrayInvalidType =>
                "hidden impl assoc-const providing historical counter identifier's hash generic array collection \
                has an invalid type",
        ImplCounterIdentHashHistoricalCollectionGenArrayHasGenerics =>
                "hidden impl assoc-const providing historical counter identifier's hash generic array collection \
                has unnecessary generics",
        ImplCounterIdentHashHistoricalCollectionGenArrayInvalidExpr =>
                "hidden impl assoc-const providing historical counter identifier's hash generic array collection \
                has an invalid expression",

        ImplCounterIdentExpectedHashCheckerNotFound =>
                "hidden impl unit type assoc-const to check identifiers consistency on parent counters via its hashes \
                is not found",
        ImplCounterIdentExpectedHashCheckerWrongIdent =>
                "hidden impl unit type assoc-const to check identifiers consistency on parent counters via its hashes \
                does not have a determinisitic random name",
        ImplCounterIdentExpectedHashCheckerNotUnitType =>
                "hidden impl unit type assoc-const to check identifiers consistency on parent counters via its hashes \
                has an invalid type",
        ImplCounterIdentExpectedHashCheckerHasGenerics =>
                "hidden impl unit type assoc-const to check identifiers consistency on parent counters via its hashes \
                has unnecessary generics",
        ImplCounterIdentExpectedHashCheckerInvalidExpr =>
                "hidden impl unit type assoc-const to check identifiers consistency on parent counters via its hashes \
                has an invalid expression",
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ``````````````````````` LAST INSTANCE IMPL (REPLACEMENT) ``````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

diagnostics!(
    space: LastInstanceErrors,

    pub(super) enum LastErrors {

        InstanceImplMacroNotApplied {
            fields: {
                exp: Ident,
            },
            msg:
                format!("expected to have a outer attribute #[{}(...)] to this attribute", TRAIT_IMPL_MACRO_NAME),
            tags: [Unexpected],
            span: { span: &Span::call_site() },
            help: [
                {format!("if misplaced, keep $[{}(..)] outer attribute and #[{}(..)] inner attribute", TRAIT_IMPL_MACRO_NAME, LAST_INSTANCE_MACRO_NAME)}
            ],
            note: [
                "this originated because the outer attribute produces associated items which this attribute expects",
                format!("expected ident `{}` from outer attribute, found none", exp)
            ]

        },

        NotACounterGenericIndex {
            fields: {
                invalid: LitInt,
                available: String,
            },
            msg:
                "not a valid instance counter generic index for declaring its boundary",
            tags: [Unexpected],
            span: { tokens: invalid },
            help: [
                {format!("available indexes are {}", available)}
            ],
            note: [
                "choose any one (or multiple) of available indexes",
                "from the given indexes, the most parent counter is \
                chosen and all its child counters are also marked as boundaries",
            ]
        },

        AnExistingAffiliatesCountersTypeUnavailableToReplace {
            fields: {
                ident: Ident,
            },
            msg:
                "hidden impl assoc-type of a affiliate counter is expected to be replaced, but not found",
            tags: [Bug],
            span: { span: &Span::call_site() },
            note: [
                format!("expected associated type of ident `{}`", ident),
                LAST_BUG.to_string(),
                LOGICAL_BUG.to_string(),
                REPORT_BUG.to_string(),
            ]
        },

        AnExistingAssocConstUnavailableToReplace {
            fields: {
                ident: Ident,
            },
            msg:
                "a hidden impl assoc-const is expected to be replaced, but not found",
            tags: [Bug],
            span: { span: &Span::call_site() },
            note: [
                format!("expected associated const of ident `{}`", ident),
                LAST_BUG.to_string(),
                LOGICAL_BUG.to_string(),
                REPORT_BUG.to_string(),
            ]
        }

    }
);

bug_diagnostics! {
    space: LastInstanceErrors,
    bug: IDENT_BUG.to_string(),

    pub(super) enum LastBugs {

        CountersIndexesMetaNotARefExpr =>
                "hidden impl assoc-const that reflects counters generic indexes via \
                instance impl outer attribute's is not a reference expression",
        CountersIndexesMetaNotArrayExpr =>
                "hidden impl assoc-const that reflects counters generic indexes via \
                instance impl outer attribute's is not a array expression",
        CountersIndexesMetaNotLitArrayExpr =>
                "hidden impl assoc-const that reflects counters generic indexes via \
                instance impl outer attribute's is not a array expression containing literals",
        CountersIndexesMetaNotLitIntArrayExpr =>
                "hidden impl assoc-const that reflects counters generic indexes via \
                instance impl outer attribute's is not a array expression containing integer literals",


        OriginalCounterAssocNotLitExpr =>
                "hidden impl assoc-const that reflects the original counter's value via\
                instance impl outer attribute's is not a literal expression",
        OriginalCounterAssocNotLitIntExpr =>
                "hidden impl assoc-const that reflects the original counter's value via\
                instance impl outer attribute's is not a integer literal expression",

        CollectedCounterArgsAssocIsEmpty =>
                "collected counter arguments via hidden original counters assoc-consts are empty",

        CounterArgsPostValidateOnEmptyArgs =>
                "post validation of extracted counter args from original counters cannot
                be done on empty counter args",
        RegainingOriginalCounterFailedOnPostValidate =>
                "post validation of extracted counter args from original counters cannot
                find the original counters associated constants",
        RegainedOriginalCounterExprIsNotLit =>
                "post validation of extracted counter args from original counters is
                inconsistent as its expressions are not literal",
        RegainedOriginalCounterExprIsNotLitInt =>
                "post validation of extracted counter args from original counters is
                inconsistent as its expressions are not integer literal",
        RegainedOriginalCounterExprNotMatch =>
                "post validation of extracted counter args from original counters is
                inconsistent as its expressions are not matching",

        CounterArgsAssocIsEmptyForFirstCounterAccess =>
                "collected counter arguments via hidden original counters assoc-consts are empty \
                extraction phase is violated, cannot proceed with first counter access",

        LastInstanceArgNotFoundInCountersMeta =>
                "the last instance canonical counter's index extracted is not an actual \
                instance counter found in the meta",
        LastInstanceArgFromDuplicateIndexes =>
                "the last instance canonical counter's index extracted is from duplicated indexes \
                provided through the macro arguments",
        LastInstanceArgFromNoIndexesButNotFirstCounter =>
                "the last instance canonical counter's index extracted should be the first counter \
                as fallback of no macro arguments but it ain't",
        LastInstanceArgNotFromCounterIndexes =>
                "the last instance canonical counter's index extracted is not from one of the \
                indexes provided via macro arguments",
        InvalidLastInstanceArgGiven =>
                "the last instance argument extracted is not consistent with available
                counter indexes arguments to proceed",

        BestLastInstanceArgNotFound =>
                "the best last instance argument of a counter's index cannot be computed",

        FloorAffiliatesCountersReplacementNotFound =>
                "hidden impl assoc-type replaced for specifying boundary via a floor affiliate counter \
                is not found",
        FloorAffiliatesCountersReplacementWrongIdent =>
                "hidden impl assoc-type replaced for specifying boundary via a floor affiliate counter \
                does not have a determinisitic random name",
        FloorAffiliatesCountersReplacementInvalidType =>
                "hidden impl assoc-type replaced for specifying boundary via a floor affiliate counter \
                has an invalid type",
        FloorAffiliatesCountersReplacementHasGenerics =>
                "hidden impl assoc-type replaced for specifying boundary via a floor affiliate counter \
                has unnecessary generics",



        NextAffiliatesCountersReplacementNotFound =>
                "hidden impl assoc-type replaced for providing impl side accurate next instance counters \
                is not found",
        NextAffiliatesCountersReplacementWrongIdent =>
                "hidden impl assoc-type replaced for providing impl side accurate next instance counters \
                does not have a determinisitic random name",
        NextAffiliatesCountersReplacementInvalidType =>
                "hidden impl assoc-type replaced for providing impl side accurate next instance counters \
                has an invalid type",
        NextAffiliatesCountersReplacementHasGenerics =>
                "hidden impl assoc-type replaced for providing impl side accurate next instance counters \
                has unnecessary generics",

        LastInstanceArgIndexCannotFetchItsConstLit =>
                "the last instance canonical counter index extracted but the index's \
                actual counter argument not able to access",

        BackAffiliatesCountersReplacementNotFound =>
                "hidden impl assoc-type replaced for providing impl side accurate back instance counters \
                is not found",
        BackAffiliatesCountersReplacementWrongIdent =>
                "hidden impl assoc-type replaced for providing impl side accurate back instance counters \
                does not have a determinisitic random name",
        BackAffiliatesCountersReplacementInvalidType =>
                "hidden impl assoc-type replaced for providing impl side accurate back instance counters \
                has an invalid type",
        BackAffiliatesCountersReplacementHasGenerics =>
                "hidden impl assoc-type replaced for providing impl side accurate back instance counters \
                has unnecessary generics",

        AffiliateCountersCheckerReplacementNotFound =>
                "hidden impl assoc-const replaced for asserting terminal instance in the canonical instance chain \
                is not found",
        AffiliateCountersCheckerReplacementWrongIdent =>
                "hidden impl assoc-const replaced for asserting terminal instance in the canonical instance chain \
                does not have a determinisitic random name",
        AffiliateCountersCheckerReplacementInvalidExpr =>
                "hidden impl assoc-const replaced for asserting terminal instance in the canonical instance chain \
                has an invalid expression",
        AffiliateCountersCheckerReplacementInvalidType =>
                "hidden impl assoc-const replaced for asserting terminal instance in the canonical instance chain \
                has an invalid const-type",
        AffiliateCountersCheckerReplacementHasGenerics =>
                "hidden impl assoc-const replaced for asserting terminal instance in the canonical instance chain \
                has unnecessary generics",
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ``````````````````````` POST INSTANCE IMPL (REPLACEMENT) ``````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

bug_diagnostics! {
    space: PostInstanceImplErrors,
    bug: POST_BUG.to_string(),

    pub(super) enum PostBugs {
        CounterAttrNotRemoved =>
            "counter ident attribute removal didn't followed as part of post cleanup phase",
        SelfBoundsAttrNotRemoved =>
            "instance impl Self type's bounds attribute removal didn't followed as part of post cleanup phase",
        LastInstanceAttrNotRemoved =>
            "last instance proc macro attribute removal didn't followed as part of post cleanup phase",
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ````````````````````````````` INSTANCE IMPL ADDONS ````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

diagnostics!(
    space: InstanceImplAddonErrors,

    pub(super) enum AddonErrors {

        IdentifierUnavailableForCounterIndex {
            fields: {
                int: LitInt,
                index: usize,
                impl_of: ItemImpl
            },
            msg:
                "instance ident assoc-const unavailable for this counter generic index",
            tags: [Unsupported],
            span: { tokens: int },
            help: [
                {format!("expected `#[counter({index})]` on its counter's identifier in the impl")},
                {
                    tokens: impl_of,
                    msg: format!("expected `#[counter({index})]` on its counter's identifier inside this impl"),
                }
            ],
            note: [
                "this is to identify which byte string assoc const is the counter's"
            ]
        },
    }
);

bug_diagnostics! {
    space: InstanceImplAddonErrors,
    bug: ADDON_BUG.to_string(),

    pub(super) enum AddonBugs {
        ExtractedIdentifierRawExprsLengthMismatch =>
            "the extracted counter identifiers raw byte string exprs length mismatch during validation",
        ExtractedIdentifierRawExprIsInconsistent =>
            "the extracted counter identifier raw byte string expr is inconsistent during validation",

        InvalidCounterAccessImplIdent =>
            "addon counter access trait impl's trait ident is invalid",
        CounterAccessImplAddonNotFound =>
            "addon counter access trait impl is not found",
        InvalidGlobalTypePathForCounterAccess =>
            "addon counter access trait impl's trait path is invalid",
        InvalidCounterAccessImplPathSegmentsLen =>
            "addon counter access trait impl's trait path's segments length is invalid",
        InvalidCounterAccessImplPathSegment =>
            "addon counter access trait impl's trait path's one of the segment is invalid",
        InvalidCounterAccessImplTraitSegment =>
            "addon counter access trait impl's trait's segments is invalid",
        CounterAccessImplHasInvalidGenericArgs =>
            "addon counter access trait impl's trait's generic arguments are invalid kind",
        CounterAccessImplExpectedGenericArgsLenInvalid =>
            "addon counter access trait impl's trait's generic arguments length is invalid",
        CounterAccessImplOriginalTypeGenericArgUnavailable =>
            "addon counter access trait impl's trait's generic providing \
            original instance type is not found",
        CounterAccessImplOriginalTypeNotTypeGenericArg =>
            "addon counter access trait impl's trait's generic providing \
            original instance type is not a type generic",
        CounterAccessImplOriginalTypeGenericArgInvalid =>
            "addon counter access trait impl's trait's type generic providing \
            original instance type is invalid",
        CounterAccessImplExpectedConstGenericArgsLenInvalid =>
            "addon counter access trait impl's trait's const generic arguments length is invalid",
        CounterAccessImplExpectedConstGenericFoundElse =>
            "addon counter access trait impl's trait's generic argument expected \
            a const generic, but found else",
        CounterAccessImplHashExprInvalid =>
            "addon counter access trait impl's trait's const generic providing \
            counter identifier hash is invalid",
        CounterAccessImplOriginalTypeGenericWhereClausesInvalid =>
            "addon counter access trait impl's where clause for original type's
            one of generic bounds is invalid",
        CounterAccessImplOriginalTypeGenericParamsInvalid =>
            "addon counter access trait impl's generic params for original type's
            generics is invalid",
        CounterAccessImplAddonWithNoAssocItems =>
            "addon counter access trait impl has no associated items at all",
        CounterAccessImplCounterAssocNotType =>
            "addon counter access trait impl has the first non-type \
            associated item providing the instance's counters typenum",
        CounterAccessImplAddonTypeAssocIdentInvalid =>
            "addon counter access trait impl's only type associated item \
            providing the instance's counters typenum has invalid ident",
        CounterAccessImplInvalidCounterTy =>
            "addon counter access trait impl's only type associated item \
            providing the instance's counters typenum has invalid type",
        CounterAccessImplHasExcessAssocs =>
            "addon counter access trait impl has excess associated items",

        InvalidIdentHashLenAccessImplIdent =>
            "addon ident hash len access trait impl's trait ident is invalid",
        IdentHashLenAccessImplAddonNotFound =>
            "addon ident hash len access trait impl is not found",
        IdentHashLenAccessImplAddonMissing =>
            "addon ident hash len access trait an impl is missing",
        InvalidGlobalTypePathForIdentHashLenAccess =>
            "addon ident hash len access trait impl's trait path is invalid",
        InvalidIdentHashLenAccessImplPathSegmentsLen =>
            "addon ident hash len access trait impl's trait path's segments length is invalid",
        InvalidIdentHashLenAccessImplPathSegment =>
            "addon ident hash len access trait impl's trait path's one of the segment is invalid",
        InvalidIdentHashLenAccessImplTraitSegment =>
            "addon ident hash len access trait impl's trait's segments is invalid",
        IdentHashLenAccessImplHasInvalidGenericArgs =>
            "addon ident hash len access trait impl's trait's generic arguments are invalid kind",
        IdentHashLenAccessImplExpectedGenericArgsLenInvalid =>
            "addon ident hash len access trait impl's trait's generic arguments length is invalid",
        IdentHashLenAccessImplOriginalTypeGenericArgUnavailable =>
            "addon ident hash len access trait impl's trait's generic providing \
            original instance type is not found",
        IdentHashLenAccessImplOriginalTypeNotTypeGenericArg =>
            "addon ident hash len access trait impl's trait's generic providing \
            original instance type is not a type generic",
        IdentHashLenAccessImplOriginalTypeGenericArgInvalid =>
            "addon ident hash len access trait impl's trait's type generic providing \
            original instance type is invalid",
        IdentHashLenAccessImplExpectedConstGenericArgsLenInvalid =>
            "addon ident hash len access trait impl's trait's const generic arguments length is invalid",
        IdentHashLenAccessImplExpectedConstGenericFoundElse =>
            "addon ident hash len access trait impl's trait's generic argument expected \
            a const generic, but found else",
        IdentHashLenAccessImplHashExprInvalid =>
            "addon ident hash len access trait impl's trait's const generic providing \
            counter identifier hash is invalid",
        IdentHashLenAccessImplOriginalTypeGenericWhereClausesInvalid =>
            "addon ident hash len access trait impl's where clause for original type's
            one of generic bounds is invalid",
        IdentHashLenAccessImplOriginalTypeGenericParamsInvalid =>
            "addon ident hash len access trait impl's generic params for original type's
            generics is invalid",
        IdentHashLenAccessImplAddonWithNoAssocItems =>
            "addon ident hash len access trait impl has no associated items at all",
        IdentHashLenAccessImplLengthAssocNotType =>
            "addon ident hash len access trait impl has the first non-type \
            associated item providing the typenum array length",
        IdentHashLenAccessImplAddonTypeAssocIdentInvalid =>
            "addon ident hash len access trait impl's only type associated item \
            providing the typenum array length",
        IdentHashLenAccessGivenCounterIndexInvalid =>
            "addon ident hash len access trait impl's counter index generic literal \
            is invalid as there's no counter value literal found",
        IdentHashLenAccessImplInvalidLengthTy =>
            "addon ident hash len access trait impl's only type associated item \
            providing the typenum array length has invalid type",
        IdentHashLenAccessImplHasExcessAssocs =>
            "addon ident hash len access trait impl has excess associated items",
        IdentHashLenAccessCounterIndexGenericArgUnavailable =>
            "addon ident hash len access trait impl's trait's generic providing \
            the instance counters generic index as generic arg is not found",
        IdentHashLenAccessCounterIndexNotConstGenericArg =>
            "addon ident hash len access trait impl's trait's generic providing \
            the instance counters generic index as generic arg is not a const generic",
        IdentHashLenAccessCounterIndexNotConstExprLit =>
            "addon ident hash len access trait impl's trait's type generic providing \
            the instance counters generic index as generic arg is not a const generic literal",
        IdentHashLenAccessCounterIndexNotConstLitInt =>
            "addon ident hash len access trait impl's trait's type generic providing \
            the instance counters generic index as generic arg is not a const generic integer literal",

        InvalidTerminalAccessImplIdent =>
            "last instance addon terminal access trait impl's trait ident is invalid",
        InvalidGlobalTypePathForTerminalAccess =>
            "last instance addon terminal access trait impl's trait path is invalid",
        InvalidTerminalAccessImplPathSegmentsLen =>
            "last instance addon terminal access trait impl's trait path's segments length is invalid",
        InvalidTerminalAccessImplPathSegment =>
            "last instance addon terminal access trait impl's trait path's one of the segment is invalid",
        InvalidTerminalAccessImplTraitSegment =>
            "last instance addon terminal access trait impl's trait's segments is invalid",
        TerminalAccessImplHasInvalidGenericArgs =>
            "last instance addon terminal access trait impl's trait's generic arguments are invalid kind",
        TerminalAccessImplExpectedGenericArgsLenInvalid =>
            "last instance addon terminal access trait impl's trait's generic arguments length is invalid",
        TerminalAccessImplOriginalTypeGenericArgUnavailable =>
            "last instance addon terminal access trait impl's trait's generic providing \
            original instance type is not found",
        TerminalAccessImplOriginalTypeNotTypeGenericArg =>
            "last instance addon terminal access trait impl's trait's generic providing \
            original instance type is not a type generic",
        TerminalAccessImplOriginalTypeGenericArgInvalid =>
            "last instance addon terminal access trait impl's trait's type generic providing \
            original instance type is invalid",
        TerminalAccessImplOriginalTypeGenericWhereClausesInvalid =>
            "last instance addon terminal access trait impl's where clause for original type's \
            one of generic bounds is invalid",
        TerminalAccessImplOriginalTypeGenericParamsInvalid =>
            "last instance addon terminal access trait impl's generic params for original type's \
            generics is invalid",
        TerminalAccessImplAddonWithNoAssocItems =>
            "last instance addon terminal access trait impl has no associated items at all",
        TerminalAccessImplCounterAssocNotType =>
            "last instance addon terminal access trait impl has the first non-type \
            associated item providing the terminal counter typenum",
        TerminalAccessImplAddonTypeAssocIdentInvalid =>
            "last instance addon terminal access trait impl's only type associated item \
            providing the terminal counter typenum has invalid ident",
        TerminalAccessImplInvalidCounterTy =>
            "last instance addon terminal access trait impl's only type associated item \
            providing the terminal counter typenum has invalid type",
        TerminalAccessImplHasExcessAssocs =>
            "last instance addon terminal access trait impl has excess associated items",
        TerminalAccessImplNotFound =>
            "last instance addon terminal access trait impl is not found",
        TerminalAccessImplAttemptedForNonTerminalCounter =>
            "attempted to generate last instance addon terminal access trait impl for a non-terminal counter",

        TerminalCheckerConstAttemptedForNonTerminalCounter =>
            "attempted to generate last instance addon terminal checker const for a non-terminal counter",
        TerminalCheckerConstInvalidIdent =>
            "last instance addon terminal checker const has invalid ident",
        TerminalCheckerConstAttemptedForGenericImpl =>
            "attempted to generate last instance addon terminal checker const for generic impl",
        TerminalCheckerConstInvalidExpr =>
            "last instance addon terminal checker const has invalid expr",
        TerminalCheckerConstNotFound =>
            "expected last instance addon terminal checker const is not found",


        LastInstanceCounterInvalid =>
            "the provided last instance counter does not exist among the instance counters",

        BoundaryAccessHashOfIndexFromHashExprsUnavailable =>
            "addon boundary access trait impl's expected boundary hash expression is unavailable",
        InvalidGlobalTypePathForBoundaryAccess =>
            "addon boundary access trait impl's trait path is invalid",
        InvalidBoundaryAccessImplPathSegmentsLen =>
            "addon boundary access trait impl's trait path's segments length is invalid",
        InvalidBoundaryAccessImplPathSegment =>
            "addon boundary access trait impl's trait path's one of the segment is invalid",
        InvalidBoundaryAccessImplTraitSegment =>
            "addon boundary access trait impl's trait's segments is invalid",
        BoundaryAccessImplHasInvalidGenericArgs =>
            "addon boundary access trait impl's trait's generic arguments are invalid kind",
        BoundaryAccessImplOriginalTypeGenericArgUnavailable =>
            "addon boundary access trait impl's trait's generic providing \
            original instance type is not found",
        BoundaryAccessImplOriginalTypeNotTypeGenericArg =>
            "addon boundary access trait impl's trait's generic providing \
            original instance type is not a type generic",
        BoundaryAccessImplOriginalTypeGenericArgInvalid =>
            "addon boundary access trait impl's trait's type generic providing \
            original instance type is invalid",
        BoundaryAccessHashExprArgsLenInvalid =>
            "addon boundary access trait impl's trait's hash expression generic arguments length is invalid",
        BoundaryAccessHashExprArgNotConst =>
            "addon boundary access trait impl's trait's hash expression generic argument expected \
            a const generic, but found else",
        BoundaryAccessHashExprArgConstInvalid =>
            "addon boundary access trait impl's trait's hash expression const generic is invalid",
        BoundaryAccessImplOriginalTypeGenericWhereClausesInvalid =>
            "addon boundary access trait impl's where clause for original type's \
            one of generic bounds is invalid",
        BoundaryAccessImplOriginalTypeGenericParamsInvalid =>
            "addon boundary access trait impl's generic params for original type's \
            generics is invalid",
        BoundaryAccessImplAddonWithNoAssocItems =>
            "addon boundary access trait impl has no associated items at all",
        BoundaryAccessImplHasExcessAssocs =>
            "addon boundary access trait impl has excess associated items",
        BoundaryAccessImplCounterAssocTypeNotFound =>
            "addon boundary access trait impl has no associated type providing the boundary typenum",
        BoundaryAccessImplAddonTypeAssocIdentInvalid =>
            "addon boundary access trait impl's associated type providing the boundary typenum has invalid ident",
        BoundaryAccessImplInvalidCounterTy =>
            "addon boundary access trait impl's associated type providing the boundary typenum has invalid type",
        BoundaryAccessImplHashAssocConstNotFound =>
            "addon boundary access trait impl has no associated const providing the boundary hash",
        BoundaryAccessImplAddonConstAssocIdentInvalid =>
            "addon boundary access trait impl's associated const providing the boundary hash has invalid ident",
        BoundaryAccessImplInvalidHashConstTy =>
            "addon boundary access trait impl's associated const providing the boundary hash has invalid type",
        BoundaryAccessImplInvalidConstAssocExpr =>
            "addon boundary access trait impl's associated const providing the boundary hash has invalid expr",
        BoundaryAccessImplNotFound =>
            "expected addon boundary access trait impl is not found",

        OnSetAccessHashOfIndexFromHashExprsUnavailable =>
            "addon onset access trait impl's expected onset hash expression is unavailable",
        InvalidGlobalTypePathForOnSetAccess =>
            "addon onset access trait impl's trait path is invalid",
        InvalidOnSetAccessImplPathSegmentsLen =>
            "addon onset access trait impl's trait path's segments length is invalid",
        InvalidOnSetAccessImplPathSegment =>
            "addon onset access trait impl's trait path's one of the segment is invalid",
        InvalidOnSetAccessImplTraitSegment =>
            "addon onset access trait impl's trait's segments is invalid",
        OnSetAccessImplHasInvalidGenericArgs =>
            "addon onset access trait impl's trait's generic arguments are invalid kind",
        OnSetAccessImplOriginalTypeGenericArgUnavailable =>
            "addon onset access trait impl's trait's generic providing \
            original instance type is not found",
        OnSetAccessImplOriginalTypeNotTypeGenericArg =>
            "addon onset access trait impl's trait's generic providing \
            original instance type is not a type generic",
        OnSetAccessImplOriginalTypeGenericArgInvalid =>
            "addon onset access trait impl's trait's type generic providing \
            original instance type is invalid",
        OnSetAccessHashExprArgsLenInvalid =>
            "addon onset access trait impl's trait's hash expression generic arguments length is invalid",
        OnSetAccessHashExprArgNotConst =>
            "addon onset access trait impl's trait's hash expression generic argument expected \
            a const generic, but found else",
        OnSetAccessHashExprArgConstInvalid =>
            "addon onset access trait impl's trait's hash expression const generic is invalid",
        OnSetAccessImplOriginalTypeGenericWhereClausesInvalid =>
            "addon onset access trait impl's where clause for original type's \
            one of generic bounds is invalid",
        OnSetAccessImplOriginalTypeGenericParamsInvalid =>
            "addon onset access trait impl's generic params for original type's \
            generics is invalid",
        OnSetAccessImplAddonWithNoAssocItems =>
            "addon onset access trait impl has no associated items at all",
        OnSetAccessImplHasExcessAssocs =>
            "addon onset access trait impl has excess associated items",
        OnSetAccessImplCounterAssocTypeNotFound =>
            "addon onset access trait impl has no associated type providing the onset typenum",
        OnSetAccessImplAddonTypeAssocIdentInvalid =>
            "addon onset access trait impl's associated type providing the onset typenum has invalid ident",
        OnSetAccessImplInvalidCounterTy =>
            "addon onset access trait impl's associated type providing the onset typenum has invalid type",
        OnSetAccessImplHashAssocConstNotFound =>
            "addon onset access trait impl has no associated const providing the onset hash",
        OnSetAccessImplAddonConstAssocIdentInvalid =>
            "addon onset access trait impl's associated const providing the onset hash has invalid ident",
        OnSetAccessImplInvalidHashConstTy =>
            "addon onset access trait impl's associated const providing the onset hash has invalid type",
        OnSetAccessImplInvalidConstAssocExpr =>
            "addon onset access trait impl's associated const providing the onset hash has invalid expr",
        OnSetAccessImplNotFound =>
            "expected addon onset access trait impl is not found",
    }
}
