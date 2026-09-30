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
// ````````````````````````` INSTANCE ACCESS TRAIT UTILS `````````````````````````
// ===============================================================================

//! Utilities for constructing and consuming generated instance access
//! traits (addons).
//!
//! Every generated instance implementation produces one or more supplementary access
//! trait implementations. These traits provide a compile-time interface
//! through which later transformation phases can recover information
//! about an instance entirely through Rust's trait system, without
//! revisiting the original generated implementation.
//!
//! Each access trait represents a compile-time query whose generic
//! arguments encode the context required to resolve a particular piece
//! of instance metadata. That query is performed by projecting an
//! associated type (or associated constant) from the corresponding
//! access trait.
//!
//! This module provides the utilities required throughout that
//! projection lifecycle:
//!
//! ```text
//! Access TraitBound
//!        |
//!        V
//! <QSelf as AccessTrait<...>>::Projection
//!        |
//!        V
//! Exact<<QSelf as AccessTrait<...>>::Projection>
//! ```
//!
//! Specifically, these utilities:
//!
//! - construct generated access [`TraitBound`]s,
//! - build qualified associated-type projections,
//! - rewrite target contexts to consume those projections,
//! - construct `Exact` bounds that require an access projection to
//!   resolve to its concrete type, and
//! - validate every stage of the generated access pipeline.
//!
//! Together, these abstractions provide a uniform interface for
//! constructing, projecting, normalizing, and validating every
//! generated access trait.

// ===============================================================================
// ``````````````````````````````````` IMPORTS ```````````````````````````````````
// ===============================================================================

// --- Std crate ---
use std::{collections::HashMap, fmt::Debug, marker::PhantomData};

// --- Local crate ---
use crate::{
    Extraction, Instance, Transformation, Utilization,
    impls::utils::ImplTraitPath,
    node::{
        errors::{AccessBugs, AccessError},
        state::{FINAL_NODE, INITIAL_NODE, InstanceNode},
    },
};

// --- Proc Suite ---
use proc_suite::{BStringList, IntList, SupportCrate, misc::*};

// --- Proc-Macro crates ---
use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote};
use syn::{
    Expr, GenericArgument, Ident, ItemImpl, Lit, LitInt, PathArguments, PathSegment, TraitBound,
    Type, parse_quote, punctuated::Punctuated, token::Comma,
};

// ===============================================================================
// ```````````````````````````` ACCESS TRAIT CONTEXT `````````````````````````````
// ===============================================================================

/// Represents the type performing an access projection.
///
/// Every instance generates supplementary implementations of one or
/// more access traits ([`CounterAccess`], [`BoundaryAccess`],
/// [`OnSetAccess`], or [`TerminalAccess`]). These traits are consumed
/// through qualified associated-type projections.
///
/// ```ignore
/// <QSelf as AccessTrait<...>>::Projection
/// ```
///
/// This wrapper represents `QSelf`, the type from which the projection
/// is initiated. `QSelf` may be any valid Rust type, including a
/// concrete type, generic parameter, associated type, or another type
/// containing nested qualified paths.
///
/// During type checking, `QSelf` may therefore remain an unresolved
/// associated type or qualified projection. Ultimately, however, Rust
/// normalizes the projection until it reaches the generated access-trait
/// implementation on the global marker type, from which the requested
/// associated type is obtained.
///
/// It is used to construct projections such as:
///
/// ```ignore
/// <QSelf as AccessTrait<...>>::Projection
/// ```
pub(crate) struct AccessFrom<'a>(&'a Type);

/// Represents the contextual instance type supplied to an access trait.
///
/// Every instance generates supplementary implementations of one or
/// more access traits, all of which are implemented for the global
/// marker type.
///
/// ```ignore
/// impl CounterAccess2<HASH0, HASH1, ViaType> for Global { ... }
/// ```
///
/// In an access projection:
///
/// ```ignore
/// <QSelf as AccessTrait<ViaType>>::Projection
/// ```
///
/// this type corresponds to `ViaType`, identifying the instance type
/// whose generated access-trait implementation should be selected.
///
/// Additional generic arguments preceding `ViaType`, when present,
/// represent the identifier hash sequence used to resolve the desired
/// instance.
pub(crate) struct AccessVia<'a>(&'a Type);

/// Shared context used to construct an access projection.
///
/// This extends [`TypeNumBound`] with the information required to build
/// a complete qualified projection [`Type::Path`]:
///
/// ```ignore
/// <QSelf as AccessTrait<CounterContext..., ViaType>>::Projection
/// ```
///
/// The access [`TraitBound`] is constructed from the contextual
/// instance type and counter context, while [`AccessFrom`] supplies the
/// projection's `QSelf`.
///
/// This context is therefore sufficient to construct the complete
/// access projection associated with an instance.
///
/// The `normalized` flag indicates whether the target context has
/// already been rewritten to use the generated access projection.
/// When `false`, the transformation first normalizes the target before
/// using the access projection.
pub(crate) struct TypeNumAccess<'a> {
    /// Type supplying the projection's `QSelf`.
    from: AccessFrom<'a>,

    /// Contextual instance type supplied to the access trait.
    via: AccessVia<'a>,

    /// Positions of the instance-counter generic arguments.
    indexes: &'a IntList,

    /// Instance identifier sequence supplying the counter context.
    idents: &'a BStringList,

    /// Whether the target context has already been normalized.
    normalized: bool,
}

