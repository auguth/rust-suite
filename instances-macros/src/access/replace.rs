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
// ````````````````````````` INSTANCE ACCESS EXTRACTION ``````````````````````````
// ===============================================================================

//! Defines the replacement passes that lower validated instance-access syntax
//! into explicit Rust syntax that can exist independently of the original
//! implementation context.
//!
//! Replacement is performed after validation ([`crate::access::valid`]) and
//! extraction ([`crate::access::extract`]) have established
//! the syntactic structure and instance-bound information required by these
//! passes. The transformations progressively remove the syntax that depends
//! on the surrounding `impl` and replace it with explicit paths, arguments,
//! and generic parameters.
//!
//! The replacement passes primarily perform:
//!
//! ## Generic propagation
//!
//! - Merge impl generics into associated items that use `Self`.
//! - Propagate the corresponding impl generic arguments into `Self::Assoc`
//!   path segments.
//! - Preserve generic arguments already present on associated-item references.
//! - Avoid propagating impl lifetime arguments into associated-function calls,
//!   where explicit lifetime arguments can conflict with late-bound lifetimes.
//!
//! ## Impl-local associated-item paths
//!
//! - Strip `Self::` from associated items declared directly by the impl once
//!   those items are being lowered into standalone items.
//! - Replace standalone `Self` type references with the concrete impl self type.
//!
//! ## Instance-bound associated types
//!
//! - Replace `Self::Assoc` with an explicit qualified access through the
//!   extracted instance bound.
//! - Apply the requested leaf access to the resulting associated path.
//! - Perform the corresponding replacement separately for `#[sum]` associated
//!   types, which use the non-leaf terminal-path transformation.
//!
//! ## Instance-bound associated functions
//!
//! - Replace `Self::foo(...)` calls with explicit instance-bound accesses.
//! - Preserve the complete call expression and its existing arguments.
//! - Apply either the leaf or non-leaf access transformation to the call.
//! - Reject bare `Self::foo` function paths where an explicit invocation is
//!   required.
//!
//! ## Non-leaf function arguments
//!
//! - Introduce the dynamic byte-slice arguments required by non-leaf accesses.
//! - Propagate those arguments through associated-function call chains.
//! - Update callers and their call sites so the generated arguments flow through
//!   the complete dependency chain.
//!
//! ## Generated return types
//!
//! - Replace inferred `Result` error types such as `Result<T, _>` with the
//!   concrete macro error type required by the generated standalone function.
//!
//! Every replacement pass has a corresponding confirmation step. Replacement
//! establishes the required post-transformation form, while confirmation checks
//! that no transformation-dependent syntax remains and that the generated
//! representation satisfies the invariant expected by subsequent passes.

// ===============================================================================
// ``````````````````````````````````` IMPORTS ```````````````````````````````````
// ===============================================================================

// --- Std Crate ---
use std::ops::DerefMut;

// --- Local Crate ---
use crate::{
    Extraction, Instance, Transformation, Utilization,
    access::{
        AssocIdents, InstanceBound, SelfUsageAssocs, ValidBugs, ValidErrors,
        args::{
            AccessArgs, AccessArgsIdents, AccessArgsIndexes, AssocAccessor, BStrInput, LeafAccess,
        },
        errors::{ReplaceBugs, ReplaceErrors},
        leaf::InstanceLeafAccess,
        nonleaf::{InstanceNonLeafExprCallAccess, InstanceNonLeafTerminalPathAccess},
    },
    utils::{merge_generics, validate_merged},
};

// --- Proc Macro Crates ---
use proc_macro2::TokenStream;
use syn::{
    AngleBracketedGenericArguments, Expr, ExprCall, ExprPath, FnArg, GenericArgument, Ident,
    ImplItem, ImplItemFn, ItemImpl, ParenthesizedGenericArguments, Pat, Path, PathArguments,
    ReturnType, Type, TypePath, parse_quote,
    punctuated::Punctuated,
    token::Comma,
    visit::{self, Visit},
    visit_mut::{self, VisitMut},
};

// --- Proc Suite ---
use proc_suite::SupportCrate;

// ===============================================================================
// ```````````````````````````````````` ImplS ```````````````````````````````````
// ===============================================================================

/// Replaces the source representation and confirms the resulting transformation invariants.
pub(super) trait ReplaceConfirm<T, Context = ()>: Sized {
    /// Replaces the source representation.
    fn replace(from: &mut T, via: &Context) -> Result<(), TokenStream>;

    /// Confirms the resulting transformation invariants.
    fn confirm(from: &T, via: &Context) -> Result<(), TokenStream>;
}

// ===============================================================================
// ````````````````````` MERGE IMPL GENERICS TO ASSOC ITEMS ``````````````````````
// ===============================================================================

/// Merges the impl's generic parameters into associated items whose
/// declarations use `Self`.
///
/// Associated items that reference `Self` are eventually lowered by the macro
/// into standalone items. Once lowered, they no longer have the surrounding
/// impl declaration from which their generic context would otherwise be
/// inherited.
///
/// The impl generics must therefore be propagated into each affected
/// associated item's own generics before lowering. This allows the generated
/// standalone item to retain the generic parameters required by expressions
/// and types that originally depended on the impl's generic context.
///
/// For example:
///
/// ```ignore
/// impl<T, K> Impl for T {
///     type Value<U> = <Self as Impl>::Assoc<U>;
///     fn get() -> Self::Value<U> { ... }
/// }
/// ```
///
/// If `Value` and `get` are lowered into independent items, the generic
/// parameters from the impl must also be present on those generated items.
/// Otherwise the resulting standalone declarations would contain references
/// to generic parameters that are no longer declared in their own scope.
///
/// ```ignore
/// type Value<U, T, K> = <Self as Impl>::Assoc<U>
/// ```
///
/// Only associated items recorded by [`SelfUsageAssocs::self_idents`] are
/// modified. Items that do not use `Self` do not require the impl's generic
/// parameters to be propagated.
///
/// Existing generics declared directly on the associated item are preserved.
/// The impl generics are merged with those generics, and duplicate generic
/// parameter identifiers are rejected by [`merge_generics`].
///
/// - associated constants receive the impl generics in their `generics`;
/// - associated functions receive them in their `Signature`'s `generics`;
/// - associated types receive them in their `generics`.
#[derive(Debug, Clone)]
pub(super) struct MergeImplGenericsToAssocs;

