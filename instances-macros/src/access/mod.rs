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
// ```````````````````````````````` INSTANCE ACCESS ``````````````````````````````
// ===============================================================================

//! Transforms an instance-bound `impl` into standalone Rust items that expose
//! the associated types, constants, and functions through the instance
//! hierarchy.
//!
//! The transformation begins with an `impl` whose `Self` type represents an
//! instance node. The associated items inside that `impl` can use `Self` to
//! refer to the instance-bound types and functions.
//!
//! For example, an instance-node associated type's inherent
//! implementation may contain:
//!
//! ```ignore
//! impl<T: NodeBound> for T::Node
//! where
//!     Self: InstanceBound,
//! {
//!     type Value = Self::Data;
//!
//!     fn get() -> Self::Value {
//!         Self::value()
//!     }
//! }
//! ```
//!
//! The access transformation does not leave these items inside the `impl`.
//! It validates the restricted instance-access syntax, extracts the instance
//! information, propagates the required generics, and replaces the `Self`
//! references with explicit instance-bound representations.
//!
//! A simple associated-type access such as:
//!
//! ```ignore
//! Self::Value
//! ```
//!
//! is transformed into an explicit instance-bound access:
//!
//! ```ignore
//! <<T as NodeBound>::Node as InstanceBound>::Value
//! ```
//!
//! For a leaf instance, where the instance is already known, the access is
//! specialized to the fixed instance representation:
//!
//! ```ignore
//! Self::Value
//! ```
//!
//! becomes conceptually:
//!
//! ```ignore
//! <<T as NodeBound>::Node as InstanceBound>::Value
//! ```
//!
//! The same transformation applies to associated-function calls. For example:
//!
//! ```ignore
//! Self::get()
//! ```
//!
//! becomes an explicit access to the instance-bound function:
//!
//! ```ignore
//! <<T as NodeBound>::Node as InstanceBound>::get()
//! ```
//!
//! The original associated items are then emitted as standalone items rather
//! than remaining inside the implementation:
//!
//! ```ignore
//! impl<T: NodeBound> for T::Node
//! where
//!     Self: InstanceBound,
//! {
//!     type Value = Self::Data;
//!
//!     fn get() -> Self::Value {
//!         Self::value()
//!     }
//! }
//! ```
//!
//! becomes conceptually:
//!
//! ```ignore
//! type Value<T> = <<T as NodeBound>::Node as InstanceBound>::Data;
//!
//! fn get<T>() -> Value<T> {
//!     <<T as NodeBound>::Node as InstanceBound>::value()
//! }
//! ```
//!
//! The exact generated representation depends on whether the access is a leaf
//! or non-leaf access ([`AccessArgs`]), but the important property is that the
//! generated items no longer depend on the surrounding `impl` for their instance context.
//!
//! ## Validation
//!
//! Before transformation, the input is checked against the syntax supported by
//! instance access. This includes the form of the impl self type, the supported
//! forms of `Self`, associated-item qualification, associated-type
//! dependencies, function receivers, constants, and predicates.
//!
//! For example, the following `Self` forms are supported:
//!
//! ```ignore
//! Self
//! Self::Item
//! Self::Item<T>
//! <Self as Trait>::Item
//! <Self::Item as Trait>::Item
//! ```
//!
//! Unsupported forms are rejected before replacement is attempted:
//!
//! ```ignore
//! Self<T>
//! <T as Self>::Item
//! <T as Trait>::Self
//! ```
//!
//! This keeps the later transformation passes working on syntax whose meaning
//! is already unambiguous within the instance-access model.
//!
//! ## Extraction
//!
//! Once the input has been validated, the instance node ([`InstanceNodeBound`])
//! and its instance bound ([`InstanceBound`]) are extracted from the implementation.
//!
//! For example, an implementation rooted at an instance type provides the
//! information required to interpret:
//!
//! ```ignore
//! Self::Item
//! ```
//!
//! as an access to an associated item belonging to that instance rather than
//! as an ordinary Rust associated-item lookup.
//!
//! The extracted information is then used by the subsequent replacement
//! passes, so those passes do not need to rediscover the instance hierarchy
//! from the original syntax.
//!
//! ## Generic propagation
//!
//! Because associated items are eventually emitted as standalone items, generic
//! parameters that originally belonged to the `impl` must be made available to
//! those generated items.
//!
//! ## Leaf access
//!
//! Leaf access is used when the requested instance is already known.
//! Consequently, the transformation can directly specialize the instance
//! representation.
//!
//! For example:
//!
//! ```ignore
//! Self::Value
//! ```
//!
//! can become:
//!
//! ```ignore
//! <T::Node as InstanceBound<Concrete>>::Value
//! ```
//!
//! and:
//!
//! ```ignore
//! Self::get()
//! ```
//!
//! can become:
//!
//! ```ignore
//! <T::Node as InstanceBound<Concrete>>::get()
//! ```
//!
//! No runtime instance-position lookup is required because the instance is
//! fixed by the generated type representation.
//!
//! ## Non-leaf access
//!
//! Non-leaf access is different because the requested instance is selected at
//! runtime. Rust does not provide dynamic selection of a trait instance in the
//! form required by this transformation, so non-leaf access provides this
//! ability by generating a statically known instance hierarchy and performing
//! the selection over that hierarchy at runtime.
//!
//! The generated function therefore receives additional byte-string arguments
//! containing the dynamic identifiers used to select the instance.
//!
//! An access such as:
//!
//! ```ignore
//! Self::get(id)
//! ```
//!
//! represents a runtime instance selection. The transformation starts from a
//! known initial instance and optimistically derives the reachable trait
//! instances as a range of statically generated positions. The runtime
//! identifier is then used to select the corresponding position within that
//! generated range.
//!
//! The hierarchy and all possible positions are generated statically. Runtime
//! execution only determines which of those already-generated positions is
//! selected.
//!
//! Conceptually, a single dynamic identifier is resolved as:
//!
//! ```ignore
//! match check(id) {
//!     Some(index) => accessor(index),
//!     None => Error::OutOfRange,
//! }
//! ```
//!
//! When multiple dynamic identifiers are involved, each identifier selects a
//! position at its corresponding level of the generated hierarchy:
//!
//! ```ignore
//! match check(id1) {
//!     Some(index1) => match check_next(index1, id2) {
//!         Some(index2) => accessor(index1, index2),
//!         None => Error::OutOfRange,
//!     },
//!     None => Error::OutOfRange,
//! }
//! ```
//!
//! The additional runtime cost is therefore minimized to the generated
//! selection matches. Each dynamic identifier incurs one such selection,
//! which the compiler can optimize into a jump table where appropriate.
//! There is no runtime traversal of the trait hierarchy itself; the hierarchy
//! has already been derived and materialized during code generation.
//!
//! The runtime cost is additionally O(D), where D is the number of dynamic identifiers.
//! Each dynamic identifier contributes one generated selection, typically
//! compiled as a jump table, so the runtime performs one positional dispatch
//! per dynamic identifier.
//!
//! `Error::OutOfRange` represents an identifier that cannot be resolved at the
//! required position. `Error::Exhausted` represents a resolved numeric position
//! for which no generated accessor exists, even though the identifier itself
//! exists.
//!
//! Associated-type paths in non-leaf access do not themselves contain these
//! runtime identifiers. For example:
//!
//! ```ignore
//! Self::Assoc
//! ```
//!
//! is transformed into the terminal representation:
//!
//! ```ignore
//! <T::Node as InstanceBound<TerminalAccess>>::Assoc
//! ```
//!
//! The dynamic identifiers are introduced when the enclosing function call is
//! transformed.
//!
//! ## Standalone `Self` references
//!
//! `Self` references that refer to the implementation itself are also replaced
//! because the generated items no longer have an enclosing `impl` providing
//! `Self`.
//!
//! For example:
//!
//! ```ignore
//! fn make() -> Self {
//!     ...
//! }
//! ```
//!
//! must become a standalone function whose return type explicitly identifies
//! the original implementation self type.
//!
//! ## Documentation representation
//!
//! The transformation also produces a documentation representation of the
//! generated items. The implementation is transformed separately for
//! `cfg(doc)` and `cfg(not(doc))` so that documentation can expose the
//! instance-access structure without requiring the runtime machinery used by
//! non-leaf access.
//!
//! The executable representation therefore contains the runtime resolution
//! required for dynamic instance access, while the documentation
//! representation preserves the corresponding generated API structure.
//!
//! ## Final result
//!
//! The complete transformation can therefore be viewed from the user's
//! perspective as taking an instance-bound implementation such as:
//!
//! ```ignore
//! impl<T: NodeBound> T::Node {
//!     type Value = Self::Data;
//!
//!     fn get(id: &[u8]) -> Self::Value {
//!         Self::value()
//!     }
//! }
//! ```
//!
//! and producing standalone generated items whose instance accesses are
//! explicit:
//!
//! ```ignore
//! type Value<T> =
//!     <<T as NodeBound>::Node as InstanceBound<...>>::Data;
//!
//! fn get<T>(id: &[u8]) -> Value<T> {
//!     /* generated instance resolution */
//! }
//! ```
//!
//! The implementation is no longer required after lowering. Validation
//! establishes that the source can be represented by the instance-access
//! model, extraction identifies the instance information, replacement lowers
//! the `Self`-based representation into explicit generated syntax, and final
//! confirmation verifies that the resulting standalone items satisfy the
//! invariants of the generated representation.