/// Shared context used to construct an access [`TraitBound`].
///
/// Access trait bounds generally have the form:
///
/// ```ignore
/// AccessTrait<CounterContext..., ViaType>
/// ```
///
/// where the leading generic arguments encode an instance's counter
/// context, and the final generic argument identifies the contextual
/// instance type.
///
/// Access traits without a counter context, such as
/// [`TerminalAccess`], instead have the form:
///
/// ```ignore
/// AccessTrait<ViaType>
/// ```
///
/// This context supplies the information required to generate the
/// former. The counter indexes determine the number of counter-context
/// generic arguments, while the identifier sequence provides the values
/// used to construct those arguments. Finally, the contextual instance
/// type is appended as the last generic argument.
///
/// It is consumed by [`CounterAccess`], [`BoundaryAccess`], and
/// [`OnSetAccess`] when constructing their corresponding access
/// `TraitBound`s.
pub(crate) struct TypeNumBound<'a> {
    /// Contextual instance type appended as the final generic argument.
    via: &'a Type,

    /// Positions of the instance-counter generic arguments used to
    /// determine the counter-context arity.
    indexes: &'a IntList,

    /// Instance counters identifiers sequence supplying the counter-context
    /// generic arguments.
    idents: &'a BStringList,
}

/// Shared context used to construct a unique access [`TraitBound`].
///
/// Unlike [`TypeNumBound`], unique access traits do not require a
/// counter context. Their bounds therefore have the form:
///
/// ```ignore
/// AccessTrait<ViaType>
/// ```
///
/// Since no counter-context generic arguments are generated, the
/// instance identifier sequence is unnecessary. Only the contextual
/// instance type is required to construct the resulting `TraitBound`.
///
/// E.g., this context is consumed by [`TerminalAccess`] when constructing its
/// corresponding access `TraitBound`.
pub(crate) struct UniqueBound<'a> {
    /// Contextual instance type appended as the only generic argument.
    via: &'a Type,

    /// Positions of the instance-counter generic arguments.
    ///
    /// If they not used to construct the access `TraitBound`,
    /// they are retained so surrounding transformations can normalize
    /// target contexts consistently.
    indexes: &'a IntList,
}

/// Shared context used to construct a unique access projection.
///
/// This extends [`UniqueBound`] with the information required to build
/// a complete qualified projection:
///
/// ```ignore
/// <QSelf as AccessTrait<ViaType>>::Projection
/// ```
///
/// The access [`TraitBound`] is constructed from the contextual
/// instance type, while [`AccessFrom`] supplies the projection's
/// `QSelf`.
///
/// This context is therefore sufficient to construct the complete
/// unique access projection.
///
/// The `normalized` flag indicates whether the target context has
/// already been rewritten to use the generated access projection.
/// When `false`, the transformation first normalizes the target before
/// utilizing the access projection.
pub(crate) struct UniqueAccess<'a> {
    /// Type supplying the projection's `QSelf`.
    from: AccessFrom<'a>,

    /// Contextual instance type supplied to the access trait.
    via: &'a Type,

    /// Positions of the instance-counter generic arguments.
    indexes: &'a IntList,

    /// Whether the target context has already been normalized.
    normalized: bool,
}

/// Converts raw access-bound inputs into a [`TypeNumAccess`] context.
impl<'a>
    From<(
        AccessFrom<'a>,
        AccessVia<'a>,
        &'a IntList,
        &'a BStringList,
        bool,
    )> for TypeNumAccess<'a>
{
    fn from(
        value: (
            AccessFrom<'a>,
            AccessVia<'a>,
            &'a IntList,
            &'a BStringList,
            bool,
        ),
    ) -> Self {
        let (from, via, indexes, idents, normalized) = value;
        Self {
            from,
            via,
            indexes,
            idents,
            normalized,
        }
    }
}

/// Converts raw access-bound inputs into a [`TypeNumBound`] context.
impl<'a> From<(&'a Type, &'a IntList, &'a BStringList)> for TypeNumBound<'a> {
    fn from(value: (&'a Type, &'a IntList, &'a BStringList)) -> Self {
        let (via, indexes, idents) = value;
        Self {
            via,
            indexes,
            idents,
        }
    }
}

/// Discards projection-specific state (`from`), yielding a [`TypeNumBound`].
impl<'a> From<TypeNumAccess<'a>> for TypeNumBound<'a> {
    fn from(value: TypeNumAccess<'a>) -> Self {
        let TypeNumAccess {
            via,
            indexes,
            idents,
            ..
        } = value;

        Self {
            via: via.0,
            indexes,
            idents,
        }
    }
}

/// Extends a [`TypeNumBound`] into a [`TypeNumAccess`] context.
impl<'a> From<(&'a Type, TypeNumBound<'a>, bool)> for TypeNumAccess<'a> {
    fn from(value: (&'a Type, TypeNumBound<'a>, bool)) -> Self {
        let (
            from,
            TypeNumBound {
                via,
                indexes,
                idents,
                ..
            },
            normalized,
        ) = value;

        Self {
            from: AccessFrom(from),
            via: AccessVia(via),
            indexes,
            idents,
            normalized,
        }
    }
}

/// Extends a [`TypeNumBound`] into a [`TypeNumAccess`] context.
impl<'a> From<(&'a Type, TypeNumBound<'a>)> for TypeNumAccess<'a> {
    fn from(value: (&'a Type, TypeNumBound<'a>)) -> Self {
        let (
            from,
            TypeNumBound {
                via,
                indexes,
                idents,
                ..
            },
        ) = value;

        Self {
            from: AccessFrom(from),
            via: AccessVia(via),
            indexes,
            idents,
            normalized: true,
        }
    }
}

/// Converts raw unique-access inputs into a [`UniqueBound`] context.
impl<'a> From<(&'a Type, &'a IntList)> for UniqueBound<'a> {
    fn from(value: (&'a Type, &'a IntList)) -> Self {
        let (via, indexes) = value;

        Self { via, indexes }
    }
}

/// Converts a tuple into a [`UniqueAccess`] context.
impl<'a> From<(AccessFrom<'a>, &'a Type, &'a IntList, bool)> for UniqueAccess<'a> {
    fn from(value: (AccessFrom<'a>, &'a Type, &'a IntList, bool)) -> Self {
        let (from, via, indexes, normalized) = value;

        Self {
            from,
            via,
            indexes,
            normalized,
        }
    }
}

/// Discards projection-specific state (`from`), yielding a [`UniqueBound`].
impl<'a> From<UniqueAccess<'a>> for UniqueBound<'a> {
    fn from(value: UniqueAccess<'a>) -> Self {
        let UniqueAccess { via, indexes, .. } = value;

        Self { via, indexes }
    }
}

