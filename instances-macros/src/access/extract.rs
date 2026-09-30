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

//! Provides extraction and confirmation logic for instance-access
//! transformation.
//!
//! This module extracts structural information from validated Rust syntax
//! (see [`crate::access::valid`]) that is required by later access-transformation
//! stages. The extraction layer converts parts of an [`ItemImpl`] into smaller,
//! typed representations, such as the instance node trait bound, the instance
//! trait bound, and the classification of associated items according to
//! their usage of `Self`.
//!
//! Extraction is separated from validation and transformation. Validation
//! establishes that the input satisfies the structural requirements expected
//! by the extraction layer, while extraction identifies and removes or copies the
//! specific syntax required by the transformation process. The extracted
//! representation can subsequently be used without repeatedly traversing
//! or interpreting the original syntax.

// ===============================================================================
// ``````````````````````````````````` IMPORTS ```````````````````````````````````
// ===============================================================================

// --- Local Crate ---
use crate::access::{
    ValidBugs,
    errors::{ExtractBugs, ExtractErrors},
};

// --- Proc Macro Utils ---
use proc_macro2::TokenStream;
use syn::{
    GenericParam, Ident, ImplItem, ItemImpl, Path, TraitBound, 
    TraitBoundModifier, Type, TypeParamBound, WherePredicate, 
    visit::{self, Visit},
};

// ===============================================================================
// ```````````````````````````````````` TRAITS ```````````````````````````````````
// ===============================================================================

/// Defines extraction and consistency confirmation for a syntax representation.
pub(super) trait ExtractConfirm<T, Context = ()>: Sized {
    /// Extracts `Self` from an immutable source.
    fn extract(from: &T, via: &Context) -> Result<Self, TokenStream>;

    /// Extracts `Self` while allowing the source to be modified.
    fn extract_mut(from: &mut T, via: &Context) -> Result<Self, TokenStream>;

    /// Confirms the source is consistent with an optionally extracted value.
    fn confirm(through: Option<&Self>, from: &T, via: &Context) -> Result<(), TokenStream>;
}

// ===============================================================================
// `````````````````````````` INSTANCE NODE TRAIT BOUND ``````````````````````````
// ===============================================================================

/// Extracts the only trait bound from the available first type generic parameter.
/// Expects a pre-validation of [`crate::access::valid::ValidImplSelfTy`].
///
/// Only bounds written directly on the generic parameter are considered:
///
/// ```ignore
/// impl<T: Trait> Foo<T> { .. }
/// ```
///
/// Lifetime bounds are not permitted.
///
/// Trait bounds appearing in the impl `where` clause are not considered.
#[derive(Debug, Clone)]
pub(super) struct InstanceNodeBound(pub(super) TraitBound);

impl ExtractConfirm<ItemImpl> for InstanceNodeBound {
    fn extract_mut(_: &mut ItemImpl, _: &()) -> Result<Self, TokenStream> {
        panic!("do not attempt this function")
    }

    fn extract(from: &ItemImpl, _: &()) -> Result<Self, TokenStream> {
        let Some(first_type_generic) = &from.generics.params.iter().find_map(|param| {
            let GenericParam::Type(type_param) = param else {
                return None;
            };

            Some(type_param)
        }) else {
            return Err(ValidBugs::QSelfGenericNotDeclared {}.into());
        };

        let mut trait_bound = None;

        for bound in &first_type_generic.bounds {
            match bound {
                // Lifetime bounds are explicitly forbidden.
                //
                //     impl<T: 'a + Trait>
                //             ^^
                TypeParamBound::Lifetime(lifetime) => {
                    return Err(ExtractErrors::QSelfGenericLifetimeBoundNotAllowed {
                        segment: first_type_generic.ident.clone(),
                        lifetime: lifetime.clone(),
                    }
                    .into());
                }

                TypeParamBound::Trait(bound) => {
                    if trait_bound.is_some() {
                        return Err(ExtractErrors::QSelfGenericMultipleBounds {
                            segment: first_type_generic.ident.clone(),
                            bound: TypeParamBound::Trait(bound.clone()),
                        }
                        .into());
                    }

                    trait_bound = Some(bound.clone());
                }

                // `TypeParamBound` is non-exhaustive.
                //
                // Reject any future bound variants rather than silently
                // accepting something that is not the required trait bound.
                other => {
                    return Err(ExtractErrors::QSelfGenericMultipleBounds {
                        segment: first_type_generic.ident.clone(),
                        bound: other.clone(),
                    }
                    .into());
                }
            }
        }

        let Some(node) = trait_bound else {
            return Err(ExtractErrors::QSelfGenericTraitBoundNotFound {
                segment: first_type_generic.ident.clone(),
            }
            .into());
        };

        if node.path.segments.last().is_none() {
            return Err(ExtractErrors::QSelfGenericTraitBoundNotFound {
                segment: first_type_generic.ident.clone(),
            }
            .into());
        }

        if let Some(lt) = node.lifetimes {
            return Err(ExtractErrors::NodeBoundHrtbNotSupported { lt: lt.clone() }.into());
        }

        if let TraitBoundModifier::Maybe(_) = node.modifier {
            return Err(ExtractErrors::NodeBoundSizedBoundInvalid {
                bound: node.clone(),
            }
            .into());
        }

        Ok(Self(node))
    }

