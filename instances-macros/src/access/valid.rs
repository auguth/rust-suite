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
// ````````````````````````` INSTANCE ACCESS VALIDATION ``````````````````````````
// ===============================================================================

//! Defines the validation rules that establish the restricted syntax accepted
//! by the instance-access transformation.
//!
//! The validation deliberately rejects Rust syntax that would require later
//! passes to perform type resolution, dependency tracking, receiver semantics,
//! or ambiguous associated-item resolution.
//!
//! The accepted and rejected syntax is organized around the following rules:
//!
//! ## Impl self type
//!
//! The instance impl self type must be:
//!   - `T::Assoc`
//!   - `<T as Trait>::Assoc`
//!
//! ## `Self` syntax
//!
//! - `Self` represents the self type of the impl.
//! - `Self::X` represents `<Self as InstanceBound>::X`.
//! - `InstanceBound` is a special first trait bound of `Self` where
//!    predicate and never used anywhere.
//! - The supported forms are:
//!   - `Self`
//!   - `Self::X`
//!   - `Self::X<A>`
//!   - `<Self as Trait>::X`
//!   - `<Self::Y as Trait>::X`
//! - Other forms are rejected, including:
//!   - `<X as Self>::A`
//!   - `<X as T>::Self`
//!   - `Self<A>` (GAT instance nodes [`crate::node`])
//!   - unsupported nested `Self` paths.
//!
//! ## Impl associated items
//!
//! - The defining associated item must be accessed through `Self`, not through
//!   the raw impl generic `<T as Trait>`.
//! - Associated items declared by the impl are distinct from
//!   `Self::X`, which denotes an instance-bound associated item.
//! - Associated items that require qualification must use their explicit
//!   trait qualification.
//!
//! ## Associated-type dependencies
//!
//! - Associated type declarations cannot cross-reference one another through
//!   `Self::X`.
//! - This avoids requiring dependency tracking or transitive resolution
//!   between associated type declarations.
//!
//! ## Associated-function receivers
//!
//! - Impl methods cannot use:
//!   - `self`
//!   - `&self`
//!   - `&mut self`
//! - The receiver must instead be expressed through the supported `Self`
//!   syntax.
//!
//! ## Associated constants
//!
//! - `Self` cannot be referenced from associated constants.
//! - Associated constants are lowered to standalone constants, so they cannot
//!   retain the impl's `Self` context.
//!
//! ## Predicates
//!
//! - `Self::X`, when it represents an instance-bound associated item, cannot
//!   be used as a predicate target.
//! - Predicates on associated items declared directly by the impl remain
//!   distinguishable from predicates on the instance-bound projection.
//!
//! These restrictions allow each transformation pass to operate on explicit,
//! locally recognizable syntax rather than performing semantic resolution or
//! dependency analysis. Validation establishes these assumptions before the
//! transformation passes, while confirmation verifies that the transformations
//! preserve or produce the required restricted form.

// ===============================================================================
// ``````````````````````````````````` IMPORTS ```````````````````````````````````
// ===============================================================================

// --- Std Crate ---
use std::collections::HashSet;

// --- Local Crate ---
use crate::{
    ParseBug,
    access::{InstanceBound, InstanceNodeBound, ValidBugs, ValidErrors, errors::ExtractBugs},
};

// --- Proc Macro Crates ---
use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
use syn::{
    Expr, FnArg, GenericArgument, GenericParam, Generics, Ident, ImplItem, Item, ItemImpl, Path,
    PathArguments, PathSegment, Stmt, TraitBoundModifier, Type, TypePath, WherePredicate,
    parse_quote,
    punctuated::Punctuated,
    spanned::Spanned,
    visit::{self, Visit},
};

// ===============================================================================
// ```````````````````````````````````` TRAITS ```````````````````````````````````
// ===============================================================================

/// Validates the source representation and confirms the resulting transformation invariants.
pub(super) trait ValidateConfirm<T, Context = ()> {
    /// Validates the source representation.
    fn validate(what: &T, via: &Context) -> Result<(), TokenStream>;

    /// Confirms the resulting transformation invariants.
    fn confirm(what: &T, via: &Context) -> Result<(), TokenStream>;
}

// ===============================================================================
// ``````````````````````````` INHERENT IMPL VALIDATION ``````````````````````````
// ===============================================================================

/// Validates and confirms that an [`ItemImpl`] is an inherent implementation.
///
/// Trait implementations are rejected during validation.
pub(super) struct ValidInherentImpl;

impl ValidateConfirm<ItemImpl> for ValidInherentImpl {
    fn validate(what: &ItemImpl, _: &()) -> Result<(), TokenStream> {
        if let Some((_, trait_, _)) = &what.trait_ {
            return Err(ValidErrors::TraitImplNotAllowed {
                item: trait_.clone(),
            }
            .into());
        }
        Ok(())
    }

    fn confirm(what: &ItemImpl, _: &()) -> Result<(), TokenStream> {
        if what.trait_.is_some() {
            return Err(ValidBugs::TraitImplNotAllowed {}.into());
        }
        Ok(())
    }
}

// ===============================================================================
// `````````````````````````` IMPL SELF TYPE VALIDATION ``````````````````````````
// ===============================================================================

/// Validates and confirms the instance node's associated type self target.
///
/// Ensures that an instance implementation is rooted in a generic
/// associated type, with the impl's first type parameter serving as the
/// associated type's originating generic node.
///
/// The self target may be expressed either directly through the generic
/// (`T::Assoc`) or through an explicit trait qualification
/// (`<T as Trait>::Assoc`).
#[derive(Debug, Clone)]
pub(super) struct ValidImplSelfTy;

impl ValidateConfirm<ItemImpl> for ValidImplSelfTy {
    fn validate(what: &ItemImpl, _: &()) -> Result<(), TokenStream> {
        let Type::Path(type_path) = what.self_ty.as_ref() else {
            return Err(ValidErrors::SelfTypeNotPath {
                ty: what.self_ty.as_ref().clone(),
            }
            .into());
        };

        match &type_path.qself {
            // Qualified form:
            // `<T as Config>::Lake`
            Some(qself) => {
                // QSelf must be a path
                let Type::Path(qself_ty) = qself.ty.as_ref() else {
                    return Err(ValidErrors::QSelfTypeNotGeneric {
                        ty: *qself.ty.clone(),
                    }
                    .into());
                };

                // Nested QSelf types are not supported.
                if qself_ty.qself.is_some() {
                    return Err(ValidErrors::NestedQSelfNotAllowed {
                        ty: qself_ty.clone(),
                    }
                    .into());
                }

                // Generic Params are One Segment Path
                if qself_ty.path.segments.len() != 1 {
                    return Err(ValidErrors::QSelfTypeNotGeneric {
                        ty: *qself.ty.clone(),
                    }
                    .into());
                }

                let generic = &qself_ty.path.segments[0];

                // The generic parameter itself cannot have generic arguments (no HKTs).
                if !matches!(generic.arguments, PathArguments::None) {
                    return Err(ValidErrors::GenericParameterHasArguments {
                        segment: generic.clone(),
                    }
                    .into());
                }

                let Some(first_type_generic) = what.generics.params.iter().find_map(|param| {
                    let GenericParam::Type(type_param) = param else {
                        return None;
                    };

                    Some(&type_param.ident)
                }) else {
                    return Err(ValidErrors::QSelfGenericNotDeclared {
                        segment: generic.clone(),
                    }
                    .into());
                };

                // It must be the FIRST type generic parameter of the impl.
                if generic.ident != *first_type_generic {
                    return Err(ValidErrors::QSelfGenericNotFirst {
                        segment: generic.clone(),
                    }
                    .into());
                }

                // `position` identifies where the
                // associated-type path begins.
                let position = qself.position as usize;

                // `<T as Trait>::Lake` - correct
                // `<T as Trait>::Lake::Foo` - incorrect
                let mut associated = type_path.path.segments.iter().skip(position);

                let Some(assoc) = associated.next() else {
                    return Err(ParseBug::SynParseInconsistent {}.into());
                };

                if associated.next().is_some() {
                    return Err(ValidErrors::MultipleAssociatedSegments {
                        ty: type_path.clone(),
                    }
                    .into());
                }

                // Generic associated types are not
                // supported.
                if !matches!(assoc.arguments, PathArguments::None) {
                    return Err(ValidErrors::AssociatedTypeHasArguments {
                        // no gat support for instance nodes currently
                        segment: assoc.clone(),
                    }
                    .into());
                }
            }

            // Unqualified form (ambiguous)
            // `T::Lake`
            None => {
                if type_path.path.segments.len() != 2 {
                    return Err(ValidErrors::AssociatedTypeMissing {
                        ty: type_path.clone(),
                    }
                    .into());
                }

                let generic = &type_path.path.segments[0];
                let associated = &type_path.path.segments[1];

                // The generic parameter itself cannot
                // have generic arguments.
                if !matches!(generic.arguments, PathArguments::None) {
                    return Err(ValidErrors::GenericParameterHasArguments {
                        segment: generic.clone(),
                    }
                    .into());
                }

                let Some(first_type_generic) = what.generics.params.iter().find_map(|param| {
                    let GenericParam::Type(type_param) = param else {
                        return None;
                    };

                    Some(&type_param.ident)
                }) else {
                    return Err(ValidErrors::QSelfGenericNotDeclared {
                        segment: generic.clone(),
                    }
                    .into());
                };

                // The first segment must be the FIRST
                // type generic parameter.
                if generic.ident != *first_type_generic {
                    return Err(ValidErrors::FirstTypeNotGeneric {
                        segment: generic.clone(),
                    }
                    .into());
                }

                // Generic associated types are not
                // supported.
                if !matches!(associated.arguments, PathArguments::None) {
                    return Err(ValidErrors::AssociatedTypeHasArguments {
                        segment: associated.clone(),
                    }
                    .into());
                }
            }
        }

        Ok(())
    }