/// Extends a [`UniqueBound`] into a [`UniqueAccess`] context.
impl<'a> From<(&'a Type, UniqueBound<'a>, bool)> for UniqueAccess<'a> {
    fn from(value: (&'a Type, UniqueBound<'a>, bool)) -> Self {
        let (from, UniqueBound { via, indexes, .. }, normalized) = value;

        Self {
            from: AccessFrom(from),
            via,
            indexes,
            normalized,
        }
    }
}

/// Narrows a [`TypeNumBound`] into a [`UniqueBound`].
impl<'a> From<TypeNumBound<'a>> for UniqueBound<'a> {
    fn from(value: TypeNumBound<'a>) -> Self {
        let TypeNumBound { via, indexes, .. } = value;

        Self { via, indexes }
    }
}

/// Narrows a [`TypeNumAccess`] into a [`UniqueAccess`] context.
impl<'a> From<TypeNumAccess<'a>> for UniqueAccess<'a> {
    fn from(value: TypeNumAccess<'a>) -> Self {
        let TypeNumAccess {
            from,
            via,
            indexes,
            normalized,
            ..
        } = value;

        Self {
            from,
            via: via.0,
            indexes,
            normalized,
        }
    }
}

// ===============================================================================
// `````````````````````````` ACCESS TRAIT PROJECTIONS ```````````````````````````
// ===============================================================================

/// Common interface implemented by every access projection.
///
/// Each implementation represents a family of generated access traits
/// and provides the metadata required to construct and project through
/// those traits.
///
/// This abstraction allows the access generation and transformation
/// utilities to operate uniformly across every access family.
pub(crate) trait AccessProjection: Debug + Clone {
    /// Returns the access [`TraitBound`] describing this projection.
    ///
    /// Example:
    ///
    /// ```ignore
    /// AccessTrait2<CTX0, CTX1, ViaType>
    /// ```
    fn bound(&self) -> &TraitBound;

    /// Returns the identifier of the primary associated type exposed by
    /// the generated access trait.
    ///
    /// Example:
    ///
    /// ```ignore
    /// <QSelf as AccessTrait<...>>::Projection
    ///                              ^^^^^^^^^^
    /// ```
    fn projection() -> Ident;

    /// Returns the identifier prefix of the generated access-trait
    /// family.
    ///
    /// Access traits are typically generated as indexed families:
    ///
    /// ```text
    /// Prefix0
    /// Prefix1
    /// ...
    /// PrefixN
    /// ```
    ///
    /// where the suffix may denote the number of counter-context generic
    /// arguments.
    fn prefix() -> Ident;
}

// ===============================================================================
// ```````````````````````````` TERMINAL ACCESS TRAIT ````````````````````````````
// ===============================================================================

/// Represents the unique terminal access-trait bound.
///
/// Unlike the other access families, terminal access is generated only
/// once for an instance type. It carries no counter-context generic
/// arguments because a terminal instance is globally unique for the
/// contextual instance type.
///
/// For example:
///
/// ```text
/// InstanceTrait<0,0>
/// InstanceTrait<0,1>
/// InstanceTrait<1,0>   #[last_instance]
/// ```
///
/// gives rise to the unique access trait:
///
/// ```ignore
/// TerminalAccess<InstanceType>
/// ```
///
/// Since there is exactly one terminal instance for an implementing
/// type, no parent identifier sequence or counter context is required
/// to resolve it.
///
/// This type supports both extraction and transformation:
///
/// - **Extraction** constructs the corresponding terminal access
///   [`TraitBound`].
/// - **Transformation** rewrites a target trait path by replacing the
///   original user-facing counter generic arguments with the generated
///   terminal access projection, inserting it at the first
///   instance-counter generic position.
#[derive(Debug, Clone)]
pub(crate) struct TerminalAccess(pub(crate) TraitBound);

impl AccessProjection for TerminalAccess {
    fn bound(&self) -> &TraitBound {
        &self.0
    }

    fn projection() -> Ident {
        format_ident!("Terminal")
    }

    fn prefix() -> Ident {
        format_ident!("TerminalAccess")
    }
}

impl<'a> Extraction<UniqueBound<'a>> for TerminalAccess {
    fn raw_extract(from: &UniqueBound<'a>, _: &()) -> Result<Self, proc_macro2::TokenStream> {
        let UniqueBound { via, .. } = from;
        let bound = build_access_bound::<Self>(0, &Vec::new(), via, true)?;
        Ok(Self(bound))
    }

    fn validate_extract(
        &self,
        from: &UniqueBound<'a>,
        _: Option<&()>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let UniqueBound { via, .. } = from;
        validate_bound(&self.0, || {
            build_access_bound::<Self>(0, &Vec::new(), via, true)
        })?;
        Ok(())
    }
}

impl<'a> Transformation<PathSegment, UniqueAccess<'a>> for TerminalAccess {
    fn raw_transform(
        &self,
        transform: &mut PathSegment,
        context: &UniqueAccess<'a>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let UniqueAccess {
            from,
            indexes,
            normalized,
            ..
        } = context;
        if !normalized {
            remove_gen_args(transform, indexes)?;
        }
        insert_gen_arg(transform, indexes, || build_access_gen_arg(from.0, self))?;
        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &PathSegment,
        context: Option<&UniqueAccess<'a>>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let Some(UniqueAccess { from, indexes, .. }) = context else {
            return Ok(());
        };
        let arg = build_access_gen_arg(from.0, self)?;
        validate_gen_arg(transform, indexes, || Ok(arg))?;
        Ok(())
    }
}

// ===============================================================================
// ````````````````````````````` ONSET ACCESS TRAIT ``````````````````````````````
// ===============================================================================