    fn confirm(through: Option<&Self>, from: &ItemImpl, _: &()) -> Result<(), TokenStream> {
        let Some(first_type_generic) = from.generics.params.iter().find_map(|param| {
            let GenericParam::Type(type_param) = param else {
                return None;
            };

            Some(type_param.clone())
        }) else {
            return Err(ValidBugs::QSelfGenericNotDeclared {}.into());
        };

        let mut found = None;

        for bound in &first_type_generic.bounds {
            match bound {
                TypeParamBound::Lifetime(_) => {
                    return Err(ExtractBugs::QSelfGenericLifetimeBoundNotAllowed {}.into());
                }

                TypeParamBound::Trait(bound) => {
                    if found.is_some() {
                        return Err(ExtractBugs::QSelfGenericMultipleBounds {}.into());
                    }

                    found = Some(bound);
                }

                _ => {
                    return Err(ExtractBugs::QSelfGenericMultipleBounds {}.into());
                }
            }
        }

        let Some(found) = found else {
            return Err(ExtractBugs::QSelfGenericTraitBoundNotFound {}.into());
        };

        if found.path.segments.last().is_none() {
            return Err(ExtractBugs::QSelfGenericTraitBoundNotFound {}.into());
        }

        if found.lifetimes.is_some() {
            return Err(ExtractBugs::NodeBoundHrtbNotSupported {}.into());
        }

        if let TraitBoundModifier::Maybe(_) = found.modifier {
            return Err(ExtractBugs::NodeBoundSizedBoundInvalid {}.into());
        }

        if let Some(through) = through {
            if found.path != through.0.path {
                return Err(ExtractBugs::InstanceNodeExtractionInconsistent {}.into());
            }
        }

        Ok(())
    }
}

// ===============================================================================
// ```````````````````````````` INSTANCE TRAIT BOUND `````````````````````````````
// ===============================================================================

/// Extracts the instance trait bound from the `Self: ...` predicate of the
/// impl's `where` clause.
///
/// The instance trait is required to be the first bound of the `Self`
/// predicate. Any additional bounds remain on the predicate and are not
/// considered part of the extracted instance trait.
///
/// For example:
///
/// ```ignore
/// impl<T> Foo<T>
/// where
///     Self: InstanceTrait<T> + SomeOtherTrait,
/// {
///     // ...
/// }
/// ```
///
/// extracts:
///
/// ```text
/// InstanceTrait<T>
/// ```
///
/// while `SomeOtherTrait` remains as an ordinary bound.
///
/// Only an unqualified `Self` type predicate is considered:
///
/// ```text
/// Self: InstanceTrait
/// ```
///
/// Qualified forms such as `<Self as Trait>: ...` are not treated as the
/// instance-trait predicate.
///
/// The instance trait must be a normal trait bound. Higher-ranked trait
/// bounds and optional (`?Trait`) bounds are not supported.
///
/// Only one `Self: ...` predicate is permitted, and the first bound in that
/// predicate must be the instance trait. If another `Self`'s bound has the same final
/// trait identifier as the extracted instance trait, it is rejected as a
/// repeated instance trait.
#[derive(Debug, Clone)]
pub(super) struct InstanceBound(pub(super) TraitBound);