// ===============================================================================
// ``````````````````````````````````` MODULES ```````````````````````````````````
// ===============================================================================

pub(crate) mod args;
pub(crate) mod direct;
pub(crate) mod errors;
mod extract;
pub(crate) mod getter;
mod leaf;
mod nonleaf;
mod replace;
mod valid;
mod wrappers;
pub(crate) use crate::access::wrappers::impls_to_items;

// ===============================================================================
// ``````````````````````````````````` IMPORTS ```````````````````````````````````
// ===============================================================================

// --- Proc Macro Crates ---
use syn::{
    File, Ident, Item, ItemImpl,
    spanned::Spanned,
    visit::{self, Visit},
};

// --- Local Crate ---
use crate::{
    Extraction, Transformation,
    access::{
        args::AccessArgs,
        errors::{KeysError, ValidBugs, ValidErrors},
        extract::*,
        replace::{
            AppendDynFnArg, MergeImplGenericsToAssocs, MergeImplTyGenericsToSegments,
            ReplaceConfirm, ReplaceDocSelfAssocCalls, ReplaceDocSelfAssocs,
            ReplaceLeafSelfAssocCalls, ReplaceLeafSelfAssocs, ReplaceLeafSumTypeSelfAssocs,
            ReplaceNonLeafSelfAssocCalls, ReplaceNonLeafSelfAssocs, ReplaceReturnTyInferedError,
            ReplaceStandAloneSelf, StripSelfKeyOfAssocs, SumAttrRemoval,
        },
        valid::*,
        wrappers::append_cfg,
    },
};