/// Represents an onset access-trait bound.
///
/// An onset access trait resolves the beginning of a child instance
/// hierarchy beneath a partially resolved parent identifier sequence.
/// Consequently, it carries the counter-context generic arguments
/// required to uniquely identify that parent context.
///
/// Given only the parent identifier sequence together with the
/// contextual instance type, the generated access trait allows the
/// onset (first instance) of the corresponding child hierarchy to be
/// recovered without requiring any of the remaining child identifiers.
///
/// For example:
///
/// ```text
/// InstanceTrait<0,0,0>
/// InstanceTrait<0,0,1>
/// InstanceTrait<0,0,2>
/// InstanceTrait<0,1,0>
/// InstanceTrait<0,2,0>
/// InstanceTrait<1,0,0>
/// ```
///
/// Once the parent identifier sequence corresponding to:
///
/// ```text
/// InstanceTrait<1>
/// ```
///
/// has been resolved, the beginning of its child hierarchy is already
/// known to be:
///
/// ```text
/// InstanceTrait<1,0,0>
/// ```
///
/// Likewise, resolving:
///
/// ```text
/// InstanceTrait<0,1>
/// ```
///
/// immediately determines the onset of that child hierarchy as:
///
/// ```text
/// InstanceTrait<0,1,0>
/// ```
///
/// These mappings are represented by access trait bounds such as:
///
/// ```ignore
/// OnSetAccess1<HASH_OF_1, InstanceType>
///
/// OnSetAccess2<HASH_OF_1, HASH_OF_0, InstanceType>
/// ```
///
/// This type supports both extraction and transformation:
///
/// - **Extraction** constructs the corresponding onset access
///   [`TraitBound`].
/// - **Transformation** rewrites a target trait path by replacing the
///   original user-facing counter generic arguments with the generated
///   onset access projection, inserting it at the first
///   instance-counter generic position.
#[derive(Debug, Clone)]
pub(crate) struct OnSetAccess(pub(crate) TraitBound);

impl AccessProjection for OnSetAccess {
    fn bound(&self) -> &TraitBound {
        &self.0
    }

    fn projection() -> Ident {
        format_ident!("OnSet")
    }

    fn prefix() -> Ident {
        format_ident!("OnSetAccess")
    }
}

impl<'a> Extraction<TypeNumBound<'a>> for OnSetAccess {
    fn raw_extract(from: &TypeNumBound<'a>, _: &()) -> Result<Self, proc_macro2::TokenStream> {
        let TypeNumBound { via, idents, .. } = from;

        let hash_exprs = build_hash_exprs(idents);
        let bound = build_access_bound::<Self>(idents.bytes.len(), &hash_exprs, via, false)?;
        Ok(Self(bound))
    }

    fn validate_extract(
        &self,
        from: &TypeNumBound<'a>,
        _: Option<&()>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let TypeNumBound { via, idents, .. } = from;
        validate_bound(&self.0, || {
            let hash_exprs = build_hash_exprs(idents);
            build_access_bound::<Self>(idents.bytes.len(), &hash_exprs, via, false)
        })?;
        Ok(())
    }
}

impl<'a> Transformation<PathSegment, TypeNumAccess<'a>> for OnSetAccess {
    fn raw_transform(
        &self,
        transform: &mut PathSegment,
        context: &TypeNumAccess<'a>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let TypeNumAccess {
            from,
            indexes,
            normalized,
            ..
        } = context;
        if !normalized {
            remove_gen_args(transform, indexes)?;
        }
        insert_gen_arg(transform, indexes, || build_access_gen_arg(from.0, self))?;
        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &PathSegment,
        context: Option<&TypeNumAccess<'a>>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let Some(TypeNumAccess { from, indexes, .. }) = context else {
            return Ok(());
        };
        let arg = build_access_gen_arg(from.0, self)?;
        validate_gen_arg(transform, indexes, || Ok(arg))?;
        Ok(())
    }
}

// ===============================================================================
// ```````````````````````````` BOUNDARY ACCESS TRAIT ````````````````````````````
// ===============================================================================

/// Represents a boundary access-trait bound.
///
/// A boundary access trait resolves the terminal (boundary) instance of
/// a child hierarchy beneath a partially resolved parent identifier
/// sequence. Consequently, it carries the counter-context generic
/// arguments required to uniquely identify that parent context.
///
/// Given only the parent identifier sequence together with the
/// contextual instance type, the generated access trait allows the
/// boundary (last instance) of the corresponding child hierarchy to be
/// recovered without requiring any of the remaining child identifiers.
///
/// For example:
///
/// ```text
/// InstanceTrait<0,0,0>
/// InstanceTrait<0,0,1>
/// InstanceTrait<0,0,2>   #[last_instance(2)]
/// InstanceTrait<0,1,0>   #[last_instance(2)]
/// InstanceTrait<0,2,0>   #[last_instance(1)]
/// InstanceTrait<1,0,0>   #[last_instance(0,1,2)]
/// ```
///
/// Once the parent identifier sequence corresponding to:
///
/// ```text
/// InstanceTrait<0>
/// ```
///
/// has been resolved, the boundary of its child hierarchy is already
/// known to be:
///
/// ```text
/// InstanceTrait<0,2,0>
/// ```
///
/// Likewise, resolving:
///
/// ```text
/// InstanceTrait<0,0>
/// ```
///
/// immediately determines the boundary of that child hierarchy as:
///
/// ```text
/// InstanceTrait<0,0,2>
/// ```
///
/// These mappings are represented by access trait bounds. For example,
/// `InstanceTrait<0,2,0>` declares counter dimension `1` terminal,
/// thereby establishing the boundary for both counter dimensions `1`
/// and `2`, yielding trait bounds such as:
///
/// ```ignore
/// BoundaryAccess1<HASH_OF_0, InstanceType>
///
/// BoundaryAccess2<HASH_OF_0, HASH_OF_2, InstanceType>
/// ```
///
/// This type supports both extraction and transformation:
///
/// - **Extraction** constructs the corresponding boundary access
///   [`TraitBound`].
/// - **Transformation** rewrites a target trait path by replacing the
///   original user-facing counter generic arguments with the generated
///   boundary access projection, inserting it at the first
///   instance-counter generic position.
#[derive(Debug, Clone)]
pub(crate) struct BoundaryAccess(pub(crate) TraitBound);

