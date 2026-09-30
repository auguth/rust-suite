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
// ````````````````````````````` TRAIT INSTANCE ERROR ````````````````````````````
// ===============================================================================

//! Diagnostics emitted by the instance-trait proc-macro pipeline.
//!
//! Defines error spaces, structured diagnostics, and internal invariant
//! violations covering counter extraction, metadata generation, typenum
//! transformation, affiliate generation, and identifier reflection.

// ===============================================================================
// ``````````````````````````````````` IMPORTS ```````````````````````````````````
// ===============================================================================

// --- Proc-Suite ---
use proc_suite::{
    IdentList, IntList, LOGICAL_BUG, REPORT_BUG, bug_diagnostics, diagnostics, error_spaces,
};

// --- Local Crate ---
use crate::{VALID_COUNTER_TYPES, errors::MAINTAINERS};

// --- Const Utils ---
use const_format::concatcp;

// --- Proc-Macro Utils ---
use proc_macro2::Span;
use syn::{ConstParam, Expr, Generics, Ident, LitInt, TraitItem, Type, TypeParam};

// ===============================================================================
// `````````````````````````````````` CONSTANTS ``````````````````````````````````
// ===============================================================================

pub(super) const EXTRACTION_BUG: &'static str =
    "instance trait's counter params extraction-phase didn't follow invariants";

pub(super) const META_EXTENSION_BUG: &'static str =
    "instance trait's counters meta (reflection) extension phase didn't follow invariants";

pub(super) const SUM_TYPE_EXTENSION_BUG: &'static str =
    "instance trait's sum-types extension phase didn't follow invariants";

pub(super) const TYPENUM_TRANSFORM_BUG: &'static str =
    "instance trait's counter params transformation-phase to type-num generics 
    didn't follow invariants";