// ===============================================================================
// ````````````````````````` INSTANCE ACCESS ENTRY-POINT `````````````````````````
// ===============================================================================

/// Performs the complete instance-access transformation on a Rust file.
///
/// The entry point locates the instance-bound `impl`, clones it into separate
/// documentation and executable representations, and runs the corresponding
/// transformation pipeline on each representation. The transformed
/// implementation items are then converted into standalone functions,
/// constants, and types and assigned their respective `cfg` attributes.
///
/// The executable representation uses [`InstanceAccessImpl`] and therefore
/// includes the runtime machinery required for non-leaf instance selection.
/// The documentation representation uses [`InstanceAccessImplDoc`] and
/// preserves the documentation-facing form without that runtime machinery.
///
/// For example, an input containing:
///
/// ```ignore
/// impl<T: NodeBound> T::Node {
///     type Value = Self::Data;
///
///     fn get(id: &[u8]) -> Self::Value {
///         Self::value()
///     }
/// }
/// ```
///
/// is lowered into standalone items conceptually represented as:
///
/// ```ignore
/// #[cfg(doc)]
/// type Value<T> = ...;
///
/// #[cfg(doc)]
/// fn get<T>(id: &[u8]) -> ... {
///     ...
/// }
///
/// #[cfg(not(doc))]
/// type Value<T> = ...;
///
/// #[cfg(not(doc))]
/// fn get<T>(id: &[u8]) -> ... {
///     ...
/// }
/// ```
///
/// The original `impl` is removed from the file after its supported associated
/// items have been transformed. The resulting file therefore contains only
/// the generated standalone representation, with matching documentation and
/// non-documentation items.
///
/// The final confirmation verifies that the transformation produced only the
/// supported standalone item kinds, that every documentation item has a
/// corresponding non-documentation item and vice versa, that each item has
/// exactly one of the expected `cfg(doc)` or `cfg(not(doc))` attributes, and
/// that no `impl` or unresolved `Self` remains.
#[derive(Debug, Clone)]
pub(crate) struct InstanceAccess;