impl AccessProjection for BoundaryAccess {
    fn bound(&self) -> &TraitBound {
        &self.0
    }

    fn projection() -> Ident {
        format_ident!("Boundary")
    }

    fn prefix() -> Ident {
        format_ident!("BoundaryAccess")
    }
}

impl<'a> Extraction<TypeNumBound<'a>> for BoundaryAccess {
    fn raw_extract(from: &TypeNumBound<'a>, _: &()) -> Result<Self, proc_macro2::TokenStream> {
        let TypeNumBound { via, idents, .. } = from;
        let hash_exprs = build_hash_exprs(idents);
        let bound = build_access_bound::<Self>(idents.bytes.len(), &hash_exprs, via, false)?;
        Ok(Self(bound))
    }

    fn validate_extract(
        &self,
        from: &TypeNumBound<'a>,
        _: Option<&()>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let TypeNumBound { via, idents, .. } = from;
        validate_bound(&self.0, || {
            let hash_exprs = build_hash_exprs(idents);
            build_access_bound::<Self>(idents.bytes.len(), &hash_exprs, via, false)
        })?;
        Ok(())
    }
}

impl<'a> Transformation<PathSegment, TypeNumAccess<'a>> for BoundaryAccess {
    fn raw_transform(
        &self,
        transform: &mut PathSegment,
        context: &TypeNumAccess<'a>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let TypeNumAccess {
            from,
            indexes,
            normalized,
            ..
        } = context;
        if !normalized {
            remove_gen_args(transform, indexes)?;
        }
        insert_gen_arg(transform, indexes, || build_access_gen_arg(from.0, self))?;
        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &PathSegment,
        context: Option<&TypeNumAccess<'a>>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let Some(TypeNumAccess { from, indexes, .. }) = context else {
            return Ok(());
        };
        let arg = build_access_gen_arg(from.0, self)?;
        validate_gen_arg(transform, indexes, || Ok(arg))?;
        Ok(())
    }
}

// ===============================================================================
// ````````````````````````````` COUNTER ACCESS TRAIT ````````````````````````````
// ===============================================================================

/// Represents a counter access-trait bound.
///
/// A counter access trait directly resolves an instance's typenum
/// counter tuple from its identifier sequence.
///
/// Rather than requiring the numeric counter values themselves, the
/// generated access trait uses the identifier sequence together with
/// the contextual instance type to uniquely identify the corresponding
/// instance.
///
/// For example:
///
/// ```text
/// InstanceTrait<0,0,0>
/// InstanceTrait<0,0,1>
/// InstanceTrait<0,0,2>
/// InstanceTrait<0,1,0>
/// InstanceTrait<0,2,0>
/// InstanceTrait<1,0,0>
/// ```
///
/// Resolving the identifier sequence corresponding to:
///
/// ```text
/// InstanceTrait<0,1,0>
/// ```
///
/// allows the associated typenum counter tuple:
///
/// ```text
/// (U0, U1, U0)
/// ```
///
/// to be recovered directly without explicitly knowing the numeric
/// counter values.
///
/// These mappings are represented by access trait bounds such as:
///
/// ```ignore
/// CounterAccess3<HASH_OF_0, HASH_OF_1, HASH_OF_0, InstanceType>
/// ```
///
/// This type supports both extraction and transformation:
///
/// - **Extraction** constructs the corresponding counter access
///   [`TraitBound`].
/// - **Transformation** rewrites a target trait path by replacing the
///   original user-facing counter generic arguments with the generated
///   counter access projection, inserting it at the first
///   instance-counter generic position.
#[derive(Debug, Clone)]
pub(crate) struct CounterAccess(pub(crate) TraitBound);

impl AccessProjection for CounterAccess {
    fn bound(&self) -> &TraitBound {
        &self.0
    }

    fn projection() -> Ident {
        format_ident!("Counter")
    }

    fn prefix() -> Ident {
        format_ident!("CounterAccess")
    }
}

impl<'a> Extraction<TypeNumBound<'a>> for CounterAccess {
    fn raw_extract(from: &TypeNumBound<'a>, _: &()) -> Result<Self, proc_macro2::TokenStream> {
        let TypeNumBound {
            via,
            idents,
            indexes,
            ..
        } = from;
        let hash_exprs = build_hash_exprs(idents);
        let bound = build_access_bound::<Self>(indexes.ints.len(), &hash_exprs, via, false)?;
        Ok(Self(bound))
    }

    fn validate_extract(
        &self,
        from: &TypeNumBound<'a>,
        _: Option<&()>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let TypeNumBound {
            via,
            idents,
            indexes,
            ..
        } = from;
        validate_bound(&self.0, || {
            let hash_exprs = build_hash_exprs(idents);
            build_access_bound::<Self>(indexes.ints.len(), &hash_exprs, via, false)
        })?;
        Ok(())
    }
}

impl<'a> Transformation<PathSegment, TypeNumAccess<'a>> for CounterAccess {
    fn raw_transform(
        &self,
        transform: &mut PathSegment,
        context: &TypeNumAccess<'a>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let TypeNumAccess {
            from,
            indexes,
            normalized,
            ..
        } = context;
        if !normalized {
            remove_gen_args(transform, indexes)?;
        }
        insert_gen_arg(transform, indexes, || build_access_gen_arg(from.0, self))?;
        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &PathSegment,
        context: Option<&TypeNumAccess<'a>>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let Some(TypeNumAccess { from, indexes, .. }) = context else {
            return Ok(());
        };
        let arg = build_access_gen_arg(from.0, self)?;
        validate_gen_arg(transform, indexes, || Ok(arg))?;
        Ok(())
    }
}

// ===============================================================================
// `````````````````````````````` EXACT ACCESS TRAIT `````````````````````````````
// ===============================================================================