impl ReplaceConfirm<ItemImpl, SelfUsageAssocs> for MergeImplGenericsToAssocs {
    fn replace(from: &mut ItemImpl, via: &SelfUsageAssocs) -> Result<(), TokenStream> {
        let SelfUsageAssocs { self_idents, .. } = via;
        let AssocIdents { types, consts, fns } = self_idents;
        let impl_generics = from.generics.clone();

        for item in &mut from.items {
            match item {
                ImplItem::Const(item) => {
                    let ident = &item.ident;
                    if consts.iter().any(|exp| ident == exp) {
                        item.generics = merge_generics(&item.generics, &impl_generics)?;
                    }
                }

                ImplItem::Fn(item) => {
                    let ident = &item.sig.ident;
                    if fns.iter().any(|exp| ident == exp) {
                        item.sig.generics = merge_generics(&item.sig.generics, &impl_generics)?;
                    }
                }

                ImplItem::Type(item) => {
                    let ident = &item.ident;
                    if types.iter().any(|exp| ident == exp) {
                        item.generics = merge_generics(&item.generics, &impl_generics)?;
                    }
                    item.attrs.push(parse_quote!(
                        #[allow(type_alias_bounds)]
                    ));
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn confirm(from: &ItemImpl, via: &SelfUsageAssocs) -> Result<(), TokenStream> {
        let SelfUsageAssocs { self_idents, .. } = via;
        let AssocIdents { types, consts, fns } = self_idents;
        let impl_generics = &from.generics;

        for item in &from.items {
            match item {
                ImplItem::Const(item) => {
                    if consts.iter().any(|ident| ident == &item.ident) {
                        if let Err(_) = validate_merged(&item.generics, impl_generics) {
                            return Err(ReplaceBugs::ImplGenericsNotAppendedToSelfAssoc {}.into());
                        }
                    }
                }

                ImplItem::Fn(item) => {
                    if fns.iter().any(|ident| ident == &item.sig.ident) {
                        if let Err(_) = validate_merged(&item.sig.generics, impl_generics) {
                            return Err(ReplaceBugs::ImplGenericsNotAppendedToSelfAssoc {}.into());
                        }
                    }
                }

                ImplItem::Type(item) => {
                    if types.iter().any(|ident| ident == &item.ident) {
                        if let Err(_) = validate_merged(&item.generics, impl_generics) {
                            return Err(ReplaceBugs::ImplGenericsNotAppendedToSelfAssoc {}.into());
                        }
                    }
                }

                _ => {}
            }
        }

        Ok(())
    }
}

// ===============================================================================
// ```````````````````` MERGE IMPL GENERICS TO ASSOC SEGMENTS ````````````````````
// ===============================================================================

/// Propagates the impl's generic arguments into `Self::Assoc` path segments
/// used by associated items that reference `Self`.
///
/// Associated items that use `Self` are eventually lowered into standalone
/// items. The previous generic-merging phase [`MergeImplGenericsToAssocs`]
/// gives those generated items declarations for the impl's generic parameters,
/// but references to other associated items must also retain the corresponding
/// generic arguments.
///
/// For example:
///
/// ```ignore
/// impl<T, K> Impl for T {
///     type Other<U, T, K> = <Self as Impl>::Assoc<U>;
///     type Value<U, T, K> = Self::Other<U>;
/// }
/// ```
///
/// After the associated items are separated from the impl, the reference to
/// `Self::Other` must carry the generic arguments that originally came from
/// the surrounding impl:
///
/// ```ignore
///     type Other<U, T, K> = <Self as Impl>::Assoc<U>;
///     type Value<U, T, K> = Self::Other<U, T, K>;
/// ```
///
/// This transformation therefore takes the type, const, and lifetime generic
/// arguments produced by `split_for_impl` and appends them to the
/// generic arguments of matching `Self::Assoc` path segments.
///
/// For associated function identifiers, impl lifetime arguments are
/// intentionally excluded. Adding the impl's lifetime arguments to an
/// associated function path would cause the generated path to explicitly
/// specify lifetime arguments for the function. When those lifetimes are late
/// bound, Rust rejects such explicit lifetime arguments and emits the
/// corresponding "cannot specify lifetime arguments explicitly if late bound
/// lifetime parameters are present" error.
///
/// The associated functions represented by these identifiers are cross-call
/// usages within the impl and, after lowering, are given the same generic
/// context and bounds. Their lifetime parameters therefore remain implicit at
/// the call site; only the applicable type and const generic arguments are
/// propagated.
///
/// Associated types and constants, in contrast, retain the complete impl
/// generic argument list, including lifetime arguments.
///
/// Only associated-item identifiers recorded by
/// [`SelfUsageAssocs::self_idents`] are considered. The associated identifier
/// must therefore correspond to an associated type, constant, or function
/// whose declaration uses `Self`.
///
/// Existing generic arguments on the associated path are preserved. The impl
/// generic arguments are appended after those existing arguments rather than
/// replacing them.
///
/// For example:
///
/// ```text
/// Self::Assoc<A>
/// ```
///
/// becomes conceptually:
///
/// ```text
/// Self::Assoc<A, T, K>
/// ```
///
/// when the impl contributes `T` and `K`.
///
/// For an associated function, lifetime arguments from the impl are omitted:
///
/// ```text
/// Self::function<A>
/// ```
///
/// receives only the applicable type and const arguments from the impl.
///
/// If the associated path has no generic arguments, an angle-bracketed
/// argument list is created before the impl arguments are appended.
#[derive(Debug, Clone)]
pub(super) struct MergeImplTyGenericsToSegments;

/// Visitor that finds matching `Self::Assoc` path segments and appends the
/// impl's generic arguments to their existing argument lists.
///
/// Matching is based on the associated item's identifier recorded by
/// [`SelfUsageAssocs::self_idents`]. Existing arguments are preserved and the
/// impl arguments are appended to the end.
struct AppendableGenericArgsMutVisitor<'a> {
    idents: &'a Vec<Ident>,
    append_args: Punctuated<GenericArgument, Comma>,
    error: Option<ParenthesizedGenericArguments>,
}

impl<'a> VisitMut for AppendableGenericArgsMutVisitor<'a> {
    fn visit_path_mut(&mut self, path: &mut Path) {
        let Some(self_segment) = path.segments.first_mut() else {
            return;
        };

        if self_segment.ident != "Self" {
            visit_mut::visit_path_mut(self, path);
            return;
        }

        let Some(assoc_segment) = path.segments.iter_mut().nth(1) else {
            return;
        };

        if !self
            .idents
            .iter()
            .any(|ident| ident == &assoc_segment.ident)
        {
            visit_mut::visit_path_mut(self, path);
            return;
        }

        let args = match &mut assoc_segment.arguments {
            PathArguments::None => {
                assoc_segment.arguments =
                    PathArguments::AngleBracketed(AngleBracketedGenericArguments {
                        colon2_token: Default::default(),
                        lt_token: Default::default(),
                        args: Punctuated::new(),
                        gt_token: Default::default(),
                    });

                let PathArguments::AngleBracketed(args) = &mut assoc_segment.arguments else {
                    return;
                };

                args
            }

            PathArguments::AngleBracketed(args) => args,

            PathArguments::Parenthesized(paren) => {
                self.error = Some(paren.clone());
                return;
            }
        };

        for arg in &self.append_args {
            args.args.push(arg.clone());
        }

        visit_mut::visit_path_mut(self, path);
    }
}

/// Visitor that verifies that every matching `Self::Assoc` path segment
/// contains the generic arguments propagated from the surrounding impl.
///
/// The visitor records whether any matching path fails the post-transformation
/// invariant. A matching segment without angle-bracketed arguments, or without
/// one of the expected impl arguments, is considered an incomplete
/// transformation.
struct ConfirmAppendedGenericArgsVisitor<'a> {
    idents: &'a [Ident],
    expected_args: &'a Punctuated<GenericArgument, Comma>,
    error: Option<()>,
}

impl<'b, 'a> Visit<'b> for ConfirmAppendedGenericArgsVisitor<'a> {
    fn visit_path(&mut self, path: &'b Path) {
        if self.error.is_some() {
            return;
        }

        let Some(first) = path.segments.first() else {
            return;
        };

        if first.ident != "Self" {
            visit::visit_path(self, path);
            return;
        }

        let Some(assoc) = path.segments.iter().nth(1) else {
            visit::visit_path(self, path);
            return;
        };

        if !self.idents.iter().any(|ident| ident == &assoc.ident) {
            visit::visit_path(self, path);
            return;
        }

        let PathArguments::AngleBracketed(args) = &assoc.arguments else {
            self.error = Some(());
            return;
        };

        for expected in self.expected_args {
            if !args.args.iter().any(|actual| actual == expected) {
                self.error = Some(());
                return;
            }
        }

        visit::visit_path(self, path);
    }
}

impl ReplaceConfirm<ItemImpl, SelfUsageAssocs> for MergeImplTyGenericsToSegments {
    fn replace(from: &mut ItemImpl, via: &SelfUsageAssocs) -> Result<(), TokenStream> {
        let (_, type_generics, _) = from.generics.split_for_impl();

        let append_args: AngleBracketedGenericArguments = parse_quote!(#type_generics);

        let mut append_fn_args = append_args.clone();
        append_fn_args.args = append_fn_args
            .args
            .into_iter()
            .filter(|arg| !matches!(arg, GenericArgument::Lifetime(_)))
            .collect();

        let SelfUsageAssocs { self_idents, .. } = via;

        let AssocIdents { types, consts, fns } = self_idents;

        let mut types_visitor = AppendableGenericArgsMutVisitor {
            idents: &types,
            append_args: append_args.args.clone(),
            error: None,
        };

        types_visitor.visit_item_impl_mut(from);

        if let Some(paren) = types_visitor.error {
            return Err(ReplaceErrors::SelfAssocParenArgsNotSupported { paren }.into());
        }

        let mut const_visitor = AppendableGenericArgsMutVisitor {
            idents: &consts,
            append_args: append_args.args.clone(),
            error: None,
        };

        const_visitor.visit_item_impl_mut(from);

        if let Some(paren) = const_visitor.error {
            return Err(ReplaceErrors::SelfAssocParenArgsNotSupported { paren }.into());
        }

        // Associated function paths cannot receive lifetime arguments
        // To avoid late bound warnings
        let mut fn_visitor = AppendableGenericArgsMutVisitor {
            idents: &fns,
            append_args: append_fn_args.args,
            error: None,
        };

        fn_visitor.visit_item_impl_mut(from);

        if let Some(paren) = fn_visitor.error {
            return Err(ReplaceErrors::SelfAssocParenArgsNotSupported { paren }.into());
        }

        Ok(())
    }

    fn confirm(from: &ItemImpl, via: &SelfUsageAssocs) -> Result<(), TokenStream> {
        let (_, type_generics, _) = from.generics.split_for_impl();

        let append_args: AngleBracketedGenericArguments = parse_quote!(#type_generics);

        let mut append_fn_args = append_args.clone();
        append_fn_args.args = append_fn_args
            .args
            .into_iter()
            .filter(|arg| !matches!(arg, GenericArgument::Lifetime(_)))
            .collect();

        let SelfUsageAssocs { self_idents, .. } = via;

        let AssocIdents { types, consts, fns } = self_idents;

        let mut types_visitor = ConfirmAppendedGenericArgsVisitor {
            idents: &types,
            expected_args: &append_args.args,
            error: None,
        };

        types_visitor.visit_item_impl(from);

        if types_visitor.error.is_some() {
            return Err(ReplaceBugs::ImplGenericsNotAppendedToSelfAssoc {}.into());
        }

        let mut const_visitor = ConfirmAppendedGenericArgsVisitor {
            idents: &consts,
            expected_args: &append_args.args,
            error: None,
        };

        const_visitor.visit_item_impl(from);

        if const_visitor.error.is_some() {
            return Err(ReplaceBugs::ImplGenericsNotAppendedToSelfAssoc {}.into());
        }

        let mut fn_visitor = ConfirmAppendedGenericArgsVisitor {
            idents: &fns,
            expected_args: &append_fn_args.args,
            error: None,
        };

        fn_visitor.visit_item_impl(from);

        if fn_visitor.error.is_some() {
            return Err(ReplaceBugs::ImplGenericsNotAppendedToSelfAssoc {}.into());
        }

        Ok(())
    }
}

// ===============================================================================
// ``````````````````````````` STRIP SELF KEY OF ASSOCS ``````````````````````````
// ===============================================================================

/// Strips the leading `Self::` qualifier from paths referring to associated
/// items declared by the current impl.
///
/// Associated items are eventually lowered out of the impl and transformed
/// into independent items. At that stage, references to the impl's associated
/// items must no longer retain the `Self::` qualifier, because the generated
/// items are no longer being interpreted in the original impl-item context.
///
/// For example:
///
/// ```text
/// Self::Value
/// Self::get()
/// Self::CONST
/// ```
///
/// are transformed into:
///
/// ```text
/// Value
/// get()
/// CONST
/// ```
///
/// The transformation is performed only when the path has `Self` as its
/// first segment and its second segment matches an associated item declared
/// by the impl. The associated-item identifiers are supplied by
/// [`SelfUsageAssocs::all_idents`].
///
/// The same transformation is also applied to the extracted
/// [`InstanceBound`]. This is necessary because the instance bound is retained
/// outside the original impl-item context and may itself contain references
/// to associated items through `Self::`.
///
/// Paths that do not begin with `Self`, or whose second segment does not
/// identify an associated item of the impl (instance associated types),
/// are left unchanged.
pub(super) struct StripSelfKeyOfAssocs;

/// Visitor that strips the leading `Self::` qualifier from paths whose
/// associated-item identifier belongs to the supplied set.
///
/// For a matching path, all segments after the leading `Self` segment are
/// retained. Thus:
///
/// ```text
/// Self::Assoc
/// ```
///
/// becomes:
///
/// ```text
/// Assoc
/// ```
///
/// while a non-matching path is traversed normally.
struct SelfAssocStripVisitor<'a> {
    idents: &'a [Ident],
}

impl VisitMut for SelfAssocStripVisitor<'_> {
    fn visit_path_mut(&mut self, path: &mut Path) {
        if path.segments.len() >= 2
            && path
                .segments
                .first()
                .is_some_and(|segment| segment.ident == "Self")
        {
            let second = &path.segments[1].ident;

            if self.idents.iter().any(|ident| ident == second) {
                let segments = path.segments.iter().skip(1).cloned().collect();

                path.segments = segments;
            }
        }

        visit_mut::visit_path_mut(self, path);
    }
}

/// Visitor that finds `Self::Assoc` paths whose associated identifier is
/// contained in the supplied set of associated-item identifiers.
///
/// Such a path should have had its leading `Self` segment stripped by the
/// replacement phase. Finding one therefore indicates that the transformation
/// did not completely remove the `Self` qualifier.
struct SelfAssocStripConfirmVisitor<'a> {
    idents: &'a [Ident],
    error: Option<()>,
}

impl<'b> Visit<'b> for SelfAssocStripConfirmVisitor<'_> {
    fn visit_path(&mut self, path: &'b Path) {
        if self.error.is_some() {
            return;
        }

        let Some(first) = path.segments.first() else {
            return;
        };

        if first.ident != "Self" {
            visit::visit_path(self, path);
            return;
        }

        let Some(second) = path.segments.iter().nth(1) else {
            visit::visit_path(self, path);
            return;
        };

        if self.idents.iter().any(|ident| ident == &second.ident) {
            self.error = Some(());
            return;
        }

        visit::visit_path(self, path);
    }
}