impl<'a> Transformation<File, AccessArgs> for InstanceAccess {
    fn raw_transform(
        &self,
        transform: &mut File,
        context: &AccessArgs,
    ) -> Result<(), proc_macro2::TokenStream> {
        let Some(impl_) = transform.items.iter().find_map(|item| {
            if let Item::Impl(impl_) = item {
                return Some(impl_);
            }
            None
        }) else {
            return Err(ValidBugs::ImplNotProvidedInFile {}.into());
        };

        let mut doc_impl = impl_.clone();
        let mut not_doc_impl = impl_.clone();

        InstanceAccessImpl::checked_transform(&InstanceAccessImpl, &mut not_doc_impl, context)?;
        let mut not_doc_items = impls_to_items(not_doc_impl.items);
        append_cfg(&mut not_doc_items, false);

        InstanceAccessImplDoc::checked_transform(&InstanceAccessImplDoc, &mut doc_impl, context)?;
        let mut doc_items = impls_to_items(doc_impl.items);
        append_cfg(&mut doc_items, true);

        *transform = File {
            shebang: None,
            attrs: Default::default(),
            items: Default::default(),
        };
        transform.items.extend(doc_items);
        transform.items.extend(not_doc_items);

        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &File,
        _: Option<&AccessArgs>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let mut doc_items = Vec::new();
        let mut not_doc_items = Vec::new();

        if transform.items.is_empty() {
            return Err(ValidBugs::ItemNotProvidedInFile {}.into());
        }

        for item in &transform.items {
            let (ident, attrs) = match item {
                Item::Fn(item) => (&item.sig.ident, &item.attrs),
                Item::Const(item) => (&item.ident, &item.attrs),
                Item::Type(item) => (&item.ident, &item.attrs),

                Item::Impl(_) => {
                    return Err(ValidBugs::ImplProvidedInFile {}.into());
                }

                _ => {
                    return Err(ValidBugs::InvalidItemCfg {}.into());
                }
            };

            let mut is_doc = false;
            let mut is_not_doc = false;

            for attr in attrs {
                if !attr.path().is_ident("cfg") {
                    continue;
                }

                let _ = attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("doc") {
                        is_doc = true;
                    } else if meta.path.is_ident("not") {
                        meta.parse_nested_meta(|nested| {
                            if nested.path.is_ident("doc") {
                                is_not_doc = true;
                            }

                            Ok(())
                        })?;
                    }

                    Ok(())
                });
            }

            match (is_doc, is_not_doc) {
                (true, false) => doc_items.push(ident),
                (false, true) => not_doc_items.push(ident),

                (false, false) | (true, true) => {
                    return Err(ValidBugs::InvalidItemCfg {}.into());
                }
            }
        }

        for ident in &doc_items {
            if !not_doc_items.iter().any(|other| other == ident) {
                return Err(ValidBugs::NotDocItemNotProvided {}.into());
            }
        }

        for ident in &not_doc_items {
            if !doc_items.iter().any(|other| other == ident) {
                return Err(ValidBugs::DocItemNotProvided {}.into());
            }
        }

        struct Visitor {
            found: bool,
        }

        impl Visit<'_> for Visitor {
            fn visit_ident(&mut self, ident: &Ident) {
                if ident == "Self" {
                    self.found = true;
                }

                visit::visit_ident(self, ident);
            }
        }

        let mut visit = Visitor { found: false };
        visit.visit_file(transform);

        if visit.found {
            return Err(ValidBugs::SelfFoundAfterTransformation {}.into());
        }

        Ok(())
    }
}

// ===============================================================================
// ```````````````````````````` INSTANCE ACCESS IMPL `````````````````````````````
// ===============================================================================

/// Performs the complete executable instance-access transformation for an
/// implementation.
///
/// The transformation first validates the implementation's basic structure,
/// extracts the instance node and bound, and performs the validations that
/// depend on the extracted instance information. It then prepares the
/// implementation for standalone lowering by propagating impl generics and
/// normalizing instance-bound paths.
///
/// The final replacement depends on the requested [`AccessArgs`]. Leaf access
/// replaces instance-bound associated items with their fixed representations,
/// while non-leaf access introduces the dynamic arguments and generated
/// runtime instance selection required to resolve the requested instance.
/// Finalization removes transformation-only attributes from the resulting
/// implementation.
///
/// Each stage performs its own confirmation so that the complete pipeline
/// preserves the invariants required by the following stage and produces a
/// representation suitable for conversion into standalone items.
#[derive(Debug, Clone)]
pub(crate) struct InstanceAccessImpl;