    fn confirm(what: &ItemImpl, _: &()) -> Result<(), TokenStream> {
        let Type::Path(type_path) = what.self_ty.as_ref() else {
            return Err(ValidBugs::SelfTypeNotPath {}.into());
        };

        let Some(first_type_generic) = what.generics.params.iter().find_map(|param| {
            let GenericParam::Type(type_param) = param else {
                return None;
            };

            Some(&type_param.ident)
        }) else {
            return Err(ValidBugs::QSelfGenericNotDeclared {}.into());
        };

        match &type_path.qself {
            Some(qself) => {
                let Type::Path(qself_ty) = qself.ty.as_ref() else {
                    return Err(ValidBugs::QSelfTypeNotGeneric {}.into());
                };

                if qself_ty.qself.is_some() {
                    return Err(ValidBugs::NestedQSelfNotAllowed {}.into());
                }

                if qself_ty.path.segments.len() != 1 {
                    return Err(ValidBugs::QSelfTypeNotGeneric {}.into());
                }

                let generic = &qself_ty.path.segments[0];

                if !matches!(generic.arguments, PathArguments::None) {
                    return Err(ValidBugs::GenericParameterHasArguments {}.into());
                }

                if generic.ident != *first_type_generic {
                    return Err(ValidBugs::QSelfGenericNotFirst {}.into());
                }

                let position = qself.position as usize;

                let mut associated = type_path.path.segments.iter().skip(position);

                if associated.clone().count() != 1 {
                    return Err(ValidBugs::MultipleAssociatedSegments {}.into());
                }

                let associated = associated.next().unwrap();

                if !matches!(associated.arguments, PathArguments::None) {
                    return Err(ValidBugs::AssociatedTypeHasArguments {}.into());
                }
            }

            None => {
                if type_path.path.segments.len() != 2 {
                    return Err(ValidBugs::AssociatedTypeMissing {}.into());
                }

                let generic = &type_path.path.segments[0];

                let associated = &type_path.path.segments[1];

                if !matches!(generic.arguments, PathArguments::None) {
                    return Err(ValidBugs::GenericParameterHasArguments {}.into());
                }

                if generic.ident != *first_type_generic {
                    return Err(ValidBugs::FirstTypeNotGeneric {}.into());
                }

                if !matches!(associated.arguments, PathArguments::None) {
                    return Err(ValidBugs::AssociatedTypeHasArguments {}.into());
                }
            }
        }

        Ok(())
    }
}

// ===============================================================================
// ```````````````````````` SELF KEYWORD USAGE VALIDATION ````````````````````````
// ===============================================================================

/// Validates and confirms `Self` usage within instance key paths, ensuring
/// that it appears only in supported positions and qualified-self forms.
///
/// Allowed:
/// - `Self`
/// - `Self::X`
/// - `Self::X<A>`
/// - `<Self as Trait>::X`
/// - `<Self::Y as Trait>::X`
///
/// Not allowed:
/// - `Self<A>`
/// - `Foo::Self`
/// - `Self::X::Y`
/// - `<Foo as Self>::X`
/// - `<Foo as Trait>::Self`
pub(super) struct ValidSelfKeyPaths;

/// Represents invalid uses of `Self` within key paths.
#[derive(Debug)]
enum InvalidSelfKeyPath {
    /// `Self` appears after another path segment.
    ///
    /// Examples:
    /// ```text
    /// Foo::Self
    /// Foo::Bar::Self
    /// ```
    PreviousSegment { seg: PathSegment },

    /// `Self` has generic arguments.
    ///
    /// Invalid: `Self<A>`
    SelfHasGenericArguments { seg: PathSegment },

    /// More than one segment follows `Self`.
    ///
    /// `Self` and `Self::X` are valid:
    ///
    /// ```text
    /// Self
    /// Self::X
    /// Self::X<T>
    /// ```
    ///
    /// But these are invalid:
    ///
    /// ```text
    /// Self::X::Y
    /// Self::X::Y::Z
    /// ```
    MultipleFollowingSegments { seg: PathSegment },

    /// `Self` appears inside a qualified-self (`QSelf`) path.
    ///
    /// Examples:
    /// ```text
    /// <Self as Foo>::X - allowed
    /// <Foo as Self>::X - disallowed
    /// <Foo as Bar>::Self - disallowed
    /// ```
    QSelf { seg: PathSegment },
}

/// Collects invalid `Self` path usages found during AST traversal.
struct SelfKeyPathVisitor {
    errors: Vec<InvalidSelfKeyPath>,
}

impl<'b> Visit<'b> for SelfKeyPathVisitor {
    fn visit_path(&mut self, path: &'b Path) {
        let mut segments = path.segments.iter();

        let Some(first) = segments.next() else {
            return;
        };

        if first.ident == "Self" {
            // `Self` itself cannot have generic arguments.
            //
            // Invalid:
            // `Self<A>`
            // `Self<A, B>`
            if !matches!(first.arguments, PathArguments::None) {
                self.errors
                    .push(InvalidSelfKeyPath::SelfHasGenericArguments { seg: first.clone() });
            }

            // `Self` may be followed by at most one segment.
            //
            // Valid:
            // `Self`
            // `Self::X`
            // `Self::X<T>`
            //
            // Invalid:
            // `Self::X::Y`
            //           ^^
            if segments.clone().count() > 1 {
                let _ = segments.next();

                if let Some(seg) = segments.next() {
                    self.errors
                        .push(InvalidSelfKeyPath::MultipleFollowingSegments { seg: seg.clone() });
                }
            }
        } else {
            // `Self` is only valid as the first segment.
            //
            // Invalid:
            // `Foo::Self`
            //      ^^^^
            // `Foo::Bar::Self`
            //           ^^^^
            for segment in segments {
                if segment.ident == "Self" {
                    self.errors.push(InvalidSelfKeyPath::PreviousSegment {
                        seg: segment.clone(),
                    });
                }
            }
        }

        // Continue into nested path arguments.
        //
        // Example:
        // `Foo::<Self::Koo>` -> visits the nested `Self::Koo` path.
        visit::visit_path(self, path);
    }

    fn visit_type_path(&mut self, type_path: &'b TypePath) {
        let Some(qself) = &type_path.qself else {
            visit::visit_type_path(self, type_path);
            return;
        };

        // `Self` is allowed as the qualified self type.
        //
        // Valid:
        // `<Self as Foo>::Assoc`
        // `<Self::Some as Foo>::Assoc`
        visit::visit_type(self, qself.ty.as_ref());

        // `Self` is forbidden in the qualified path.
        //
        // Invalid:
        // `<Foo as Self>::Assoc`
        //          ^^^^
        // `<Foo as Bar>::Self`
        //                ^^^^
        for segment in &type_path.path.segments {
            if segment.ident == "Self" {
                self.errors.push(InvalidSelfKeyPath::QSelf {
                    seg: segment.clone(),
                });

                return;
            }
        }

        // Continue into nested arguments of the qualified path.
        //
        // Example:
        // `<Foo as Bar>::Assoc<Self::Path>`
        // visits the nested `Self::Path`.
        visit::visit_path(self, &type_path.path);
    }
}

impl ValidateConfirm<ItemImpl> for ValidSelfKeyPaths {
    fn validate(what: &ItemImpl, _: &()) -> Result<(), TokenStream> {
        let mut visitor = SelfKeyPathVisitor { errors: Vec::new() };
        visitor.visit_item_impl(what);

        for invalid in visitor.errors {
            match invalid {
                // `Foo::Self` or `Foo::Bar::Self`.
                InvalidSelfKeyPath::PreviousSegment { seg } => {
                    return Err(ValidErrors::SelfNotFirstPathSegment { seg }.into());
                }

                // `Self::X::Y` or deeper.
                InvalidSelfKeyPath::MultipleFollowingSegments { seg } => {
                    return Err(ValidErrors::SelfHasMultipleFollowingSegments { seg }.into());
                }

                // `<Foo as Self>::X`, etc.
                InvalidSelfKeyPath::QSelf { seg } => {
                    return Err(ValidErrors::SelfUsedInQualifiedSelfPath { seg }.into());
                }

                // `Self<A>` or `Self::<A>`
                InvalidSelfKeyPath::SelfHasGenericArguments { seg } => {
                    return Err(ValidErrors::SelfHasGenericArguments { seg }.into());
                }
            }
        }

        Ok(())
    }

    fn confirm(what: &ItemImpl, _: &()) -> Result<(), TokenStream> {
        let mut visitor = SelfKeyPathVisitor { errors: Vec::new() };

        visitor.visit_item_impl(what);

        for invalid in visitor.errors {
            match invalid {
                InvalidSelfKeyPath::PreviousSegment { .. } => {
                    return Err(ValidBugs::SelfNotFirstPathSegment {}.into());
                }

                InvalidSelfKeyPath::MultipleFollowingSegments { .. } => {
                    return Err(ValidBugs::SelfHasMultipleFollowingSegments {}.into());
                }

                InvalidSelfKeyPath::QSelf { .. } => {
                    return Err(ValidBugs::SelfUsedInQualifiedSelfPath {}.into());
                }

                InvalidSelfKeyPath::SelfHasGenericArguments { .. } => {
                    return Err(ValidBugs::SelfHasGenericArguments {}.into());
                }
            }
        }
        Ok(())
    }
}

// ===============================================================================
// ```````````````````````` IMPL VALID EXPECTED GENERICS `````````````````````````
// ===============================================================================

/// Validates that every generic parameter referenced by the
/// trait bound [`InstanceNodeBound`] is declared by the impl.
///
/// This validation only checks that the generics required by the bound
/// exist among the impl's declared generic parameters. It does not require
/// the impl to contain only those generics, nor does it validate their order.
///
/// For example:
///
/// ```text
/// impl : <'a, T, K, Extra>
/// bound: Node<'a, T, K>
/// ```
///
/// is valid for this validation because every generic used by the bound
/// (`'a`, `T`, and `K`) is declared by the impl. The additional `Extra`
/// generic is not rejected here.
///
/// Likewise:
///
/// ```text
/// impl : <T, K, B>
/// bound: Node<T, K>
/// ```
///
/// is valid here because all generics required by the bound (`T` and `K`)
/// are present in the impl.
///
/// However:
///
/// ```text
/// impl : <T, K>
/// bound: Node<T, U>
/// ```
///
/// is invalid because `U` is referenced by the bound but is not declared
/// by the impl.
///
/// The requirement that the impl's generic parameters contain exactly the
/// expected parameters, and that they appear in the required order, is
/// handled separately by [`ValidOrderedImplGenerics`].
#[derive(Debug, Clone)]
pub(super) struct ValidExpectedImplGenerics;