impl ExtractConfirm<ItemImpl> for InstanceBound {
    fn extract(from: &ItemImpl, _: &()) -> Result<Self, TokenStream> {
        let Some(where_clause) = &from.generics.where_clause else {
            return Err(ExtractErrors::SelfTraitPredicateWhereClauseMissing {
                generics: from.generics.clone(),
            }
            .into());
        };

        // Find the `Self: ...` predicate.
        let self_predicates = where_clause
            .predicates
            .iter()
            .enumerate()
            .filter_map(|(idx, predicate)| {
                let WherePredicate::Type(predicate) = predicate else {
                    return None;
                };

                let is_self = matches!(
                    &predicate.bounded_ty,
                    Type::Path(type_path)
                        if type_path.qself.is_none()
                            && type_path.path.segments.len() == 1
                            && type_path.path.segments[0].ident == "Self"
                );

                is_self.then_some((idx, predicate))
            })
            .collect::<Vec<_>>();

        let Some(&(_, predicate)) = self_predicates.first() else {
            return Err(ExtractErrors::SelfTraitPredicateMissing {
                where_: where_clause.clone(),
            }
            .into());
        };

        // Only one `Self: ...` predicate is permitted.
        if let Some(&(_, duplicate)) = self_predicates.get(1) {
            return Err(ExtractErrors::MultipleSelfTraitPredicates {
                pred: duplicate.clone(),
            }
            .into());
        }

        // The first bound must be the instance trait.
        let Some(param_bound) = predicate.bounds.first() else {
            return Err(ExtractErrors::SelfTraitBoundUnknownFirstBound {
                pred: predicate.clone(),
            }
            .into());
        };

        let TypeParamBound::Trait(bound) = param_bound else {
            return Err(ExtractErrors::SelfTraitBoundNotTrait {
                bound: param_bound.clone(),
            }
            .into());
        };

        Ok(Self(bound.clone()))
    }

    fn extract_mut(from: &mut ItemImpl, _: &()) -> Result<Self, TokenStream> {
        let Some(where_clause) = from.generics.where_clause.as_mut() else {
            return Err(ExtractErrors::SelfTraitPredicateWhereClauseMissing {
                generics: from.generics.clone(),
            }
            .into());
        };

        // Find the `Self: ...` predicate.
        let self_predicates = where_clause
            .predicates
            .iter()
            .enumerate()
            .filter_map(|(idx, predicate)| {
                let WherePredicate::Type(predicate) = predicate else {
                    return None;
                };

                let is_self = matches!(
                    &predicate.bounded_ty,
                    Type::Path(type_path)
                        if type_path.qself.is_none()
                            && type_path.path.segments.len() == 1
                            && type_path.path.segments[0].ident == "Self"
                );

                is_self.then_some((idx, predicate))
            })
            .collect::<Vec<_>>();

        let Some(&(predicate_idx, predicate)) = self_predicates.first() else {
            return Err(ExtractErrors::SelfTraitPredicateMissing {
                where_: where_clause.clone(),
            }
            .into());
        };

        // Only one `Self: ...` predicate is permitted.
        if let Some(&(_, duplicate)) = self_predicates.get(1) {
            return Err(ExtractErrors::MultipleSelfTraitPredicates {
                pred: duplicate.clone(),
            }
            .into());
        }

        // The first bound must be the instance trait.
        let Some(param_bound) = predicate.bounds.first() else {
            return Err(ExtractErrors::SelfTraitBoundUnknownFirstBound {
                pred: predicate.clone(),
            }
            .into());
        };

        let TypeParamBound::Trait(bound) = param_bound else {
            return Err(ExtractErrors::SelfTraitBoundNotTrait {
                bound: param_bound.clone(),
            }
            .into());
        };

        let bound = bound.clone();

        // HRTB instance traits are not supported.
        if let Some(lt) = &bound.lifetimes {
            return Err(ExtractErrors::HrtbInstanceTraitNotAllowed { lt: lt.clone() }.into());
        }

        if let TraitBoundModifier::Maybe(_) = &bound.modifier {
            return Err(ExtractErrors::SizedInstanceTraitNotAllowed {
                bound: bound.clone(),
            }
            .into());
        }

        let Some(instance_segment) = &bound.path.segments.last() else {
            return Err(ExtractErrors::SelfTraitBoundNotTrait {
                bound: param_bound.clone(),
            }
            .into());
        };

        let instance_ident = &instance_segment.ident;

        // Any other trait with the same final identifier is a repeated
        // instance trait. Generic arguments and qualification are irrelevant.
        for other in predicate.bounds.iter().skip(1) {
            let TypeParamBound::Trait(other) = other else {
                continue;
            };

            let Some(other_segment) = other.path.segments.last() else {
                continue;
            };

            if other_segment.ident == *instance_ident {
                return Err(ExtractErrors::InstanceTraitRepeated {
                    bound: other.clone(),
                }
                .into());
            }
        }

        // Remove the first `Self: Trait` bound.
        let predicate = where_clause
            .predicates
            .iter_mut()
            .nth(predicate_idx)
            .expect("predicate index exists");

        let WherePredicate::Type(predicate) = predicate else {
            unreachable!();
        };

        let mut bounds = std::mem::take(&mut predicate.bounds)
            .into_iter()
            .collect::<Vec<_>>();

        bounds.remove(0);

        predicate.bounds = bounds.into_iter().collect();

        // Remove the entire predicate if the extracted trait was its only bound.
        if predicate.bounds.is_empty() {
            let mut predicates = std::mem::take(&mut where_clause.predicates)
                .into_iter()
                .collect::<Vec<_>>();

            predicates.remove(predicate_idx);

            where_clause.predicates = predicates.into_iter().collect();
        }

        Ok(Self(bound))
    }