impl<'a> Transformation<ItemImpl, AccessArgs> for InstanceAccessImpl {
    fn raw_transform(
        &self,
        transform: &mut ItemImpl,
        context: &AccessArgs,
    ) -> Result<(), proc_macro2::TokenStream> {
        InstanceAccessPreValidation::checked_transform(
            &InstanceAccessPreValidation,
            transform,
            &(),
        )?;
        let _ = InstanceNodeExtraction::checked_extract(transform, &())?;
        let mut inst = InstanceBound::extract_mut(transform, &())?;
        InstanceAccessPostValidation::checked_transform(
            &InstanceAccessPostValidation,
            transform,
            &inst,
        )?;
        InstanceAccessPreReplacement::checked_transform(
            &InstanceAccessPreReplacement,
            transform,
            &inst,
        )?;
        InstanceAccessPostReplacement::checked_transform(
            &InstanceAccessPostReplacement,
            &mut (transform, &mut inst),
            context,
        )?;
        InstanceAccessFinalize::checked_transform(&InstanceAccessFinalize, transform, context)?;
        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &ItemImpl,
        context: Option<&AccessArgs>,
    ) -> Result<(), proc_macro2::TokenStream> {
        InstanceAccessPreValidation::validate_transform(
            &InstanceAccessPreValidation,
            transform,
            None,
        )?;
        InstanceBound::confirm(None, transform, &())?;
        InstanceAccessFinalize::validate_transform(&InstanceAccessFinalize, transform, context)?;
        Ok(())
    }
}

// ===============================================================================
// `````````````````````````` INSTANCE ACCESS IMPL DOC ```````````````````````````
// ===============================================================================

/// Performs the complete documentation instance-access transformation for an
/// implementation.
///
/// The transformation follows the same validation, instance extraction, and
/// preparation stages as the executable representation, but uses the
/// documentation-specific replacement instead of generating runtime
/// non-leaf instance selection. This allows the documentation representation
/// to expose the generated instance-bound API without carrying the executable
/// resolution machinery.
///
/// After validation and preparation, standalone `Self` references and
/// instance-bound associated paths and function calls are replaced with their
/// documentation representations. The transformed implementation is then
/// finalized so that it can be converted into the standalone items used by the
/// documentation build.
///
/// Each stage confirms its resulting invariants, ensuring that the
/// documentation representation remains structurally consistent with the
/// executable transformation while using its separate documentation-facing
/// replacement rules.
#[derive(Debug, Clone)]
pub(crate) struct InstanceAccessImplDoc;

impl<'a> Transformation<ItemImpl, AccessArgs> for InstanceAccessImplDoc {
    fn raw_transform(
        &self,
        transform: &mut ItemImpl,
        context: &AccessArgs,
    ) -> Result<(), proc_macro2::TokenStream> {
        InstanceAccessPreValidation::checked_transform(
            &InstanceAccessPreValidation,
            transform,
            &(),
        )?;
        let _ = InstanceNodeExtraction::checked_extract(transform, &())?;
        let mut inst = InstanceBound::extract_mut(transform, &())?;
        InstanceAccessPostValidation::checked_transform(
            &InstanceAccessPostValidation,
            transform,
            &inst,
        )?;
        InstanceAccessPreReplacement::checked_transform(
            &InstanceAccessPreReplacement,
            transform,
            &inst,
        )?;
        InstanceAccessDocReplacement::checked_transform(
            &InstanceAccessDocReplacement,
            &mut (transform, &mut inst),
            context,
        )?;
        InstanceAccessFinalize::checked_transform(&InstanceAccessFinalize, transform, context)?;

        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &ItemImpl,
        context: Option<&AccessArgs>,
    ) -> Result<(), proc_macro2::TokenStream> {
        InstanceAccessPreValidation::validate_transform(
            &InstanceAccessPreValidation,
            transform,
            None,
        )?;
        InstanceBound::confirm(None, transform, &())?;
        InstanceAccessFinalize::validate_transform(&InstanceAccessFinalize, transform, context)?;
        Ok(())
    }
}

// ===============================================================================
// ```````````````````````` INSTANCE ACCESS PRE-VALIDATION ```````````````````````
// ===============================================================================

/// Performs the initial validation of the implementation before any
/// instance-specific information is extracted.
///
/// The implementation is required to be an inherent `impl` with a supported
/// self type, and all `Self`-based key paths must use the syntax accepted by
/// instance access. These checks establish that the implementation has the
/// basic structure required for instance-node extraction and the subsequent
/// transformation stages.
///
/// Confirmation repeats the same structural checks on the representation
/// produced by this stage.
#[derive(Debug, Clone)]
pub(crate) struct InstanceAccessPreValidation;