/// Represents an exact access-trait bound.
///
/// This type wraps an [`AccessProjection`] within the `Exact` trait,
/// producing a bound of the form:
///
/// ```ignore
/// Exact< <AccessFrom as AccessTrait<...>>::Projection >
/// ```
///
/// Since the library provides only:
///
/// ```ignore
/// impl<T> Exact<T> for T {}
/// ```
///
/// satisfying this bound requires the associated projection to resolve
/// exactly to the projected type itself. Consequently, the projection
/// cannot remain an unresolved associated type or participate in any
/// further indirection.
///
/// For example, given the access bound:
///
/// ```ignore
/// CounterAccess3<CTX0, CTX1, CTX2, InstanceType>
/// ```
///
/// this type constructs:
///
/// ```ignore
/// Exact<
///     <AccessFrom as CounterAccess3<CTX0, CTX1, CTX2, InstanceType>>::Counter
/// >
/// ```
///
/// which guarantees that the associated `Counter` projection resolves
/// to its concrete type before the `Exact` bound can be satisfied.
#[derive(Debug, Clone)]
pub(crate) struct ExactAccess<A: AccessProjection> {
    pub(crate) bound: TraitBound,
    marker: PhantomData<A>,
}

impl<A: AccessProjection> ExactAccess<A> {
    pub(crate) fn new(t: &TraitBound) -> Self {
        Self {
            bound: t.clone(),
            marker: PhantomData,
        }
    }
}

impl<A: AccessProjection + 'static> Extraction<A, Type> for ExactAccess<A> {
    fn raw_extract(from: &A, context: &Type) -> Result<Self, proc_macro2::TokenStream> {
        let bound = from.bound();
        let project = A::projection();
        let crate_of = Instance::support_crate();
        let new_bound = parse_quote!(
            #crate_of::Exact< <#context as #bound>::#project >
        );
        Ok(Self {
            bound: new_bound,
            marker: PhantomData,
        })
    }

    fn validate_extract(
        &self,
        from: &A,
        context: Option<&Type>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let bound = &self.bound;
        let mut iter = bound.path.segments.iter();
        let Some(crate_exp) = iter.next() else {
            return Err(AccessBugs::ExactAccessGenBoundQualifierNotFound {}.into());
        };
        let crate_of = Instance::support_crate();
        if crate_exp.ident != crate_of {
            return Err(AccessBugs::ExactAccessGenBoundQualifierNotSupportCrate {}.into());
        };
        let Some(exact_exp) = iter.next() else {
            return Err(AccessBugs::ExactAccessGenBoundIdentNotFound {}.into());
        };
        let exact_trait = format_ident!("Exact");
        if exact_exp.ident != exact_trait {
            return Err(AccessBugs::ExactAccessGenBoundIdentInvalid {}.into());
        };
        let PathArguments::AngleBracketed(angle) = &exact_exp.arguments else {
            return Err(AccessBugs::ExactAccessGenBoundArgsNotAngle {}.into());
        };
        if angle.args.len() > 1 {
            return Err(AccessBugs::ExactAccessGenBoundArgsInconsistent {}.into());
        }
        let arg = &angle.args[0];
        let GenericArgument::Type(ty) = arg else {
            return Err(AccessBugs::ExactAccessGenBoundArgNotType {}.into());
        };
        let Type::Path(path) = ty else {
            return Err(AccessBugs::ExactAccessGenBoundArgNotTypePath {}.into());
        };
        let Some(q_self) = &path.qself else {
            return Err(AccessBugs::ExactAccessGenBoundArgTypeQSelfNotFound {}.into());
        };
        let pos = q_self.position;
        if let Some(context) = context {
            if q_self.ty.to_token_stream().to_string() != context.to_token_stream().to_string() {
                return Err(AccessBugs::ExactAccessGenBoundArgTypeQSelfInvalid {}.into());
            }
        }
        let mut iter = path.path.segments.iter().take(pos + 1).rev();
        let Some(project_exp) = iter.next() else {
            return Err(AccessBugs::ExactAccessGenBoundArgTypePathProjectionNotFound {}.into());
        };
        if project_exp.ident != A::projection() {
            return Err(AccessBugs::ExactAccessGenBoundArgTypePathProjectionInvalid {}.into());
        }

        let given_bound = from.bound();
        let Some(given_bound_first) = given_bound.path.segments.first() else {
            return Err(AccessBugs::ExactAccessBoundQualifierNotFound {}.into());
        };
        if given_bound_first.ident != crate_of {
            return Err(AccessBugs::ExactAccessBoundQualifierInvalid {}.into());
        };
        let Some(bound_exp) = iter.next() else {
            return Err(AccessBugs::ExactAccessGenBoundPathNotFound {}.into());
        };
        let Some(given_bound_seg) = given_bound.path.segments.last() else {
            return Err(AccessBugs::ExactAccessBoundPathNotFound {}.into());
        };
        if bound_exp.to_token_stream().to_string() != given_bound_seg.to_token_stream().to_string()
        {
            return Err(AccessBugs::ExactAccessGenBoundPathInconsistent {}.into());
        }
        Ok(())
    }
}

// ===============================================================================
// ``````````````````````````` INITIAL INSTANCE ACCESS ```````````````````````````
// ===============================================================================

#[derive(Debug, Clone)]
pub(crate) struct InitialAssocAccess;

pub(crate) struct NeighbourBound<'a> {
    base: &'a Ident,
    indexes: &'a IntList,
    normalized: bool,
    impl_of: Option<&'a ItemImpl>,
}

impl<'a> From<(&'a Ident, &'a IntList, bool, Option<&'a ItemImpl>)> for NeighbourBound<'a> {
    fn from(value: (&'a Ident, &'a IntList, bool, Option<&'a ItemImpl>)) -> Self {
        let (base, indexes, normalized, impl_of) = value;
        Self {
            base,
            indexes,
            normalized,
            impl_of,
        }
    }
}