impl ValidateConfirm<ItemImpl, InstanceNodeBound> for ValidExpectedImplGenerics {
    fn validate(what: &ItemImpl, via: &InstanceNodeBound) -> Result<(), TokenStream> {
        let mut impl_generic_idents = HashSet::<String>::new();

        // Collect the generic identifiers uniformly declared by the impl.
        for param in &what.generics.params {
            let ident = match param {
                GenericParam::Type(param) => &param.ident,
                GenericParam::Const(param) => &param.ident,
                GenericParam::Lifetime(param) => &param.lifetime.ident,
            };

            let ident_string = ident.to_string();

            // early detection before the compiler
            if !impl_generic_idents.insert(ident_string) {
                return Err(ValidErrors::DuplicateImplGenericParameter {
                    ident: ident.clone(),
                }
                .into());
            }
        }

        let bound = &via.0;
        if bound.lifetimes.is_some() {
            return Err(ExtractBugs::NodeBoundHrtbNotSupported {}.into());
        }
        if let TraitBoundModifier::Maybe(_) = bound.modifier {
            return Err(ExtractBugs::NodeBoundSizedBoundInvalid {}.into());
        }

        let mut segments = bound.path.segments.iter().rev();

        let Some(last) = segments.next() else {
            return Err(ExtractBugs::QSelfGenericTraitBoundNotFound {}.into());
        };

        if let Some(seg) = segments.rev().find_map(|seg| {
            if !seg.arguments.is_none() {
                return Some(seg);
            }
            return None;
        }) {
            return Err(ValidErrors::NodeBoundIntermediatePathHasArgs { seg: seg.clone() }.into());
        }

        // Inspect the generic arguments of the extracted bound.
        //
        // Every argument must represent exactly one generic parameter:
        //
        //     Config<T>
        //     Config<'a>
        //     Config<{ B }>
        //
        // No concrete type, expression, path, etc. is permitted.
        let PathArguments::AngleBracketed(arguments) = &last.arguments else {
            return Ok(());
        };

        for argument in &arguments.args {
            let err = ValidErrors::NodeBoundGenericArgNotGenericParam {
                arg: argument.clone(),
            }
            .into();
            let ident = match argument {
                // Type argument (may also be ambiguous to syn due to const idents without {} braces):
                //     Config<T>
                //            ^
                GenericArgument::Type(Type::Path(type_path)) => {
                    // complex type arguments except a simple ident
                    if type_path.qself.is_some() || type_path.path.segments.len() != 1 {
                        return Err(err);
                    }

                    let segment = type_path.path.segments.first().unwrap();

                    if !matches!(segment.arguments, PathArguments::None) {
                        return Err(err);
                    }

                    &segment.ident
                }

                // A non-path type is not a generic
                // parameter.
                GenericArgument::Type(_) => {
                    return Err(err);
                }

                // Const argument:
                //     Config<{ B }>
                //            ^^^
                GenericArgument::Const(Expr::Block(expr_block)) => {
                    if expr_block.block.stmts.len() != 1 {
                        return Err(err);
                    }

                    let Stmt::Expr(Expr::Path(expr_path), _) = &expr_block.block.stmts[0] else {
                        return Err(err);
                    };

                    if expr_path.qself.is_some() || expr_path.path.segments.len() != 1 {
                        return Err(err);
                    }

                    let segment = expr_path.path.segments.first().unwrap();

                    if !matches!(segment.arguments, PathArguments::None) {
                        return Err(err);
                    }

                    &segment.ident
                }

                // Also accept a bare const expression if syn
                // represents it directly as a path:
                //     Config<B>
                GenericArgument::Const(Expr::Path(expr_path)) => {
                    if expr_path.qself.is_some() || expr_path.path.segments.len() != 1 {
                        return Err(err);
                    }

                    let segment = expr_path.path.segments.first().unwrap();

                    if !matches!(segment.arguments, PathArguments::None) {
                        return Err(err);
                    }

                    &segment.ident
                }

                // Any other const expression is not a
                // generic parameter.
                GenericArgument::Const(_) => {
                    return Err(err);
                }

                // Lifetime:
                //     Config<'a>
                //            ^^
                GenericArgument::Lifetime(lifetime) => &lifetime.ident,

                _ => {
                    // Anything else is associated type constraint of the bound
                    continue;
                }
            };

            // The extracted identifier must have been declared
            // by the impl.
            if !impl_generic_idents.contains(&ident.to_string()) {
                return Err(ValidErrors::NodeBoundGenericArgNotExpectedGeneric {
                    ident: ident.clone(),
                }
                .into());
            }
        }

        Ok(())
    }

    fn confirm(what: &ItemImpl, via: &InstanceNodeBound) -> Result<(), TokenStream> {
        let mut impl_generic_idents = HashSet::<String>::new();
        for param in &what.generics.params {
            let ident = match param {
                GenericParam::Type(param) => &param.ident,
                GenericParam::Const(param) => &param.ident,
                GenericParam::Lifetime(param) => &param.lifetime.ident,
            };

            let ident_string = ident.to_string();

            if !impl_generic_idents.insert(ident_string) {
                return Err(ValidBugs::DuplicateImplGenericParameter {}.into());
            }
        }
        let bound = &via.0;
        if bound.lifetimes.is_some() {
            return Err(ExtractBugs::NodeBoundHrtbNotSupported {}.into());
        }
        if let TraitBoundModifier::Maybe(_) = bound.modifier {
            return Err(ExtractBugs::NodeBoundSizedBoundInvalid {}.into());
        }
        let mut segments = bound.path.segments.iter().rev();
        let Some(last) = segments.next() else {
            return Err(ExtractBugs::QSelfGenericTraitBoundNotFound {}.into());
        };
        if segments.rev().any(|seg| {
            if !seg.arguments.is_none() {
                return true;
            }
            return false;
        }) {
            return Err(ValidBugs::NodeBoundIntermediatePathHasArgs {}.into());
        }

        let PathArguments::AngleBracketed(arguments) = &last.arguments else {
            return Ok(());
        };

        for argument in &arguments.args {
            let err = ValidBugs::NodeBoundGenericArgNotGenericParam {}.into();
            let ident = match argument {
                GenericArgument::Type(Type::Path(type_path)) => {
                    // complex type arguments except a simple ident
                    if type_path.qself.is_some() || type_path.path.segments.len() != 1 {
                        return Err(err);
                    }

                    let segment = type_path.path.segments.first().unwrap();

                    if !matches!(segment.arguments, PathArguments::None) {
                        return Err(err);
                    }

                    &segment.ident
                }

                GenericArgument::Type(_) => {
                    return Err(err);
                }

                GenericArgument::Const(Expr::Block(expr_block)) => {
                    if expr_block.block.stmts.len() != 1 {
                        return Err(err);
                    }
                    let Stmt::Expr(Expr::Path(expr_path), _) = &expr_block.block.stmts[0] else {
                        return Err(err);
                    };
                    if expr_path.qself.is_some() || expr_path.path.segments.len() != 1 {
                        return Err(err);
                    }
                    let segment = expr_path.path.segments.first().unwrap();
                    if !matches!(segment.arguments, PathArguments::None) {
                        return Err(err);
                    }
                    &segment.ident
                }

                GenericArgument::Const(Expr::Path(expr_path)) => {
                    if expr_path.qself.is_some() || expr_path.path.segments.len() != 1 {
                        return Err(err);
                    }
                    let segment = expr_path.path.segments.first().unwrap();
                    if !matches!(segment.arguments, PathArguments::None) {
                        return Err(err);
                    }
                    &segment.ident
                }

                GenericArgument::Const(_) => {
                    return Err(err);
                }

                GenericArgument::Lifetime(lifetime) => &lifetime.ident,

                _ => {
                    // rest bound's associated type constraints
                    // arguments are never required to be a generic param
                    continue;
                }
            };

            if !impl_generic_idents.contains(&ident.to_string()) {
                return Err(ValidBugs::NodeBoundGenericArgNotExpectedGeneric {}.into());
            }
        }

        Ok(())
    }
}

// ===============================================================================
// ````````````````````````` IMPL VALID ORDERED GENERICS `````````````````````````
// ===============================================================================

/// Validates that the impl's generic parameters consist of exactly the
/// generics required by the trait bound [`InstanceNodeBound`] and that
/// they appear in the required order.
///
/// This validation is performed only after [`ValidExpectedImplGenerics`],
/// which ensures that every generic required by the extracted bound is
/// declared by the impl. Unlike that validation, this validation also
/// rejects additional generics and enforces their exact ordering.
///
/// The required order is derived from the extracted trait bound. Leading
/// lifetime arguments are kept first, followed by the impl's first type
/// generic parameter, followed by the type and const generics appearing in
/// the bound.
///
/// For example, given:
///
/// ```text
/// impl : <'a, T, K, const B: bool>
/// bound: Node<'a, T, K, B>
/// ```
///
/// the required generic parameter order is:
///
/// ```text
/// 'a, T, K, B
/// ```
///
/// Therefore:
///
/// ```text
/// impl<'a, T, K, const B: bool> ...
/// ```
///
/// is valid, while:
///
/// ```text
/// impl<T, 'a, K, const B: bool> ...
/// ```
///
/// is invalid because the generics are not in the required order.
///
/// Likewise:
///
/// ```text
/// impl<'a, T, K, Extra, const B: bool> ...
/// ```
///
/// is invalid because `Extra` is not required by the extracted bound.
#[derive(Debug, Clone)]
pub(super) struct ValidOrderedImplGenerics;

impl ValidateConfirm<ItemImpl, InstanceNodeBound> for ValidOrderedImplGenerics {
    fn validate(what: &ItemImpl, via: &InstanceNodeBound) -> Result<(), TokenStream> {
        // first type generic parameter of the impl.
        let Some(first_type_generic) = what.generics.params.iter().find_map(|param| {
            let GenericParam::Type(param) = param else {
                return None;
            };

            Some(param.ident.clone())
        }) else {
            return Err(ValidBugs::QSelfGenericNotDeclared {}.into());
        };
        let bound = &via.0;
        if bound.lifetimes.is_some() {
            return Err(ExtractBugs::NodeBoundHrtbNotSupported {}.into());
        }
        if let TraitBoundModifier::Maybe(_) = bound.modifier {
            return Err(ExtractBugs::NodeBoundSizedBoundInvalid {}.into());
        }
        let mut segments = bound.path.segments.iter().rev();
        let Some(last) = segments.next() else {
            return Err(ExtractBugs::QSelfGenericTraitBoundNotFound {}.into());
        };
        if segments.rev().any(|seg| {
            if !seg.arguments.is_none() {
                return true;
            }
            return false;
        }) {
            return Err(ValidBugs::NodeBoundIntermediatePathHasArgs {}.into());
        }
        // Extract dentifiers of generic arguments from the instance trait

        let mut bound_lifetimes = Vec::<String>::new();
        let mut bound_type_generics = Vec::<String>::new();
        let mut bound_const_generics = Vec::<String>::new();
        let mut seen_non_lifetime = false;

        if let PathArguments::AngleBracketed(arguments) = &last.arguments {
            for argument in &arguments.args {
                match argument {
                    GenericArgument::Lifetime(lifetime) => {
                        // Once a type/const argument has occurred,
                        // no lifetime may occur afterwards.
                        if seen_non_lifetime {
                            return Err(ValidErrors::ExtractedBoundLifetimeAfterGeneric {
                                lifetime: lifetime.clone(),
                            }
                            .into());
                        }
                        bound_lifetimes.push(lifetime.ident.to_string());
                    }

                    GenericArgument::Type(Type::Path(type_path)) => {
                        let Some(segment) = type_path.path.segments.first() else {
                            return Err(ValidBugs::NodeBoundGenericArgNotGenericParam {}.into());
                        };
                        seen_non_lifetime = true;
                        bound_type_generics.push(segment.ident.to_string());
                    }

                    // A Non generic-param-argument
                    GenericArgument::Type(_) => {
                        return Err(ValidBugs::NodeBoundGenericArgNotGenericParam {}.into());
                    }

                    // Config<{B}> also ambiguous with Config<B> (hence we use only param idents)
                    //        ^^^
                    GenericArgument::Const(Expr::Path(expr_path)) => {
                        let Some(segment) = expr_path.path.segments.first() else {
                            return Err(ValidBugs::NodeBoundGenericArgNotGenericParam {}.into());
                        };
                        seen_non_lifetime = true;
                        bound_const_generics.push(segment.ident.to_string());
                    }

                    // Config<{ B }> - explicit braces
                    //        ^^^^^
                    GenericArgument::Const(Expr::Block(expr_block)) => {
                        let Some(Stmt::Expr(Expr::Path(expr_path), _)) =
                            expr_block.block.stmts.first()
                        else {
                            return Err(ValidBugs::NodeBoundGenericArgNotGenericParam {}.into());
                        };
                        let Some(segment) = expr_path.path.segments.first() else {
                            return Err(ValidBugs::NodeBoundGenericArgNotGenericParam {}.into());
                        };
                        seen_non_lifetime = true;
                        bound_const_generics.push(segment.ident.to_string());
                    }

                    _ => {
                        continue;
                    }
                }
            }
        }

        // Eg: Trait<'a, 'b, G, {B}>
        // first type generic = T
        // required: <'a, 'b, T, G, B> (just param idents)
        let mut expected = Vec::<String>::new();

        expected.extend(bound_lifetimes.clone());
        expected.push(first_type_generic.to_string());
        expected.extend(bound_type_generics.clone());
        expected.extend(bound_const_generics.clone());

        // actual generic param idents order from the impl.
        let actual = what
            .generics
            .params
            .iter()
            .map(|param| match param {
                GenericParam::Lifetime(param) => param.lifetime.ident.to_string(),
                GenericParam::Type(param) => param.ident.to_string(),
                GenericParam::Const(param) => param.ident.to_string(),
            })
            .collect::<Vec<_>>();

        // Compare
        if actual != expected {
            let mut expected_bound_args = Vec::<String>::new();
            expected_bound_args.extend(bound_lifetimes.iter().map(|lt| format!("'{lt}")));
            expected_bound_args.push(format!(
                "{} : {}",
                first_type_generic,
                bound.to_token_stream().to_string()
            ));
            expected_bound_args.extend(bound_type_generics);
            expected_bound_args.extend(bound_const_generics.iter().map(|c| format!("{{ {c} }}")));

            let expected = expected_bound_args.join(", ");

            return Err(ValidErrors::ExtractedBoundGenericOrderMismatch {
                expected,
                params: what.generics.params.clone(),
            }
            .into());
        }

        Ok(())
    }

