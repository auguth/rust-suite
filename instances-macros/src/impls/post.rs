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
// `````````````````````````````` POST INSTANCE IMPL `````````````````````````````
// ===============================================================================

//! Post-expansion implementation cleanup phase.
//!
//! This module runs after all implementation-side expansion and
//! validation phases have completed.
//!
//! ## Purpose
//!
//! Several helper attributes are introduced solely for proc-macro
//! processing and internal metadata extraction.
//!
//! Examples include attributes used to:
//!
//! - identify counter-associated items,
//! - describe generated bounds,
//! - provide expansion metadata,
//! - or guide intermediate transformation phases.
//!
//! Such attributes are not intended to remain in the final expanded
//! implementation and may not represent meaningful user-facing Rust
//! attributes.
//!
//! Because multiple expansion phases may depend on these attributes,
//! they cannot always be removed immediately after first use.
//!
//! Instead, cleanup is deferred until all transformations that depend on
//! them have completed.
//!
//! ## Responsibilities
//!
//! This phase performs final implementation cleanup, including:
//!
//! - removal of temporary proc-macro attributes,
//! - removal of consumed expansion metadata,
//! - elimination of internal markers no longer required after
//!   generation,
//! - and verification that no transient expansion attributes remain.

// ===============================================================================
// ``````````````````````````````````` IMPORTS ```````````````````````````````````
// ===============================================================================

// --- Local Crate ---
use crate::{LAST_INSTANCE_MACRO_NAME, Transformation, impls::errors::PostBugs};

use quote::format_ident;
// --- Proc Macro Crates ---
use syn::{ImplItem, ItemImpl, Meta};

// ===============================================================================
// `````````````````````````` COUNTERS IDENT ATTR REMOVAL ````````````````````````
// ===============================================================================

/// Conditionally removes temporary counter-identification metadata after
/// expansion.
///
/// Counter-identification attributes are consumed by expansion phases to
/// associate implementation items with particular counter dimensions.
///
/// However, not every expansion phase necessarily consumes the metadata
/// at the same point in the expansion pipeline.
///
/// Example:
///
/// ```ignore
/// #[instance_impl(1, 2, 3)]
/// #[last_instance(...)]
/// impl Example<0, 0, 2> for MyType {
///     #[counter(1)]
///     const X_IDENT: &'static [u8] = b"x";
///
///     #[counter(2)]
///     const Y_IDENT: &'static [u8] = b"y";
///
///     #[counter(3)]
///     const Z_IDENT: &'static [u8] = b"z";
/// }
/// ```
///
/// Earlier transformations may already have recovered the mapping:
///
/// ```text
/// X_IDENT -> counter 1
/// Y_IDENT -> counter 2
/// Z_IDENT -> counter 3
/// ```
///
/// but the final instance expansion may still require the same metadata
/// while producing its remaining generated items.
///
/// Consequently, this transformation first determines whether the
/// implementation still contains the final instance macro responsible
/// for consuming `#[counter(...)]`.
///
/// If such a macro is present, the attributes are intentionally
/// preserved so the remaining expansion phase can recover the required
/// counter-identification information.
///
/// Otherwise, no further transformation depends on the metadata, and
/// cleanup is delegated to [`CounterIdentAttrRemovalStrict`].
///
/// This allows every expansion phase requiring `#[counter(...)]` to
/// observe identical metadata while ensuring the attributes are
/// eventually removed before the final expanded implementation is
/// produced.
#[derive(Debug, Clone)]
pub struct CounterIdentAttrRemoval;

impl<'a> Transformation<ItemImpl> for CounterIdentAttrRemoval {
    fn raw_transform(
        &self,
        transform: &mut ItemImpl,
        context: &(),
    ) -> Result<(), proc_macro2::TokenStream> {
        // Preserve metadata while the final instance expansion still
        // requires it.
        for attr in &transform.attrs {
            if attr.path().segments.iter().any(|seg| {
                seg.ident == format_ident!("{}", LAST_INSTANCE_MACRO_NAME)
                    || seg.ident.to_string().contains("last_instance")
            }) {
                return Ok(());
            }
        }

        CounterIdentAttrRemovalStrict::checked_transform(
            &CounterIdentAttrRemovalStrict,
            transform,
            context,
        )?;

        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &ItemImpl,
        context: Option<&()>,
    ) -> Result<(), proc_macro2::TokenStream> {
        // If the final instance expansion has not yet executed, the
        // metadata is expected to remain.
        for attr in &transform.attrs {
            if attr.path().segments.iter().any(|seg| {
                seg.ident == format_ident!("{}", LAST_INSTANCE_MACRO_NAME)
                    || seg.ident.to_string().contains("last_instance")
            }) {
                return Ok(());
            }
        }

        CounterIdentAttrRemovalStrict::validate_transform(
            &CounterIdentAttrRemovalStrict,
            transform,
            context,
        )?;

        Ok(())
    }
}