impl<'a> Transformation<PathSegment, NeighbourBound<'a>> for InitialAssocAccess {
    fn raw_transform(
        &self,
        transform: &mut PathSegment,
        context: &NeighbourBound<'a>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let NeighbourBound {
            base,
            indexes,
            normalized,
            impl_of,
        } = context;
        if !normalized {
            remove_gen_args(transform, indexes)?;
        }
        insert_gen_arg(transform, indexes, || {
            let ident = gen_type_ident_from_ident_with_suffix::<InstanceNode>(base, INITIAL_NODE);
            let prefix = match impl_of {
                Some(impl_of) => {
                    let self_ty = &impl_of.self_ty;
                    let trait_path = ImplTraitPath::checked_utilize(impl_of, &())?.0;
                    quote! {<#self_ty as #trait_path>}
                }
                None => quote! {Self},
            };
            Ok(parse_quote!(#prefix::#ident))
        })?;
        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &PathSegment,
        context: Option<&NeighbourBound<'a>>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let Some(NeighbourBound {
            base,
            indexes,
            impl_of,
            ..
        }) = context
        else {
            return Ok(());
        };
        let ident = gen_type_ident_from_ident_with_suffix::<InstanceNode>(base, INITIAL_NODE);
        let prefix = match impl_of {
            Some(impl_of) => {
                let self_ty = &impl_of.self_ty;
                let trait_path = ImplTraitPath::checked_utilize(impl_of, &())?.0;
                quote! {<#self_ty as #trait_path>}
            }
            None => quote! {Self},
        };
        let arg = parse_quote!(#prefix::#ident);
        validate_gen_arg(transform, indexes, || Ok(arg))?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub(crate) struct FinalAssocAccess;

impl<'a> Transformation<PathSegment, NeighbourBound<'a>> for FinalAssocAccess {
    fn raw_transform(
        &self,
        transform: &mut PathSegment,
        context: &NeighbourBound<'a>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let NeighbourBound {
            base,
            indexes,
            normalized,
            impl_of,
        } = context;
        if !normalized {
            remove_gen_args(transform, indexes)?;
        }
        insert_gen_arg(transform, indexes, || {
            let ident = gen_type_ident_from_ident_with_suffix::<InstanceNode>(base, FINAL_NODE);
            let prefix = match impl_of {
                Some(impl_of) => {
                    let self_ty = &impl_of.self_ty;
                    let trait_path = ImplTraitPath::checked_utilize(impl_of, &())?.0;
                    quote! {<#self_ty as #trait_path>}
                }
                None => quote! {Self},
            };
            Ok(parse_quote!(#prefix::#ident))
        })?;
        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &PathSegment,
        context: Option<&NeighbourBound<'a>>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let Some(NeighbourBound {
            base,
            indexes,
            impl_of,
            ..
        }) = context
        else {
            return Ok(());
        };
        let ident = gen_type_ident_from_ident_with_suffix::<InstanceNode>(base, FINAL_NODE);
        let prefix = match impl_of {
            Some(impl_of) => {
                let self_ty = &impl_of.self_ty;
                let trait_path = ImplTraitPath::checked_utilize(impl_of, &())?.0;
                quote! {<#self_ty as #trait_path>}
            }
            None => quote! {Self},
        };
        let arg = parse_quote!(#prefix::#ident);
        validate_gen_arg(transform, indexes, || Ok(arg))?;
        Ok(())
    }
}

// ===============================================================================
// `````````````````````````````` HELPER UTILITIES ```````````````````````````````
// ===============================================================================

/// Builds the counter-context expressions used by generated access
/// traits.
///
/// Every identifier in the supplied sequence is transformed into the
/// canonical compile-time expression used to represent that identifier
/// within access-trait generic arguments.
///
/// ```text
/// [b"x", b"yy", b"zzz"]
/// ```
///
/// becomes:
///
/// ```ignore
/// [
///     { crate::hash_ident(b"x") },
///     { crate::hash_ident(b"yy") },
///     { crate::hash_ident(b"zzz") },
/// ]
/// ```
///
/// These expressions are later supplied to [`build_access_bound`] when
/// constructing generated access trait bounds.
fn build_hash_exprs(idents: &BStringList) -> Vec<Expr> {
    let crate_of = Instance::support_crate();
    let mut collect = Vec::new();
    for ident in &idents.bytes {
        let expr: Expr = parse_quote!({#crate_of::hash_ident(#ident)});
        collect.push(expr);
    }
    collect
}

/// Inserts an access projection into an instance-trait generic list.
///
/// The first counter generic index determines where the generated
/// projection should be inserted after any user-facing counter generic
/// arguments have been normalized.
///
/// For example:
///
/// ```ignore
/// Example<'a, T, U>
/// ```
///
/// with:
///
/// ```text
/// first counter index = 1
/// generated projection = <Self as AccessTrait<...>>::Projection
/// ```
///
/// becomes:
///
/// ```ignore
/// Example<'a,
///     <Self as AccessTrait<...>>::Projection,
///     T,
///     U,
/// >
/// ```
///
/// The supplied closure is evaluated only after the insertion position
/// has been validated.
pub(crate) fn insert_gen_arg(
    trait_path: &mut PathSegment,
    indexes: &IntList,
    build: impl FnOnce() -> Result<GenericArgument, TokenStream>,
) -> Result<(), TokenStream> {
    let PathArguments::AngleBracketed(angle) = &mut trait_path.arguments else {
        return Err(AccessError::TraitPathNotAngleArgs {
            segment: trait_path.clone(),
        }
        .into());
    };

    let Some(first) = indexes.ints.first() else {
        return Err(AccessError::CountersGenericsIndexesEmpty {
            indexes: indexes.clone(),
        }
        .into());
    };

    let index = parse_pos_usize(first)?;
    let arg = build()?;

    let len = angle.args.len();
    if index > len {
        return Err(AccessError::InvalidFirstCounter {
            lit: first.clone(),
            index,
            len,
        }
        .into());
    }

    angle.args.insert(index, arg);

    Ok(())
}

/// Constructs the access-projection generic argument.
///
/// Given an implementing type and an access trait bound, this helper
/// produces the qualified associated-type projection used as the
/// replacement generic argument.
///
/// ```ignore
/// <QSelf as AccessTrait<...>>::Projection
/// ```
///
/// This projection later replaces the original user-facing counter
/// generic arguments during access normalization.
fn build_access_gen_arg<P: AccessProjection>(
    self_ty: &Type,
    access: &P,
) -> Result<GenericArgument, TokenStream> {
    let bound = access.bound();
    let projection = P::projection();

    Ok(parse_quote!(
        <#self_ty as #bound>::#projection
    ))
}

/// Constructs an access [`TraitBound`].
///
/// Access trait bounds generally have one of the following forms:
///
/// ```ignore
/// AccessTraitN<CounterContext..., ViaType>
/// ```
///
/// or, for unique access traits (when suffix len is also == 0):
///
/// ```ignore
/// AccessTrait<ViaType>
/// ```
///
/// The generated trait identifier is derived from the projection's
/// prefix together with the supplied counter-context arity.
///
/// The resulting bound is suitable for constructing qualified access
/// projections such as:
///
/// ```ignore
/// <QSelf as AccessTrait<...>>::Projection
/// ```
fn build_access_bound<P: AccessProjection>(
    suffix_len: usize,
    hash_exprs: &[Expr],
    base_ty: &Type,
    unique: bool,
) -> Result<TraitBound, TokenStream> {
    let trait_prefix = P::prefix();
    let crate_of = Instance::support_crate();
    let trait_ident = format_ident!("{trait_prefix}{suffix_len}");

    if suffix_len == 0 {
        if unique {
            let unique_trait_ident = format_ident!("{trait_prefix}");
            return Ok(parse_quote!(
                #crate_of::#unique_trait_ident<#base_ty>
            ));
        }

        return Ok(parse_quote!(
            #crate_of::#trait_ident<#base_ty>
        ));
    }

    Ok(parse_quote!(
        #crate_of::#trait_ident<#(#hash_exprs),*, #base_ty>
    ))
}

/// Validates a generated access projection generic argument.
///
/// The first counter generic position is expected to contain the access
/// projection previously produced by [`build_access_gen_arg`]. This
/// helper reconstructs the expected projection and verifies that the
/// existing generic argument is structurally identical.
///
/// Validation is performed using token equivalence after reconstructing
/// the expected projection.
fn validate_gen_arg(
    trait_path: &PathSegment,
    indexes: &IntList,
    build: impl FnOnce() -> Result<GenericArgument, TokenStream>,
) -> Result<(), TokenStream> {
    let Some(first) = indexes.ints.first() else {
        return Err(AccessBugs::CountersGenericsIndexesEmpty {}.into());
    };
    let index = parse_pos_usize(first)?;

    let PathArguments::AngleBracketed(angle) = &trait_path.arguments else {
        return Err(AccessBugs::TraitPathNotAngleArgs {}.into());
    };

    let Some(arg) = angle.args.get(index) else {
        return Err(AccessBugs::AccessTyGenericArgNotFound {}.into());
    };

    let GenericArgument::Type(found) = arg else {
        return Err(AccessBugs::AccessGenericArgNotType {}.into());
    };

    let built = build()?;
    let exp: Type = parse_quote!(#built);

    if found.to_token_stream().to_string() != exp.to_token_stream().to_string() {
        return Err(AccessBugs::UnexpectedAccessTyGenericArg {}.into());
    }

    Ok(())
}

/// Validates a generated access [`TraitBound`].
///
/// The expected bound is reconstructed using the supplied builder and
/// compared against the existing bound using token equivalence.
///
/// This helper centralizes validation for every generated access trait
/// family while allowing each caller to provide its own bound
/// construction logic.
fn validate_bound(
    found: &TraitBound,
    build: impl FnOnce() -> Result<TraitBound, TokenStream>,
) -> Result<(), TokenStream> {
    let built = build()?;
    if found.to_token_stream().to_string() != built.to_token_stream().to_string() {
        return Err(AccessBugs::UnexpectedAccessTraitBound {}.into());
    }
    Ok(())
}

/// Removes the user-facing counter const generic arguments from an
/// instance-trait path.
///
/// The supplied counter indexes identify the const generic arguments
/// reserved for instance counters. Each indexed argument is verified to
/// be an integer const generic before being removed.
///
/// ```ignore
/// <T as Example<'a, 0, U, 2, V>>::VALUE
/// ```
///
/// with counter indexes:
///
/// ```text
/// [1, 3]
/// ```
///
/// becomes:
///
/// ```ignore
/// <T as Example<'a, U, V>>::VALUE
/// ```
pub(crate) fn remove_gen_args(
    trait_path: &mut PathSegment,
    indexes: &IntList,
) -> Result<(), TokenStream> {
    let mut map = HashMap::<usize, LitInt>::new();
    for int in &indexes.ints {
        let idx = parse_pos_usize(int)?;
        let _ = map.insert(idx, int.clone());
    }

    let args = &mut trait_path.arguments;

    let PathArguments::AngleBracketed(angle) = args else {
        return Err(AccessError::TraitPathNotAngleArgs {
            segment: trait_path.clone(),
        }
        .into());
    };

    let args = &mut angle.args;

    let mut collect = Punctuated::<GenericArgument, Comma>::new();
    for (i, arg) in args.iter().enumerate() {
        let Some(int) = map.get(&i) else {
            collect.push(arg.clone());
            continue;
        };

        let GenericArgument::Const(c) = arg else {
            return Err(AccessError::CounterGenericArgNotConstGeneric {
                arg: arg.clone(),
                index: i,
                counter: int.clone(),
            }
            .into());
        };

        let Expr::Lit(lit) = c else {
            return Err(AccessError::CounterGenericArgNotConstLit {
                expr: c.clone(),
                index: i,
                counter: int.clone(),
            }
            .into());
        };

        let Lit::Int(_) = lit.lit else {
            return Err(AccessError::CounterGenericArgNotConstLitInt {
                lit: lit.lit.clone(),
                index: i,
                counter: int.clone(),
            }
            .into());
        };
    }

    *args = collect;

    Ok(())
}