    fn confirm(what: &ItemImpl, via: &InstanceNodeBound) -> Result<(), TokenStream> {
        let Some(first_type_generic) = what.generics.params.iter().find_map(|param| {
            let GenericParam::Type(param) = param else {
                return None;
            };

            Some(param.ident.clone())
        }) else {
            return Err(ValidBugs::QSelfGenericNotDeclared {}.into());
        };
        let bound = &via.0;
        if bound.lifetimes.is_some() {
            return Err(ExtractBugs::NodeBoundHrtbNotSupported {}.into());
        }
        if let TraitBoundModifier::Maybe(_) = bound.modifier {
            return Err(ExtractBugs::NodeBoundSizedBoundInvalid {}.into());
        }
        let mut segments = bound.path.segments.iter().rev();
        let Some(last) = segments.next() else {
            return Err(ExtractBugs::QSelfGenericTraitBoundNotFound {}.into());
        };
        if segments.rev().any(|seg| {
            if !seg.arguments.is_none() {
                return true;
            }
            return false;
        }) {
            return Err(ValidBugs::NodeBoundIntermediatePathHasArgs {}.into());
        }

        let mut bound_lifetimes = Vec::<String>::new();
        let mut bound_type_generics = Vec::<String>::new();
        let mut bound_const_generics = Vec::<String>::new();
        let mut seen_non_lifetime = false;

        if let PathArguments::AngleBracketed(arguments) = &last.arguments {
            for argument in &arguments.args {
                match argument {
                    GenericArgument::Lifetime(lifetime) => {
                        if seen_non_lifetime {
                            return Err(ValidBugs::ExtractedBoundLifetimeAfterGeneric {}.into());
                        }
                        bound_lifetimes.push(lifetime.ident.to_string());
                    }

                    GenericArgument::Type(Type::Path(type_path)) => {
                        let Some(segment) = type_path.path.segments.first() else {
                            return Err(ValidBugs::NodeBoundGenericArgNotGenericParam {}.into());
                        };
                        seen_non_lifetime = true;
                        bound_type_generics.push(segment.ident.to_string());
                    }

                    GenericArgument::Type(_) => {
                        return Err(ValidBugs::NodeBoundGenericArgNotGenericParam {}.into());
                    }

                    GenericArgument::Const(Expr::Path(expr_path)) => {
                        let Some(segment) = expr_path.path.segments.first() else {
                            return Err(ValidBugs::NodeBoundGenericArgNotGenericParam {}.into());
                        };
                        seen_non_lifetime = true;
                        bound_const_generics.push(segment.ident.to_string());
                    }

                    GenericArgument::Const(Expr::Block(expr_block)) => {
                        let Some(Stmt::Expr(Expr::Path(expr_path), _)) =
                            expr_block.block.stmts.first()
                        else {
                            return Err(ValidBugs::NodeBoundGenericArgNotGenericParam {}.into());
                        };
                        let Some(segment) = expr_path.path.segments.first() else {
                            return Err(ValidBugs::NodeBoundGenericArgNotGenericParam {}.into());
                        };
                        seen_non_lifetime = true;
                        bound_const_generics.push(segment.ident.to_string());
                    }

                    _ => {
                        continue;
                    }
                }
            }
        }

        let mut expected = Vec::<String>::new();

        expected.extend(bound_lifetimes);
        expected.push(first_type_generic.to_string());
        expected.extend(bound_type_generics);
        expected.extend(bound_const_generics);

        let actual = what
            .generics
            .params
            .iter()
            .map(|param| match param {
                GenericParam::Lifetime(param) => param.lifetime.ident.to_string(),
                GenericParam::Type(param) => param.ident.to_string(),
                GenericParam::Const(param) => param.ident.to_string(),
            })
            .collect::<Vec<_>>();

        if actual != expected {
            return Err(ValidBugs::ExtractedBoundGenericOrderMismatch {}.into());
        }
        Ok(())
    }
}

// ===============================================================================
// ````````````````````` INSTANCE NODE BOUND IN IMPL SELF-TY `````````````````````
// ===============================================================================

/// Validates the trait path of a qualified associated type used as the impl
/// self type against the extracted [`InstanceNodeBound`].
///
/// This validation applies only to qualified associated types of the form:
///
/// ```text
/// <T as Trait>::Assoc
/// ```
///
/// and does not validate the unqualified form:
///
/// ```text
/// T::Assoc
/// ```
///
/// The generic validity of `T` and `Assoc` is intentionally assumed here.
/// [`ValidImplSelfTy`] has already established that the impl self type is an
/// associated type, that its generic is the first type parameter of the impl,
/// and that the [`InstanceNodeBound`] is actually derived from that generic.
///
/// Likewise, all validation specific to [`InstanceNodeBound`] must already
/// have completed before this validator runs. This allows this validation to
/// focus exclusively on whether the trait qualification written in the
/// qualified self type matches the extracted node bound.
///
/// For example, given an extracted bound:
///
/// ```text
/// Trait<K>
/// ```
///
/// the following qualified self type is valid:
///
/// ```text
/// <T as Trait<K>>::Assoc
/// ```
///
/// while the following are invalid:
///
/// ```text
/// <T as OtherTrait<K>>::Assoc
/// <T as Trait<B>>::Assoc
/// <T as foo::Trait<K>>::Assoc
/// ```
///
/// The trait path, including its segments and generic arguments, must match
/// the extracted node bound exactly.
pub(super) struct ValidQAssocImplSelfTy;

impl ValidateConfirm<ItemImpl, InstanceNodeBound> for ValidQAssocImplSelfTy {
    fn validate(what: &ItemImpl, via: &InstanceNodeBound) -> Result<(), TokenStream> {
        let Type::Path(type_path) = what.self_ty.as_ref() else {
            return Err(ValidBugs::SelfTypeNotPath {}.into());
        };

        let Some(qself) = &type_path.qself else {
            // `T::Assoc` has no explicit trait path.
            return Ok(());
        };

        // `<T as Trait>::Assoc`
        //
        // `qself.position` is the number of path segments
        // belonging to the trait path.
        let position = qself.position as usize;
        let path_len = type_path.path.segments.len();
        let bound = &via.0;

        if position == 0 || position > path_len {
            return Err(ValidErrors::SelfTypeNodeBoundMismatch {
                expected: bound.path.clone(),
                span: type_path.span(),
            }
            .into());
        }

        // Build the span covering ONLY the trait path:
        //
        // `<T as foo::Trait<K>>::Assoc`
        //          ^^^^^^^^^^^^^
        //
        // This intentionally excludes `::Assoc`.
        let mut trait_span: Option<Span> = None;

        let mut actual = type_path.path.segments.iter();

        let mut expected = bound.path.segments.iter();

        for _ in 0..position {
            let Some(actual_segment) = actual.next() else {
                return Err(ValidErrors::SelfTypeNodeBoundMismatch {
                    expected: bound.path.clone(),
                    span: type_path.span(),
                }
                .into());
            };

            let segment_span = actual_segment.ident.span();

            trait_span = Some(match trait_span {
                None => segment_span,
                Some(previous) => previous.join(segment_span).unwrap_or(previous),
            });

            let Some(expected_segment) = expected.next() else {
                // The self type contains more trait
                // qualification than the extracted trait.
                if let Some(span) = trait_span {
                    return Err(ValidErrors::SelfTypeNodeBoundMismatch {
                        expected: bound.path.clone(),
                        span,
                    }
                    .into());
                }

                return Err(ValidErrors::SelfTypeNodeBoundMismatch {
                    expected: bound.path.clone(),
                    span: type_path.span(),
                }
                .into());
            };

            if actual_segment.ident != expected_segment.ident
                || actual_segment.arguments != expected_segment.arguments
            {
                let span = actual_segment.ident.span();

                return Err(ValidErrors::SelfTypeNodeBoundMismatch {
                    expected: bound.path.clone(),
                    span,
                }
                .into());
            }
        }

        // The extracted trait has more qualification
        // segments than the trait appearing in the QSelf.
        //
        // Example:
        //
        // self:
        //     <T as Trait>::Assoc
        //
        // extracted:
        //     foo::Trait
        //
        // The actual `Trait` path is the useful diagnostic
        // location.
        if expected.next().is_some() {
            let span = trait_span.unwrap_or_else(|| type_path.span());

            return Err(ValidErrors::SelfTypeNodeBoundQualifierMismatch {
                expected: bound.path.clone(),
                span,
            }
            .into());
        }

        Ok(())
    }