/// Unconditionally removes temporary counter-identification metadata.
///
/// This transformation represents the final cleanup stage for the
/// `#[counter(...)]` attribute.
///
/// It assumes every transformation requiring the metadata has already
/// completed, including the final instance macro attached to the
/// implementation.
///
/// Example:
///
/// Before:
///
/// ```ignore
/// #[instance_impl(...)]
/// impl Example<0, 0, 2> for MyType {
///     #[counter(1)]
///     const X_IDENT: &'static [u8] = b"x";
///
///     #[counter(2)]
///     const Y_IDENT: &'static [u8] = b"y";
///
///     #[counter(3)]
///     const Z_IDENT: &'static [u8] = b"z";
/// }
/// ```
///
/// After:
///
/// ```ignore
/// impl Example<0, 0, 2> for MyType {
///     const X_IDENT: &'static [u8] = b"x";
///     const Y_IDENT: &'static [u8] = b"y";
///     const Z_IDENT: &'static [u8] = b"z";
/// }
/// ```
///
/// At this stage, the information previously carried by
/// `#[counter(...)]` has already been incorporated into the generated
/// implementation and auxiliary expansion artifacts.
///
/// The attributes therefore serve no further purpose and are removed
/// from every associated constant.
///
/// Validation then verifies that no temporary counter-identification
/// metadata survives into the final expanded output, ensuring expansion
/// leaves behind only user-visible Rust syntax.
#[derive(Debug, Clone)]
pub struct CounterIdentAttrRemovalStrict;

impl<'a> Transformation<ItemImpl> for CounterIdentAttrRemovalStrict {
    fn raw_transform(
        &self,
        transform: &mut ItemImpl,
        _: &(),
    ) -> Result<(), proc_macro2::TokenStream> {
        for item in &mut transform.items {
            let ImplItem::Const(c) = item else {
                continue;
            };

            c.attrs.retain(|attr| {
                let Meta::List(list) = &attr.meta else {
                    return true;
                };

                !list.path.is_ident("counter")
            });
        }

        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &ItemImpl,
        _: Option<&()>,
    ) -> Result<(), proc_macro2::TokenStream> {
        for item in &transform.items {
            let ImplItem::Const(c) = item else {
                continue;
            };

            if c.attrs.iter().any(|attr| {
                let Meta::List(list) = &attr.meta else {
                    return false;
                };

                list.path.is_ident("counter")
            }) {
                return Err(PostBugs::CounterAttrNotRemoved {}.into());
            }
        }

        Ok(())
    }
}

// ===============================================================================
// ``````````````````````````` SELF BOUNDS ATTR REMOVAL ``````````````````````````
// ===============================================================================

/// Conditionally removes temporary self-bound metadata after expansion.
///
/// Self-bound attributes are consumed by several transformation phases
/// to recover bounds belonging specifically to the implementing `Self`
/// type.
///
/// However, not every expansion phase necessarily consumes the metadata
/// at the same point in the expansion pipeline.
///
/// Example:
///
/// ```ignore
/// #[instance_impl(...)]
/// #[last_instance(...)]
/// #[self_bounds(R: MarkerTrait)]
/// impl<'a, T, R: MarkerTrait> Example<T> for Phantom<R> {
///     ...
/// }
/// ```
///
/// Earlier transformations may already have extracted the
/// `Self`-specific bounds from:
///
/// ```ignore
/// #[self_bounds(R: MarkerTrait)]
/// ```
///
/// but the final instance macro attached to the implementation may still
/// require the same metadata.
///
/// Consequently, this transformation first determines whether the
/// implementation still contains the final instance macro responsible
/// for consuming `self_bounds`.
///
/// If such a macro is present, the attribute is intentionally preserved
/// so the remaining expansion phase can recover the required
/// `Self`-bound information.
///
/// Otherwise, no further transformation depends on the metadata, and
/// cleanup is delegated to [`SelfBoundsAttrRemovalStrict`].
///
/// This allows every expansion phase requiring `self_bounds` to observe
/// identical metadata while ensuring the attribute is eventually removed
/// before the final expanded implementation is produced.
#[derive(Debug, Clone)]
pub struct SelfBoundsAttrRemoval;

impl<'a> Transformation<ItemImpl> for SelfBoundsAttrRemoval {
    fn raw_transform(
        &self,
        transform: &mut ItemImpl,
        context: &(),
    ) -> Result<(), proc_macro2::TokenStream> {
        for attr in &transform.attrs {
            if attr
                .path()
                .segments
                .iter()
                .any(|seg| seg.ident == format_ident!("{}", LAST_INSTANCE_MACRO_NAME))
            {
                return Ok(());
            }
        }
        SelfBoundsAttrRemovalStrict::checked_transform(
            &SelfBoundsAttrRemovalStrict,
            transform,
            context,
        )?;
        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &ItemImpl,
        context: Option<&()>,
    ) -> Result<(), proc_macro2::TokenStream> {
        for attr in &transform.attrs {
            if attr
                .path()
                .segments
                .iter()
                .any(|seg| seg.ident == format_ident!("{}", LAST_INSTANCE_MACRO_NAME))
            {
                return Ok(());
            }
        }
        SelfBoundsAttrRemovalStrict::validate_transform(
            &SelfBoundsAttrRemovalStrict,
            transform,
            context,
        )?;
        Ok(())
    }
}