impl ReplaceConfirm<ItemImpl, SelfUsageAssocs> for StripSelfKeyOfAssocs {
    fn replace(
        from: &mut ItemImpl,
        via: &SelfUsageAssocs,
    ) -> Result<(), TokenStream> {
        let SelfUsageAssocs { all_idents, .. } = via;
        let AssocIdents { types, consts, fns } = all_idents;

        for idents in [types.as_slice(), consts.as_slice(), fns.as_slice()] {
            let mut visitor = SelfAssocStripVisitor { idents };
            visitor.visit_item_impl_mut(from);
        }

        Ok(())
    }

    fn confirm(
        from: &ItemImpl,
        via: &SelfUsageAssocs,
    ) -> Result<(), TokenStream> {
        let SelfUsageAssocs { all_idents, .. } = via;
        let AssocIdents { types, consts, fns } = all_idents;

        for idents in [types.as_slice(), consts.as_slice(), fns.as_slice()] {
            let mut visitor = SelfAssocStripConfirmVisitor {
                idents,
                error: None,
            };

            visitor.visit_item_impl(from);

            if visitor.error.is_some() {
                return Err(ReplaceBugs::SelfAssocSelfNotStripped {}.into());
            }
        }

        Ok(())
    }
}

// ===============================================================================
// ``````````````````````````` REPLACE STANDALONE SELF ```````````````````````````
// ===============================================================================

pub(super) struct ReplaceStandAloneSelf;

struct ReplaceSelfVisitor {
    replacement: Type,
}

impl VisitMut for ReplaceSelfVisitor {
    fn visit_type_mut(&mut self, ty: &mut Type) {
        let is_standalone_self = matches!(
            ty,
            Type::Path(type_path)
                if type_path.qself.is_none()
                    && type_path.path.segments.len() == 1
                    && type_path.path.segments[0].ident == "Self"
                    && matches!(
                        type_path.path.segments[0].arguments,
                        PathArguments::None
                    )
        );

        if is_standalone_self {
            *ty = self.replacement.clone();
            return;
        }

        visit_mut::visit_type_mut(self, ty);
    }
}

struct ReplaceSelfVisitorConfirm {
    found: bool,
}

impl<'b> Visit<'b> for ReplaceSelfVisitorConfirm {
    fn visit_type(&mut self, ty: &'b Type) {
        if self.found {
            return;
        }

        let is_standalone_self = matches!(
            ty,
            Type::Path(type_path)
                if type_path.qself.is_none()
                    && type_path.path.segments.len() == 1
                    && type_path.path.segments[0].ident == "Self"
                    && matches!(
                        type_path.path.segments[0].arguments,
                        PathArguments::None
                    )
        );

        if is_standalone_self {
            self.found = true;
            return;
        }

        visit::visit_type(self, ty);
    }
}

impl ReplaceConfirm<ItemImpl> for ReplaceStandAloneSelf {
    fn replace(from: &mut ItemImpl, _: &()) -> Result<(), TokenStream> {
        let replacement = from.self_ty.clone();

        let mut visitor = ReplaceSelfVisitor {
            replacement: *replacement,
        };

        visitor.visit_item_impl_mut(from);
        Ok(())
    }

    fn confirm(from: &ItemImpl, _: &()) -> Result<(), TokenStream> {
        
        let mut visitor = ReplaceSelfVisitorConfirm { found: false };

        visitor.visit_item_impl(from);

        if visitor.found {
            return Err(ValidBugs::StandaloneSelfNotReplaced {}.into());
        }

        Ok(())
    }
}

impl ReplaceConfirm<InstanceBound, ItemImpl> for ReplaceStandAloneSelf {
    fn replace(from: &mut InstanceBound, context: &ItemImpl) -> Result<(), TokenStream> {
        let replacement = context.self_ty.clone();

        let mut visitor = ReplaceSelfVisitor {
            replacement: *replacement,
        };

        visitor.visit_trait_bound_mut(&mut from.0);
        Ok(())
    }

    fn confirm(from: &InstanceBound, _: &ItemImpl) -> Result<(), TokenStream> {
        
        let mut visitor = ReplaceSelfVisitorConfirm { found: false };

        visitor.visit_trait_bound(&from.0);

        if visitor.found {
            return Err(ValidBugs::StandaloneSelfNotReplaced {}.into());
        }

        Ok(())
    }
}

// ===============================================================================
// `````````````````````` VISIT SELF ASSOCS FOR REPLACEMENT ``````````````````````
// ===============================================================================

/// Visitor used to locate unqualified `Self::...` type paths and pass them to
/// a caller-provided replacement function.
///
/// A path is considered a `Self` associated path when it has no `QSelf`, has
/// at least two segments, and begins with `Self`.
///
/// The visitor therefore handles paths such as:
///
/// ```text
/// Self::Assoc
/// Self::Assoc<T>
/// Self::Assoc::Nested
/// Self::Assoc<T>::Nested
/// ```
///
/// while leaving qualified paths and unrelated paths untouched:
///
/// ```text
/// Self
/// <Self as Impl>::Assoc
/// Other::Assoc
/// ```
///
/// Before invoking the replacement function, generic arguments on the final
/// path segment are temporarily removed. This allows the replacement function
/// to operate on the structural path without having to account for
/// use-site generic arguments. The original arguments are restored after the
/// replacement completes, regardless of whether the replacement succeeds.
///
/// Once a matching path has been handed to the replacement function, the
/// visitor does not recursively visit the transformed path. This prevents the
/// replacement itself from being interpreted as another `Self::...` usage and
/// avoids repeatedly transforming the same path.
///
/// The visitor stops traversal after the first replacement error and stores
/// that diagnostic for propagation by the enclosing helper.
struct SelfAssocMutVisitor<F> {
    f: F,
    error: Option<TokenStream>,
}

impl<F> VisitMut for SelfAssocMutVisitor<F>
where
    F: FnMut(&mut TypePath) -> Result<(), TokenStream>,
{
    fn visit_type_path_mut(&mut self, type_path: &mut TypePath) {
        if self.error.is_some() {
            return;
        }

        // Match:
        //
        // `Self::Assoc`
        // `Self::Assoc<T>`
        // `Self::Assoc::Nested`
        // `Self::Assoc<T>::Nested`
        //
        // but not:
        //
        // `Self`
        // `<Self as Impl>::Assoc`
        // `Other::Assoc`
        let is_self_path = type_path.qself.is_none()
            && type_path.path.segments.len() >= 2
            && type_path
                .path
                .segments
                .first()
                .is_some_and(|segment| segment.ident == "Self");

        if is_self_path {
            // Preserve the generic arguments on the final associated
            // segment while the transformation operates on the path.
            let args = type_path
                .path
                .segments
                .last_mut()
                .map(|segment| std::mem::replace(&mut segment.arguments, PathArguments::None))
                .expect("Self path must contain a segment");

            let result = (self.f)(type_path);

            // Restore the original use-site generic arguments.
            type_path
                .path
                .segments
                .last_mut()
                .expect("Self path must contain a segment")
                .arguments = args;

            if let Err(error) = result {
                self.error = Some(error);
            }

            // Do not recursively visit the transformed path.
            return;
        }

        visit_mut::visit_type_path_mut(self, type_path);
    }
}