    fn confirm(what: &ItemImpl, via: &InstanceNodeBound) -> Result<(), TokenStream> {
        let Type::Path(type_path) = what.self_ty.as_ref() else {
            return Err(ValidBugs::SelfTypeNotPath {}.into());
        };

        let Some(qself) = &type_path.qself else {
            // `T::Assoc` has no explicit trait path.
            return Ok(());
        };

        let position = qself.position as usize;
        let path_len = type_path.path.segments.len();
        let bound = &via.0;

        if position == 0 || position > path_len {
            return Err(ValidBugs::SelfTypeNodeBoundMismatch {}.into());
        }

        let mut actual = type_path.path.segments.iter();
        let mut expected = bound.path.segments.iter();

        for _ in 0..position {
            let Some(actual_segment) = actual.next() else {
                return Err(ValidBugs::SelfTypeNodeBoundMismatch {}.into());
            };

            let Some(expected_segment) = expected.next() else {
                return Err(ValidBugs::SelfTypeNodeBoundMismatch {}.into());
            };

            if actual_segment.ident != expected_segment.ident
                || actual_segment.arguments != expected_segment.arguments
            {
                return Err(ValidBugs::SelfTypeNodeBoundMismatch {}.into());
            }
        }

        if expected.next().is_some() {
            return Err(ValidBugs::SelfTypeNodeBoundQualifierMismatch {}.into());
        }

        Ok(())
    }
}

// ===============================================================================
// ```````````````````````````` FORCE SELF-TY AS SELF ````````````````````````````
// ===============================================================================

/// Validates associated-item accesses through the generic type used by the
/// impl self type and ensures that accesses to the defining associated item
/// are written through `Self`.
///
/// If the impl self type is:
///
/// ```text
/// T::X
/// ```
///
/// or:
///
/// ```text
/// <T as Y>::X
/// ```
///
/// then `X` is the associated item that defines the impl self type. Any
/// subsequent access to that same associated item through `T` must instead
/// use `Self`:
///
/// ```text
/// T::X
///     -> Self
///
/// <T as Y>::X
///     -> Self
/// ```
///
/// Other associated items accessed through the same generic type are
/// ambiguous in this context and must be explicitly trait-qualified:
///
/// ```text
/// T::Z
///     -> <T as Y>::Z
/// ```
///
/// An already-qualified access is valid and is left unchanged:
///
/// ```text
/// <T as Y>::Z
/// ```
///
/// This validation does not validate the structure of the impl self type
/// itself. It requires [`ValidImplSelfTy`] to have already established the
/// associated-type structure and relies on the necessary validation of the
/// extracted [`InstanceNodeBound`] having completed.
#[derive(Debug, Clone)]
pub(super) struct ForceSelfTyAsSelf;

/// An invalid associated-item access through the generic type used by the
/// impl self type.
#[derive(Debug)]
enum InvalidInstAssocPath {
    /// The associated item that defines the impl self type was accessed
    /// directly through the impl's generic type.
    ///
    /// Example:
    ///
    /// ```text
    /// impl<T> T::Assoc
    ///
    /// T::Assoc
    /// ^^^^^^^^
    /// ```
    ///
    /// The defining associated item must instead be accessed through `Self`.
    Defining { path: Path },

    /// The associated item that defines the impl self type was accessed
    /// through a qualified path whose generic type is the impl's self
    /// generic.
    ///
    /// Example:
    ///
    /// ```text
    /// impl<T> <T as Trait>::Assoc
    ///
    /// <T as Trait>::Assoc
    /// ^^^^^^^^^^^^^^^^^^^
    /// ```
    ///
    /// When both the trait and associated item identify the defining
    /// associated type, the access must instead use `Self`.
    DefiningQ { t_path: TypePath },

    /// A different associated item was accessed through the bare impl
    /// generic type without the trait qualification required to disambiguate
    /// it.
    ///
    /// Example:
    ///
    /// ```text
    /// impl<T> <T as Trait>::Assoc
    ///
    /// T::Other
    /// ^^^^^^^^
    /// ```
    ///
    /// The access must be explicitly qualified:
    ///
    /// ```text
    /// <T as Trait>::Other
    /// ```
    ///
    /// `may_be_expected` contains a may-be corresponding qualified form:
    ///
    /// ```text
    /// <T as ExpectedTrait>::Other
    /// ```
    Unqualified {
        path: Path,
        may_be_expected: TypePath,
    },
}

/// Visits the impl body and identifies associated-item accesses made through
/// the generic type used by the impl self type.
///
/// The impl self type itself is intentionally not visited; the visitor is
/// applied to the impl generics and items only.
struct RawSelfTyVisitor<'a> {
    // Generic param ident used by the impl self type:
    //
    // <T as Trait<..>>::Assoc
    //  ^
    qself_ident: &'a Ident,

    // Trait used by the impl self type:
    //
    // <T as Trait<..>>::Assoc
    //       ^^^^^^^^^^
    defining_trait: &'a Path,

    // Associated type ident used by the impl self type:
    //
    // <T as Trait<...>>::Assoc
    //                    ^^^^^
    assoc_ident: &'a Ident,

    errors: Vec<InvalidInstAssocPath>,
}

impl<'b> Visit<'b> for RawSelfTyVisitor<'_> {
    fn visit_type_path(&mut self, type_path: &'b TypePath) {
        // T::X / T::Z
        if type_path.qself.is_none() {
            let mut segments = type_path.path.segments.iter();

            // requires at least two segments Generic::Assoc
            let Some(first) = segments.next() else {
                visit::visit_type_path(self, type_path);
                return;
            };

            let Some(second) = segments.next() else {
                visit::visit_type_path(self, type_path);
                return;
            };

            // Only inspect exactly T::Assoc.
            if segments.next().is_some() {
                // In case of more than two segments
                visit::visit_type_path(self, type_path);
                return;
            }

            // Check the Generic Param ident
            if first.ident != *self.qself_ident {
                visit::visit_type_path(self, type_path);
                return;
            }

            // T::X -> Self
            if second.ident == *self.assoc_ident {
                self.errors.push(InvalidInstAssocPath::Defining {
                    path: type_path.path.clone(),
                });

                return;
            }

            // Accessing some other Assoc from same Generic Param
            // requires qualification due to ambiguity in this evaluation
            //
            // So disambiguation of trait is required for future
            //
            // T::Z -> <T as ExtractedTrait>::Z
            let qself_ty: Type = parse_quote!(#first);
            let trait_path = self.defining_trait.clone();
            let assoc = second.ident.clone();
            let may_be_expected: TypePath = parse_quote!(
                <#qself_ty as #trait_path>::#assoc
            );
            self.errors.push(InvalidInstAssocPath::Unqualified {
                path: type_path.path.clone(),
                may_be_expected,
            });

            return;
        }

        // Fully Qualified Associated Type Access

        // <T as Y>::X / <T as Y>::Z
        let Some(qself) = &type_path.qself else {
            unreachable!();
        };

        // Generic Param should be a simple type path with 1 segment only
        let Type::Path(qself_ty) = qself.ty.as_ref() else {
            visit::visit_type_path(self, type_path);
            return;
        };

        // No nested QSelfs
        if qself_ty.qself.is_some() || qself_ty.path.segments.len() != 1 {
            visit::visit_type_path(self, type_path);
            return;
        }

        let Some(qself_segment) = qself_ty.path.segments.first() else {
            visit::visit_type_path(self, type_path);
            return;
        };

        if qself_segment.ident != *self.qself_ident {
            visit::visit_type_path(self, type_path);
            return;
        }

        let position = qself.position as usize;

        // Only inspect <T as Trait>::Assoc
        //                            ^^^^^
        let Some(assoc_segment) = type_path.path.segments.iter().nth(position) else {
            visit::visit_type_path(self, type_path);
            return;
        };

        // If further path segment exists
        if type_path.path.segments.iter().nth(position + 1).is_some() {
            visit::visit_type_path(self, type_path);
            return;
        }

        // Determine the actual trait in:
        //
        //     <T as Trait>::Assoc
        //            ^^^^^
        let mut actual_trait = Path {
            leading_colon: None,
            segments: Punctuated::new(),
        };

        for segment in type_path.path.segments.iter().take(position) {
            actual_trait.segments.push(segment.clone());
        }

        // <T as Y>::X -> Self
        //
        // ONLY if BOTH the associated item AND
        // trait match the impl self type.
        let same_assoc = assoc_segment.ident == *self.assoc_ident;

        let same_trait = actual_trait == *self.defining_trait;

        if same_assoc && same_trait {
            self.errors.push(InvalidInstAssocPath::DefiningQ {
                // <T as Y>::X
                // should replace to Self
                t_path: type_path.clone(),
            });

            return;
        }

        // <T as Y>::Z
        // or could be <T as K>::L which is a true positive
        visit::visit_type_path(self, type_path);
    }
}

impl ValidateConfirm<ItemImpl, InstanceNodeBound> for ForceSelfTyAsSelf {
    fn validate(what: &ItemImpl, via: &InstanceNodeBound) -> Result<(), TokenStream> {
        // ensured by `ValidImplSelfTy`
        let Type::Path(self_ty) = what.self_ty.as_ref() else {
            return Err(ValidBugs::SelfTypeNotPath {}.into());
        };

        // Extract the generic QSelf, defining associated
        // item, and defining trait from the impl self type.
        let err = ValidBugs::SelfTypeNotValidated {}.into();
        let (qself_ident, assoc_ident) = match &self_ty.qself {
            Some(qself) => {
                // <T as Y>::X
                let Type::Path(qself_ty) = qself.ty.as_ref() else {
                    return Err(err);
                };
                if qself_ty.qself.is_some() || qself_ty.path.segments.len() != 1 {
                    return Err(err);
                }
                let Some(qself_segment) = qself_ty.path.segments.first() else {
                    return Err(err);
                };
                let position = qself.position as usize;

                let Some(assoc_segment) = self_ty.path.segments.iter().nth(position) else {
                    return Err(err);
                };

                (qself_segment.ident.clone(), assoc_segment.ident.clone())
            }

            None => {
                // T::X

                let mut segments = self_ty.path.segments.iter();

                let Some(qself_segment) = segments.next() else {
                    return Err(err);
                };

                let Some(assoc_segment) = segments.next() else {
                    return Err(err);
                };

                (qself_segment.ident.clone(), assoc_segment.ident.clone())
            }
        };

        let mut visitor = RawSelfTyVisitor {
            qself_ident: &qself_ident,
            assoc_ident: &assoc_ident,
            defining_trait: &via.0.path,
            errors: Vec::new(),
        };

        // Never visit the impl self type itself.
        visitor.visit_generics(&what.generics);

        for item in &what.items {
            visitor.visit_impl_item(item);
        }

        for error in visitor.errors {
            match error {
                InvalidInstAssocPath::Defining { path } => {
                    return Err(ValidErrors::InstanceAssocMustUseSelf { path }.into());
                }

                InvalidInstAssocPath::DefiningQ { t_path } => {
                    return Err(ValidErrors::InstanceQAssocMustUseSelf { t_path }.into());
                }

                InvalidInstAssocPath::Unqualified {
                    path,
                    may_be_expected,
                } => {
                    return Err(ValidErrors::InstanceAssocTraitQualificationRequired {
                        path,
                        may_be_expected,
                    }
                    .into());
                }
            }
        }

        Ok(())
    }