impl<'a> Transformation<ItemImpl> for InstanceAccessPreValidation {
    fn raw_transform(
        &self,
        transform: &mut ItemImpl,
        context: &(),
    ) -> Result<(), proc_macro2::TokenStream> {
        let what = transform;
        let via = context;
        ValidInherentImpl::validate(what, via)?;
        ValidImplSelfTy::validate(what, via)?;
        ValidSelfKeyPaths::validate(what, via)?;
        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &ItemImpl,
        _: Option<&()>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let what = transform;
        let via = &();
        ValidInherentImpl::confirm(what, via)?;
        ValidImplSelfTy::confirm(what, via)?;
        ValidSelfKeyPaths::confirm(what, via)?;
        Ok(())
    }
}

// ===============================================================================
// ``````````````````````````` INSTANCE NODE EXTRACTION ``````````````````````````
// ===============================================================================

/// Extracts the instance node and bound from the implementation and validates
/// the structural information required by the subsequent instance-access
/// transformations.
///
/// The extracted node is checked against the expected implementation
/// generics, their required ordering, the qualified associated type form of
/// the implementation self type, and the requirement that the implementation
/// self type is represented as `Self`.
///
/// Confirmation verifies both the extracted [`InstanceNodeBound`] and all of
/// the structural invariants established during extraction.
#[derive(Debug, Clone)]
pub(crate) struct InstanceNodeExtraction(InstanceNodeBound);

impl Extraction<ItemImpl> for InstanceNodeExtraction {
    fn raw_extract(from: &ItemImpl, context: &()) -> Result<Self, proc_macro2::TokenStream> {
        let node = InstanceNodeBound::extract(from, context)?;
        ValidExpectedImplGenerics::validate(from, &node)?;
        ValidOrderedImplGenerics::validate(from, &node)?;
        ValidQAssocImplSelfTy::validate(from, &node)?;
        ForceSelfTyAsSelf::validate(from, &node)?;
        RestrictInnerSelfItems::validate(from, &())?;
        Ok(Self(node))
    }

    fn validate_extract(
        &self,
        from: &ItemImpl,
        _: Option<&()>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let node = &self.0;
        InstanceNodeBound::confirm(Some(node), from, &())?;
        ValidExpectedImplGenerics::confirm(from, node)?;
        ValidOrderedImplGenerics::confirm(from, node)?;
        ValidQAssocImplSelfTy::confirm(from, node)?;
        ForceSelfTyAsSelf::confirm(from, node)?;
        RestrictInnerSelfItems::confirm(from, &())?;
        Ok(())
    }
}

// ===============================================================================
// ``````````````````````` INSTANCE ACCESS POST-VALIDATION ```````````````````````
// ===============================================================================

type NewMutateSpace<'a> = (&'a mut ItemImpl, &'a mut InstanceBound);

/// Performs the validation that depends on the instance-bound information
/// extracted from the implementation.
///
/// This stage verifies that the implementation uses the instance trait only
/// in the locations supported by instance access, that required `Self`
/// references use qualified paths, and that associated types do not form
/// unsupported cross-references. Associated-function arguments are checked
/// for valid instance-access syntax, and `Self` usage in associated constants
/// is restricted because those constants are lowered into standalone items.
///
/// The validation is performed after instance extraction so that restrictions
/// involving the extracted instance bound can be checked directly. The
/// confirmation pass repeats the same checks against the resulting
/// representation to ensure that the post-validation invariants remain
/// satisfied.
#[derive(Debug, Clone)]
pub(crate) struct InstanceAccessPostValidation;

impl Transformation<ItemImpl, InstanceBound> for InstanceAccessPostValidation {
    fn raw_transform(
        &self,
        transform: &mut ItemImpl,
        context: &InstanceBound,
    ) -> Result<(), proc_macro2::TokenStream> {
        RequireSelfQPaths::validate(&transform, &())?;
        RestrictAssocTypeCrossReferences::validate(&transform, &())?;
        ValidAssocFnArgs::validate(&transform, &())?;
        RestrictSelfInCostAssocs::validate(&transform, &())?;
        RestrictInstTraitInImpl::validate(&transform, &context)?;
        RestrictSelfAssocsInInstBound::validate(context, &())?;
        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &ItemImpl,
        context: Option<&InstanceBound>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let what = transform;
        RequireSelfQPaths::confirm(what, &())?;
        RestrictAssocTypeCrossReferences::confirm(what, &())?;
        ValidAssocFnArgs::confirm(what, &())?;
        RestrictSelfInCostAssocs::confirm(what, &())?;
        if let Some(context) = context {
            RestrictInstTraitInImpl::confirm(what, context)?;
            RestrictSelfAssocsInInstBound::confirm(context, &())?;
        }
        Ok(())
    }
}