pub(super) const AFFILIATE_BUG: &'static str =
    "instance trait's affiliates extension (assoc-types) and transformation 
    (trait's post-predicates) didn't follow invariants";

pub(super) const IDENTS_BUG: &'static str =
    "instance trait's identifiers collections & lengths extension (assoc-types) and transformation 
    (trait's post-predicates) didn't follow invariants";

pub(super) const VALID_CONST_PARAMS: &'static str =
    "either provide valid const-generic param identifier/s or its positional index/es.";

pub(super) const ALLOWED_CONST_GEN_TY: &'static str = concatcp!(
    "allowed const generic types are: \"",
    VALID_COUNTER_TYPES,
    "\"."
);

pub(super) const ENSURE_IDENTICAL_CONST_PARAMS: &'static str =
    "ensure given identifiers/indexes unsigned types are all identical and unsigned";

// ===============================================================================
// ````````````````````````````````` ERROR-SPACES ````````````````````````````````
// ===============================================================================

error_spaces! {
    space: "INSTANCE_TRAIT",
    maintain: MAINTAINERS,

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // `````````````````````````` COUNTER PARAM EXTRACTION ```````````````````````````
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

    pub(super) enum CounterParamError {
        range: 0..=100,
        variants: {
            IdentsReCheckLenDifference,
            IdentsReCheckWrongIdent,
            IntsReCheckLenDifference,
            IntsReCheckWrongIndex,
            IdentNotConstGeneric,
            TraitNeedsCounterGenerics,
            OutOfBoundsGenericIndex,
            IndexNotConstGeneric,
            TraitNeedsLeadConstGenerics,
            ExpConstGeneric,
            ExpNonDefaultConstGeneric,
            ExpCountersTy,
            ExpConsistentCountersTy,
            ParamsReCheckWrongIdent,
            ParamsReCheckWrongType,
            ParamsReCheckOutOfBounds,
            DuplicateRawCounterIdents,
            DuplicateCounterIndexes,

            CountersNonSameTypesPassed,
            NoValidCountersProvided,
        }
    }

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // ``````````````````````````` META ITEMS (REFLECTION) ```````````````````````````
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

    pub(super) enum MetaItemsErrors {
        range: 101..=300,
        variants: {
            CountersTyMetaNotFound,
            CountersTyMetaWrongIdent,
            CountersTyMetaExprNotFound,
            CountersTyMetaInvalidType,
            CountersTyMetaInvalidExpr,
            CountersTyMetaHasGenerics,

            CountersTyNotFound,
            CountersTyWrongIdent,
            CountersTyHasGenerics,
            CountersTyHasBounds,

            CountersTyCheckerNotFound,
            CountersTyCheckerWrongIdent,
            CountersTyCheckerNotUnitType,
            CountersTyCheckerHasExpr,
            CountersTyCheckerHasGenerics,

            CountersGenericsMetaNotFound,
            CountersGenericsMetaWrongIdent,
            CountersGenericsMetaExprNotFound,
            CountersGenericsMetaInvalidType,
            CountersGenericsMetaInvalidExpr,
            CountersGenericsMetaHasGenerics,

            CountersGenericsIndexesMetaNotFound,
            CountersGenericsIndexesMetaWrongIdent,
            CountersGenericsIndexesMetaInvalidType,
            CountersGenericsIndexesMetaHasExpr,
            CountersGenericsIndexesMetaHasGenerics,

            CountersLenMetaNotFound,
            CountersLenMetaWrongIdent,
            CountersLenMetaInvalidType,
            CountersLenMetaHasGenerics,
            CountersLenMetaHasExpr,

            CountersGenericsCheckerNotFound,
            CountersGenericsCheckerWrongIdent,
            CountersGenericsCheckerNotUnitType,
            CountersGenericsCheckerHasExpr,
            CountersGenericsCheckerHasGenerics,

            InstanceTraitDocsRawAttrMissing,
            InstanceTraitDocsMacroInlineAttrMissing,
            InstanceTraiDocsNotDocsAttr,
            InstanceTraiDocsAttrNotLiteralExpr,
            InstanceTraiDocsAttrNotStrLiteral,
            InstanceTraitDocsInvalidRawAttr,
            InstanceTraitDocsInvalidMacroInlineAttr,

            OriginalCounterConstNotFound,
            OriginalCounterConstWrongIdent,
            OriginalCounterConstWrongType,
            OriginalCounterConstHasExpr,
            OriginalCounterConstHasGenerics,

            CounterConstAnnotatedExtractionFailed,

            CumulatedConstCheckerNotFound,
            CumulatedConstCheckerWrongIdent,
            CumulatedConstCheckerNotUnitType,
            CumulatedConstCheckerHasExpr,
            CumulatedConstCheckerHasGenerics,

            CountersParamMetaExtractionGenericsMetaNotFound,
            CountersParamMetaExtractionCountersTyMetaNotFound,
            CountersParamMetaExtractionCountersTyMetaDefaultExprNotFound,
            CountersParamMetaExtractionCountersTyMetaDefaultExprNotLitExpr,
            CountersParamMetaExtractionCountersTyMetaDefaultExprNotStrLit,

            CountersParamMetaExtractionGenericsMetaDefaultExprNotFound,
            CountersParamMetaExtractionGenericsMetaDefaultExprNotArray,
            CountersParamMetaExtractionGenericsMetaDefaultExprArrayElemNotTuple,
            CountersParamMetaExtractionGenericsMetaDefaultExprArrayElemTupleWrongLen,
            CountersParamMetaExtractionGenericsMetaDefaultExprArrayElemTupleCounterGenericIndexNotFound,
            CountersParamMetaExtractionGenericsMetaDefaultExprArrayElemTupleCounterGenericIndexNotExprLit,
            CountersParamMetaExtractionGenericsMetaDefaultExprArrayElemTupleCounterGenericIndexNotLitInt,
            CountersParamMetaExtractionGenericsMetaDefaultExprArrayElemTupleCounterGenericIdentNotFound,
            CountersParamMetaExtractionGenericsMetaDefaultExprArrayElemTupleCounterGenericIdentNotExprLit,
            CountersParamMetaExtractionGenericsMetaDefaultExprArrayElemTupleCounterGenericIdentNotLitStr,

            CountersParamMetaExtractionInconsistentWithGenericIdentArgs,
            CountersParamMetaExtractionInconsistentWithGenericIndexArgs,

            WhereClauseNotFoundForMarkerBound,
            MarkerBoundPredNotFound,

        }
    }

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // `````````````````````````````` TYPE NUM COUNTERS ``````````````````````````````
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

    pub(super) enum TypeNumCountersErrors {
        range: 301..=400,
        variants: {
            InvalidCounterParamIndex,
            CounterParamNotTypeGeneric,
            CounterTypeGenericInvalidIdent,
            CounterTypeGenericHasBounds,
            CounterTypeGenericHasDefault,

            CounterParamsAreEmpty,

            TypeGenericRequiresDefault,
            ConstGenericRequiresDefault,

            InvalidFirstCounter,

            Maximum4Counters,
        }

    }

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // ````````````````````````````````` AFFILIATES ``````````````````````````````````
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

    pub(super) enum AffiliatesErrors {
        range: 401..=700,
        variants: {
            ReverseAffiliateCountersMissing,
            ReverseAffiliateCountersHasGenerics,
            ReverseAffiliateCountersInvalidIdent,
            ReverseAffiliateCountersBoundNotFound,

            ReverseAffiliateInstanceMissing,
            ReverseAffiliateInstanceHasGenerics,
            ReverseAffiliateInstanceInvalidIdent,
            ReverseAffiliateInstanceBoundNotFound,
            ReverseAffiliateInstanceBoundInvalid,

            CountersForReverseAffiliatePredicateMissing,
            CountersForReverseAffiliatePredicateHasLifetimes,
            CountersForReverseAffiliateNotTypePredicate,
            CountersForReverseAffiliatePredicateWrongBoundedTy,
            CountersForReverseAffiliatePredicateBoundNotFound,

            PostReverseAffiliateTraitWhereClauseMissing,

            NextAffiliateCountersMissing,
            NextAffiliateCountersHasGenerics,
            NextAffiliateCountersInvalidIdent,
            NextAffiliateCountersBoundNotFound,

            NextAffiliateInstanceMissing,
            NextAffiliateInstanceHasGenerics,
            NextAffiliateInstanceInvalidIdent,
            NextAffiliateInstanceBoundNotFound,
            NextAffiliateInstanceBoundInvalid,

            PostNextAffiliateTraitWhereClauseMissing,

            BackAffiliateCountersMissing,
            BackAffiliateCountersHasGenerics,
            BackAffiliateCountersInvalidIdent,
            BackAffiliateCountersBoundNotFound,

            BackAffiliateInstanceMissing,
            BackAffiliateInstanceHasGenerics,
            BackAffiliateInstanceInvalidIdent,
            BackAffiliateInstanceBoundNotFound,
            BackAffiliateInstanceBoundInvalid,

            PostBackAffiliateTraitWhereClauseMissing,

            FloorAffiliatesCountersMissing,
            FloorAffiliatesCountersHasGenerics,
            FloorAffiliatesCountersInvalidIdent,
            FloorAffiliatesCountersBoundNotFound,

            FloorAffiliatesInstancesMissing,
            FloorAffiliatesInstancesHasGenerics,
            FloorAffiliatesInstancesInvalidIdent,
            FloorAffiliatesInstancesBoundNotFound,
            FloorAffiliatesInstancesBoundInvalid,

            PostFloorAffiliatesTraitWhereClauseMissing,

            CeilAffiliateCountersMissing,
            CeilAffiliateCountersHasGenerics,
            CeilAffiliateCountersInvalidIdent,
            CeilAffiliateCountersBoundNotFound,

            CeilAffiliateInstanceMissing,
            CeilAffiliateInstanceHasGenerics,
            CeilAffiliateInstanceInvalidIdent,
            CeilAffiliateInstanceBoundNotFound,
            CeilAffiliateInstanceBoundInvalid,

            CountersForCeilAffiliatePredicateMissing,
            CountersForCeilAffiliatePredicateHasLifetimes,
            CountersForCeilAffiliateNotTypePredicate,
            CountersForCeilAffiliatePredicateWrongBoundedTy,
            CountersForCeilAffiliatePredicateBoundNotFound,

            PostCeilAffiliateTraitWhereClauseMissing,

            AffiliateCounterNotFound,

            AffiliateCountersCheckerNotFound,
            AffiliateCountersCheckerHasGenerics,
            AffiliateCountersCheckerHasExpr,
            AffiliateCountersCheckerWrongIdent,
            AffiliateCountersCheckerNotUnitType,
        }

    }

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // ````````````````````````````````` IDENT ITEMS `````````````````````````````````
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

    pub(super) enum IdentItemsErrors {
        range: 701..=1000,
        variants: {
                CounterIdentHashNullIdentProvided,

                CounterIdentNotFound,
                CounterIdentWrongIdent,
                CounterIdentWrongType,
                CounterIdentHasGenerics,
                CounterIdentHasExpr,

                IndexedCounterIdentNotFound,
                IndexedCounterIdentWrongIdent,
                IndexedCounterIdentWrongType,
                IndexedCounterIdentHasGenerics,
                IndexedCounterIdentExprNotFound,
                IndexedCounterIdentInvalidExpr,

                CounterIdentHashNotFound,
                CounterIdentHashWrongIdent,
                CounterIdentHashWrongType,
                CounterIdentHashHasGenerics,
                CounterIdentHashExprNotFound,
                CounterIdentHashInvalidExpr,

                CounterIdentCollectionLenTypeNumNotFound,
                CounterIdentCollectionLenTypeNumHasGenerics,
                CounterIdentCollectionLenTypeNumWrongIdent,
                CounterIdentCollectionLenTypeNumBoundNotFound,

                CounterTypeNumCollectionLenBoundsPredicateMissing,
                CounterTypeNumCollectionLenBoundsPredicateHasLifetimes,
                CounterTypeNumCollectionLenBoundsNotTypePredicate,
                CounterTypeNumCollectionLenBoundsPredicateWrongBoundedTy,
                CounterTypeNumCollectionLenBoundsPredicateBoundNotFound,
                InstanceTraitWhereClauseMissing,

                AllAffiliateCountersCollectionLenBoundsNotFound,
                AllAffiliateCountersCollectionLenBoundsHasGenerics,
                AllAffiliateCountersCollectionLenBoundsWrongIdent,
                AllAffiliateCountersCollectionLenBoundsBoundNotFound,

                CounterIdentCollectionSliceNotFound,
                CounterIdentCollectionSliceWrongIdent,
                CounterIdentCollectionSliceWrongType,
                CounterIdentCollectionSliceHasGenerics,
                CounterIdentCollectionSliceHasExpr,

                CounterIdentCollectionGenArrayNotFound,
                CounterIdentCollectionGenArrayWrongIdent,
                CounterIdentCollectionGenArrayWrongType,
                CounterIdentCollectionGenArrayHasGenerics,
                CounterIdentCollectionGenArrayHasExpr,

                CounterIdentHashCollectionSliceNotFound,
                CounterIdentHashCollectionSliceWrongIdent,
                CounterIdentHashCollectionSliceWrongType,
                CounterIdentHashCollectionSliceHasGenerics,
                CounterIdentHashCollectionSliceHasExpr,

                CounterIdentHashCollectionGenArrayNotFound,
                CounterIdentHashCollectionGenArrayWrongIdent,
                CounterIdentHashCollectionGenArrayWrongType,
                CounterIdentHashCollectionGenArrayHasGenerics,
                CounterIdentHashCollectionGenArrayHasExpr,

                CounterIdentHistoricalCollectionLenConstNotFound,
                CounterIdentHistoricalCollectionLenConstWrongIdent,
                CounterIdentHistoricalCollectionLenConstWrongType,
                CounterIdentHistoricalCollectionLenConstHasGenerics,
                CounterIdentHistoricalCollectionLenConstHasExpr,

                CounterIdentHistoricalCollectionLenTypeNumNotFound,
                CounterIdentHistoricalCollectionLenTypeNumHasGenerics,
                CounterIdentHistoricalCollectionLenTypeNumWrongIdent,
                CounterIdentHistoricalCollectionLenTypeNumBoundNotFound,

                CounterIdentHistoricalCollectionSliceNotFound,
                CounterIdentHistoricalCollectionSliceWrongIdent,
                CounterIdentHistoricalCollectionSliceWrongType,
                CounterIdentHistoricalCollectionSliceHasGenerics,
                CounterIdentHistoricalCollectionSliceHasExpr,

                CounterIdentHistoricalCollectionGenArrayNotFound,
                CounterIdentHistoricalCollectionGenArrayWrongIdent,
                CounterIdentHistoricalCollectionGenArrayWrongType,
                CounterIdentHistoricalCollectionGenArrayHasGenerics,
                CounterIdentHistoricalCollectionGenArrayHasExpr,

                CounterIdentHashHistoricalCollectionSliceNotFound,
                CounterIdentHashHistoricalCollectionSliceWrongIdent,
                CounterIdentHashHistoricalCollectionSliceWrongType,
                CounterIdentHashHistoricalCollectionSliceHasGenerics,
                CounterIdentHashHistoricalCollectionSliceHasExpr,

                CounterIdentHashHistoricalCollectionGenArrayNotFound,
                CounterIdentHashHistoricalCollectionGenArrayWrongIdent,
                CounterIdentHashHistoricalCollectionGenArrayWrongType,
                CounterIdentHashHistoricalCollectionGenArrayHasGenerics,
                CounterIdentHashHistoricalCollectionGenArrayHasExpr,

                CounterIdentExpectedHashCheckerNotFound,
                CounterIdentExpectedHashCheckerWrongIdent,
                CounterIdentExpectedHashCheckerNotUnitType,
                CounterIdentExpectedHashCheckerHasGenerics,
                CounterIdentExpectedHashCheckerHasExpr,
        }
    }
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // ``````````````````````````````` SUM-TYPE ITEMS ````````````````````````````````
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

    pub(super) enum SumTypeErrors {
        range: 1001..=1200,
        variants: {
                GlobalAssocNotFound,
                GlobalAssocWrongIdent,
                GlobalAssocHasGenerics,
                GlobalAssocMissingBound,

                GlobalTerminalExactNotFound,
                GlobalTerminalExactWrongIdent,
                GlobalTerminalExactHasGenerics,
                GlobalTerminalExactMissingBound,

                SelfTerminalNotFound,
                SelfTerminalWrongIdent,
                SelfTerminalHasGenerics,
                SelfTerminalMissingBound,
                SelfTerminalInvalidBound,

                SumAttrNotRemoved,
                CounterParamsAreEmpty,
                TypeNumGenericNotFound,
                SumOnlyForAssocTypes,
        }

    }

}

// ===============================================================================
// ````````````````````````````````` DIAGNOSTICS `````````````````````````````````
// ===============================================================================

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// `````````````````````````` COUNTER PARAM EXTRACTION ```````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

diagnostics!(
    space: CounterParamError,

    pub(super) enum CounterParamExtractionError {

        IdentsReCheckLenDifference {
            fields: {
                idents: IdentList,
            },
            msg:
                "const-generic idents list's length mismatch with extracted \
                counters params list",
            tags: [Bug],
            span: { tokens: idents.idents },
            note: [
                EXTRACTION_BUG.to_string(),
                LOGICAL_BUG.to_string(),
                REPORT_BUG.to_string()
            ]
        },

        IntsReCheckLenDifference {
            fields: {
                ints: IntList,
            },
            msg:
                "const-generic indexes list's length mismatch with extract-validated \
                counters params list",
            tags: [Bug],
            span: { tokens: ints.ints },
            note: [
                EXTRACTION_BUG.to_string(),
                LOGICAL_BUG.to_string(),
                REPORT_BUG.to_string()
            ]
        },

        IntsReCheckWrongIndex {
            fields: {
                exp: String,
                found: String,
            },
            msg: format!(
                "expected const-generic-param of positional index `{}` in the given instance \
                counter params, but found `{}`", exp, found
            ),
            tags: [Bug],
            span: { tokens: &found },
            note: [
                EXTRACTION_BUG.to_string(),
                LOGICAL_BUG.to_string(),
                REPORT_BUG.to_string()
            ]
        },


        IdentsReCheckWrongIdent {
            fields: {
                exp: String,
                found: String,
            },
            msg: format!(
                "expected const-generic-param named `{}` in extract-validated counter \
                param, but found `{}`", exp, found
            ),
            tags: [Bug],
            span: { tokens: found },
            note: [
                EXTRACTION_BUG.to_string(),
                LOGICAL_BUG.to_string(),
                REPORT_BUG.to_string()
            ]
        },

        ParamsReCheckOutOfBounds {
            fields: {
                idx: usize,
                trait_generics: Generics,
                trait_ident: Ident,
            },
            msg: format!(
                "extract-validated counter param's const-generic index `{}` is out \
                of bounds on the trait `{}` generics", idx, trait_ident
            ),
            tags: [Bug],
            span: { tokens: trait_generics },
            note: [
                EXTRACTION_BUG.to_string(),
                LOGICAL_BUG.to_string(),
                REPORT_BUG.to_string()
            ]
        },

        ParamsReCheckWrongIdent {
            fields: {
                exp: String,
                found: String,
                trait_ident: Ident,
            },
            msg: format!(
                "extract-validated const-generic-param named `{}` is expected \
                in the given instance trait `{}`, but found `{}`", exp, trait_ident, found
            ),
            tags: [Bug],
            span: { tokens: found },
            note: [
                EXTRACTION_BUG.to_string(),
                LOGICAL_BUG.to_string(),
                REPORT_BUG.to_string()
            ]
        },

        ParamsReCheckWrongType {
            fields: {
                param_ident: Ident,
                trait_ident: Ident,
                exp_ty: String,
                found_ty: String,
            },
            msg: format!(
                "extract-validated const-generic-param named `{}` in the instance trait \
                `{}` expected its type as `{}`, but found `{}`",
                param_ident, trait_ident, exp_ty, found_ty
            ),
            tags: [Bug],
            span: { tokens: found_ty },
            note: [
                EXTRACTION_BUG.to_string(),
                LOGICAL_BUG.to_string(),
                REPORT_BUG.to_string()
            ]
        },

        IdentNotConstGeneric {
            fields: {
                unknown_param_ident: Ident,
                available_params: String,
                trait_ident: Ident,
                trait_generics: Generics,
            },
            msg: format!(
                "cannot identify `{}` as a const-generic param of trait `{}`",
                unknown_param_ident, trait_ident
            ),
            tags: [Unsupported],
            span: { tokens: unknown_param_ident },
            help: [
                {
                    tokens: trait_generics,
                    msg:
                        "check the trait's generics for identifying valid instance \
                        counter const-generics"
                },
                {
                    format!(
                        "available unsigned const-generic identifiers (may-be) \
                        are : \"{}\"", available_params
                    )
                },
            ],
            note: [
                VALID_CONST_PARAMS.to_string(),
                ALLOWED_CONST_GEN_TY.to_string(),
                ENSURE_IDENTICAL_CONST_PARAMS.to_string()
            ],
        },

        TraitNeedsCounterGenerics {
            fields: {
                trait_generics : Generics,
                trait_ident: Ident,
            },
            msg: format!(
                "trait `{}` has no unsigned const generics to qualify as \
                a instance trait", trait_ident
            ),
            tags : [Unsupported],
            span: {tokens: trait_generics},
            help: [
                "provide suitable unsigned const-generics as instance counter/s",
                {ALLOWED_CONST_GEN_TY.to_string()},
                {ENSURE_IDENTICAL_CONST_PARAMS.to_string()},
                {
                    span: Span::call_site(),
                    msg:
                        "provide valid instance counter qualified const-generic \
                        identifiers or positional indexes in macro arguments"
                }
            ]
        },

        OutOfBoundsGenericIndex {
            fields: {
                gen_idx: usize,
                given_idx: LitInt,
                trait_generics: Generics,
                trait_ident: Ident,
                available_indexes: String,
            },
            msg: format!(
                "given generic's index `{}` of trait `{}` is out of bounds \
                to access", gen_idx, trait_ident
            ),
            tags: [Unsupported],
            span: { tokens: given_idx },
            help: [
                {
                    tokens: trait_generics,
                    msg:
                        "check the trait's generics for identifying valid \
                        instance counter const-generics"
                },
                {
                    format!(
                        "available unsigned const-generic indexes (may-be) \
                        are : \"{}\"", available_indexes
                    )
                },
            ],
            note: [
                VALID_CONST_PARAMS.to_string(),
                ALLOWED_CONST_GEN_TY.to_string(),
                ENSURE_IDENTICAL_CONST_PARAMS.to_string()
            ],
        },

        IndexNotConstGeneric {
            fields: {
                unknown_index: usize,
                given_lit: LitInt,
                available_indexes: String,
                trait_ident: Ident,
                trait_generics: Generics,
            },
            msg: format!(
                "not a const-generic param at index `{}` of trait `{}`",
                unknown_index, trait_ident
            ),
            tags: [Unsupported],
            span: { tokens: given_lit },
            help: [
                {
                    tokens: trait_generics,
                    msg:
                        "check the trait's generics for identifying valid \
                        instance counter const-generics indexes"
                },
                {
                    format!(
                        "available unsigned const-generic indexes (may-be) \
                        are : \"{}\"", available_indexes
                    )
                },
            ],
            note: [
                VALID_CONST_PARAMS.to_string(),
                ALLOWED_CONST_GEN_TY.to_string(),
                ENSURE_IDENTICAL_CONST_PARAMS.to_string()
            ],
        },

        TraitNeedsLeadConstGenerics {
            fields: {
                trait_generics : Generics,
                trait_ident: Ident,
            },
            msg: format!(
                "trait `{}` has no leading unsigned const generics to qualify \
                as a instance trait", trait_ident
            ),
            tags : [Unsupported],
            span: {tokens: trait_generics},
            help: [
                "provide suitable leading unsigned const-generics as instance counter/s",
                {ALLOWED_CONST_GEN_TY.to_string()},
                {ENSURE_IDENTICAL_CONST_PARAMS.to_string()},
                "if multiple-leading given then from the prelim identical \
                type const-generic are taken in order",
                {
                    span: Span::call_site(),
                    msg:
                        "in case of non-leading positions provide const-generics \
                        identifiers or positional indexes (in-order) to the macro arguments",
                },
            ]
        },

        ExpConstGeneric {
            fields: {
                found: Ident,
            },
            msg: "expected a const generic, found other kind generic",
            tags: [Unsupported],
            span: {tokens: found},
            help: [
                {
                    span: Span::call_site(),
                    msg:
                        "would have given the unexpected generic's identifier \
                        or positional index by mistake ?"
                },
            ]
        },

        ExpNonDefaultConstGeneric {
            fields: {
                found: Ident,
                default: Expr,
            },
            msg: "expected a non-default const generic, found default argument",
            tags: [Unsupported],
            span: {tokens: found},
            help: [
                {
                    tokens: default,
                    msg:
                        "remove this default const-generic expression"
                },
                {
                    span: Span::call_site(),
                    msg:
                        "would have given the unexpected generic's identifier \
                        or positional index by mistake ?"
                },
            ]
        },

        ExpCountersTy {
            fields: {
                found: Type,
            },
            msg: format!(
                "expected one of types \"{}\", but found else",
                VALID_COUNTER_TYPES
            ),
            tags: [Unsupported],
            span: {tokens: found},
            help: [
               {
                    span: Span::call_site(),
                    msg:
                        "would have given the wrong typed const-generic's \
                        identifier or positional index by mistake"
                },
            ]
        },

        ExpConsistentCountersTy {
            fields: {
                exp_ty: String,
                first_ty_ident: String,
                found: Type,
            },
            msg: format!(
                "expected unsigned const-generic type `{}` for consistent with \
                type `{}`, but found else", exp_ty, first_ty_ident
            ),
            tags: [Unsupported],
            span: {tokens: found},
            help: [
                {
                    span: Span::call_site(),
                    msg:
                        "would have added a inconsistent typed const-generic's \
                        identifier or positional index by mistake"
                },

            ]
        },
    }

);

bug_diagnostics! {
    space: CounterParamError,
    bug: EXTRACTION_BUG.to_string(),

    pub(super) enum CounterParamBugs {

        CountersNonSameTypesPassed =>
                "instance counters passed doesn't have same type counters,
                counter params extraction phase violated",
        NoValidCountersProvided =>
                "empty instance counters list passed, counter params extraction
                phase violated, requires atleast one",
        }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ``````````````````````````` META ITEMS (REFLECTION) ```````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

bug_diagnostics! {
    space: MetaItemsErrors,
    bug: META_EXTENSION_BUG.to_string(),

    pub(super) enum MetaTraitItemsBugs {

        CountersTyMetaNotFound =>
                "hidden default const-assoc-type to reflect instance counters \
                type via string-literal is not found",
        CountersTyMetaWrongIdent =>
                "hidden default const-assoc-type to reflect instance counters \
                type via string-literal does not have a determinisitic random name",
        CountersTyMetaExprNotFound =>
                "hidden default const-assoc-type to reflect instance counters \
                type via string literal, default expression is not found",
        CountersTyMetaInvalidExpr =>
                "hidden default const-assoc-type to reflect instance counters \
                type via string-literal has an invalid expression",
        CountersTyMetaInvalidType =>
                "hidden default const-assoc-type to reflect instance counters \
                type via string-literal has an invalid const-type",
        CountersTyMetaHasGenerics =>
                "hidden default const-assoc-type to reflect instance counters \
                type via string-literal has unnecessary generics",


        CountersTyNotFound =>
                "user-facing assoc-type to receive instance counters \
                type from impl side is not found",
        CountersTyWrongIdent =>
                "user-facing assoc-type to receive instance counters \
                type from impl side does not have a determinisitic random name",
        CountersTyHasGenerics =>
                "user-facing assoc-type to receive instance counters \
                type from impl side has unnecessary GAT generics",
        CountersTyHasBounds =>
                "user-facing assoc-type to receive instance counters \
                type from impl side has unnecessary trait bounds",


        CountersTyCheckerWrongIdent =>
                "hidden non-user-facing const-assoc to validate instance counters \
                assoc type from impl side does not have a deterministic random identifier",
        CountersTyCheckerNotUnitType =>
                "hidden non-user-facing const-assoc to validate instance counter's \
                assoc type from impl side does not have a unit type",
        CountersTyCheckerHasExpr =>
                "hidden non-user-facing const-assoc to validate instance counter's \
                assoc type from impl side has default expression",
        CountersTyCheckerNotFound =>
                "hidden non-user-facing const-assoc to validate instance counter's \
                assoc type from impl side is not found",
        CountersTyCheckerHasGenerics =>
                "hidden non-user-facing const-assoc to validate instance counter's \
                assoc type from impl side has unnecessary generics",


        CountersGenericsMetaNotFound =>
                "hidden const-assoc-type to reflect all instance counters generic \
                index and ident is not found",
        CountersGenericsMetaWrongIdent =>
                "hidden const-assoc-type to reflect all instance counters generic \
                index and ident does not have a deterministic random identifier",
        CountersGenericsMetaExprNotFound =>
                "hidden const-assoc-type to reflect all instance counters generic \
                index and ident default expression is not found",
        CountersGenericsMetaInvalidExpr =>
                "hidden const-assoc-type to reflect all instance counters generic \
                index and ident, has an invalid expression",
        CountersGenericsMetaInvalidType =>
                "hidden const-assoc-type to reflect all instance counters generic \
                index and ident, has an invalid const-type",
        CountersGenericsMetaHasGenerics =>
                "hidden const-assoc-type to reflect all instance counters generic \
                index and ident, has unnecessary generics",


        CountersGenericsIndexesMetaNotFound =>
                "hidden const-assoc-type to receive all instance counters generic \
                indexes from impl side not found",
        CountersGenericsIndexesMetaWrongIdent =>
                "hidden const-assoc-type to receive all instance counters generic \
                indexes from impl side does not have a deterministic random idetifier",
        CountersGenericsIndexesMetaHasExpr =>
                "hidden const-assoc-type to receive all instance counters generic \
                indexes from impl side has a default expression",
        CountersGenericsIndexesMetaInvalidType =>
                "hidden const-assoc-type to receive all instance counters generic \
                indexes from impl side  has an invalid const-type",
        CountersGenericsIndexesMetaHasGenerics =>
                "hidden const-assoc-type to receive all instance counters generic \
                indexes from impl side has unnecessary generics",


        CountersLenMetaNotFound =>
                "hidden const-assoc-type to reflect instance counters length \
                is not found",
        CountersLenMetaWrongIdent =>
                "hidden const-assoc-type to reflect instance counters length \
                does not have a determinisitic random identifier",
        CountersLenMetaInvalidType =>
                "hidden const-assoc-type to reflect instance counters length \
                has invalid const-type",
        CountersLenMetaHasGenerics =>
                "hidden const-assoc-type to reflect instance counters length \
                has unnnessary generics",
        CountersLenMetaHasExpr =>
                "hidden const-assoc-type to reflect instance counters length \
                has default expression",


        CountersGenericsCheckerWrongIdent =>
                "hidden non-user-facing const-assoc to validate instance counters \
                generics params from impl side does not have a deterministic random identifier",
        CountersGenericsCheckerNotUnitType =>
                "hidden non-user-facing const-assoc to validate instance counter's \
                generics params from impl side does not have a unit type",
        CountersGenericsCheckerHasExpr =>
                "hidden non-user-facing const-assoc to validate instance counter's \
                generics params from impl side has default expression",
        CountersGenericsCheckerNotFound =>
                "hidden non-user-facing const-assoc to validate instance counter's \
                generics params from impl side is not found",
        CountersGenericsCheckerHasGenerics =>
                "hidden non-user-facing const-assoc to validate instance counter's \
                generics params from impl side has unnecessary generics",


        InstanceTraitDocsRawAttrMissing =>
                "appended trait docs reflecting requirement of impl side macro, \
                its info docs attribute missing",
        InstanceTraitDocsMacroInlineAttrMissing =>
                "appended trait docs reflecting requirement of impl side macro, \
                its inline macro docs attribute missing",
        InstanceTraiDocsNotDocsAttr =>
                "appended trait docs reflecting requirement of impl side macro, \
                its not an rust docs attribute",
        InstanceTraiDocsAttrNotLiteralExpr =>
                "appended trait docs reflecting requirement of impl side macro, \
                its not an rust docs attribute's literal expr value",
        InstanceTraiDocsAttrNotStrLiteral =>
                "appended trait docs reflecting requirement of impl side macro, \
                its not an rust docs attribute's string literal value",
        InstanceTraitDocsInvalidRawAttr =>
                "appended trait docs reflecting requirement of impl side macro, \
                its info docs attribute is invalid",
        InstanceTraitDocsInvalidMacroInlineAttr =>
                "appended trait docs reflecting requirement of impl side macro, \
                its inline macro docs attribute is invalid",

        OriginalCounterConstNotFound =>
                "hidden const-assoc-type to take original const-generic counter via impl \
                not found",
        OriginalCounterConstWrongIdent =>
                "hidden const-assoc-type to take original const-generic counter via impl \
                does not have a deterministic random idetifier",
        OriginalCounterConstHasExpr =>
                "hidden const-assoc-type to take original const-generic counter via impl \
                has a default expression",
        OriginalCounterConstWrongType =>
                "hidden const-assoc-type to take original const-generic counter via impl \
                 has an invalid const-type",
        OriginalCounterConstHasGenerics =>
                "hidden const-assoc-type to take original const-generic counter via impl \
                has unnecessary generics",

        CounterConstAnnotatedExtractionFailed =>
                "const-assoc-type to take original const-generic counter via impl \
                is not extracted properly",

        CumulatedConstCheckerNotFound =>
                "hidden cumulated const-assoc-checker for the current instance impl \
                not found",
        CumulatedConstCheckerWrongIdent =>
                "hidden cumulated const-assoc-checker for the current instance impl \
                does not have a deterministic random idetifier",
        CumulatedConstCheckerHasExpr =>
                "hidden cumulated const-assoc-checker for the current instance impl \
                has a default expression",
        CumulatedConstCheckerNotUnitType =>
                "hidden cumulated const-assoc-checker for the current instance impl \
                 not a unit type",
        CumulatedConstCheckerHasGenerics =>
                "hidden cumulated const-assoc-checker for the current instance impl \
                has unnecessary generics",


        CountersParamMetaExtractionGenericsMetaNotFound =>
                "hidden const-assoc-type reflecting instance counters generic \
                index and ident is not found",
        CountersParamMetaExtractionCountersTyMetaNotFound =>
                "hidden const-assoc-type reflecting instance counters \
                type is not found",
        CountersParamMetaExtractionCountersTyMetaDefaultExprNotFound =>
                "hidden const-assoc-type reflecting instance counters \
                type default expression is not found",
        CountersParamMetaExtractionCountersTyMetaDefaultExprNotLitExpr =>
                "hidden const-assoc-type reflecting instance counters \
                type default expression is not a literal expression",
        CountersParamMetaExtractionCountersTyMetaDefaultExprNotStrLit =>
                "hidden const-assoc-type reflecting instance counters \
                type default expression is not a string literal",
        CountersParamMetaExtractionGenericsMetaDefaultExprNotFound =>
                "hidden const-assoc-type reflecting instance counters generic \
                metadata default expression is not found",
        CountersParamMetaExtractionGenericsMetaDefaultExprNotArray =>
                "hidden const-assoc-type reflecting instance counters generic \
                metadata default expression is not an array expression",
        CountersParamMetaExtractionGenericsMetaDefaultExprArrayElemNotTuple =>
                "hidden const-assoc-type reflecting instance counters generic \
                metadata array element is not a tuple",
        CountersParamMetaExtractionGenericsMetaDefaultExprArrayElemTupleWrongLen =>
                "hidden const-assoc-type reflecting instance counters generic \
                metadata tuple does not contain exactly two elements",
        CountersParamMetaExtractionGenericsMetaDefaultExprArrayElemTupleCounterGenericIndexNotFound =>
                "hidden const-assoc-type reflecting instance counters generic \
                metadata tuple is missing the counter generic index",
        CountersParamMetaExtractionGenericsMetaDefaultExprArrayElemTupleCounterGenericIndexNotExprLit =>
                "hidden const-assoc-type reflecting instance counters generic \
                metadata counter generic index is not a literal expression",
        CountersParamMetaExtractionGenericsMetaDefaultExprArrayElemTupleCounterGenericIndexNotLitInt =>
                "hidden const-assoc-type reflecting instance counters generic \
                metadata counter generic index is not an integer literal",
        CountersParamMetaExtractionGenericsMetaDefaultExprArrayElemTupleCounterGenericIdentNotFound =>
                "hidden const-assoc-type reflecting instance counters generic \
                metadata tuple is missing the counter generic identifier",
        CountersParamMetaExtractionGenericsMetaDefaultExprArrayElemTupleCounterGenericIdentNotExprLit =>
                "hidden const-assoc-type reflecting instance counters generic \
                metadata counter generic identifier is not a literal expression",
        CountersParamMetaExtractionGenericsMetaDefaultExprArrayElemTupleCounterGenericIdentNotLitStr =>
                "hidden const-assoc-type reflecting instance counters generic \
                metadata counter generic identifier is not a string literal",
        CountersParamMetaExtractionInconsistentWithGenericIdentArgs =>
                "reconstructed counter parameters are inconsistent with the \
                user-provided counter generic identifier arguments",
        CountersParamMetaExtractionInconsistentWithGenericIndexArgs =>
                "reconstructed counter parameters are inconsistent with the \
                user-provided counter generic index arguments",

        WhereClauseNotFoundForMarkerBound =>
                "marker bound predicate where clause is not found",
        MarkerBoundPredNotFound =>
                "marker bound predicate is not found in where clause",
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ```````````````````````` TYPE NUM PARAM TRANSFORMATION ````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

diagnostics!(
    space: TypeNumCountersErrors,

    pub(super) enum TypeNumErrors {

        TypeGenericRequiresDefault {
            fields: {
                generic: TypeParam,
            },
            msg:
                "type generics for instance traits requires defaults, for stable const-evaluations",
            tags: [Unsupported],
            span: { tokens: generic },
            note: [
                "Utilized only for testing compile time validations"
            ]
        },
        ConstGenericRequiresDefault {
            fields: {
                generic: ConstParam,
            },
            msg:
                "const generics for instance traits requires defaults",
            tags: [Unsupported],
            span: { tokens: generic },
            note: [
                "Utilized only for testing compile time validations"
            ]
        },

        InvalidFirstCounter {
            fields: {
                param: ConstParam,
                index: usize,
                len: usize,
            },
            msg: "first instance counter const-param placement creates out of bounds",
            tags: [Unsupported],
            span: { tokens: param },
            help: [
                {format!("try placing counter in generic of index less than {}", len)},
            ],
            note: [
                format!("current counter placement is index {}, expected less than (<) {}", index, len),
                "when removing the existing counter constant generic params, \
                the first counter must be inside available slots",
            ]
        },

        Maximum4Counters {
            fields: {},
            msg:
                "maximum 4 instance counter params are allowed",
            tags: [Unsupported],
            span: { span: &Span::call_site() },
            note: [
                "found more than four instance counter params"
            ]
        },
    }
);

bug_diagnostics! {
    space: TypeNumCountersErrors,
    bug: TYPENUM_TRANSFORM_BUG.to_string(),

    pub(super) enum TypeNumCountersBugs {

        InvalidCounterParamIndex =>
                "const-generic instance counter provided is not found in the trait \
                for type-num conversion, maybe extraction phase violated",

        CounterParamNotTypeGeneric =>
                "instance counter param is not a type-generic, type-num \
                transformation violated",

        CounterTypeGenericInvalidIdent =>
                "instance counter param has invalid ident must be same as \
                const-generic counter param ident, type-num transformation violated",

        CounterTypeGenericHasBounds =>
                "instance counter param has unnecessary bounds as its only should \
                be in post-predicates, type-num transformation violated",

        CounterTypeGenericHasDefault =>
                "instance counter param has a default generic type, \
                type-num transformation violated",

        CounterParamsAreEmpty =>
                "provided instance counter params are empty, counters extraction phase violated"
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ``````````````````````` AFFILIATES (EXTEND & TRANSFORM) ```````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

bug_diagnostics! {
    space: AffiliatesErrors,
    bug: AFFILIATE_BUG.to_string(),

    pub(super) enum AffiliateBugs {

        AffiliateCounterNotFound =>
                "hidden-typenum-assoc type of an affilaite counter is not found,
                affiliate extension phase violated",

        ReverseAffiliateCountersMissing =>
                "reverse affiliate counter of the instance trait's associated type \
                is missing",
        ReverseAffiliateCountersHasGenerics =>
                "reverse affiliate counter of the instance trait's associated type \
                has unnecessary generics",
        ReverseAffiliateCountersInvalidIdent =>
                "reverse affiliate counter of the instance trait's associated type \
                does not have a deterministic random identifier",
        ReverseAffiliateCountersBoundNotFound =>
                "reverse affiliate counter of the instance trait's associated type's \
                bounds are missing",

        ReverseAffiliateInstanceMissing =>
                "reverse affiliate instance of the instance trait's associated type \
                is missing",
        ReverseAffiliateInstanceHasGenerics =>
                "reverse affiliate instance of the instance trait's associated type \
                has unnecessary generics",
        ReverseAffiliateInstanceInvalidIdent =>
                "reverse affiliate instance of the instance trait's associated type \
                does not have a deterministic random identifier",
        ReverseAffiliateInstanceBoundNotFound =>
                "reverse affiliate instance of the instance trait's associated type's \
                bounds are missing",
        ReverseAffiliateInstanceBoundInvalid =>
                "reverse affiliate instance of the instance trait's associated type's \
                bounds is invalid",

        CountersForReverseAffiliatePredicateMissing =>
                "instance counters bounded for reverse affiliate on the instance trait's \
                where predicate is missing",
        CountersForReverseAffiliatePredicateHasLifetimes =>
                "instance counters bounded for reverse affiliate on the instance trait's \
                where predicate has unnecessary lifetimes",
        CountersForReverseAffiliateNotTypePredicate =>
                "instance counters bounded for reverse affiliate on the instance trait's \
                where predicate is not a type predicate",
        CountersForReverseAffiliatePredicateWrongBoundedTy =>
                "instance counters bounded for reverse affiliate on the instance trait's \
                where predicate is not a type predicate",
        CountersForReverseAffiliatePredicateBoundNotFound =>
                "instance counters bounded for reverse affiliate on the instance trait's \
                where predicate does not have a predicate bound",

        PostReverseAffiliateTraitWhereClauseMissing =>
                "reverse affiliate applied instance trait's where predicates are missing",


        NextAffiliateCountersMissing =>
                "next affiliate counter of the instance trait's associated type \
                is missing",
        NextAffiliateCountersHasGenerics =>
                "next affiliate counter of the instance trait's associated type \
                has unnecessary generics",
        NextAffiliateCountersInvalidIdent =>
                "next affiliate counter of the instance trait's associated type \
                does not have a deterministic random identifier",
        NextAffiliateCountersBoundNotFound =>
                "next affiliate counter of the instance trait's associated type's \
                bounds are missing",

        NextAffiliateInstanceMissing =>
                "next affiliate instance of the instance trait's associated type \
                is missing",
        NextAffiliateInstanceHasGenerics =>
                "next affiliate instance of the instance trait's associated type \
                has unnecessary generics",
        NextAffiliateInstanceInvalidIdent =>
                "next affiliate instance of the instance trait's associated type \
                does not have a deterministic random identifier",
        NextAffiliateInstanceBoundNotFound =>
                "next affiliate instance of the instance trait's associated type's \
                bounds are missing",
        NextAffiliateInstanceBoundInvalid =>
                "next affiliate instance of the instance trait's associated type's \
                bounds is invalid",

        PostNextAffiliateTraitWhereClauseMissing =>
                "next affiliate applied instance trait's where predicates are missing",


        BackAffiliateCountersMissing =>
                "back affiliate counter of the instance trait's associated type \
                is missing",
        BackAffiliateCountersHasGenerics =>
                "back affiliate counter of the instance trait's associated type \
                has unnecessary generics",
        BackAffiliateCountersInvalidIdent =>
                "back affiliate counter of the instance trait's associated type \
                does not have a deterministic random identifier",
        BackAffiliateCountersBoundNotFound =>
                "back affiliate counter of the instance trait's associated type's \
                bounds are missing",

        BackAffiliateInstanceMissing =>
                "back affiliate instance of the instance trait's associated type \
                is missing",
        BackAffiliateInstanceHasGenerics =>
                "back affiliate instance of the instance trait's associated type \
                has unnecessary generics",
        BackAffiliateInstanceInvalidIdent =>
                "back affiliate instance of the instance trait's associated type \
                does not have a deterministic random identifier",
        BackAffiliateInstanceBoundNotFound =>
                "back affiliate instance of the instance trait's associated type's \
                bounds are missing",
        BackAffiliateInstanceBoundInvalid =>
                "back affiliate instance of the instance trait's associated type's \
                bounds is invalid",

        PostBackAffiliateTraitWhereClauseMissing =>
                "back affiliate applied instance trait's where predicates are missing",


        FloorAffiliatesCountersMissing =>
                "one of floor affiliate counter of the instance trait's associated type \
                is missing",
        FloorAffiliatesCountersHasGenerics =>
                "one of floor affiliate counter of the instance trait's associated type \
                has unnecessary generics",
        FloorAffiliatesCountersInvalidIdent =>
                "one of floor affiliate counter of the instance trait's associated type \
                does not have a deterministic random identifier",
        FloorAffiliatesCountersBoundNotFound =>
                "one of floor affiliate counter of the instance trait's associated type's \
                bounds are missing",

        FloorAffiliatesInstancesMissing =>
                "one of floor affiliate instance of the instance trait's associated type \
                is missing",
        FloorAffiliatesInstancesHasGenerics =>
                "one of floor affiliate instance of the instance trait's associated type \
                has unnecessary generics",
        FloorAffiliatesInstancesInvalidIdent =>
                "one of floor affiliate instance of the instance trait's associated type \
                does not have a deterministic random identifier",
        FloorAffiliatesInstancesBoundNotFound =>
                "one of floor affiliate instance of the instance trait's associated type's \
                bounds are missing",
        FloorAffiliatesInstancesBoundInvalid =>
                "one of floor affiliate instance of the instance trait's associated type's \
                bounds is invalid",

        PostFloorAffiliatesTraitWhereClauseMissing =>
                "one of floor affiliate applied instance trait's where predicates are missing",

        CeilAffiliateCountersMissing =>
                "ceil affiliate counter of the instance trait's associated type \
                is missing",
        CeilAffiliateCountersHasGenerics =>
                "ceil affiliate counter of the instance trait's associated type \
                has unnecessary generics",
        CeilAffiliateCountersInvalidIdent =>
                "ceil affiliate counter of the instance trait's associated type \
                does not have a deterministic random identifier",
        CeilAffiliateCountersBoundNotFound =>
                "ceil affiliate counter of the instance trait's associated type's \
                bounds are missing",

        CeilAffiliateInstanceMissing =>
                "ceil affiliate instance of the instance trait's associated type \
                is missing",
        CeilAffiliateInstanceHasGenerics =>
                "ceil affiliate instance of the instance trait's associated type \
                has unnecessary generics",
        CeilAffiliateInstanceInvalidIdent =>
                "ceil affiliate instance of the instance trait's associated type \
                does not have a deterministic random identifier",
        CeilAffiliateInstanceBoundNotFound =>
                "ceil affiliate instance of the instance trait's associated type's \
                bounds are missing",
        CeilAffiliateInstanceBoundInvalid =>
                "ceil affiliate instance of the instance trait's associated type's \
                bounds is invalid",

        CountersForCeilAffiliatePredicateMissing =>
                "instance counters bounded for ceil affiliate on the instance trait's \
                where predicate is missing",
        CountersForCeilAffiliatePredicateHasLifetimes =>
                "instance counters bounded for ceil affiliate on the instance trait's \
                where predicate has unnecessary lifetimes",
        CountersForCeilAffiliateNotTypePredicate =>
                "instance counters bounded for ceil affiliate on the instance trait's \
                where predicate is not a type predicate",
        CountersForCeilAffiliatePredicateWrongBoundedTy =>
                "instance counters bounded for ceil affiliate on the instance trait's \
                where predicate is not a type predicate",
        CountersForCeilAffiliatePredicateBoundNotFound =>
                "instance counters bounded for ceil affiliate on the instance trait's \
                where predicate does not have a predicate bound",
        PostCeilAffiliateTraitWhereClauseMissing =>
                "ceil affiliate applied instance trait's where predicates are missing",

        AffiliateCountersCheckerNotFound =>
                "hidden unit const-assoc to evaluate and find rougue instance impls\
                is not found",
        AffiliateCountersCheckerWrongIdent =>
                "hidden unit const-assoc to evaluate and find rougue instance impls\
                does not have a determinisitic random identifier",
        AffiliateCountersCheckerNotUnitType =>
                "hidden unit const-assoc to evaluate and find rougue instance impls\
                has invalid const-type",
        AffiliateCountersCheckerHasGenerics =>
                "hidden unit const-assoc to evaluate and find rougue instance impls\
                has unnnessary generics",
        AffiliateCountersCheckerHasExpr =>
                "hidden unit const-assoc to evaluate and find rougue instance impls\
                has default expression",
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ``````````````````````````` IDENT ITEMS (EXTENSION) ```````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

diagnostics!(
    space: IdentItemsErrors,

    pub(super) enum IdentItemsExpansionErrors {

        CounterIdentHashNullIdentProvided {
            fields: {
                counter: Ident,
                trait_of: Ident,
            },
            msg:
                format!("empty instance-counter identifier provided for `{}` for trait `{}`", counter, trait_of),
            tags: [Unsupported],
            span: { span: &Span::call_site() },
            help: [
                {format!("found empty literal instead of `const {}: 'static [u8] = b\"<some-ascii-identifier>\"`", counter)}
            ],
            note: [
                "provide a valid, non-empty ascii identifier (byte string)"
            ]
        },

    }
);

bug_diagnostics! {
    space: IdentItemsErrors,
    bug: IDENTS_BUG.to_string(),
    pub(super) enum IdentTraitItemsBugs {
        CounterIdentNotFound =>
                "assoc-const to take the instance counter's ascii identifier \
                is not found",
        CounterIdentWrongIdent =>
                "assoc-const to take the instance counter's ascii identifier \
                does not have a determinisitic random identifier",
        CounterIdentWrongType =>
                "assoc-const to take the instance counter's ascii identifier \
                has invalid const-type",
        CounterIdentHasGenerics =>
                "assoc-const to take the instance counter's ascii identifier \
                has unnnessary generics",
        CounterIdentHasExpr =>
                "assoc-const to take the instance counter's ascii identifier \
                has default expression",

        IndexedCounterIdentNotFound =>
                "hidden const-assoc to reflect counter's ascii ident via counter generic index \
                is not found",
        IndexedCounterIdentWrongIdent =>
                "hidden const-assoc to reflect counter's ascii ident via counter generic index \
                does not have a deterministic random identifier",
        IndexedCounterIdentExprNotFound =>
                "hidden const-assoc to reflect counter's ascii ident via counter generic index \
                default expression is not found",
        IndexedCounterIdentInvalidExpr =>
                "hidden const-assoc to reflect counter's ascii ident via counter generic index \
                , has an invalid expression",
        IndexedCounterIdentWrongType =>
                "hidden const-assoc to reflect counter's ascii ident via counter generic index \
                , has an invalid const-type",
        IndexedCounterIdentHasGenerics =>
                "hidden const-assoc to reflect counter's ascii ident via counter generic index \
                , has unnecessary generics",

        CounterIdentHashNotFound =>
                "hidden const-assoc that generates stable hash for counter ident \
                is not found",
        CounterIdentHashWrongIdent =>
                "hidden const-assoc that generates stable hash for counter ident \
                does not have a deterministic random identifier",
        CounterIdentHashExprNotFound =>
                "hidden const-assoc that generates stable hash for counter ident \
                default expression is not found",
        CounterIdentHashInvalidExpr =>
                "hidden const-assoc that generates stable hash for counter ident \
                , has an invalid expression",
        CounterIdentHashWrongType =>
                "hidden const-assoc that generates stable hash for counter ident \
                , has an invalid const-type",
        CounterIdentHashHasGenerics =>
                "hidden const-assoc that generates stable hash for counter ident \
                , has unnecessary generics",


        CounterIdentCollectionLenTypeNumNotFound =>
                "hidden const-assoc to take counter's ident collection's length in typenum \
                is not found",
        CounterIdentCollectionLenTypeNumHasGenerics =>
                "hidden const-assoc to take counter's ident collection's length in typenum \
                has unnecessary generics",
        CounterIdentCollectionLenTypeNumWrongIdent =>
                "hidden const-assoc to take counter's ident collection's length in typenum \
                does not have a deterministic random identifier",
        CounterIdentCollectionLenTypeNumBoundNotFound =>
                "hidden const-assoc to take counter's ident collection's length in typenum \
                bounds are missing",

        CounterTypeNumCollectionLenBoundsPredicateMissing =>
                "instance counters bounded for ident/hash collection len bounds\
                where predicate is missing",
        CounterTypeNumCollectionLenBoundsPredicateHasLifetimes =>
                "instance counters bounded for ident/hash collection len bounds\
                where predicate has unnecessary lifetimes",
        CounterTypeNumCollectionLenBoundsNotTypePredicate =>
                "instance counters bounded for ident/hash collection len bounds\
                where predicate is not a type predicate",
        CounterTypeNumCollectionLenBoundsPredicateWrongBoundedTy =>
                "instance counters bounded for ident/hash collection len bounds\
                where predicate is not a type predicate",
        CounterTypeNumCollectionLenBoundsPredicateBoundNotFound =>
                "instance counters bounded for ident/hash collection len bounds\
                where predicate does not have a predicate bound",
        InstanceTraitWhereClauseMissing =>
                "instance trait's where predicates for validating idet/hash collection len bounds \
                are missing",

        AllAffiliateCountersCollectionLenBoundsNotFound =>
                "hidden affiliate counter assoc-type with bounds for counter's collection's length \
                is missing",
        AllAffiliateCountersCollectionLenBoundsHasGenerics =>
                "hidden affiliate counter assoc-type with bounds for counter's collection's length \
                has unnecessary generics",
        AllAffiliateCountersCollectionLenBoundsWrongIdent =>
                "hidden affiliate counter assoc-type with bounds for counter's collection's length \
                does not have a deterministic random identifier",
        AllAffiliateCountersCollectionLenBoundsBoundNotFound =>
                "hidden affiliate counter assoc-type with bounds for counter's collection's length \
                bounds are missing",

        CounterIdentCollectionSliceNotFound =>
                "hidden const-assoc to take counter's ident collection as a const-slice \
                is not found",
        CounterIdentCollectionSliceWrongIdent =>
                "hidden const-assoc to take counter's ident collection as a const-slice \
                does not have a determinisitic random identifier",
        CounterIdentCollectionSliceWrongType =>
                "hidden const-assoc to take counter's ident collection as a const-slice \
                has invalid const-type",
        CounterIdentCollectionSliceHasGenerics =>
                "hidden const-assoc to take counter's ident collection as a const-slice \
                has unnnessary generics",
        CounterIdentCollectionSliceHasExpr =>
                "hidden const-assoc to take counter's ident collection as a const-slice \
                has default expression",

        CounterIdentCollectionGenArrayNotFound =>
                "hidden const-assoc to take counter's ident collection as a const-generic-array \
                is not found",
        CounterIdentCollectionGenArrayWrongIdent =>
                "hidden const-assoc to take counter's ident collection as a const-generic-array \
                does not have a determinisitic random identifier",
        CounterIdentCollectionGenArrayWrongType =>
                "hidden const-assoc to take counter's ident collection as a const-generic-array \
                has invalid const-type",
        CounterIdentCollectionGenArrayHasGenerics =>
                "hidden const-assoc to take counter's ident collection as a const-generic-array \
                has unnnessary generics",
        CounterIdentCollectionGenArrayHasExpr =>
                "hidden const-assoc to take counter's ident collection as a const-generic-array \
                has default expression",

        CounterIdentHashCollectionSliceNotFound =>
                "hidden const-assoc to take counter's ident's hash collection as a const-slice \
                is not found",
        CounterIdentHashCollectionSliceWrongIdent =>
                "hidden const-assoc to take counter's ident's hash collection as a const-slice \
                does not have a determinisitic random identifier",
        CounterIdentHashCollectionSliceWrongType =>
                "hidden const-assoc to take counter's ident's hash collection as a const-slice \
                has invalid const-type",
        CounterIdentHashCollectionSliceHasGenerics =>
                "hidden const-assoc to take counter's ident's hash collection as a const-slice \
                has unnnessary generics",
        CounterIdentHashCollectionSliceHasExpr =>
                "hidden const-assoc to take counter's ident's hash collection as a const-slice \
                has default expression",

        CounterIdentHashCollectionGenArrayNotFound =>
                "hidden const-assoc to take counter's ident's hash collection as a const-generic-array \
                is not found",
        CounterIdentHashCollectionGenArrayWrongIdent =>
                "hidden const-assoc to take counter's ident's hash collection as a const-generic-array \
                does not have a determinisitic random identifier",
        CounterIdentHashCollectionGenArrayWrongType =>
                "hidden const-assoc to take counter's ident's hash collection as a const-generic-array \
                has invalid const-type",
        CounterIdentHashCollectionGenArrayHasGenerics =>
                "hidden const-assoc to take counter's ident's hash collection as a const-generic-array \
                has unnnessary generics",
        CounterIdentHashCollectionGenArrayHasExpr =>
                "hidden const-assoc to take counter's ident's hash collection as a const-generic-array \
                has default expression",

        CounterIdentHistoricalCollectionLenConstNotFound =>
                "hidden const-assoc to take counter's ident hash collection length \
                is not found",
        CounterIdentHistoricalCollectionLenConstWrongIdent =>
                "hidden const-assoc to take counter's ident hash collection length \
                does not have a determinisitic random identifier",
        CounterIdentHistoricalCollectionLenConstWrongType =>
                "hidden const-assoc to take counter's ident hash collection length \
                has invalid const-type",
        CounterIdentHistoricalCollectionLenConstHasGenerics =>
                "hidden const-assoc to take counter's ident hash collection length \
                has unnnessary generics",
        CounterIdentHistoricalCollectionLenConstHasExpr =>
                "hidden const-assoc to take counter's ident hash collection length \
                has default expression",

        CounterIdentHistoricalCollectionLenTypeNumNotFound =>
                "hidden const-assoc to take counter's ident hash collection's length in typenum \
                is not found",
        CounterIdentHistoricalCollectionLenTypeNumHasGenerics =>
                "hidden const-assoc to take counter's ident hash collection's length in typenum \
                has unnecessary generics",
        CounterIdentHistoricalCollectionLenTypeNumWrongIdent =>
                "hidden const-assoc to take counter's ident hash collection's length in typenum \
                does not have a deterministic random identifier",
        CounterIdentHistoricalCollectionLenTypeNumBoundNotFound =>
                "hidden const-assoc to take counter's ident hash collection's length in typenum \
                bounds are missing",

        CounterIdentHistoricalCollectionSliceNotFound =>
                "hidden const-assoc to take counter's ident historical collection as slice \
                is not found",
        CounterIdentHistoricalCollectionSliceWrongIdent =>
                "hidden const-assoc to take counter's ident historical collection as slice \
                does not have a determinisitic random identifier",
        CounterIdentHistoricalCollectionSliceWrongType =>
                "hidden const-assoc to take counter's ident historical collection as slice \
                has invalid const-type",
        CounterIdentHistoricalCollectionSliceHasGenerics =>
                "hidden const-assoc to take counter's ident historical collection as slice \
                has unnnessary generics",
        CounterIdentHistoricalCollectionSliceHasExpr =>
                "hidden const-assoc to take counter's ident historical collection as slice \
                has default expression",

        CounterIdentHistoricalCollectionGenArrayNotFound =>
                "hidden const-assoc to take counter's ident historical collection as generic-array \
                is not found",
        CounterIdentHistoricalCollectionGenArrayWrongIdent =>
                "hidden const-assoc to take counter's ident historical collection as generic-array\
                does not have a determinisitic random identifier",
        CounterIdentHistoricalCollectionGenArrayWrongType =>
                "hidden const-assoc to take counter's ident historical collection as generic-array \
                has invalid const-type",
        CounterIdentHistoricalCollectionGenArrayHasGenerics =>
                "hidden const-assoc to take counter's ident historical collection as generic-array \
                has unnnessary generics",
        CounterIdentHistoricalCollectionGenArrayHasExpr =>
                "hidden const-assoc to take counter's ident historical collection as generic-array \
                has default expression",

        CounterIdentHashHistoricalCollectionSliceNotFound =>
                "hidden const-assoc to take counter's ident's hash historical collection as slice \
                is not found",
        CounterIdentHashHistoricalCollectionSliceWrongIdent =>
                "hidden const-assoc to take counter's ident's hash historical collection as slice \
                does not have a determinisitic random identifier",
        CounterIdentHashHistoricalCollectionSliceWrongType =>
                "hidden const-assoc to take counter's ident's hash historical collection as slice \
                has invalid const-type",
        CounterIdentHashHistoricalCollectionSliceHasGenerics =>
                "hidden const-assoc to take counter's ident's hash historical collection as slice \
                has unnnessary generics",
        CounterIdentHashHistoricalCollectionSliceHasExpr =>
                "hidden const-assoc to take counter's ident's hash historical collection as slice \
                has default expression",

        CounterIdentHashHistoricalCollectionGenArrayNotFound =>
                "hidden const-assoc to take counter's ident's hash historical collection as generic-array \
                is not found",
        CounterIdentHashHistoricalCollectionGenArrayWrongIdent =>
                "hidden const-assoc to take counter's ident's hash historical collection as generic-array\
                does not have a determinisitic random identifier",
        CounterIdentHashHistoricalCollectionGenArrayWrongType =>
                "hidden const-assoc to take counter's ident's hash historical collection as generic-array \
                has invalid const-type",
        CounterIdentHashHistoricalCollectionGenArrayHasGenerics =>
                "hidden const-assoc to take counter's ident's hash historical collection as generic-array \
                has unnnessary generics",
        CounterIdentHashHistoricalCollectionGenArrayHasExpr =>
                "hidden const-assoc to take counter's ident's hash historical collection as generic-array \
                has default expression",

        CounterIdentExpectedHashCheckerNotFound =>
                "hidden unit const-assoc to assert parent counters identifier consistency via hashes \
                is not found",
        CounterIdentExpectedHashCheckerWrongIdent =>
                "hidden unit const-assoc to assert parent counters identifier consistency via hashes \
                does not have a determinisitic random identifier",
        CounterIdentExpectedHashCheckerNotUnitType =>
                "hidden unit const-assoc to assert parent counters identifier consistency via hashes \
                has invalid const-type",
        CounterIdentExpectedHashCheckerHasGenerics =>
                "hidden unit const-assoc to assert parent counters identifier consistency via hashes \
                has unnnessary generics",
        CounterIdentExpectedHashCheckerHasExpr =>
                "hidden unit const-assoc to assert parent counters identifier consistency via hashes \
                has default expression",
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ```````````````````````````` SUM TYPES (EXTENSION) ````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

diagnostics!(
    space: SumTypeErrors,

    pub(super) enum SumTypeErr {

        SumOnlyForAssocTypes {
            fields: {
                item: TraitItem,
            },
            msg:
                "sum types are allowed on associated type declarations only, found else",
            tags: [Unsupported],
            span: { tokens: item },
        },
    }
);

bug_diagnostics! {
    space: SumTypeErrors,
    bug: SUM_TYPE_EXTENSION_BUG.to_string(),
    pub(super) enum SumTypeBugs {
        GlobalAssocNotFound =>
        "global associated type for sum-type bounds is not found",
        GlobalAssocWrongIdent =>
        "global associated type for sum-type bounds does not have the expected identifier",
        GlobalAssocHasGenerics =>
        "global associated type for sum-type bounds has generics",
        GlobalAssocMissingBound =>
        "global associated type for sum-type bounds is missing its terminal bound",

        GlobalTerminalExactNotFound =>
        "global exact terminal counter associated type for sum-type bounds is not found",
        GlobalTerminalExactWrongIdent =>
        "global exact terminal counter associated type for sum-type bounds does not have the expected identifier",
        GlobalTerminalExactHasGenerics =>
        "global exact terminal counter associated type for sum-type bounds has generics",
        GlobalTerminalExactMissingBound =>
        "global exact terminal counter associated type for sum-type bounds is missing its terminal bound",

        SelfTerminalNotFound =>
        "Self terminal associated type for sum-type bounds is not found",
        SelfTerminalWrongIdent =>
        "Self terminal associated type for sum-type bounds does not have the expected identifier",
        SelfTerminalHasGenerics =>
        "Self terminal associated type for sum-type bounds has generics",
        SelfTerminalMissingBound =>
        "Self terminal associated type for sum-type bounds is missing its terminal bound",
        SelfTerminalInvalidBound =>
        "Self terminal associated type for sum-type bounds is invalid",

        SumAttrNotRemoved =>
        "sum placeholder attribute is not removed from instance trait",

        CounterParamsAreEmpty =>
        "provided instance counter params are empty, sum-type phase violated",

        TypeNumGenericNotFound =>
        "provided instance trait bound does not have typenum counters generic",
    }
}