    fn confirm(through: Option<&Self>, from: &ItemImpl, _: &()) -> Result<(), TokenStream> {
        let Some(through) = through else {
            return Ok(());
        };

        let bound = &through.0;
        let Some(bound_segment) = bound.path.segments.last() else {
            return Err(ExtractBugs::SelfTraitBoundNotTrait {}.into());
        };

        if !bound.lifetimes.is_none() {
            return Err(ExtractBugs::HrtbInstanceTraitNotAllowed {}.into());
        }

        if let TraitBoundModifier::Maybe(_) = bound.modifier {
            return Err(ExtractBugs::SizedInstanceTraitNotAllowed {}.into());
        }

        if let Some(where_clause) = from.generics.where_clause.as_ref() {
            for predicate in &where_clause.predicates {
                let WherePredicate::Type(predicate) = predicate else {
                    continue;
                };

                let is_self = matches!(
                    &predicate.bounded_ty,
                    Type::Path(type_path)
                        if type_path.qself.is_none()
                            && type_path.path.segments.len() == 1
                            && type_path.path.segments[0].ident == "Self"
                );

                if !is_self {
                    continue;
                }

                for existing in &predicate.bounds {
                    let TypeParamBound::Trait(existing) = existing else {
                        continue;
                    };

                    let Some(existing_segment) = existing.path.segments.last() else {
                        continue;
                    };

                    if existing_segment.ident == bound_segment.ident {
                        return Err(ExtractBugs::InstanceTraitNotRemoved {}.into());
                    }
                }
            }
        };

        Ok(())
    }
}

// ===============================================================================
// `````````````````````````````` SELF USAGE ASSOCS ``````````````````````````````
// ===============================================================================

/// Describes how the associated items of an impl use `Self`.
///
/// The extracted identifiers are divided into three categories:
///
/// - [`SelfUsageAssocs::all_idents`] contains every associated type, constant,
///   and function declared by the impl.
/// - [`SelfUsageAssocs::self_idents`] contains associated items whose complete
///   declaration contains at least one use of `Self`.
/// - [`SelfUsageAssocs::self_assoc_idents`] contains associated items whose
///   declaration contains a `Self::Assoc` path.
///
/// These categories are intentionally kept separate because later
/// transformation phases need different levels of information. An item that
/// merely uses `Self` may need the impl generics merged into its generated
/// standalone item, while an item that specifically uses `Self::Assoc` also
/// requires its associated-item generic arguments to be propagated into the
/// corresponding path segment.
///
/// The extraction is purely syntactic: it records the presence of `Self` and
/// `Self::Assoc` without attempting to resolve what those paths ultimately
/// refer to.
pub(super) struct SelfUsageAssocs {
    pub(super) all_idents: AssocIdents,
    pub(super) self_idents: AssocIdents,
    #[allow(dead_code)]
    pub(super) self_assoc_idents: AssocIdents,
}