// ===============================================================================
// ``````````````````````` INSTANCE ACCESS PRE-REPLACEMENT ```````````````````````
// ===============================================================================

/// Prepares the instance-bound implementation for the final instance-access
/// replacement.
///
/// Associated items that use `Self` are first identified so that the impl's
/// generic parameters can be merged into those standalone items, and the
/// corresponding impl type arguments can be propagated into their
/// `Self::Assoc` path segments. The `Self` key is then removed from associated
/// items (of impl) that are being lowered out of the implementation, and their
/// predicates are checked against the restrictions required by the generated
/// representation.
///
/// Confirmation repeats these checks on the transformed representation to
/// ensure that generic parameters and type arguments were propagated
/// correctly, `Self` keys were removed where required, and associated-item
/// predicates satisfy the expected restrictions.
#[derive(Debug, Clone)]
pub(crate) struct InstanceAccessPreReplacement;

impl<'a> Transformation<ItemImpl, InstanceBound> for InstanceAccessPreReplacement {
    fn raw_transform(
        &self,
        transform: &mut ItemImpl,
        _: &InstanceBound,
    ) -> Result<(), proc_macro2::TokenStream> {
        let idents = SelfUsageAssocs::extract(&transform, &())?;
        MergeImplGenericsToAssocs::replace(transform, &idents)?;
        MergeImplTyGenericsToSegments::replace(transform, &idents)?;
        StripSelfKeyOfAssocs::replace(transform, &idents)?;
        RestrictSelfAssocPreds::validate(&transform, &())?;
        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &ItemImpl,
        _: Option<&InstanceBound>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let what = transform;
        let idents = SelfUsageAssocs::extract(&what, &())?;
        MergeImplGenericsToAssocs::confirm(what, &idents)?;
        MergeImplTyGenericsToSegments::confirm(what, &idents)?;
        StripSelfKeyOfAssocs::confirm(what, &idents)?;
        RestrictSelfAssocPreds::confirm(what, &())?;
        Ok(())
    }
}

// ===============================================================================
// `````````````````````` INSTANCE ACCESS POST-REPLACEMENT ```````````````````````
// ===============================================================================

/// Performs the final instance-access replacement after the implementation has
/// been validated, extracted, and prepared for lowering into standalone items.
///
/// Standalone `Self` references are first replaced with the concrete
/// implementation self type. Leaf access then replaces associated types,
/// `#[sum]` associated types, and associated-function calls using the fixed
/// instance representation. Non-leaf access makes inferred return error types
/// explicit, appends the dynamic byte-string arguments required for runtime
/// instance selection, and replaces associated types and function calls with
/// their generated non-leaf representations.
///
/// Unknown access arguments are rejected because the access keys required for
/// replacement are unavailable.
///
/// The transformation is followed by confirmation of every replacement,
/// ensuring that standalone `Self` references, associated paths, associated
/// calls, dynamic arguments, and inferred error types have all been lowered
/// into their required final representation.
#[derive(Debug, Clone)]
pub(crate) struct InstanceAccessPostReplacement;

impl<'a> Transformation<NewMutateSpace<'a>, AccessArgs> for InstanceAccessPostReplacement {
    fn raw_transform(
        &self,
        transform: &mut NewMutateSpace,
        context: &AccessArgs,
    ) -> Result<(), proc_macro2::TokenStream> {
        let args = context;
        let (transform, bound) = transform;

        ReplaceStandAloneSelf::replace(*bound, &transform)?;
        ReplaceStandAloneSelf::replace(*transform, &())?;

        match args {
            AccessArgs::Leaf(leaf) => {
                ReplaceLeafSumTypeSelfAssocs::replace(transform, &(bound, leaf))?;
                ReplaceLeafSelfAssocs::replace(transform, &(bound, leaf))?;
                ReplaceLeafSelfAssocCalls::replace(transform, &(bound, leaf))?;
            }
            AccessArgs::Unknown(token) => {
                return Err(KeysError::ExpectedKeys {
                    span: token.tokens.span(),
                }
                .into());
            }
            other => {
                ReplaceReturnTyInferedError::replace(transform, &())?;
                AppendDynFnArg::replace(transform, other)?;
                ReplaceNonLeafSelfAssocs::replace(transform, &(bound, other))?;
                ReplaceNonLeafSelfAssocCalls::replace(transform, &(bound, other))?;
            }
        }

        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &NewMutateSpace<'a>,
        context: Option<&AccessArgs>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let Some(args) = context else {
            return Ok(());
        };
        let (transform, bound) = transform;

        ReplaceStandAloneSelf::confirm(&**bound, &**transform)?;
        ReplaceStandAloneSelf::confirm(&**transform, &())?;

        match args {
            AccessArgs::Leaf(leaf) => {
                ReplaceLeafSumTypeSelfAssocs::confirm(transform, &(bound, leaf))?;
                ReplaceLeafSelfAssocs::confirm(transform, &(bound, leaf))?;
                ReplaceLeafSelfAssocCalls::confirm(transform, &(bound, leaf))?;
            }
            AccessArgs::Unknown(token) => {
                return Err(KeysError::ExpectedKeys {
                    span: token.tokens.span(),
                }
                .into());
            }
            other => {
                ReplaceReturnTyInferedError::confirm(transform, &())?;
                AppendDynFnArg::confirm(transform, other)?;
                ReplaceNonLeafSelfAssocs::confirm(transform, &(bound, other))?;
                ReplaceNonLeafSelfAssocCalls::confirm(transform, &(bound, other))?;
            }
        }

        Ok(())
    }
}