    fn confirm(what: &ItemImpl, via: &InstanceNodeBound) -> Result<(), TokenStream> {
        let Type::Path(self_ty) = what.self_ty.as_ref() else {
            return Err(ValidBugs::SelfTypeNotPath {}.into());
        };

        let err = ValidBugs::SelfTypeNotValidated {}.into();

        let (qself_ident, assoc_ident) = match &self_ty.qself {
            Some(qself) => {
                let Type::Path(qself_ty) = qself.ty.as_ref() else {
                    return Err(err);
                };
                if qself_ty.qself.is_some() || qself_ty.path.segments.len() != 1 {
                    return Err(err);
                }
                let Some(qself_segment) = qself_ty.path.segments.first() else {
                    return Err(err);
                };

                let position = qself.position as usize;
                let Some(assoc_segment) = self_ty.path.segments.iter().nth(position) else {
                    return Err(err);
                };

                (qself_segment.ident.clone(), assoc_segment.ident.clone())
            }

            None => {
                let mut segments = self_ty.path.segments.iter();
                let Some(qself_segment) = segments.next() else {
                    return Err(err);
                };
                let Some(assoc_segment) = segments.next() else {
                    return Err(err);
                };
                (qself_segment.ident.clone(), assoc_segment.ident.clone())
            }
        };

        let mut visitor = RawSelfTyVisitor {
            qself_ident: &qself_ident,
            assoc_ident: &assoc_ident,
            defining_trait: &via.0.path,
            errors: Vec::new(),
        };

        // Never visit the impl self type itself.
        visitor.visit_generics(&what.generics);

        for item in &what.items {
            visitor.visit_impl_item(item);
        }

        for error in visitor.errors {
            match error {
                InvalidInstAssocPath::Defining { .. } => {
                    return Err(ValidBugs::InstanceAssocMustUseSelf {}.into());
                }

                InvalidInstAssocPath::DefiningQ { .. } => {
                    return Err(ValidBugs::InstanceQAssocMustUseSelf {}.into());
                }

                InvalidInstAssocPath::Unqualified { .. } => {
                    return Err(ValidBugs::InstanceAssocTraitQualificationRequired {}.into());
                }
            }
        }

        Ok(())
    }
}
// ===============================================================================
// `````````````````````````` IMPL RESTRICT INNER ITEMS ``````````````````````````
// ===============================================================================

/// Validates that an inner item does not reference `Self`.
///
/// Impl items are extracted from the original impl and transformed into
/// independently existing items. During this transformation, the impl header
/// generics and predicates are merged into each extracted impl item so that
/// the extracted item retains the generic context it originally inherited
/// from the impl.
///
/// Inner items are different. An item declared inside an associated constant
/// expression or an associated function body is not itself an impl item and
/// is therefore not included in this extraction or generic-context merging.
///
/// In particular, an inner item that references `Self` cannot be treated as
/// an independently existing item. `Self` is resolved through the enclosing
/// impl and may therefore carry the impl's generic parameters and associated
/// generic context with it. Once the inner item is separated from the impl,
/// that context is no longer implicitly available to it.
///
/// For example:
///
/// ```ignore
/// impl<T: Bound> T::Assoc {
///     fn foo() {
///         struct Inner(Self);
///     }
/// }
/// ```
///
/// `Inner` references `Self`, whose meaning is established by the enclosing
/// impl. Unlike the directly extracted `foo`, `Inner` does not receive the
/// impl header's generics and predicates through the extraction process.
///
/// The restriction therefore applies specifically to inner items containing
/// `Self`. Inner items that do not reference `Self` do not depend on the
/// impl's `Self` or its associated generic context and may remain independent.
///
/// This validation is performed before extraction so that no independently
/// existing item contains a `Self` reference whose enclosing impl context has
/// been removed.
#[derive(Debug, Clone)]
pub(super) struct RestrictInnerSelfItems;

/// Collects all items found within an impl item.
struct FindItem {
    items: Vec<Item>,
}

impl<'ast> syn::visit::Visit<'ast> for FindItem {
    fn visit_item(&mut self, item: &'ast syn::Item) {
        self.items.push(item.clone());

        syn::visit::visit_item(self, item);
    }
}

/// Finds whether an item contains `Self`.
struct FindSelf {
    has_self: bool,
}

impl<'ast> syn::visit::Visit<'ast> for FindSelf {
    fn visit_ident(&mut self, i: &'ast proc_macro2::Ident) {
        if i == "Self" {
            self.has_self = true;
        }
    }
}

impl FindItem {
    /// Returns the first collected item containing `Self`.
    fn first_self_item(&self) -> Option<Item> {
        self.items.iter().find_map(|item| {
            let mut visitor = FindSelf { has_self: false };

            visitor.visit_item(item);

            visitor.has_self.then(|| item.clone())
        })
    }
}