impl<F> SelfAssocMutVisitor<F>
where
    F: FnMut(&mut TypePath) -> Result<(), TokenStream>,
{
    /// Replaces matching `Self::...` associated-type references throughout
    /// the complete implementation.
    ///
    /// The visitor traverses the impl itself, including its generics,
    /// where-clauses, and impl items.
    fn visit_impl(item_impl: &mut ItemImpl, f: F) -> Result<(), TokenStream> {
        let mut visitor = SelfAssocMutVisitor { f, error: None };

        // Visit the complete implementation.
        visitor.visit_item_impl_mut(item_impl);

        match visitor.error {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    /// Replaces matching `Self::...` associated-type references only within
    /// associated types marked with `#[sum]`.
    ///
    /// The `#[sum]` attribute is not consumed (removed) as part of this operation.
    ///
    /// Unlike [`SelfAssocMutVisitor::visit_impl`], this helper deliberately
    /// visits only the type expression of each matching associated type rather
    /// than the complete impl item.
    fn visit_sum_types(item_impl: &mut ItemImpl, mut f: F) -> Result<(), TokenStream> {
        for item in &mut item_impl.items {
            let ImplItem::Type(item_type) = item else {
                continue;
            };

            let is_sum = item_type
                .attrs
                .iter()
                .any(|attr| attr.path().is_ident("sum"));

            if !is_sum {
                continue;
            }

            let mut visitor = SelfAssocMutVisitor {
                f: &mut f,
                error: None,
            };

            visitor.visit_impl_item_type_mut(item_type);

            if let Some(error) = visitor.error {
                return Err(error);
            }
        }

        Ok(())
    }
}

// ===============================================================================
// ````````````````` VISIT SELF ASSOC EXPR-CALLS FOR REPLACEMENT `````````````````
// ===============================================================================

/// Visitor used to locate calls to associated functions through an
/// unqualified `Self::...` path and replace the complete call expression.
///
/// A matching call has the form:
///
/// ```text
/// Self::foo()
/// Self::foo(arg)
/// Self::foo::<T>(arg)
/// ```
///
/// Qualified calls such as:
///
/// ```text
/// <Self as Impl>::foo()
/// ```
///
/// are intentionally excluded because they already carry an explicit Impl
/// qualification.
///
/// The visitor handles the call at the `Expr` level so that the complete
/// [`ExprCall`] can be supplied to the replacement function. This allows the
/// replacement to inspect both the called associated function and its
/// arguments before producing the replacement expression.
///
/// A matching call is not recursively visited after replacement. This
/// prevents the newly generated expression from being immediately interpreted
/// as another `Self::...` call.
///
/// The visitor also checks bare `Self::...` expression paths. A matching
/// associated-function path that occurs without a call is rejected because
/// associated functions represented by this transformation must be invoked,
/// rather than used as bare paths.
///
/// For example, given an instance associated function a call
/// through `Self` is valid:
///
/// ```text
/// Self::foo()
/// ```
///
/// but using the instance associated function as a bare expression is rejected:
///
/// ```text
/// Self::foo
/// ```
///
/// The latter is not an invocation of the associated function and therefore
/// cannot be handled by the call replacement performed by this visitor. The
/// macro requires the associated function to be explicitly invoked so that
/// the complete call expression can be transformed.
///
/// Generic calls are likewise expected to remain calls:
///
/// ```text
/// Self::foo::<T>()
/// ```
///
/// rather than a bare function path:
///
/// ```text
/// Self::foo::<T>
/// ```
struct SelfAssocCallMutVisitor<F> {
    f: F,
    error: Option<TokenStream>,
}

impl<F> VisitMut for SelfAssocCallMutVisitor<F>
where
    F: FnMut(&ExprCall) -> Result<Expr, TokenStream>,
{
    fn visit_expr_mut(&mut self, expr: &mut Expr) {
        if self.error.is_some() {
            return;
        }

        let Expr::Call(expr_call) = expr else {
            visit_mut::visit_expr_mut(self, expr);
            return;
        };

        let Expr::Path(expr_path) = expr_call.func.as_ref() else {
            visit_mut::visit_expr_mut(self, expr);
            return;
        };

        let is_self_call = expr_path.qself.is_none()
            && expr_path.path.segments.len() >= 2
            && expr_path
                .path
                .segments
                .first()
                .is_some_and(|segment| segment.ident == "Self");

        if is_self_call {
            match (self.f)(expr_call) {
                Ok(new_expr) => {
                    *expr = new_expr;
                }

                Err(error) => {
                    self.error = Some(error);
                }
            }

            return;
        }

        visit_mut::visit_expr_mut(self, expr);
    }

    fn visit_expr_call_mut(&mut self, expr_call: &mut ExprCall) {
        if self.error.is_some() {
            return;
        }

        let Expr::Path(expr_path) = expr_call.func.as_ref() else {
            visit_mut::visit_expr_call_mut(self, expr_call);
            return;
        };

        let is_self_call = expr_path.qself.is_none()
            && expr_path.path.segments.len() >= 2
            && expr_path
                .path
                .segments
                .first()
                .is_some_and(|segment| segment.ident == "Self");

        if is_self_call {
            return;
        }

        visit_mut::visit_expr_call_mut(self, expr_call);
    }

    fn visit_expr_path_mut(&mut self, expr_path: &mut ExprPath) {
        if self.error.is_some() {
            return;
        }

        let is_bare_self_call = expr_path.qself.is_none()
            && expr_path.path.segments.len() >= 2
            && expr_path
                .path
                .segments
                .first()
                .is_some_and(|segment| segment.ident == "Self");

        if is_bare_self_call {
            self.error = Some(
                ValidErrors::SelfAssocFunctionMustBeCalled {
                    seg: expr_path.path.segments.last().unwrap().clone(),
                }
                .into(),
            );

            return;
        }

        visit_mut::visit_expr_path_mut(self, expr_path);
    }
}

impl<F> SelfAssocCallMutVisitor<F>
where
    F: FnMut(&ExprCall) -> Result<Expr, TokenStream>,
{
    /// Replaces matching `Self::call()` associated-function call references
    /// throughout the complete implementation.
    ///
    /// Every unqualified `Self::...(...)` call encountered by the visitor is
    /// passed to `f` as the complete [`ExprCall`, allowing the replacement
    /// function to inspect both the associated-function path and its
    /// arguments.
    fn visit_impl(item_impl: &mut ItemImpl, f: F) -> Result<(), TokenStream> {
        let mut visitor = SelfAssocCallMutVisitor { f, error: None };

        // Visit the complete implementation.
        visitor.visit_item_impl_mut(item_impl);

        match visitor.error {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}

// ===============================================================================
// ```````````````````` LEAF SUM TYPES SELF-ASSOC REPLACEMENT ````````````````````
// ===============================================================================

/// Replaces `Self::...` associated-type references inside `#[sum]` associated
/// type declarations with the corresponding qualified instance-bound access.
///
/// Sum types require a separate replacement pass because their type
/// expressions are transformed through [`InstanceNonLeafTerminalPathAccess`] rather than
/// the ordinary [`InstanceLeafAccess`] transformation.
///
/// A `Self::Assoc` reference is first rewritten into a qualified path rooted
/// at the implementation's self type and the extracted instance bound. The
/// resulting expression path is then passed to [`InstanceNonLeafTerminalPathAccess`],
/// which applies the leaf access transformation using the supplied indexes.
///
/// Only associated types marked with the `#[sum]` attribute are processed.
/// The replacement visitor handles the `Self::...` type path itself, while
/// this operation constructs the qualified instance-bound path and performs
/// the non-leaf transformation.
#[derive(Debug, Clone)]
pub(super) struct ReplaceLeafSumTypeSelfAssocs;

/// Visits `Self::Assoc` Type Paths
struct SelfAssocVisitor {
    found: bool,
}

impl<'b> Visit<'b> for SelfAssocVisitor {
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

impl ReplaceConfirm<ItemImpl, (&InstanceBound, &LeafAccess)> for ReplaceLeafSumTypeSelfAssocs {
    fn replace(
        from: &mut ItemImpl,
        via: &(&InstanceBound, &LeafAccess),
    ) -> Result<(), TokenStream> {
        let &(bound, leaf) = via;
        let LeafAccess { indexes, .. } = leaf;
        let self_ty = *from.self_ty.clone();
        let bound = &bound.0;

        SelfAssocMutVisitor::visit_sum_types(from, |type_path| {
            let associated = type_path
                .path
                .segments
                .iter()
                .skip(1)
                .cloned()
                .collect::<Vec<_>>();

            type_path.qself = Some(syn::QSelf {
                lt_token: Default::default(),
                ty: Box::new(self_ty.clone()),
                position: bound.path.segments.len(),
                as_token: Some(Default::default()),
                gt_token: Default::default(),
            });

            type_path.path = syn::Path {
                leading_colon: bound.path.leading_colon,
                segments: {
                    let mut segments = bound.path.segments.clone();

                    segments.extend(associated);

                    segments
                },
            };

            let expr_path: ExprPath = parse_quote!(#type_path);

            let mut expr = Expr::Path(expr_path.clone());

            InstanceNonLeafTerminalPathAccess::checked_transform(
                &InstanceNonLeafTerminalPathAccess,
                &mut expr,
                &(&expr_path, indexes),
            )?;

            let new_ty_path: TypePath = parse_quote!(#expr);

            *type_path = new_ty_path;

            Ok(())
        })?;

        Ok(())
    }

    fn confirm(from: &ItemImpl, _: &(&InstanceBound, &LeafAccess)) -> Result<(), TokenStream> {
        for item in &from.items {
            let ImplItem::Type(item_type) = item else {
                continue;
            };
            let is_sum = item_type
                .attrs
                .iter()
                .any(|attr| attr.path().is_ident("sum"));
            if !is_sum {
                continue;
            }
            let mut visitor = SelfAssocVisitor { found: false };
            visitor.visit_impl_item_type(item_type);
            if visitor.found {
                return Err(ReplaceBugs::SumTypeSelfAssocNotReplaced {}.into());
            }
        }
        Ok(())
    }
}

// ===============================================================================
// ````````````````````````` LEAF SELF-ASSOC REPLACEMENT `````````````````````````
// ===============================================================================

/// Replaces `Self::...` associated-type references throughout the implementation
/// with qualified accesses rooted at the implementation's self type and
/// instance bound.
///
/// Associated items that reference `Self` are eventually lowered out of the
/// surrounding `impl` and transformed into independent items. A `Self::Assoc`
/// reference therefore cannot remain in its original form; it must first be
/// converted into an explicit qualified path that identifies the instance-bound
/// associated type from which the leaf access is derived.
///
/// The replacement changes the path from:
///
/// `Self::Assoc`
///
/// into the corresponding qualified path rooted at the implementation self
/// type and the extracted instance bound. The resulting associated access is
/// then passed through [`InstanceLeafAccess`] so that the requested leaf
/// indexes and identifiers are applied to the access.
///
/// The original associated path after `Self::` is preserved and appended to the
/// instance-bound path. Existing generic arguments on the final associated
/// segment are preserved by [`SelfAssocMutVisitor`] while the replacement is
/// performed.
///
/// This transformation is applied throughout the complete implementation,
/// including associated items whose type expressions contain nested
/// `Self::...` references.
#[derive(Debug, Clone)]
pub(super) struct ReplaceLeafSelfAssocs;

impl ReplaceConfirm<ItemImpl, (&InstanceBound, &LeafAccess)> for ReplaceLeafSelfAssocs {
    fn replace(
        from: &mut ItemImpl,
        via: &(&InstanceBound, &LeafAccess),
    ) -> Result<(), TokenStream> {
        let &(bound, leaf) = via;
        let LeafAccess { indexes, idents } = leaf;
        let self_ty = *from.self_ty.clone();
        let bound = &bound.0;

        SelfAssocMutVisitor::visit_impl(from, |type_path| {
            let associated = type_path
                .path
                .segments
                .iter()
                .skip(1)
                .cloned()
                .collect::<Vec<_>>();

            type_path.qself = Some(syn::QSelf {
                lt_token: Default::default(),
                ty: Box::new(self_ty.clone()),
                position: bound.path.segments.len(),
                as_token: Some(Default::default()),
                gt_token: Default::default(),
            });

            type_path.path = syn::Path {
                leading_colon: bound.path.leading_colon,
                segments: {
                    let mut segments = bound.path.segments.clone();

                    segments.extend(associated);

                    segments
                },
            };

            let expr: Expr = parse_quote!(#type_path);

            let mut assoc = match AssocAccessor::try_from(expr) {
                Ok(a) => a,
                Err(e) => {
                    return Err(e.into_compile_error());
                }
            };

            let mut mut_assoc = (&mut assoc).into();

            InstanceLeafAccess::checked_transform(
                &InstanceLeafAccess,
                &mut mut_assoc,
                &(idents, indexes),
            )?;

            let new_ty_path: TypePath = parse_quote!(#mut_assoc);

            *type_path = new_ty_path;

            Ok(())
        })?;

        Ok(())
    }

    fn confirm(from: &ItemImpl, _: &(&InstanceBound, &LeafAccess)) -> Result<(), TokenStream> {
        let mut visitor = SelfAssocVisitor { found: false };
        visitor.visit_item_impl(from);
        if visitor.found {
            return Err(ReplaceBugs::SelfAssocNotReplaced {}.into());
        }
        Ok(())
    }
}


// ===============================================================================
// `````````````````````` LEAF SELF-ASSOC CALLS REPLACEMENT ``````````````````````
// ===============================================================================

/// Replaces calls to associated functions through an unqualified `Self::...`
/// path with the corresponding qualified instance-bound access.
///
/// Associated functions that reference `Self` are eventually lowered out of
/// the surrounding `impl` and transformed into independent items. A call such
/// as `Self::foo(...)` therefore cannot remain dependent on the original
/// implementation context. The call must first be rewritten to explicitly
/// identify the instance-bound associated function from which the leaf access
/// is derived.
///
/// The replacement changes the called function path from:
///
/// `Self::foo(...)`
///
/// into the corresponding qualified path rooted at the implementation's self
/// type and the extracted instance bound. The resulting associated access is
/// then passed through [`InstanceLeafAccess`] so that the requested leaf
/// indexes and identifiers are applied to the access.
///
/// The complete [`ExprCall`] is passed to the replacement callback rather than
/// only its function path. This preserves the call's arguments while allowing
/// the function path to be replaced independently.
///
/// Existing generic arguments on the associated-function path are preserved by
/// [`SelfAssocCallMutVisitor`] while the replacement is performed.
///
/// The `confirm` phase verifies the post-transformation invariant: no
/// unqualified `Self::...` expression path remains in the implementation.
#[derive(Debug, Clone)]
pub(super) struct ReplaceLeafSelfAssocCalls;

/// Visits `Self::call()` Expr Calls
struct SelfAssocCallVisitor {
    found: bool,
}

impl<'b> Visit<'b> for SelfAssocCallVisitor {
    fn visit_expr_call(&mut self, expr_call: &'b ExprCall) {
        if self.found {
            return;
        }

        let Expr::Path(expr_path) = expr_call.func.as_ref() else {
            visit::visit_expr_call(self, expr_call);
            return;
        };

        let is_self_call = expr_path.qself.is_none()
            && expr_path.path.segments.len() >= 2
            && expr_path
                .path
                .segments
                .first()
                .is_some_and(|segment| segment.ident == "Self");

        if is_self_call {
            self.found = true;
            return;
        }

        visit::visit_expr_call(self, expr_call);
    }

    fn visit_expr_path(&mut self, expr_path: &'b ExprPath) {
        if self.found {
            return;
        }

        let is_self_path = expr_path.qself.is_none()
            && expr_path.path.segments.len() >= 2
            && expr_path
                .path
                .segments
                .first()
                .is_some_and(|segment| segment.ident == "Self");

        if is_self_path {
            self.found = true;
            return;
        }

        visit::visit_expr_path(self, expr_path);
    }
}

impl ReplaceConfirm<ItemImpl, (&InstanceBound, &LeafAccess)> for ReplaceLeafSelfAssocCalls {
    fn replace(
        from: &mut ItemImpl,
        via: &(&InstanceBound, &LeafAccess),
    ) -> Result<(), TokenStream> {
        let &(bound, leaf) = via;
        let LeafAccess { indexes, idents } = leaf;
        let self_ty = *from.self_ty.clone();
        let bound = &bound.0;

        SelfAssocCallMutVisitor::visit_impl(from, |expr_call| {
            let mut expr_call = expr_call.clone();
            let Expr::Path(expr_path) = expr_call.func.deref_mut() else {
                return Err(ValidErrors::SelfAssocFunctionPathRequired {
                    expr: *expr_call.func.clone(),
                }
                .into());
            };

            let associated = expr_path
                .path
                .segments
                .iter()
                .skip(1)
                .cloned()
                .collect::<Vec<_>>();

            expr_path.path = syn::Path {
                leading_colon: bound.path.leading_colon,
                segments: {
                    let mut segments = bound.path.segments.clone();
                    segments.extend(associated);
                    segments
                },
            };

            expr_path.qself = Some(syn::QSelf {
                lt_token: Default::default(),
                ty: Box::new(self_ty.clone()),
                position: bound.path.segments.len(),
                as_token: Some(Default::default()),
                gt_token: Default::default(),
            });

            let expr: Expr = parse_quote!(#expr_call);

            let mut call = match AssocAccessor::try_from(expr) {
                Ok(a) => a,
                Err(e) => {
                    return Err(e.into_compile_error());
                }
            };

            let mut mut_call = (&mut call).into();

            InstanceLeafAccess::checked_transform(
                &InstanceLeafAccess,
                &mut mut_call,
                &(idents, indexes),
            )?;

            let new_call: ExprCall = parse_quote!(#mut_call);

            Ok(Expr::Call(new_call))
        })?;

        Ok(())
    }

    fn confirm(from: &ItemImpl, _: &(&InstanceBound, &LeafAccess)) -> Result<(), TokenStream> {
        let mut visitor = SelfAssocCallVisitor { found: false };
        visitor.visit_item_impl(from);
        if visitor.found {
            return Err(ReplaceBugs::SelfAssocCallNotReplaced {}.into());
        }
        Ok(())
    }
}

// ===============================================================================
// ```````````````````` NON-LEAF ASSOC FNS RETURN TYPE ERROR `````````````````````
// ===============================================================================

/// Replaces an inferred `Result` error type with the macro's concrete error
/// type.
///
/// Associated functions may declare their return type as:
///
/// ```ignore
/// Result<T, _>
/// ```
///
/// Before the impl is lowered into standalone items, the inferred error type
/// must be made explicit because the generated item can no longer rely on the
/// original impl context for the intended error type.
///
/// This transformation only replaces the error type when the second generic
/// argument is [`Type::Infer`]. Existing explicit error types are left
/// untouched.
///
/// For example:
///
/// ```ignore
/// fn foo() -> Result<T, _>
/// ```
///
/// becomes:
///
/// ```ignore
/// fn foo() -> Result<T, crate::Error>
/// ```
///
/// The `confirm` phase verifies the post-transformation invariant by ensuring
/// that no function in the impl still contains `Result<_, _>` with an
/// inferred error type.
#[derive(Debug, Clone)]
pub(super) struct ReplaceReturnTyInferedError;

impl ReplaceConfirm<ItemImpl> for ReplaceReturnTyInferedError {
    fn replace(from: &mut ItemImpl, _: &()) -> Result<(), TokenStream> {
        let crate_of = Instance::support_crate();
        for item in &mut from.items {
            let ImplItem::Fn(item_fn) = item else {
                continue;
            };
            let ReturnType::Type(_, ty) = &mut item_fn.sig.output else {
                continue;
            };
            let Type::Path(type_path) = ty.as_mut() else {
                continue;
            };
            let Some(result_segment) = type_path.path.segments.last_mut() else {
                continue;
            };
            if result_segment.ident != "Result" {
                continue;
            }
            let PathArguments::AngleBracketed(args) = &mut result_segment.arguments else {
                continue;
            };
            let Some(GenericArgument::Type(Type::Infer(_))) = args.args.get(1) else {
                continue;
            };
            args.args[1] = parse_quote!(#crate_of::Error);
        }
        Ok(())
    }

    fn confirm(from: &ItemImpl, _: &()) -> Result<(), TokenStream> {
        for item in &from.items {
            let ImplItem::Fn(item_fn) = item else {
                continue;
            };
            let ReturnType::Type(_, ty) = &item_fn.sig.output else {
                continue;
            };
            let Type::Path(type_path) = &**ty else {
                continue;
            };
            let Some(result_segment) = type_path.path.segments.last() else {
                continue;
            };
            if result_segment.ident != "Result" {
                continue;
            }
            let PathArguments::AngleBracketed(args) = &result_segment.arguments else {
                continue;
            };
            if matches!(
                args.args.get(1),
                Some(GenericArgument::Type(Type::Infer(_)))
            ) {
                return Err(ReplaceBugs::ReturnTyInferedErrorNotReplaced {}.into());
            }
        }
        Ok(())
    }
}

// ===============================================================================
// ``````````````````````` NON-LEAF SELF-ASSOC FN DYN ARGS ```````````````````````
// ===============================================================================

/// Appends dynamic byte-slice arguments to non-leaf associated functions and
/// propagates those arguments through their callers.
///
/// A non-leaf access may require additional identifiers to remain available
/// while traversing the generated access path. When an associated function
/// directly calls another associated function through `Self`, those
/// identifiers must therefore be threaded through the function call chain.
///
/// For example:
///
/// ```text
/// fn c() {
///     Self::foo();
/// }
///
/// fn b() {
///     c();
/// }
///
/// fn a() {
///     b();
/// }
/// ```
///
/// with:
///
/// ```text
/// ident(x, y)
/// ```
///
/// requires the transformation:
///
/// ```text
/// fn c(x: &[u8], y: &[u8]) {
///     Self::foo();
/// }
///
/// fn b(x: &[u8], y: &[u8]) {
///     c(x, y);
/// }
///
/// fn a(x: &[u8], y: &[u8]) {
///     b(x, y);
/// }
/// ```
///
/// The transformation proceeds from the bottom of the call graph upwards
///
/// [`SelfCallVisitor`] finds the initial functions, [`CalledFnsVisitor`]
/// discovers their callers, and [`CallVisitor`] finally updates the call
/// sites to pass the generated arguments.
pub(super) struct AppendDynFnArg;

/// Finds whether an associated function directly calls another associated
/// function through `Self`.
///
/// A direct `Self` call has the form:
///
/// ```text
/// Self::foo()
/// Self::foo(arg)
/// Self::foo::<T>()
/// ```
///
/// The visitor only needs to know whether such a call exists, so it stops
/// walking the function as soon as the first matching call is found.
///
/// Paths that are not direct `Self` calls are ignored, for example:
///
/// ```text
/// foo()
/// Other::foo()
/// <Self as Impl>::foo()
/// ```
struct SelfCallFinder {
    found: bool,
}

impl<'b> Visit<'b> for SelfCallFinder {
    fn visit_expr_call(&mut self, expr_call: &'b ExprCall) {
        if self.found {
            return;
        }

        let Expr::Path(expr_path) = expr_call.func.as_ref() else {
            visit::visit_expr_call(self, expr_call);
            return;
        };

        let is_self_call = expr_path.qself.is_none()
            && expr_path.path.segments.len() >= 2
            && expr_path
                .path
                .segments
                .first()
                .is_some_and(|segment| segment.ident == "Self");

        if is_self_call {
            self.found = true;
            return;
        }

        visit::visit_expr_call(self, expr_call);
    }
}

/// Finds all associated functions that directly depend on another associated
/// function through a `Self::...()` call.
///
/// For example:
///
/// ```text
/// fn a() {
///     Self::foo();
/// }
///
/// fn b() {
///     Self::bar();
/// }
///
/// fn c() {
///     foo();
/// }
/// ```
///
/// produces:
///
/// ```text
/// affected = [a, b]
/// ```
///
/// `c` is not included because it does not directly call through `Self`.
///
/// These functions form the starting point for propagating the dynamic
/// arguments upwards through the ordinary function-call graph.
struct SelfCallVisitor {
    affected: Vec<Ident>,
}

impl SelfCallVisitor {
    fn visit_impl(item_impl: &ItemImpl) -> Vec<Ident> {
        let mut visitor = SelfCallVisitor {
            affected: Vec::new(),
        };

        visitor.visit_item_impl(item_impl);

        visitor.affected
    }
}

impl<'b> Visit<'b> for SelfCallVisitor {
    fn visit_impl_item_fn(&mut self, item_fn: &'b ImplItemFn) {
        let mut visitor = SelfCallFinder { found: false };

        visitor.visit_impl_item_fn(item_fn);

        if visitor.found {
            self.affected.push(item_fn.sig.ident.clone());
        }
    }
}

/// Collects bare function calls made by a function.
///
/// A bare call is a call whose function path contains exactly one segment:
///
/// ```text
/// foo()
/// bar(arg)
/// ```
///
/// For example:
///
/// ```text
/// fn a() {
///     b();
///     c();
///     Self::d();
///     Other::e();
/// }
/// ```
///
/// produces:
///
/// ```text
/// called = [b, c]
/// ```
///
/// `Self::d()` and `Other::e()` are ignored because their paths contain more
/// than one segment.
///
/// This visitor is used to walk the call graph from functions that already
/// require the dynamic arguments towards their callers.
struct CalledFnsVisitor {
    called: Vec<Ident>,
}

impl<'b> Visit<'b> for CalledFnsVisitor {
    fn visit_expr_call(&mut self, expr_call: &'b ExprCall) {
        let Expr::Path(expr_path) = expr_call.func.as_ref() else {
            visit::visit_expr_call(self, expr_call);
            return;
        };

        if expr_path.qself.is_none() && expr_path.path.segments.len() == 1 {
            self.called.push(
                expr_path
                    .path
                    .segments
                    .first()
                    .expect("single segment")
                    .ident
                    .clone(),
            );
        }

        visit::visit_expr_call(self, expr_call);
    }
}

/// Appends the dynamic identifiers to calls of functions that require them.
///
/// `added_to` contains the functions whose signatures have already received
/// the generated arguments, while `idents` contains the arguments that must
/// be passed to those functions.
///
/// For example, if:
///
/// ```text
/// added_to = [foo]
/// idents   = [x, y]
/// ```
///
/// then:
///
/// ```text
/// foo()
/// ```
///
/// becomes:
///
/// ```text
/// foo(x, y)
/// ```
///
/// Existing call arguments are preserved:
///
/// ```text
/// foo(a)
/// ```
///
/// becomes:
///
/// ```text
/// foo(a, x, y)
/// ```
///
/// Only bare function calls are modified. Qualified calls such as
/// `Self::foo()` or `Other::foo()` are not handled by this visitor.
struct CallVisitor<'a> {
    added_to: &'a [Ident],
    idents: &'a [Ident],
}

impl VisitMut for CallVisitor<'_> {
    fn visit_expr_call_mut(&mut self, expr_call: &mut ExprCall) {
        let Expr::Path(expr_path) = expr_call.func.as_ref() else {
            visit_mut::visit_expr_call_mut(self, expr_call);
            return;
        };

        if expr_path.qself.is_none() && expr_path.path.segments.len() == 1 {
            let fn_ident = &expr_path
                .path
                .segments
                .first()
                .expect("single segment")
                .ident;

            if self.added_to.iter().any(|ident| ident == fn_ident) {
                for ident in self.idents {
                    expr_call.args.push(parse_quote!(#ident));
                }
            }
        }

        visit_mut::visit_expr_call_mut(self, expr_call);
    }
}

impl AppendDynFnArg {
    /// Adds each dynamic identifier as a `&[u8]` parameter to one function.
    ///
    /// For example, `idents = [x, y]` turns:
    ///
    /// `fn foo()` -> `fn foo(x: &[u8], y: &[u8])`
    ///
    /// Existing parameter names are checked first to avoid collisions.
    fn fn_append_args(func: &mut ImplItemFn, idents: &[Ident]) -> Result<(), TokenStream> {
        for ident in idents {
            let conflict = func.sig.inputs.iter().find_map(|input| {
                let FnArg::Typed(arg) = input else {
                    return None;
                };

                let Pat::Ident(pat_ident) = arg.pat.as_ref() else {
                    return None;
                };

                (pat_ident.ident == *ident).then(|| pat_ident.ident.clone())
            });

            if let Some(ident) = conflict {
                return Err(ReplaceErrors::NonLeafFnArgIdentConflict { ident }.into());
            }
        }

        for ident in idents {
            func.sig
                .inputs
                .push(parse_quote!(#[allow(non_snake_case)] #ident: &[u8]));
        }

        Ok(())
    }

    /// Adds the dynamic arguments to functions that are directly required.
    ///
    /// For example, if `required = [foo, bar]`, only `foo` and `bar` receive
    /// the new parameters. The names of functions actually modified are
    /// returned.
    fn append_args(
        item_impl: &mut ItemImpl,
        required: &[Ident],
        idents: &[Ident],
    ) -> Result<Vec<Ident>, TokenStream> {
        let mut added_to = Vec::new();

        for item in &mut item_impl.items {
            let ImplItem::Fn(func) = item else {
                continue;
            };

            if !required.iter().any(|ident| ident == &func.sig.ident) {
                continue;
            }

            Self::fn_append_args(func, &idents)?;

            added_to.push(func.sig.ident.clone());
        }

        Ok(added_to)
    }

    /// `added_to` contains the functions that already added the dynamic
    /// arguments.
    ///
    /// This function then searches all other associated functions for callers of
    /// those functions. Any function that calls an `added_to` function must also
    /// receive the same dynamic arguments, because it must pass them to its
    /// callee.
    ///
    /// The search is repeated until no new caller is found.
    ///
    /// For example:
    ///
    /// ```text
    /// fn a() {
    ///     b();
    /// }
    ///
    /// fn b() {
    ///     c();
    /// }
    ///
    /// fn c() {
    ///     Self::foo();
    /// }
    /// ```
    ///
    /// If `c` was initially added because it calls `Self::foo()`:
    ///
    /// ```text
    /// added_to = [c]
    /// ```
    ///
    /// the first pass finds that `b` calls `c`:
    ///
    /// ```text
    /// added_to = [c, b]
    /// ```
    ///
    /// The next pass finds that `a` calls `b`:
    ///
    /// ```text
    /// added_to = [c, b, a]
    /// ```
    ///
    /// The process stops once no additional caller can be found.
    ///
    /// This propagates the dynamic arguments from the bottom of the call chain
    /// towards its callers
    ///
    /// For example, with `ident(x)` the final signatures become conceptually
    /// along with [`Self::callers_append_args`]:
    ///
    /// ```text
    /// fn c(x: &[u8]) { Self::foo(); }
    /// fn b(x: &[u8]) { c(x); }
    /// fn a(x: &[u8]) { b(x); }
    /// ```
    fn caller_fns_append_args(
        item_impl: &mut ItemImpl,
        added_to: &mut Vec<Ident>,
        arg_idents: &[Ident],
    ) -> Result<(), TokenStream> {
        loop {
            let mut newly_added = Vec::new();

            for item in &mut item_impl.items {
                let ImplItem::Fn(func) = item else {
                    continue;
                };

                if added_to.iter().any(|ident| ident == &func.sig.ident) {
                    continue;
                }

                let mut visitor = CalledFnsVisitor { called: Vec::new() };

                visitor.visit_impl_item_fn(func);

                let calls_added_fn = visitor
                    .called
                    .iter()
                    .any(|called| added_to.iter().any(|added| added == called));

                if !calls_added_fn {
                    continue;
                }

                Self::fn_append_args(func, arg_idents)?;

                newly_added.push(func.sig.ident.clone());
            }

            if newly_added.is_empty() {
                break;
            }

            added_to.extend(newly_added);
        }

        Ok(())
    }

    /// Appends the dynamic identifiers to calls of functions that already have
    /// those identifiers in their signatures.
    ///
    /// `added_to` contains the associated functions whose signatures were
    /// previously updated to accept the dynamic arguments. This function walks
    /// the impl and updates calls to those functions so that the new arguments
    /// are passed at every call site.
    ///
    /// Only bare calls to functions in `added_to` are modified. Qualified calls
    /// such as `Self::foo()` or `Other::foo()` are left unchanged.
    fn callers_append_args(item_impl: &mut ItemImpl, added_to: &[Ident], idents: &[Ident]) {
        let mut visitor = CallVisitor { added_to, idents };

        visitor.visit_item_impl_mut(item_impl);
    }
}

/// Confirms that every call to an affected function passes all generated
/// dynamic arguments.
///
/// If a function has been identified as requiring:
///
/// ```text
/// x, y
/// ```
///
/// then every call to that function must contain both identifiers.
///
/// For example, if `foo` is affected:
///
/// ```text
/// foo(x, y)
/// ```
///
/// is valid, while:
///
/// ```text
/// foo()
/// foo(x)
/// foo(y)
/// ```
///
/// is invalid.
///
/// The visitor only checks bare function calls because the transformation
/// records affected associated functions by their local identifiers.
struct ConfirmCallVisitor<'a> {
    added_to: &'a [Ident],
    idents: &'a [Ident],
    error: Option<()>,
}

impl Visit<'_> for ConfirmCallVisitor<'_> {
    fn visit_expr_call(&mut self, expr_call: &ExprCall) {
        if self.error.is_some() {
            return;
        }

        let Expr::Path(expr_path) = expr_call.func.as_ref() else {
            visit::visit_expr_call(self, expr_call);
            return;
        };

        if expr_path.qself.is_none() && expr_path.path.segments.len() == 1 {
            let fn_ident = &expr_path
                .path
                .segments
                .first()
                .expect("single segment")
                .ident;

            if self.added_to.iter().any(|ident| ident == fn_ident) {
                for ident in self.idents {
                    let passed = expr_call.args.iter().any(|arg| {
                        let Expr::Path(path) = arg else {
                            return false;
                        };

                        path.qself.is_none()
                            && path.path.segments.len() == 1
                            && path
                                .path
                                .segments
                                .first()
                                .is_some_and(|segment| segment.ident == *ident)
                    });

                    if !passed {
                        self.error = Some(());
                        return;
                    }
                }
            }
        }

        visit::visit_expr_call(self, expr_call);
    }
}

impl ReplaceConfirm<ItemImpl, AccessArgs> for AppendDynFnArg {
    fn replace(from: &mut ItemImpl, via: &AccessArgs) -> Result<(), TokenStream> {
        let list = &AccessArgsIdents::checked_extract(via, &())?.idents;

        let mut idents = Vec::new();
        for expr in &list.exprs {
            match expr {
                // raw exprs are directly utilized only explicit idents are taken
                BStrInput::Raw(_) => {}
                BStrInput::Ident(ident) => idents.push(ident.clone()),
            }
        }

        let direct_fns = SelfCallVisitor::visit_impl(from);

        let mut appended_fns = Self::append_args(from, &direct_fns, &idents)?;

        Self::caller_fns_append_args(from, &mut appended_fns, &idents)?;

        Self::callers_append_args(from, &appended_fns, &idents);

        Ok(())
    }

    fn confirm(from: &ItemImpl, via: &AccessArgs) -> Result<(), TokenStream> {
        let list = &AccessArgsIdents::checked_extract(via, &())?.idents;

        let mut idents = Vec::new();

        for expr in &list.exprs {
            match expr {
                BStrInput::Raw(_) => {}
                BStrInput::Ident(ident) => {
                    idents.push(ident.clone());
                }
            }
        }

        // Reconstruct the set of functions that must have the dynamic arguments.
        let direct_fns = SelfCallVisitor::visit_impl(from);

        let mut added_to = direct_fns;

        loop {
            let mut newly_added = Vec::new();

            for item in &from.items {
                let ImplItem::Fn(func) = item else {
                    continue;
                };

                if added_to.iter().any(|ident| ident == &func.sig.ident) {
                    continue;
                }

                let mut visitor = CalledFnsVisitor { called: Vec::new() };

                visitor.visit_impl_item_fn(func);

                let calls_added_fn = visitor
                    .called
                    .iter()
                    .any(|called| added_to.iter().any(|added| added == called));

                if calls_added_fn {
                    newly_added.push(func.sig.ident.clone());
                }
            }

            if newly_added.is_empty() {
                break;
            }

            added_to.extend(newly_added);
        }

        // Every affected function must contain every generated argument.
        for item in &from.items {
            let ImplItem::Fn(func) = item else {
                continue;
            };

            if !added_to.iter().any(|ident| ident == &func.sig.ident) {
                continue;
            }

            for ident in &idents {
                let found = func.sig.inputs.iter().any(|input| {
                    let FnArg::Typed(arg) = input else {
                        return false;
                    };

                    let Pat::Ident(pat_ident) = arg.pat.as_ref() else {
                        return false;
                    };

                    pat_ident.ident == *ident
                });

                if !found {
                    return Err(ReplaceBugs::DynFnArgNotAppended {}.into());
                }
            }
        }

        // Every call to an affected function must pass every generated argument.
        let mut visitor = ConfirmCallVisitor {
            added_to: &added_to,
            idents: &idents,
            error: None,
        };

        visitor.visit_item_impl(from);

        if visitor.error.is_some() {
            return Err(ReplaceBugs::DynFnCallArgNotAppended {}.into());
        }

        Ok(())
    }
}

// ===============================================================================
// ``````````````````````` NON-LEAF SELF-ASSOC REPLACEMENT ```````````````````````
// ===============================================================================

/// Replaces `Self::...` associated-type references in non-leaf access modes
/// with the corresponding qualified instance-bound access.
///
/// Associated-type references using `Self::...` cannot remain in their
/// original form once the associated items are lowered out of the impl.
/// They must instead be rewritten to a qualified path rooted at the
/// implementation's self type and the extracted instance bound.
///
/// The resulting qualified path is then passed to
/// [`InstanceNonLeafTerminalPathAccess`], which applies the transformation required
/// by the selected non-leaf access mode using its associated indexes.
///
/// This operation handles the non-leaf variants of [`AccessArgs`]:
/// [`AccessArgs::Branch`], [`AccessArgs::Prune`], [`AccessArgs::Trim`],
/// [`AccessArgs::Extend`], [`AccessArgs::Spread`], [`AccessArgs::Traverse`],
/// [`AccessArgs::Descend`], and [`AccessArgs::Root`].
///
/// [`AccessArgs::Leaf`] must not be passed to this operation because leaf
/// access is handled separately by the leaf-specific replacement passes.
/// [`AccessArgs::Unknown`] likewise must not reach this transformation;
/// it represents an unresolved access mode rather than a valid non-leaf
/// transformation. Both cases are therefore treated as unreachable.
#[derive(Debug, Clone)]
pub(super) struct ReplaceNonLeafSelfAssocs;

impl ReplaceConfirm<ItemImpl, (&InstanceBound, &AccessArgs)> for ReplaceNonLeafSelfAssocs {
    fn replace(
        from: &mut ItemImpl,
        via: &(&InstanceBound, &AccessArgs),
    ) -> Result<(), TokenStream> {
        let &(bound, args) = via;
        let indexes = AccessArgsIndexes::checked_utilize(args, &())?.0;
        let self_ty = *from.self_ty.clone();
        let bound = &bound.0;

        SelfAssocMutVisitor::visit_impl(from, |type_path| {
            let associated = type_path
                .path
                .segments
                .iter()
                .skip(1)
                .cloned()
                .collect::<Vec<_>>();

            type_path.qself = Some(syn::QSelf {
                lt_token: Default::default(),
                ty: Box::new(self_ty.clone()),
                position: bound.path.segments.len(),
                as_token: Some(Default::default()),
                gt_token: Default::default(),
            });

            type_path.path = syn::Path {
                leading_colon: bound.path.leading_colon,
                segments: {
                    let mut segments = bound.path.segments.clone();

                    segments.extend(associated);

                    segments
                },
            };

            let expr_path: ExprPath = parse_quote!(#type_path);

            let mut expr = Expr::Path(expr_path.clone());

            InstanceNonLeafTerminalPathAccess::checked_transform(
                &InstanceNonLeafTerminalPathAccess,
                &mut expr,
                &(&expr_path, indexes),
            )?;

            let new_ty_path: TypePath = parse_quote!(#expr);

            *type_path = new_ty_path;

            Ok(())
        })?;

        Ok(())
    }

    fn confirm(from: &ItemImpl, _: &(&InstanceBound, &AccessArgs)) -> Result<(), TokenStream> {
        let mut visitor = SelfAssocVisitor { found: false };
        visitor.visit_item_impl(from);
        if visitor.found {
            return Err(ReplaceBugs::SelfAssocNotReplaced {}.into());
        }
        Ok(())
    }
}

// ===============================================================================
// ```````````````````` NON-LEAF SELF-ASSOC CALLS REPLACEMENT ````````````````````
// ===============================================================================

/// Replaces calls to associated functions through an unqualified `Self::...`
/// path with the corresponding qualified instance-bound access for a
/// non-leaf access mode.
///
/// Associated functions that reference `Self` are eventually lowered out of
/// the surrounding `impl` and transformed into independent items. A call such
/// as `Self::foo(...)` therefore cannot remain dependent on the original
/// implementation context. The call must first be rewritten to explicitly
/// identify the instance-bound associated function from which the access is
/// derived.
///
/// The replacement changes the called function path from:
///
/// ```text
/// Self::foo(...)
/// ```
///
/// into the corresponding qualified path rooted at the implementation's self
/// type and the extracted instance bound.
///
/// Unlike the leaf replacement, the resulting call is passed directly to
/// [`InstanceNonLeafExprCallAccess`]. That transformation applies the selected
/// non-leaf access mode and its associated arguments to the complete function
/// call.
///
/// For example, a call such as:
///
/// ```text
/// Self::foo(arg)
/// ```
///
/// is first qualified conceptually as:
///
/// ```text
/// <Self as InstanceBound>::foo(arg)
/// ```
///
/// and is then transformed according to the supplied non-leaf access
/// arguments.
///
/// The complete [`ExprCall`] is passed to the non-leaf transformation so that
/// both the called function and its existing call arguments remain available
/// during the access transformation.
///
/// Existing generic arguments on the associated-function path are preserved
/// by [`SelfAssocCallMutVisitor`] while the replacement is performed.
#[derive(Debug, Clone)]
pub(super) struct ReplaceNonLeafSelfAssocCalls;

impl ReplaceConfirm<ItemImpl, (&InstanceBound, &AccessArgs)> for ReplaceNonLeafSelfAssocCalls {
    fn replace(
        from: &mut ItemImpl,
        via: &(&InstanceBound, &AccessArgs),
    ) -> Result<(), TokenStream> {
        let &(bound, args) = via;
        let self_ty = *from.self_ty.clone();
        let bound = &bound.0;
        let generics = from.generics.clone();

        SelfAssocCallMutVisitor::visit_impl(from, |expr_call| {
            let mut expr_call = expr_call.clone();
            let Expr::Path(expr_path) = expr_call.func.deref_mut() else {
                return Err(ValidErrors::SelfAssocFunctionPathRequired {
                    expr: *expr_call.func.clone(),
                }
                .into());
            };

            let associated = expr_path
                .path
                .segments
                .iter()
                .skip(1)
                .cloned()
                .collect::<Vec<_>>();

            expr_path.path = syn::Path {
                leading_colon: bound.path.leading_colon,
                segments: {
                    let mut segments = bound.path.segments.clone();
                    segments.extend(associated);
                    segments
                },
            };

            expr_path.qself = Some(syn::QSelf {
                lt_token: Default::default(),
                ty: Box::new(self_ty.clone()),
                position: bound.path.segments.len(),
                as_token: Some(Default::default()),
                gt_token: Default::default(),
            });

            let mut expr: Expr = parse_quote!(#expr_call);

            InstanceNonLeafExprCallAccess::checked_transform(
                &InstanceNonLeafExprCallAccess,
                &mut expr,
                &(&expr_call, &generics, args),
            )?;

            Ok(expr)
        })?;

        Ok(())
    }

    fn confirm(from: &ItemImpl, _: &(&InstanceBound, &AccessArgs)) -> Result<(), TokenStream> {
        let mut visitor = SelfAssocCallVisitor { found: false };
        visitor.visit_item_impl(from);
        if visitor.found {
            return Err(ReplaceBugs::SelfAssocCallNotReplaced {}.into());
        }
        Ok(())
    }
}

// ===============================================================================
// ```````````````````````````` REMOVE SUM ATTRIBUTES ````````````````````````````
// ===============================================================================

/// Removes the #[sum] attribute from all associated type declarations
/// within an implementation.
///
/// This is performed after the #[sum]-specific transformation has been
/// applied, ensuring that the attribute does not remain on the generated
/// associated type items.
///
/// And implies the associated type as a sum type via documentation regardless
/// of the target usage.
#[derive(Debug, Clone)]
pub(super) struct SumAttrRemoval;

impl ReplaceConfirm<ItemImpl, AccessArgs> for SumAttrRemoval {
    fn replace(from: &mut ItemImpl, args: &AccessArgs) -> Result<(), TokenStream> {
        let is_leaf = if let AccessArgs::Leaf(_) = args {
            true
        } else {
            false
        };

        for item in &mut from.items {
            let ImplItem::Type(ty) = item else {
                let attrs = match item {
                    ImplItem::Const(c) => &c.attrs,
                    ImplItem::Fn(f) => &f.attrs,
                    ImplItem::Macro(m) => &m.attrs,
                    _ => {
                        continue;
                    }
                };

                if attrs.iter().any(|attr| attr.path().is_ident("sum")) {
                    return Err(ReplaceErrors::SumAttrOnlyOnType {
                        assoc: item.clone(),
                    }
                    .into());
                };

                continue;
            };

            if let Some(attr) = ty.attrs.iter().find(|attr| attr.path().is_ident("sum")) {
                if !is_leaf {
                    return Err(ReplaceErrors::AllTypesAreSum { attr: attr.clone() }.into());
                }
                // Remove #[sum], preserving all other attributes.
                ty.attrs.retain(|attr| !attr.path().is_ident("sum"));
            }
        }

        Ok(())
    }

    fn confirm(from: &ItemImpl, args: &AccessArgs) -> Result<(), TokenStream> {
        let is_leaf = if let AccessArgs::Leaf(_) = args {
            true
        } else {
            false
        };

        for item in &from.items {
            if let ImplItem::Type(assoc_ty) = item {
                if assoc_ty
                    .attrs
                    .iter()
                    .any(|attr| attr.path().is_ident("sum"))
                {
                    if !is_leaf {
                        return Err(ReplaceBugs::AllTypesAreSum {}.into());
                    }

                    return Err(ReplaceBugs::SumAttrNotRemoved {}.into());
                }
            }
        }

        Ok(())
    }
}

// ===============================================================================
// `````````````````````````` DOC SELF-ASSOC REPLACEMENT `````````````````````````
// ===============================================================================

#[derive(Debug, Clone)]
pub(super) struct ReplaceDocSelfAssocs;

impl ReplaceConfirm<ItemImpl, InstanceBound> for ReplaceDocSelfAssocs {
    fn replace(from: &mut ItemImpl, via: &InstanceBound) -> Result<(), TokenStream> {
        let self_ty = *from.self_ty.clone();
        let bound = &via.0;

        SelfAssocMutVisitor::visit_impl(from, |type_path| {
            let Some(seg) = type_path.path.segments.last() else {
                return Err(ReplaceBugs::DocPathNotTypePath {}.into());
            };
            let ident = &seg.ident;

            *type_path = parse_quote!(<#self_ty as #bound>::#ident);

            Ok(())
        })?;

        Ok(())
    }

    fn confirm(from: &ItemImpl, _: &InstanceBound) -> Result<(), TokenStream> {
        let mut visitor = SelfAssocVisitor { found: false };
        visitor.visit_item_impl(from);
        if visitor.found {
            return Err(ReplaceBugs::SelfAssocNotReplaced {}.into());
        }
        Ok(())
    }
}

// ===============================================================================
// ``````````````````````` DOC SELF-ASSOC CALLS REPLACEMENT ``````````````````````
// ===============================================================================

#[derive(Debug, Clone)]
pub(super) struct ReplaceDocSelfAssocCalls;

impl ReplaceConfirm<ItemImpl, InstanceBound> for ReplaceDocSelfAssocCalls {
    fn replace(from: &mut ItemImpl, via: &InstanceBound) -> Result<(), TokenStream> {
        let self_ty = *from.self_ty.clone();
        let bound = &via.0;

        SelfAssocCallMutVisitor::visit_impl(from, |expr_call| {
            let args = &expr_call.args;
            let Expr::Path(expr_path) = &*expr_call.func else {
                return Err(ValidErrors::SelfAssocFunctionPathRequired {
                    expr: *expr_call.func.clone(),
                }
                .into());
            };

            let Some(seg) = expr_path.path.segments.last() else {
                return Err(ValidErrors::SelfAssocFunctionPathRequired {
                    expr: *expr_call.func.clone(),
                }
                .into());
            };
            let ident = &seg.ident;

            let expr = parse_quote!(<#self_ty as #bound>::#ident(#args));

            Ok(expr)
        })?;

        Ok(())
    }

    fn confirm(from: &ItemImpl, _: &InstanceBound) -> Result<(), TokenStream> {
        let mut visitor = SelfAssocCallVisitor { found: false };
        visitor.visit_item_impl(from);
        if visitor.found {
            return Err(ReplaceBugs::SelfAssocCallNotReplaced {}.into());
        }
        Ok(())
    }
}