/// Unconditionally removes temporary self-bound metadata.
///
/// This transformation represents the final cleanup stage for the
/// `#[self_bounds(...)]` attribute.
///
/// It assumes every transformation requiring the metadata has already
/// completed, including the final instance macro attached to the
/// implementation.
///
/// Example:
///
/// Before:
///
/// ```ignore
/// #[instance_impl(...)]
/// #[self_bounds(R: MarkerTrait)]
/// impl<'a, T, R: MarkerTrait> Example<T> for Phantom<R> {
///     ...
/// }
/// ```
///
/// After:
///
/// ```ignore
/// impl<'a, T, R: MarkerTrait> Example<T> for Phantom<R> {
///     ...
/// }
/// ```
///
/// At this stage, the information previously carried by
/// `#[self_bounds(...)]` has already been incorporated into the generated
/// implementation and auxiliary expansion artifacts.
///
/// The attribute therefore serves no further purpose and is removed from
/// the implementation.
///
/// Validation then verifies that no temporary `self_bounds` metadata
/// survives into the final expanded output, ensuring expansion leaves
/// behind only user-visible Rust syntax.
#[derive(Debug, Clone)]
pub struct SelfBoundsAttrRemovalStrict;

impl<'a> Transformation<ItemImpl> for SelfBoundsAttrRemovalStrict {
    fn raw_transform(
        &self,
        transform: &mut ItemImpl,
        _: &(),
    ) -> Result<(), proc_macro2::TokenStream> {
        transform.attrs.retain(|attr| {
            let Meta::List(list) = &attr.meta else {
                return true;
            };

            !list.path.is_ident("self_bounds")
        });
        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &ItemImpl,
        _: Option<&()>,
    ) -> Result<(), proc_macro2::TokenStream> {
        if transform.attrs.iter().any(|attr| {
            let Meta::List(list) = &attr.meta else {
                return false;
            };

            list.path.is_ident("self_bounds")
        }) {
            return Err(PostBugs::SelfBoundsAttrNotRemoved {}.into());
        }
        Ok(())
    }
}

// ===============================================================================
// `````````````````````````` LAST INSTANCE ATTR REMOVAL `````````````````````````
// ===============================================================================

/// Removes the upcoming inner attribute `#[last_instance(...)]` attribute before
/// forwarding the implementation to the last-instance expansion macro itself.
///
/// Unlike helper attributes such as `#[counter(...)]` ([`CounterIdentAttrRemoval`])
/// and `#[self_bounds(...)]` ([`SelfBoundsAttrRemoval`]), `#[last_instance(...)]`
/// is itself an attribute proc macro.
///
/// The outer expansion macro consumes the attribute as metadata to
/// determine whether last-instance processing should occur. Before the
/// modified implementation is forwarded to the inner proc macro, the
/// attribute is removed so it is not encountered again as an ordinary
/// attribute.
///
/// Example:
///
/// ```ignore
/// #[instance_impl(1, 2, 3)]
/// #[last_instance(3)]
/// impl Example<0, 0, 2> for MyType { ... }
/// ```
///
/// becomes:
///
/// ```ignore
/// impl Example<0, 0, 2> for MyType { ... }
/// ```
///
/// before the implementation is passed to the `#[last_instance]`
/// expansion pipeline.
///
/// This phase verifies that the attribute has been removed before
/// forwarding the implementation.
#[derive(Debug, Clone)]
pub struct LastInstanceAttrRemoval;

impl<'a> Transformation<ItemImpl> for LastInstanceAttrRemoval {
    fn raw_transform(
        &self,
        transform: &mut ItemImpl,
        _: &(),
    ) -> Result<(), proc_macro2::TokenStream> {
        transform.attrs.retain(|attr| {
            let Meta::List(list) = &attr.meta else {
                return true;
            };

            !list.path.is_ident(LAST_INSTANCE_MACRO_NAME)
        });
        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &ItemImpl,
        _: Option<&()>,
    ) -> Result<(), proc_macro2::TokenStream> {
        if transform.attrs.iter().any(|attr| {
            let Meta::List(list) = &attr.meta else {
                return false;
            };

            list.path.is_ident(LAST_INSTANCE_MACRO_NAME)
        }) {
            return Err(PostBugs::LastInstanceAttrNotRemoved {}.into());
        }
        Ok(())
    }
}