/// Groups associated-item identifiers by their kind.
///
/// This type contains only identifiers; it does not retain the declarations
/// themselves or any information about how those items use `Self`.
pub(super) struct AssocIdents {
    pub(super) types: Vec<Ident>,
    pub(super) consts: Vec<Ident>,
    pub(super) fns: Vec<Ident>,
}

/// Visitor that determines whether an impl item contains any use of `Self`.
///
/// The visitor walks every path within the item and records a positive result
/// whenever any path segment has the identifier `Self`. This intentionally
/// detects `Self` regardless of where it occurs in the path, including uses
/// such as `Self`, `Self::Assoc`, and qualified paths containing `Self`.
struct SelfVisitor {
    found: bool,
}

impl<'b> Visit<'b> for SelfVisitor {
    fn visit_path(&mut self, path: &'b Path) {
        if path.segments.iter().any(|segment| segment.ident == "Self") {
            self.found = true;
        }

        visit::visit_path(self, path);
    }
}

/// Visitor that determines whether an impl item contains a `Self::Assoc` path.
///
/// Unlike [`SelfVisitor`], this visitor only records `Self` when it is the
/// first segment of a path that contains at least one following segment.
/// Thus `Self` alone is not sufficient; the path must have the form
/// `Self::Assoc` or longer.
///
/// Once such a path is found, traversal stops because only the existence of
/// one `Self::Assoc` usage is required for classification.
struct SelfAssocVisitor {
    found: bool,
}

impl<'b> Visit<'b> for SelfAssocVisitor {
    fn visit_path(&mut self, path: &'b Path) {
        if self.found {
            return;
        }

        let is_self_assoc = path.segments.len() >= 2
            && path
                .segments
                .first()
                .is_some_and(|segment| segment.ident == "Self");

        if is_self_assoc {
            self.found = true;
            return;
        }

        visit::visit_path(self, path);
    }
}

impl ExtractConfirm<ItemImpl> for SelfUsageAssocs {
    fn extract_mut(_: &mut ItemImpl, _: &()) -> Result<Self, TokenStream> {
        panic!("do not attempt this function")
    }

    fn extract(from: &ItemImpl, _: &()) -> Result<Self, TokenStream> {
        let mut const_idents = Vec::new();
        let mut type_idents = Vec::new();
        let mut fn_idents = Vec::new();

        let mut self_const_idents = Vec::new();
        let mut self_type_idents = Vec::new();
        let mut self_fn_idents = Vec::new();

        let mut self_assoc_const_idents = Vec::new();
        let mut self_assoc_type_idents = Vec::new();
        let mut self_assoc_fn_idents = Vec::new();

        for item in &from.items {
            let mut self_visitor = SelfVisitor { found: false };
            let mut self_assoc_visitor = SelfAssocVisitor { found: false };

            self_visitor.visit_impl_item(item);
            self_assoc_visitor.visit_impl_item(item);

            match item {
                ImplItem::Const(item) => {
                    let ident = &item.ident;
                    const_idents.push(ident.clone());
                    if self_visitor.found {
                        self_const_idents.push(ident.clone());
                    }
                    if self_assoc_visitor.found {
                        self_assoc_const_idents.push(ident.clone());
                    }
                }

                ImplItem::Fn(item) => {
                    let ident = &item.sig.ident;
                    fn_idents.push(ident.clone());
                    if self_visitor.found {
                        self_fn_idents.push(ident.clone());
                    }
                    if self_assoc_visitor.found {
                        self_assoc_fn_idents.push(ident.clone());
                    }
                }

                ImplItem::Type(item) => {
                    let ident = &item.ident;
                    type_idents.push(ident.clone());
                    if self_visitor.found {
                        self_type_idents.push(ident.clone());
                    }
                    if self_assoc_visitor.found {
                        self_assoc_type_idents.push(ident.clone());
                    }
                }

                _ => {}
            }
        }

        Ok(Self {
            all_idents: AssocIdents {
                types: type_idents,
                consts: const_idents,
                fns: fn_idents,
            },
            self_idents: AssocIdents {
                types: self_type_idents,
                consts: self_const_idents,
                fns: self_fn_idents,
            },
            self_assoc_idents: AssocIdents {
                types: self_assoc_type_idents,
                consts: self_assoc_const_idents,
                fns: self_assoc_fn_idents,
            },
        })
    }

    fn confirm(_: Option<&Self>, _: &ItemImpl, _: &()) -> Result<(), TokenStream> {
        Ok(())
    }
}