// ===============================================================================
// ``````````````````````` INSTANCE ACCESS DOC-REPLACEMENT ```````````````````````
// ===============================================================================

/// Performs instance-access replacement for the documentation representation.
///
/// The documentation representation does not generate the runtime machinery
/// required by non-leaf access. Standalone `Self` references are replaced with
/// the concrete implementation self type, instance-bound associated paths and
/// function calls are replaced with their documentation representations, and
/// inferred return error types are made explicit when required by non-leaf
/// access. Unknown access arguments are rejected because the required access
/// keys are unavailable.
///
/// The transformation is confirmed by verifying that `Self` keyword usages on
/// standalone, documentation associated paths, documentation associated calls, and any
/// required inferred error types have all been fully replaced.
#[derive(Debug, Clone)]
pub(crate) struct InstanceAccessDocReplacement;

impl<'a> Transformation<NewMutateSpace<'a>, AccessArgs> for InstanceAccessDocReplacement {
    fn raw_transform(
        &self,
        transform: &mut NewMutateSpace,
        context: &AccessArgs,
    ) -> Result<(), proc_macro2::TokenStream> {
        let args = context;
        let (transform, bound) = transform;

        ReplaceStandAloneSelf::replace(*bound, &transform)?;
        ReplaceStandAloneSelf::replace(*transform, &())?;

        match args {
            AccessArgs::Leaf(_) => {}
            AccessArgs::Unknown(token) => {
                return Err(KeysError::ExpectedKeys {
                    span: token.tokens.span(),
                }
                .into());
            }
            _ => {
                ReplaceReturnTyInferedError::replace(transform, &())?;
            }
        }

        ReplaceDocSelfAssocs::replace(transform, bound)?;
        ReplaceDocSelfAssocCalls::replace(transform, bound)?;

        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &NewMutateSpace<'a>,
        context: Option<&AccessArgs>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let Some(args) = context else {
            return Ok(());
        };
        let (transform, bound) = transform;

        ReplaceStandAloneSelf::confirm(&**bound, &**transform)?;
        ReplaceStandAloneSelf::confirm(&**transform, &())?;
        match args {
            AccessArgs::Leaf(_) => {}
            AccessArgs::Unknown(token) => {
                return Err(KeysError::ExpectedKeys {
                    span: token.tokens.span(),
                }
                .into());
            }
            _ => {
                ReplaceReturnTyInferedError::confirm(transform, &())?;
            }
        }

        ReplaceDocSelfAssocs::confirm(transform, bound)?;
        ReplaceDocSelfAssocCalls::confirm(transform, bound)?;

        Ok(())
    }
}

// ===============================================================================
// ```````````````````````` INSTANCE ACCESS FINALIZATION `````````````````````````
// ===============================================================================

/// Finalizes the transformed implementation by removing transformation-only
/// attributes that are no longer required in the generated representation.
#[derive(Debug, Clone)]
pub(crate) struct InstanceAccessFinalize;

impl<'a> Transformation<ItemImpl, AccessArgs> for InstanceAccessFinalize {
    fn raw_transform(
        &self,
        transform: &mut ItemImpl,
        args: &AccessArgs,
    ) -> Result<(), proc_macro2::TokenStream> {
        SumAttrRemoval::replace(transform, args)?;
        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &ItemImpl,
        args: Option<&AccessArgs>,
    ) -> Result<(), proc_macro2::TokenStream> {
        if let Some(context) = args {
            SumAttrRemoval::confirm(transform, context)?;
        }
        Ok(())
    }
}