impl ValidateConfirm<ItemImpl> for RestrictInnerSelfItems {
    fn validate(what: &ItemImpl, _: &()) -> Result<(), TokenStream> {
        for item in what.items.iter() {
            match item {
                ImplItem::Const(c) => {
                    let mut visitor = FindItem { items: Vec::new() };
                    Visit::visit_expr(&mut visitor, &c.expr);
                    if let Some(item) = visitor.first_self_item() {
                        return Err(ValidErrors::InnerSelfItem { item }.into());
                    }
                }
                ImplItem::Fn(f) => {
                    let mut visitor = FindItem { items: Vec::new() };
                    Visit::visit_block(&mut visitor, &f.block);
                    if let Some(item) = visitor.first_self_item() {
                        return Err(ValidErrors::InnerSelfItem { item }.into());
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn confirm(what: &ItemImpl, _: &()) -> Result<(), TokenStream> {
        for item in what.items.iter() {
            match item {
                ImplItem::Const(c) => {
                    let mut visitor = FindItem { items: Vec::new() };
                    Visit::visit_expr(&mut visitor, &c.expr);
                    if visitor.first_self_item().is_some() {
                        return Err(ValidBugs::InnerSelfItem {}.into());
                    }
                }
                ImplItem::Fn(f) => {
                    let mut visitor = FindItem { items: Vec::new() };
                    Visit::visit_block(&mut visitor, &f.block);
                    if visitor.first_self_item().is_some() {
                        return Err(ValidBugs::InnerSelfItem {}.into());
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
}

// ===============================================================================
// ````````````````````````` IMPL RESTRICT INSTANCE TRAIT ````````````````````````
// ===============================================================================

/// Validates that the instance trait extracted from the impl is not used
/// anywhere else within the impl.
///
/// The instance trait is extracted from the impl's `Self` predicate bound by
/// [`InstanceBound`]. It exists only as an input to the procedural macro:
/// it is consumed by the macro during validation and transformation and is
/// removed from the current impl's state. It is therefore not a trait that may be
/// referenced elsewhere in the user's impl.
///
/// This restriction applies to the whole impl, including its generics and
/// all impl items. Any path containing the extracted instance trait's
/// identifier is rejected.
///
/// The restriction is based on the trait identifier itself, not on the
/// complete path or its generic arguments. Consequently, qualifying the
/// trait does not disambiguate or permit its use. For example, if the
/// extracted instance trait is `InstanceTrait`, all of the following remain
/// invalid:
///
/// ```text
/// InstanceTrait<T>
/// foo::InstanceTrait<T>
/// <T as InstanceTrait>::Assoc
/// <T as foo::InstanceTrait<U>>::Assoc
/// ```
///
/// The presence of `InstanceTrait` is sufficient to trigger the restriction,
/// regardless of its qualification or generic arguments.
///
/// If the user needs another instance bound whose trait is unrelated to the
/// extracted instance trait, they must use a different type alias rather than
/// referencing the extracted instance trait again.
#[derive(Debug, Clone)]
pub(super) struct RestrictInstTraitInImpl;

/// Visitor that walks impl paths and records uses of the instance trait's
/// identifier, regardless of its qualification or generic arguments.
///
/// For example:
/// - `InstanceTrait<T>`,
/// - `foo::InstanceTrait<T>`
/// - `<T as InstanceTrait>::Assoc`
///
/// are all detected by the `InstanceTrait` segment.
struct InstanceTraitPathVisitor<'a> {
    trait_ident: &'a Ident,
    errors: Vec<PathSegment>,
}

impl<'b, 'a> InstanceTraitPathVisitor<'a> {
    fn check_path(&mut self, path: &'b Path) {
        if let Some(seg) = path.segments.iter().find_map(|segment| {
            if segment.ident == *self.trait_ident {
                Some(segment)
            } else {
                None
            }
        }) {
            self.errors.push(seg.clone());

            return;
        }

        visit::visit_path(self, path);
    }
}

impl<'b, 'a> Visit<'b> for InstanceTraitPathVisitor<'a> {
    fn visit_path(&mut self, path: &'b Path) {
        self.check_path(path);
    }
}

impl ValidateConfirm<ItemImpl, InstanceBound> for RestrictInstTraitInImpl {
    fn validate(what: &ItemImpl, via: &InstanceBound) -> Result<(), TokenStream> {
        let Some(trait_segment) = via.0.path.segments.last() else {
            return Err(ExtractBugs::SelfTraitBoundNotTrait {}.into());
        };

        let mut visitor = InstanceTraitPathVisitor {
            trait_ident: &trait_segment.ident,
            errors: Vec::new(),
        };

        visitor.visit_item_impl(what);

        for seg in visitor.errors {
            return Err(ValidErrors::InstTraitUsedInImpl { seg }.into());
        }

        Ok(())
    }

    fn confirm(what: &ItemImpl, via: &InstanceBound) -> Result<(), TokenStream> {
        let Some(trait_segment) = via.0.path.segments.last() else {
            return Err(ExtractBugs::SelfTraitBoundNotTrait {}.into());
        };

        let mut visitor = InstanceTraitPathVisitor {
            trait_ident: &trait_segment.ident,
            errors: Vec::new(),
        };

        visitor.visit_item_impl(what);

        if !visitor.errors.is_empty() {
            return Err(ValidBugs::InstTraitNotRemovedFromImpl {}.into());
        }
        Ok(())
    }
}

// ===============================================================================
// ```````````````````````` EXPECTED SELF QUALIFIED PATHS ````````````````````````
// ===============================================================================

/// Validates that associated types, functions, and constants defined by the
/// impl are accessed through `Self` rather than through unqualified paths.
///
/// This qualification is required so that later macro transformations can
/// unambiguously determine whether a path refers to an associated item of
/// the current impl or to some unrelated item in scope. An unqualified
/// identifier such as `X`, `foo`, or `BAR` may refer to a local item, an
/// imported item, a generic item, or another name in scope; even when the
/// impl itself defines an associated item with the same identifier, the
/// syntax alone does not explicitly establish that association for the
/// macro.
///
/// Requiring the `Self::` form makes the association explicit:
///
/// ```text
/// Self::X
/// Self::foo()
/// Self::BAR
/// ```
///
/// instead of the ambiguous forms:
///
/// ```text
/// X
/// foo()
/// BAR
/// ```
///
/// Qualified paths that are already explicit, such as `Self::X`,
/// `Other::X`, or `<T as Trait>::X`, are not rejected by this validation.
/// Only a single-segment path matching an associated item defined by the
/// impl requires the `Self::` qualifier.
#[derive(Debug, Clone)]
pub(super) struct RequireSelfQPaths;

/// Visitor that walks the impl and detects unqualified uses of associated
/// types, functions, and constants that must be accessed through `Self`.
///
/// For example, if the impl defines `type X`, `fn foo`, and `const BAR`,
/// then `X`, `foo()`, and `BAR` require qualification, while
/// `Self::X`, `Self::foo()`, and `Self::BAR` are already qualified.
struct ExpectedSelfQPathVisitor<'a> {
    type_idents: &'a [&'a Ident],
    fn_idents: &'a [&'a Ident],
    const_idents: &'a [&'a Ident],
    error: Option<Ident>,
}

impl<'b, 'a> Visit<'b> for ExpectedSelfQPathVisitor<'a> {
    fn visit_path(&mut self, path: &'b Path) {
        if self.error.is_some() {
            return;
        }

        // Only an unqualified single-segment path
        // can require qualification here.
        //
        // `Type`
        // `foo`
        // `CONST`
        //
        // Qualified paths such as:
        //
        // `Self::Type`
        // `Self::foo`
        // `Self::CONST`
        // `Other::Type`
        //
        // are already qualified.
        if path.segments.len() == 1 {
            let segment = &path.segments[0];

            let is_assoc_type = self
                .type_idents
                .iter()
                .any(|ident| **ident == segment.ident);

            let is_assoc_fn = self.fn_idents.iter().any(|ident| **ident == segment.ident);

            let is_assoc_const = self
                .const_idents
                .iter()
                .any(|ident| **ident == segment.ident);

            if is_assoc_type || is_assoc_fn || is_assoc_const {
                self.error = Some(segment.ident.clone());

                return;
            }
        }

        visit::visit_path(self, path);
    }
}

impl ValidateConfirm<ItemImpl> for RequireSelfQPaths {
    fn validate(what: &ItemImpl, _: &()) -> Result<(), TokenStream> {
        let type_idents = what
            .items
            .iter()
            .filter_map(|item| {
                let ImplItem::Type(item_type) = item else {
                    return None;
                };

                Some(&item_type.ident)
            })
            .collect::<Vec<_>>();

        let fn_idents = what
            .items
            .iter()
            .filter_map(|item| {
                let ImplItem::Fn(item_fn) = item else {
                    return None;
                };

                Some(&item_fn.sig.ident)
            })
            .collect::<Vec<_>>();

        let const_idents = what
            .items
            .iter()
            .filter_map(|item| {
                let ImplItem::Const(item_const) = item else {
                    return None;
                };

                Some(&item_const.ident)
            })
            .collect::<Vec<_>>();

        let mut visitor = ExpectedSelfQPathVisitor {
            type_idents: &type_idents,
            fn_idents: &fn_idents,
            const_idents: &const_idents,
            error: None,
        };

        visitor.visit_item_impl(what);

        if let Some(ident) = visitor.error {
            return Err(ValidErrors::AssociatedItemRequiresQualifier { ident }.into());
        };

        Ok(())
    }

    fn confirm(what: &ItemImpl, _: &()) -> Result<(), TokenStream> {
        let type_idents = what
            .items
            .iter()
            .filter_map(|item| {
                let ImplItem::Type(item_type) = item else {
                    return None;
                };

                Some(&item_type.ident)
            })
            .collect::<Vec<_>>();

        let fn_idents = what
            .items
            .iter()
            .filter_map(|item| {
                let ImplItem::Fn(item_fn) = item else {
                    return None;
                };

                Some(&item_fn.sig.ident)
            })
            .collect::<Vec<_>>();

        let const_idents = what
            .items
            .iter()
            .filter_map(|item| {
                let ImplItem::Const(item_const) = item else {
                    return None;
                };

                Some(&item_const.ident)
            })
            .collect::<Vec<_>>();

        let mut visitor = ExpectedSelfQPathVisitor {
            type_idents: &type_idents,
            fn_idents: &fn_idents,
            const_idents: &const_idents,
            error: None,
        };

        visitor.visit_item_impl(what);

        if visitor.error.is_some() {
            return Err(ValidBugs::AssociatedItemRequiresQualifier {}.into());
        };
        Ok(())
    }
}

// ===============================================================================
// ````````````````````` RESTRICT ASSOCIATED ITEM REFERENCES `````````````````````
// ===============================================================================

/// Validates that associated type declarations do not reference other
/// associated types of the same impl through `Self::..`.
///
/// Although such cross-references may be valid and usable by Rust itself,
/// they are deliberately rejected here because this is a syntactic
/// procedural macro. The macro should be able to reason about each
/// associated type directly from its own declaration without having to
/// resolve, track, or construct dependencies between associated items.
///
/// For example, if the impl declares:
///
/// ```text
/// type A = ...;
/// type B = Self::A;
/// ```
///
/// Rust may be able to resolve this relationship, but for the macro the
/// declaration of `B` is no longer self-contained: understanding `B` now
/// requires following its dependency on `A`.
///
/// This restriction therefore deliberately keeps associated type
/// declarations pure and independent:
///
/// ```text
/// type A = ...;
/// type B = ...;
/// ```
///
/// rather than making the macro maintain an associated-item dependency
/// graph, determine dependency ordering, or reason about transitive
/// references.
///
/// The restriction applies to every associated type declaration and checks
/// its complete syntax, including generic parameters, bounds, where clauses,
/// and the right-hand-side type. A reference is recognized specifically by
/// its `Self::X` form, where `X` is an associated type declared by the impl.
///
/// For example:
///
/// ```text
/// type A = Self::B;
/// type B = Self::C;
/// type C<T> = Self::D<T>;
/// ```
///
/// are rejected when the referenced item is another associated type of the
/// same impl.
///
/// This is therefore a deliberate soundness and predictability boundary for
/// the macro: rather than attempting to resolve whether such dependencies
/// can be made to work, the macro requires associated type declarations to
/// remain independent and syntactically self-contained.
///
/// References to names that are not associated types declared by the impl
/// are not rejected by this validation.
#[derive(Debug, Clone)]
pub(super) struct RestrictAssocTypeCrossReferences;

/// Visitor that walks an associated type declaration and records the first
/// `Self::X` path referring to another associated type declared by the impl.
struct CollectAssocIdents<'a> {
    assoc_idents: &'a [Ident],
    path: Option<Path>,
}

impl Visit<'_> for CollectAssocIdents<'_> {
    fn visit_path(&mut self, path: &Path) {
        if self.path.is_some() {
            return;
        }

        let Some(first) = path.segments.first() else {
            return;
        };

        // Only inspect `Self::X...`.
        if first.ident == "Self" && path.segments.len() >= 2 {
            let assoc = &path.segments[1];

            if self.assoc_idents.iter().any(|ident| ident == &assoc.ident) {
                self.path = Some(path.clone());

                return;
            }
        }

        // Continue into nested generic arguments and paths.
        visit::visit_path(self, path);
    }
}

impl ValidateConfirm<ItemImpl> for RestrictAssocTypeCrossReferences {
    fn validate(what: &ItemImpl, _: &()) -> Result<(), TokenStream> {
        let assoc_idents = what
            .items
            .iter()
            .filter_map(|item| {
                let ImplItem::Type(item_type) = item else {
                    return None;
                };

                Some(item_type.ident.clone())
            })
            .collect::<Vec<_>>();

        if assoc_idents.is_empty() {
            return Ok(());
        }

        for item in &what.items {
            let ImplItem::Type(item_type) = item else {
                continue;
            };

            let mut visitor = CollectAssocIdents {
                assoc_idents: &assoc_idents,
                path: None,
            };

            // Visit the complete associated type declaration, including:
            //
            // - generics
            // - generic bounds
            // - where clause
            // - RHS type
            //
            // `visit_impl_item_type` does not treat the declaration identifier
            // itself as a path, so `type Foo = ...` does not inspect `Foo`.
            visitor.visit_impl_item_type(item_type);

            if let Some(path) = visitor.path {
                return Err(ValidErrors::InstanceAssocTypeSelfReferenceNotAllowed { path }.into());
            }
        }
        Ok(())
    }

    fn confirm(what: &ItemImpl, _: &()) -> Result<(), TokenStream> {
        let assoc_idents = what
            .items
            .iter()
            .filter_map(|item| {
                let ImplItem::Type(item_type) = item else {
                    return None;
                };

                Some(item_type.ident.clone())
            })
            .collect::<Vec<_>>();

        if assoc_idents.is_empty() {
            return Ok(());
        }

        for item in &what.items {
            let ImplItem::Type(item_type) = item else {
                continue;
            };

            let mut visitor = CollectAssocIdents {
                assoc_idents: &assoc_idents,
                path: None,
            };

            visitor.visit_impl_item_type(item_type);

            if visitor.path.is_some() {
                return Err(ValidBugs::InstanceAssocTypeSelfReferenceNotAllowed {}.into());
            }
        }
        Ok(())
    }
}

// ===============================================================================
// ````````````````````````````` VALID ASSOC FN ARGS `````````````````````````````
// ===============================================================================

/// Validates that associated functions do not declare a `self` receiver.
///
/// The macro rewrites uses of `self` throughout associated functions by
/// introducing explicit arguments and replacing those uses syntactically.
///
/// A receiver such as:
///
/// ```text
/// fn foo(&self) { ... }
/// fn foo(&mut self) { ... }
/// fn foo(self) { ... }
/// ```
///
/// is therefore deliberately rejected. The macro instead requires the
/// corresponding value to be represented by an explicit argument that it can
/// introduce, propagate, and reference consistently throughout the function.
///
/// This keeps the transformation independent of receiver semantics and
/// avoids having to reason about how `self` is bound, shadowed, or otherwise
/// interacted with across nested scopes. The restriction is therefore a
/// syntactic requirement that makes the generated code predictable and
/// unambiguous.
#[derive(Debug, Clone)]
pub(super) struct ValidAssocFnArgs;

impl ValidateConfirm<ItemImpl> for ValidAssocFnArgs {
    fn validate(what: &ItemImpl, _: &()) -> Result<(), TokenStream> {
        for item in &what.items {
            let ImplItem::Fn(item_fn) = item else {
                continue;
            };

            for arg in &item_fn.sig.inputs {
                let FnArg::Receiver(receiver) = arg else {
                    continue;
                };

                return Err(ValidErrors::InvalidSelfFnArgument {
                    self_value: receiver.self_token.clone(),
                }
                .into());
            }
        }

        Ok(())
    }

    fn confirm(what: &ItemImpl, _: &()) -> Result<(), TokenStream> {
        for item in &what.items {
            let ImplItem::Fn(item_fn) = item else {
                continue;
            };

            for arg in &item_fn.sig.inputs {
                let FnArg::Receiver(_) = arg else {
                    continue;
                };

                return Err(ValidBugs::InvalidSelfFnArgument {}.into());
            }
        }

        Ok(())
    }
}

// ===============================================================================
// ````````````````````` RESTRICT SELF KEYWORD IN COST ASSOCS ````````````````````
// ===============================================================================

/// Validates that the `Self` keyword is not used within associated constant
/// declarations.
///
/// All impl items are ultimately lowered by the macro into separate items:
/// associated types become `ItemType`s, associated functions become `ItemFn`s,
/// and associated constants become `ItemConst`s. Consequently, the final
/// constant is no longer represented as an item nested inside the impl and
/// cannot rely on the impl's self type to provide the generic context for
/// references to `Self`.
///
/// In particular, using `Self` in an associated constant would require the
/// generated standalone constant to preserve or otherwise delegate the
/// relevant generic context from the original impl. That model is not
/// available here. An independent `const` also cannot use the required
/// const-generic features on stable Rust, which prevents the generated
/// constant from faithfully carrying the impl-level generic context in the
/// same manner as the original associated constant.
///
/// For example:
///
/// ```text
/// impl<T> Type<T> {
///     const VALUE: usize = Self::SIZE;
/// }
/// ```
///
/// cannot simply be lowered to an independent constant while retaining the
/// meaning of `Self::SIZE`, because the generated `ItemConst` no longer has
/// the surrounding impl self type through which that reference could be
/// resolved.
///
/// `Self` is therefore deliberately rejected from associated constant
/// declarations rather than attempting to reconstruct, delegate, or emulate
/// the impl's generic context during the final lowering phase.
///
/// For example:
///
/// ```text
/// const X: Self::Type = ...;
/// const Y: usize = Self::VALUE;
/// ```
///
/// are rejected regardless of where the `Self` path occurs within the
/// associated constant declaration.
pub(super) struct RestrictSelfInCostAssocs;

/// Visitor that finds the first `Self` path segment
struct SelfSegmentVisitor {
    seg: Option<PathSegment>,
}

impl<'b> Visit<'b> for SelfSegmentVisitor {
    fn visit_path(&mut self, path: &'b Path) {
        if self.seg.is_some() {
            return;
        }
        for segment in &path.segments {
            if segment.ident == "Self" {
                self.seg = Some(segment.clone());
                return;
            }
        }
        visit::visit_path(self, path);
    }
}

impl ValidateConfirm<ItemImpl> for RestrictSelfInCostAssocs {
    fn validate(what: &ItemImpl, _: &()) -> Result<(), TokenStream> {
        for item in &what.items {
            let ImplItem::Const(item_const) = item else {
                continue;
            };
            let mut visitor = SelfSegmentVisitor { seg: None };
            visitor.visit_impl_item_const(item_const);
            if let Some(seg) = visitor.seg {
                return Err(ValidErrors::SelfNotAllowedInConstItem { seg }.into());
            }
        }

        Ok(())
    }

    fn confirm(what: &ItemImpl, _: &()) -> Result<(), TokenStream> {
        for item in &what.items {
            let ImplItem::Const(item_const) = item else {
                continue;
            };
            let mut visitor = SelfSegmentVisitor { seg: None };
            visitor.visit_impl_item_const(item_const);
            if visitor.seg.is_some() {
                return Err(ValidBugs::SelfNotAllowedInConstItem {}.into());
            }
        }
        Ok(())
    }
}

// ===============================================================================
// ```````````````````` RESTRICT SELF ASSOCS IN INSTANCE BOUND ```````````````````
// ===============================================================================

pub(super) struct RestrictSelfAssocsInInstBound;

/// Visits `Self::Assoc` Type Paths
struct SelfAssocVisitor {
    path: Option<TypePath>,
}

impl<'b> Visit<'b> for SelfAssocVisitor {
    fn visit_type_path(&mut self, type_path: &'b TypePath) {
        if self.path.is_some() {
            return;
        }

        let is_self_path = type_path.qself.is_none()
            && type_path.path.segments.len() >= 2
            && type_path
                .path
                .segments
                .first()
                .is_some_and(|segment| segment.ident == "Self");

        if is_self_path {
            self.path = Some(type_path.clone());
            return;
        }

        visit::visit_type_path(self, type_path);
    }
}

struct SelfAssocVisitorConfirm {
    found: bool,
}

impl<'b> Visit<'b> for SelfAssocVisitorConfirm {
    fn visit_type_path(&mut self, type_path: &'b TypePath) {
        if self.found {
            return;
        }

        let is_self_path = type_path.qself.is_none()
            && type_path.path.segments.len() >= 2
            && type_path
                .path
                .segments
                .first()
                .is_some_and(|segment| segment.ident == "Self");

        if is_self_path {
            self.found = true;
            return;
        }

        visit::visit_type_path(self, type_path);
    }
}

impl ValidateConfirm<InstanceBound> for RestrictSelfAssocsInInstBound {
    fn validate(what: &InstanceBound, _: &()) -> Result<(), TokenStream> {
        let mut visitor = SelfAssocVisitor { path: None };
        visitor.visit_trait_bound(&what.0);
        if let Some(path) = visitor.path {
            return Err(ValidErrors::SelfAssocNotAllowedInTraitBound { path }.into());
        }

        Ok(())
    }

    fn confirm(what: &InstanceBound, _: &()) -> Result<(), TokenStream> {

        let mut visitor = SelfAssocVisitorConfirm { found: false };
        visitor.visit_trait_bound(&what.0);
        if visitor.found {
            return Err(ValidBugs::SelfAssocNotAllowedInTraitBound { }.into());
        }
        Ok(())
    }
}



// ===============================================================================
// ```````````````````````` RESTRICT SELF ASSOC PREDICATES ```````````````````````
// ===============================================================================

/// Validates `Self::Assoc` type predicates while distinguishing direct
/// associated items of the impl from associated items belonging to the
/// instance-bound associated type.
///
/// A predicate on a direct associated item of the impl is valid. For example:
///
/// ```text
/// impl<T> Type<T>
/// where
///     Self::DirectAssoc: SomeTrait,
/// {
///     type DirectAssoc = ...;
/// }
/// ```
///
/// Here `DirectAssoc` is an associated item declared directly by the current
/// impl, so the predicate is a valid bound on that associated type.
///
/// The complication is that associated items of the type supplied by the
/// extracted instance bound are also represented syntactically as
/// `Self::Assoc`. In the macro's intermediate representation, an expression
/// conceptually referring to an associated item of the instance-bound type
/// can therefore have the same surface form as a direct impl-associated item.
///
/// For example, a predicate conceptually referring to:
///
/// ```text
/// <Self as InstanceBound>::NestedAssoc: SomeTrait
/// ```
///
/// may be represented during the transformation as:
///
/// ```text
/// Self::NestedAssoc: SomeTrait
/// ```
///
/// Such a predicate must not be treated as a predicate on a direct associated
/// item of the current impl.
///
/// The distinction is established by the transformation order. Direct
/// associated-item references are processed by
/// [`crate::access::replace::StripSelfKeyOfAssocs`], which
/// removes the `Self::` qualifier from references to associated items declared
/// directly by the impl. Consequently, after that transformation:
///
/// ```text
/// Self::DirectAssoc
/// ```
///
/// becomes:
///
/// ```text
/// DirectAssoc
/// ```
///
/// while a remaining:
///
/// ```text
/// Self::Assoc
/// ```
///
/// identifies a `Self::` reference that was not a reference to one of the
/// direct associated items.
///
/// This validator uses that invariant to reject predicates on associated
/// items of the instance-bound associated type while allowing predicates on
/// direct impl-associated types.
pub(super) struct RestrictSelfAssocPreds;

impl ValidateConfirm<ItemImpl> for RestrictSelfAssocPreds {
    fn validate(what: &ItemImpl, _: &()) -> Result<(), TokenStream> {
        fn check_generics(generics: &Generics) -> Result<(), TokenStream> {
            let Some(where_clause) = generics.where_clause.as_ref() else {
                return Ok(());
            };

            for predicate in &where_clause.predicates {
                let WherePredicate::Type(predicate) = predicate else {
                    continue;
                };

                let is_self_assoc = matches!(
                    &predicate.bounded_ty,
                    Type::Path(type_path)
                        if type_path.qself.is_none()
                            && type_path.path.segments.len() == 2
                            && type_path
                                .path
                                .segments
                                .first()
                                .is_some_and(|segment| {
                                    segment.ident == "Self"
                                })
                );

                if !is_self_assoc {
                    continue;
                }

                // `Self::X` is the bounded type and the predicate has
                // one or more bounds:
                //
                //     Self::X: Trait
                //             ^^^^^
                if !predicate.bounds.is_empty() {
                    return Err(ValidErrors::InstanceAssocTypePredicateNotAllowed {
                        pred: predicate.clone(),
                    }
                    .into());
                }
            }

            Ok(())
        }

        // Impl-level generic parameters and where-clause.
        check_generics(&what.generics)?;

        // Generic parameters and where-clauses belonging to every
        // associated item.
        for item in &what.items {
            match item {
                ImplItem::Const(item) => {
                    check_generics(&item.generics)?;
                }

                ImplItem::Fn(item) => {
                    check_generics(&item.sig.generics)?;
                }

                ImplItem::Type(item) => {
                    check_generics(&item.generics)?;
                }

                _ => {}
            }
        }

        Ok(())
    }

    fn confirm(what: &ItemImpl, _: &()) -> Result<(), TokenStream> {
        fn validate_generics(generics: &Generics) -> Result<(), TokenStream> {
            let Some(where_clause) = generics.where_clause.as_ref() else {
                return Ok(());
            };

            for predicate in &where_clause.predicates {
                let WherePredicate::Type(predicate) = predicate else {
                    continue;
                };

                let is_self_assoc = matches!(
                    &predicate.bounded_ty,
                    Type::Path(type_path)
                        if type_path.qself.is_none()
                            && type_path.path.segments.len() == 2
                            && type_path
                                .path
                                .segments
                                .first()
                                .is_some_and(|segment| {
                                    segment.ident == "Self"
                                })
                );

                if is_self_assoc && !predicate.bounds.is_empty() {
                    return Err(ValidBugs::InstanceAssocTypePredicateNotRemoved {}.into());
                }
            }

            Ok(())
        }

        validate_generics(&what.generics)?;

        for item in &what.items {
            match item {
                ImplItem::Const(item) => {
                    validate_generics(&item.generics)?;
                }

                ImplItem::Fn(item) => {
                    validate_generics(&item.sig.generics)?;
                }

                ImplItem::Type(item) => {
                    validate_generics(&item.generics)?;
                }

                _ => {}
            }
        }

        Ok(())
    }
}
