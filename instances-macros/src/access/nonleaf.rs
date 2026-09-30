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
// `````````````````````````` INSTANCE NON-LEAF ACCESS ```````````````````````````
// ===============================================================================

//! Resolves non-leaf instance access whose dynamic arguments are supplied at
//! runtime as additional byte-string function arguments.
//!
//! Non-leaf access differs from leaf access ([`crate::access::leaf`]) because
//! the requested instance is not known as a fixed type replacement. The dynamic
//! identifiers supplied to the access determine which instance position is selected
//! at runtime, so non-leaf access introduces additional byte-string arguments to the
//! generated function.
//!
//! For an [`ExprPath`], however, the dynamic identifiers are not part of the
//! expression itself. The path is transformed into the terminal type
//! representation ([`crate::node::access::TerminalAccess`]) of the non-leaf instance,
//! in the same general manner that leaf access performs its fixed type replacement:
//!
//! ```text
//! Self::Assoc
//!
//! <InitialInstance as InstanceBound>::Assoc
//!
//! <TerminalInstance as InstanceBound<TerminalAccess>>::Assoc
//! ```
//!
//! The difference is that non-leaf access replaces the instance with its
//! terminal representation rather than directly selecting one fixed instance.
//! The runtime dynamic identifiers are resolved when an associated function is
//! invoked.
//!
//! ## Expression calls
//!
//! Non-leaf function access is supported only through an [`ExprCall`]. A bare
//! function path such as:
//!
//! ```text
//! Self::get
//! ```
//!
//! is not a valid non-leaf access form. The function must be invoked:
//!
//! ```text
//! Self::get()
//! Self::get(x, id1)
//! Self::get(x, id1, id2)
//! ```
//!
//! This distinction is important because the call provides the additional
//! runtime arguments through which the dynamic instance positions are
//! resolved. Once `Self::get(...)` is taken as an [`ExprCall`], the
//! transformation is no longer limited to replacing the type represented by
//! the path. The call becomes the entry point for resolving the generated
//! instance hierarchy.
//!
//! ## Dynamic instance resolution
//!
//! Resolution begins from the [`crate::node::state::INITIAL_NODE`] and derives
//! the instance states that can be reached for the supplied dynamic identifiers. The
//! resolver derives the corresponding counters, borders, affiliate instances,
//! and accessor expressions required at each reachable depth ([`DerivedItems`]).
//!
//! The dynamic identifiers are converted into checker expressions which search
//! the appropriate final counter collections. A successful lookup produces the
//! numeric position of the identifier:
//!
//! ```text
//! Some(index)
//! ```
//!
//! An identifier that does not exist produces:
//!
//! ```text
//! None
//! ```
//!
//! These results are combined with the generated accessors through nested
//! `match` expressions. Each successful match selects the accessor belonging
//! to the resolved position and, when another dynamic dimension remains,
//! continues resolution at the next level.
//!
//! Conceptually, for two dynamic identifiers, the generated expression has the
//! following form:
//!
//! ```text
//! match check(id1) {
//!     Some(i1) => match check_next(i1, id2) {
//!         Some(i2) => accessor(i1, i2),
//!         None => Error::OutOfRange,
//!     },
//!     None => Error::OutOfRange,
//! }
//! ```
//!
//! `Error::OutOfRange` represents a dynamic identifier that cannot be found in
//! the corresponding counter collection. `Error::Exhausted` represents a
//! numeric position for which no generated accessor exists but its evident that
//! the dynamic identifier is available.
//!
//! The transformation therefore derives the runtime selection expression for
//! every reachable depth rather than attempting to resolve the dynamic
//! identifiers statically. The generated match clauses encode the possible
//! counter positions, while the generated accessor expressions represent the
//! instance state associated with each position.
//!
//! ## Derived hierarchy state
//!
//! [`ExprCallEssentials`] collects the information required to construct this
//! resolution, including the generated node representations, initial and final
//! instances, counter indexes, dynamic identifiers, prefixes, instance trait,
//! surrounding generics, and hierarchy depths.
//!
//! [`BorderInfo`] represents the state at an individual counter boundary.
//! Resolution of a border determines the affiliate instance and counter needed
//! to continue toward the next depth. [`DerivedItems`] collects the identifier
//! checkers, generated aliases, accessor expressions, and border information
//! produced while deriving the complete hierarchy.
//!
//! Intermediate types are materialized as generated aliases so that the
//! derived instance states can be referenced by the generated expressions.
//! [`Projection`] then describes the operation performed on each resolved
//! instance, such as projecting an associated item or invoking an associated
//! function.
//!
//! The final result of the transformation is therefore a complete expression
//! whose runtime arguments select the appropriate positions in the generated
//! instance hierarchy, whose match clauses perform that selection, whose
//! accessors represent the corresponding instance states, and whose error
//! branches handle positions that cannot be resolved.

// ===============================================================================
// ``````````````````````````````````` IMPORTS ```````````````````````````````````
// ===============================================================================

// --- Proc Macro Crates ---
use proc_macro2::TokenStream;
use quote::format_ident;
use syn::{
    Expr, ExprCall, ExprPath, GenericArgument, Generics, ItemType, Path, PathSegment, Type,
    TypePath, WherePredicate, parse_quote,
};

// --- Proc Suite ---
use proc_suite::{BStringList, IntList, SupportCrate, misc::*};

// --- Local Crate ---
use crate::{
    Extraction, Instance, Transformation, Utilization,
    access::{
        args::{AccessArgs, AccessArgsDepths, AccessArgsIdents, AccessArgsIndexes, BStrInputList},
        errors::{ResolveBugs, ResolveErrors},
    },
    node::{
        access::{ExactAccess, OnSetAccess, TerminalAccess, insert_gen_arg, remove_gen_args},
        state::*,
    },
    traits::{
        affiliates::{
            FloorAffiliatesCounters, FloorAffiliatesInstances, NextAffiliateCounters,
            NextAffiliateInstance,
        },
        idents::{CounterIdentCollectionLenTypeNum, CounterIdentHashCollectionGenArray},
    },
};

// ===============================================================================
// ```````````````````````` INSTANCE NON_LEAF PATH ACCESS ````````````````````````
// ===============================================================================

/// Resolves a non-leaf instance path to its terminal instance node.
///
/// This transformation resolves the selected terminal access for that
/// associated type and rewrites the QSelf to the corresponding terminal twin node.
///
/// The supplied [`IntList`] identifies the instance-counter positions used to
/// construct the [`TerminalAccess`] projection. The projection is inserted
/// into the trait path at the appropriate counter position, replacing the
/// original counter representation with the generated access projection.
///
/// Conceptually, an input path of the form:
///
/// ```text
/// <AssocType as InstanceTrait<...>>::Assoc
/// ```
///
/// is transformed to a path whose QSelf is the generated terminal twin of
/// `AssocType` and whose trait path contains the resolved terminal access:
///
/// ```text
/// <AssocTypeTerminalTwin as InstanceTrait<...TerminalAccess...>>::Assoc
/// ```
///
/// The terminal twin identifies the concrete terminal node reached by the
/// access, while the transformed trait path carries the corresponding
/// terminal access projection.
///
#[derive(Debug, Clone)]
pub(crate) struct InstanceNonLeafTerminalPathAccess;

impl<'a> Transformation<Expr, (&ExprPath, &IntList)> for InstanceNonLeafTerminalPathAccess {
    fn raw_transform(
        &self,
        transform: &mut Expr,
        context: &(&ExprPath, &IntList),
    ) -> Result<(), proc_macro2::TokenStream> {
        let &(expr_path, indexes) = context;
        let mut expr = expr_path.clone();

        let Some(q_base) = &mut expr.qself else {
            return Err(ResolveErrors::ExprPathRequiresQSelf {
                expr_path: expr.clone(),
            }
            .into());
        };

        let mut base_ty = &mut *q_base.ty;

        let Type::Path(type_path) = &mut base_ty else {
            return Err(ResolveErrors::QSelfNotTypePath {
                q_self: base_ty.clone(),
            }
            .into());
        };

        let pos = q_base.position;
        let trait_p = &mut expr.path.segments.iter_mut().take(pos);

        let Some(trait_path) = trait_p.last() else {
            return Err(ResolveErrors::InstanceTraitPathNotFound {
                path: expr.path.clone(),
            }
            .into());
        };

        subscriber_call_path(&type_path, trait_path, indexes)?;

        let Some(last) = type_path.path.segments.last_mut() else {
            return Err(ResolveErrors::QSelfNotTypePath {
                q_self: base_ty.clone(),
            }
            .into());
        };
        last.ident = gen_type_ident_from_ident_with_suffix::<InstanceTerminalNode>(
            &last.ident.clone(),
            TERMINAL_TWIN_NODE,
        );

        *transform = Expr::Path(expr);

        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &Expr,
        context: Option<&(&ExprPath, &IntList)>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let Some(context) = context else {
            return Ok(());
        };

        let Expr::Path(expr) = transform else {
            return Err(ResolveBugs::InstanceNonLeafPathResolutionYieldedNonPath {}.into());
        };
        let &(expr_path, indexes) = context;

        let Some(q_base) = &expr.qself else {
            return Err(ResolveBugs::ExprPathRequiresQSelf {}.into());
        };

        let pos = q_base.position;
        let trait_p = expr.path.segments.iter().take(pos);
        let Some(trait_path) = trait_p.last() else {
            return Err(ResolveBugs::InstanceTraitPathNotFound {}.into());
        };

        let Some(context_q_base) = &expr_path.qself else {
            return Err(ResolveBugs::ExprPathRequiresQSelf {}.into());
        };

        let Type::Path(context_type_path) = &*context_q_base.ty else {
            return Err(ResolveBugs::QSelfNotTypePath {}.into());
        };

        validate_subscriber_call_path(&context_type_path, trait_path, indexes)?;

        let Some(q_base) = &expr.qself else {
            return Err(ResolveBugs::InstanceNonLeafPathResolvedQSelfMissing {}.into());
        };

        let Type::Path(type_path) = &*q_base.ty else {
            return Err(ResolveBugs::InstanceNonLeafPathResolvedQSelfInvalid {}.into());
        };

        let Some(last) = type_path.path.segments.last() else {
            return Err(ResolveBugs::InstanceNonLeafPathResolvedQSelfInvalid {}.into());
        };

        let Some(context_last) = context_type_path.path.segments.last() else {
            return Err(ResolveBugs::QSelfNotTypePath {}.into());
        };
        if last.ident
            != gen_type_ident_from_ident_with_suffix::<InstanceTerminalNode>(
                &context_last.ident.clone(),
                TERMINAL_TWIN_NODE,
            )
        {
            return Err(ResolveBugs::InstanceNonLeafPathResolvedInconsistentAccess {}.into());
        }

        Ok(())
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ``````````````````````````````` PRIVATE HELPERS ```````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// Resolves the terminal subscriber access for an associated type.
fn subscriber_call_path(
    assoc_ty: &TypePath,
    trait_path: &mut PathSegment,
    indexes: &IntList,
) -> Result<(), TokenStream> {
    let mut global_ty = assoc_ty.clone();
    let Some(assoc) = global_ty.path.segments.last_mut() else {
        return Err(ResolveErrors::AssocQSelfLastIdentUnavailable {
            path: global_ty.path.clone(),
        }
        .into());
    };
    let ident = &mut assoc.ident;
    *ident =
        gen_type_ident_from_ident_with_suffix::<InstanceTerminalNode>(ident, TERMINAL_GLOBAL_NODE);
    let self_ty: Type = parse_quote!(#global_ty);

    let base_ty = Type::Path(assoc_ty.clone());
    let context = (&base_ty, indexes).into();
    let bound = TerminalAccess::checked_extract(&context, &())?;
    let t_context = (&self_ty, context, false).into();
    TerminalAccess::checked_transform(&bound, trait_path, &t_context)?;

    Ok(())
}

/// Validates the terminal subscriber access for an associated type.
fn validate_subscriber_call_path(
    assoc_path: &TypePath,
    trait_path: &PathSegment,
    indexes: &IntList,
) -> Result<(), TokenStream> {
    let mut global_ty = assoc_path.clone();
    let Some(assoc) = global_ty.path.segments.last_mut() else {
        return Err(ResolveBugs::AssocQSelfLastIdentUnavailable {}.into());
    };
    let ident = &mut assoc.ident;
    *ident =
        gen_type_ident_from_ident_with_suffix::<InstanceTerminalNode>(ident, TERMINAL_GLOBAL_NODE);
    let self_ty: Type = parse_quote!(#global_ty);

    let base_ty = Type::Path(assoc_path.clone());
    let context = (&base_ty, indexes).into();
    let bound = TerminalAccess::checked_extract(&context, &())?;
    let t_context = (&self_ty, context, true).into();
    TerminalAccess::validate_transform(&bound, trait_path, Some(&t_context))?;
    Ok(())
}

// ===============================================================================
// ```````````````````````` INSTANCE NON_LEAF CALL ACCESS ````````````````````````
// ===============================================================================

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ```````````````````````````` NON-LEAF PROJECTIONS `````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// Describes a projection from a tokenized base path to a path segment,
/// associated-function call, or trait-associated function call.
///
/// A projection stores the syntactic components that follow a base path.
/// [`Projection::project`] materializes those components by inserting the
/// supplied tokens as the base of the projection.
///
/// The base itself is not stored in the projection. It is supplied separately
/// to [`Projection::project`] as a [`TokenStream`], allowing the same
/// projection description to be materialized against different bases.
pub(crate) enum Projection<'a> {
    /// Projects a path segment from the supplied base.
    ///
    /// Given:
    ///
    /// ```text
    /// tokens  = Base
    /// segment = Item
    /// ```
    ///
    /// produces:
    ///
    /// ```text
    /// Base::Item
    /// ```
    ///
    /// This represents a simple path extension without a function call.
    #[allow(dead_code)]
    Segment { segment: &'a PathSegment },

    /// Projects a direct associated-function call from the supplied base.
    ///
    /// Given:
    ///
    /// ```text
    /// tokens = Base
    /// call   = get
    /// args   = [arg1, arg2]
    /// ```
    ///
    /// produces:
    ///
    /// ```text
    /// Base::get(arg1, arg2)
    /// ```
    ///
    /// When no arguments are stored, the resulting call is emitted as:
    ///
    /// ```text
    /// Base::get()
    /// ```
    Call {
        call: &'a PathSegment,
        args: Vec<&'a Expr>,
    },

    /// Projects a function call through a path segment and an optional trait
    /// bound.
    ///
    /// When a trait bound is present, the projection produces a qualified
    /// trait-associated call:
    ///
    /// ```text
    /// <Base::Item as Trait>::get(arg)
    /// ```
    ///
    /// When no trait bound is present, the projection produces an ordinary
    /// nested path call:
    ///
    /// ```text
    /// Base::Item::get(arg)
    /// ```
    #[allow(dead_code)]
    TraitCall {
        segment: &'a PathSegment,
        bound: Option<&'a Path>,
        call: &'a PathSegment,
        args: Vec<&'a Expr>,
    },
}

impl<'a> Projection<'a> {
    fn project(&self, tokens: &TokenStream) -> Expr {
        match self {
            Projection::Segment { segment } => {
                parse_quote!(#tokens::#segment)
            }

            Projection::Call { call, args } => {
                if !args.is_empty() {
                    parse_quote!(#tokens::#call(#(From::from(#args)),*))
                } else {
                    parse_quote!(#tokens::#call())
                }
            }

            Projection::TraitCall {
                segment,
                bound,
                call,
                args,
            } => {
                if let Some(bound) = bound {
                    if !args.is_empty() {
                        parse_quote!(
                            <#tokens::#segment as #bound>::#call(#(From::from(#args)),*)
                        )
                    } else {
                        parse_quote!(
                            <#tokens::#segment as #bound>::#call()
                        )
                    }
                } else {
                    if !args.is_empty() {
                        parse_quote!(
                            #tokens::#segment::#call(#(From::from(#args)),*)
                        )
                    } else {
                        parse_quote!(
                            #tokens::#segment::#call()
                        )
                    }
                }
            }
        }
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// `````````````````````````` NON-LEAF EXPR CALL ACCESS ``````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// Resolves non-leaf instance-associated function calls through the generated
/// instance hierarchy.
///
/// The transformation derives the subscriber call from the node, instance
/// access arguments, and projection, resolving dynamic identifiers through the
/// corresponding counter dimensions before producing the final expression.
#[derive(Debug, Clone)]
pub(crate) struct InstanceNonLeafExprCallAccess;

type ExprCallContext<'a> = (&'a ExprCall, &'a Generics, &'a AccessArgs);

impl<'a> Transformation<Expr, ExprCallContext<'a>> for InstanceNonLeafExprCallAccess {
    fn raw_transform(
        &self,
        transform: &mut Expr,
        context: &ExprCallContext<'a>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let &(expr_call, generics, args) = context;

        let expr = &*expr_call.func;

        let arguments = expr_call.args.iter().collect::<Vec<_>>();

        let Expr::Path(expr_path) = expr else {
            return Err(ResolveErrors::ExprCallPathNotExprPath { expr: expr.clone() }.into());
        };

        let Some(q_base) = &expr_path.qself else {
            return Err(ResolveErrors::ExprPathRequiresQSelf {
                expr_path: expr_path.clone(),
            }
            .into());
        };

        let base_ty = &*q_base.ty;
        let Type::Path(type_path) = &*base_ty else {
            return Err(ResolveErrors::QSelfNotTypePath {
                q_self: base_ty.clone(),
            }
            .into());
        };

        let pos = q_base.position;
        let trait_p = expr_path.path.segments.iter().take(pos);

        let Some(inst_trait) = trait_p.last() else {
            return Err(ResolveErrors::InstanceTraitPathNotFound {
                path: expr_path.path.clone(),
            }
            .into());
        };

        let Some(segment) = expr_path.path.segments.last() else {
            return Err(ResolveErrors::ExprCallPathNotExprPath { expr: expr.clone() }.into());
        };

        let mutate = subscriber_call_expr(
            type_path,
            &mut inst_trait.clone(),
            &Projection::Call {
                call: segment,
                args: arguments,
            },
            &mut generics.clone(),
            args,
        )?;

        *transform = mutate;

        Ok(())
    }

    fn validate_transform(
        &self,
        _transform: &Expr,
        context: Option<&ExprCallContext<'a>>,
    ) -> Result<(), proc_macro2::TokenStream> {
        let Some((expr_call, ..)) = context else {
            return Ok(());
        };

        let expr = &*expr_call.func;

        let Expr::Path(expr_path) = expr else {
            return Err(ResolveBugs::ExprCallPathNotExprPath {}.into());
        };

        let Some(q_base) = &expr_path.qself else {
            return Err(ResolveBugs::ExprPathRequiresQSelf {}.into());
        };

        let base_ty = &q_base.ty;

        let Type::Path(_) = &**base_ty else {
            return Err(ResolveBugs::QSelfNotTypePath {}.into());
        };

        let pos = q_base.position;
        let trait_p = expr_path.path.segments.iter().take(pos);
        let Some(_) = trait_p.last() else {
            return Err(ResolveBugs::InstanceTraitPathNotFound {}.into());
        };

        let Some(_) = expr_path.path.segments.last() else {
            return Err(ResolveBugs::ExprCallPathNotExprPath {}.into());
        };

        Ok(())
    }
}

/// Describes the generated type representations associated with an instance
/// node and its position within the instance hierarchy (see [`crate::node`]).
///
/// Each representation corresponds to a distinct role of the same instance
/// node:
///
/// - `global` - the generated global representation of the node ([`GLOBAL_NODE`]);
/// - `initial` - the representation used as the initial boundary of the node ([`INITIAL_NODE`]);
/// - `final_` - the representation used as the final boundary of the node ([`FINAL_NODE`]); and
/// - `node` - the representation identifying the node itself.
///
/// These representations are derived together because the instance-node
/// transformation needs to relate the node's global, boundary, and concrete
/// forms when constructing the generated instance hierarchy.
///
/// The four types therefore describe the same logical instance node from
/// different positions in the generated type-level hierarchy.
#[derive(Debug)]
struct NodeNeighbours {
    global: TypePath,
    initial: TypePath,
    final_: TypePath,
    node: TypePath,
}

impl NodeNeighbours {
    /// Derives the generated type paths associated with an instance node.
    ///
    /// The supplied `node` represents the original instance node. Its final path
    /// segment identifies the node, and that identifier is used to derive the
    /// corresponding generated representations:
    ///
    /// ```text
    /// Node
    /// |---NodeInitial
    /// |---NodeFinal
    /// |---NodeGlobal
    /// ```
    ///
    /// The original path is preserved as `node`, while its final segment is
    /// replaced with the generated [`InstanceNode`] variants for `initial`,
    /// `final_`, and `global`.
    ///
    /// All other segments of the original path, including its generic arguments
    /// and qualification, are preserved unchanged.
    fn get_node_path(node: &TypePath) -> Result<Self, TokenStream> {
        let Some(node_seg) = node.path.segments.last() else {
            return Err(ResolveErrors::QSelfNotTypePath {
                q_self: Type::Path(node.clone()),
            }
            .into());
        };

        let node_ident = &node_seg.ident;

        let mut try_ty = node.clone();

        let initial_ident =
            gen_type_ident_from_ident_with_suffix::<InstanceNode>(node_ident, INITIAL_NODE);
        try_ty.path.segments.last_mut().unwrap().ident = initial_ident.clone();
        let init_ty = try_ty.clone();

        let final_ident =
            gen_type_ident_from_ident_with_suffix::<InstanceNode>(node_ident, FINAL_NODE);
        try_ty.path.segments.last_mut().unwrap().ident = final_ident.clone();
        let final_ty = try_ty.clone();

        let global_ident =
            gen_type_ident_from_ident_with_suffix::<InstanceNode>(node_ident, GLOBAL_NODE);
        try_ty.path.segments.last_mut().unwrap().ident = global_ident.clone();
        let global_ty = try_ty.clone();

        Ok(Self {
            global: global_ty,
            initial: init_ty,
            final_: final_ty,
            node: node.clone(),
        })
    }

    /// Derives the initial and final instance paths for a node.
    ///
    /// The supplied `inst` contains the instance path whose node type argument
    /// identifies the current node. The node's generic arguments are first removed
    /// at the positions specified by `indexes`, and two copies of the resulting
    /// path are created.
    ///
    /// The first copy receives the generated `initial` node type, while the second
    /// receives the generated `final_` node type:
    ///
    /// ```text
    /// inst<..., Node, ...>
    ///       |
    ///       |---initial -> inst<..., NodeInitial, ...>
    ///       |---final   -> inst<..., NodeFinal, ...>
    /// ```
    ///
    /// Only the node type argument at first index-value of `indexes` is replaced; the surrounding path
    /// structure is preserved.
    ///
    /// The returned paths represent the initial and final instance boundaries of
    /// the node and are used when constructing the corresponding instance
    /// hierarchy representations.
    fn get_node_instances(
        &self,
        inst: &mut PathSegment,
        indexes: &IntList,
    ) -> Result<(PathSegment, PathSegment), TokenStream> {
        let NodeNeighbours {
            initial, final_, ..
        } = &self;
        let initial_ty = initial;
        let final_ty = final_;

        remove_gen_args(inst, indexes)?;

        let mut initial_instance = inst.clone();
        insert_gen_arg(&mut initial_instance, indexes, || {
            Ok(GenericArgument::Type(parse_quote!(#initial_ty)))
        })?;

        let mut final_instance = inst.clone();
        insert_gen_arg(&mut final_instance, indexes, || {
            Ok(GenericArgument::Type(parse_quote!(#final_ty)))
        })?;

        Ok((initial_instance, final_instance))
    }

    /// Adds a where-predicate that confirms the supplied prefixes through
    /// [`OnSetAccess`] on the node's global representation.
    ///
    /// For example, prefixes such as `b"x", b"y"` are resolved through
    /// [`OnSetAccess`] to produce the corresponding access bound, which is then
    /// added as a constraint on the generics used throughout all declarations:
    ///
    /// ```text
    /// NodeInitial: <OnSetAccess for GlobalNode using [x, y]>
    /// ```
    ///
    /// This ensures that the generated initial node satisfies the requested
    /// prefix access.
    fn prefix_bound(
        &self,
        generics: &mut Generics,
        indexes: &IntList,
        prefixes: &BStringList,
    ) -> Result<(), TokenStream> {
        let NodeNeighbours {
            initial,
            global,
            node,
            ..
        } = &self;
        let initial_ty = initial;
        let global_ty = global;
        let node_ty = node;

        let self_ty: Type = parse_quote!(#node_ty);
        let global_ty: Type = parse_quote!(#global_ty);
        let context = (&self_ty, indexes, prefixes).into();
        let onset = OnSetAccess::checked_extract(&context, &())?;
        let bound = ExactAccess::checked_extract(&onset, &global_ty)?.bound;
        let where_pred: WherePredicate = parse_quote!(
            #initial_ty : #bound
        );
        generics.make_where_clause().predicates.push(where_pred);

        Ok(())
    }
}

/// Collects the derived components required to transform an
/// instance associated function call.
///
/// The fields retain the node representations, resolved instance paths,
/// access metadata, generics, and depth information needed across the
/// transformation steps.
#[derive(Debug)]
struct ExprCallEssentials {
    /// Generated global, initial, final, and original node representations.
    neighbours: NodeNeighbours,

    /// Initial and final instance paths derived from the node.
    instances: (PathSegment, PathSegment),

    /// Type-level generic positions identifying the instance-counters being accessed.
    indexes: IntList,

    /// DYnamic Identifiers used to select the instance values.
    idents: BStrInputList,

    /// Optional prefixes used for prefix-based instance access.
    #[allow(dead_code)]
    prefixes: Option<BStringList>,

    /// Original instance path from which the derived instances are built.
    inst: PathSegment,

    /// Generic parameters and where-clause constraints for the call.
    generics: Generics,

    /// First and Second order depths of the instance within the hierarchy.
    depths: (u8, u8),
}

/// Holds the intermediate state produced while deriving a single counter
/// border.
///
/// `index` identifies the counter position being processed, `node` is the
/// current node type at that position, `inst` is the instance trait segment
/// used to resolve the node, and `aliases` contains the type aliases generated
/// while deriving the border.
#[derive(Debug, Clone)]
struct BorderInfo {
    /// Counter position represented by this border.
    index: usize,

    /// Node type from which the border is derived.
    node: Type,

    /// Instance trait segment used to resolve the node.
    inst: PathSegment,

    /// Type aliases generated while constructing this border.
    aliases: Vec<ItemType>,
}

/// Collects all intermediate artifacts needed to assemble the final
/// dynamic-counter expression.
#[derive(Debug, Clone)]
struct DerivedItems {
    /// Expressions that resolve dynamic identifiers to counter positions.
    ident_checkers: Vec<Expr>,

    /// Type aliases generated while deriving each counter chain.
    aliases: Vec<Vec<ItemType>>,

    /// Accessor expressions generated for each counter position.
    exprs: Vec<Vec<Expr>>,

    /// Type aliases generated for the borders between counter chains.
    borders: Vec<Vec<ItemType>>,
}

impl ExprCallEssentials {
    /// Derives and collects all components required to transform an associated
    /// function call from the supplied node and access arguments.
    ///
    /// This resolves the node's generated representations, instance paths,
    /// identifiers, optional prefixes, generic constraints, and hierarchy depths
    /// into a single [`ExprCallEssentials`] value.
    ///
    /// When prefixes are present, their corresponding [`OnSetAccess`] bound is
    /// added to `generics` to confirm that the requested prefix access is valid.
    /// Dynamic identifiers are limited to at most two entries.
    fn get(
        node: &TypePath,
        inst: &mut PathSegment,
        args: &AccessArgs,
        generics: &mut Generics,
    ) -> Result<Self, TokenStream> {
        let nodes = NodeNeighbours::get_node_path(node)?;
        let indexes = AccessArgsIndexes::checked_utilize(args, &())?.0;
        let (initial_instance, final_instance) = nodes.get_node_instances(inst, &indexes)?;
        let idents = AccessArgsIdents::checked_extract(args, &())?;
        let AccessArgsIdents { prefixes, idents } = idents;
        if idents.exprs.len() > 2 {
            return Err(ResolveErrors::MaxDynCounters { list: idents }.into());
        }
        let prefix_idents = if prefixes.bytes.is_empty() {
            None
        } else {
            Some(prefixes)
        };
        if let Some(prefixes) = &prefix_idents {
            nodes.prefix_bound(generics, &indexes, prefixes)?;
        }
        let depths = AccessArgsDepths::checked_extract(args, &())?.0;
        Ok(Self {
            neighbours: nodes,
            instances: (initial_instance, final_instance),
            indexes: indexes.clone(),
            idents,
            prefixes: prefix_idents,
            inst: inst.clone(),
            generics: generics.clone(),
            depths,
        })
    }

    /// Builds identifier-checker expressions for the supplied dynamic identifiers.
    ///
    /// Each expression searches the final instance's hash collection for the hash
    /// of one identifier. The search starts at the corresponding initial-instance
    /// length, so the resulting slice represents only the portion relevant to that
    /// access and therefore produces an `Option<usize>`:
    ///
    /// ```text
    /// Some(index)  -> the identifier was found at that position
    /// None         -> the identifier was not found
    /// ```
    ///
    /// The indexes and identifiers are processed in reverse order because the
    /// generated checker expressions correspond to the access hierarchy from the
    /// innermost identifier back toward the outermost one.
    fn ident_checkers(&self) -> Result<Vec<Expr>, TokenStream> {
        let ExprCallEssentials {
            neighbours,
            instances,
            indexes,
            idents,
            ..
        } = &self;
        let NodeNeighbours { node, .. } = neighbours;
        let self_ty = node;
        let (initial_instance, final_instance) = instances;

        let crate_of = Instance::support_crate();
        let rev_indexes = indexes.ints.iter().rev();
        let rev_idents = idents.exprs.iter().rev();

        let mut checker_expr = Vec::new();

        for (index, ident) in rev_indexes.zip(rev_idents) {
            let given = parse_pos_usize(index)?;

            let arr_len = gen_type_ident_with_suffix::<CounterIdentCollectionLenTypeNum>(Some(
                given.to_string().as_bytes(),
            ));

            let initial_len: Expr = parse_quote!(<<#self_ty as #initial_instance>::#arr_len as #crate_of::Unsigned>::USIZE);

            let arr = gen_const_ident_with_suffix::<CounterIdentHashCollectionGenArray>(Some(
                given.to_string().as_bytes(),
            ));

            let final_arr: Expr = parse_quote!(<#self_ty as #final_instance>::#arr);

            let relative_slc: Expr = parse_quote!(&#final_arr[#initial_len -1 ..]);

            let derive: Expr = parse_quote!(
                #crate_of::ident_check(
                    #relative_slc,
                    &#crate_of::hash_ident(#ident),
                )
            );

            checker_expr.push(derive);
        }

        if checker_expr.is_empty() {
            return Err(ResolveBugs::IdentCheckerExprsEmpty {}.into());
        }

        Ok(checker_expr)
    }

    /// Consumes the current border and prepares the derived state required to
    /// construct the next counter border.
    ///
    /// The current border provides the floor instance and floor counter
    /// (see [`crate::traits::affiliates`]) for the current counter position.
    /// These are resolved through the current border's trait segment, then used to
    /// derive the corresponding next affiliate instance and next affilaite counters
    /// (from the relative position).
    ///
    /// The newly derived instance and counter become the state of `border` itself,
    /// so the updated border can be used as the input when processing the next
    /// counter position:
    ///
    /// ```text
    /// current border
    ///      |---FloorAffiliateInstance
    ///      |---FloorAffiliateCounter
    ///             |---NextAffiliateInstance
    ///             |---NextAffiliateCounters
    ///                     |
    ///              next border state
    /// ```
    ///
    /// Type aliases are generated for the intermediate types so that the derived
    /// types can be used in the resulting generic paths while preserving the
    /// generics of the surrounding call.
    ///
    /// The resulting `border.node`, `border.inst`, and `border.aliases` therefore
    /// represent the starting state from which the next counter border can be
    /// constructed.
    fn take_border<'a>(
        &self,
        border: &mut BorderInfo,
        counter: &mut usize,
    ) -> Result<(), TokenStream> {
        let self_type = &border.node;
        let trait_seg = &border.inst;
        let given = &border.index;

        let cleaned_inst = &self.inst;
        let indexes = &self.indexes;
        let generics = &self.generics;

        let next_inst_ident = gen_type_ident::<NextAffiliateInstance>();
        let next_counter_ident = gen_type_ident::<NextAffiliateCounters>();
        let floor_inst_ident = gen_type_ident_with_suffix::<FloorAffiliatesInstances>(Some(
            given.to_string().as_bytes(),
        ));
        let floor_counter_ident = gen_type_ident_with_suffix::<FloorAffiliatesCounters>(Some(
            given.to_string().as_bytes(),
        ));

        let mut border_type_aliases = Vec::new();

        let floor_inst: Type = parse_quote!(<#self_type as #trait_seg>::#floor_inst_ident);
        let floor_counter: Type = parse_quote!(<#self_type as #trait_seg>::#floor_counter_ident);

        let [floor_inst, floor_counter] = make_type(
            [&floor_inst, &floor_counter],
            &mut border_type_aliases,
            counter,
            generics,
        );

        let mut floor_trait_seg = cleaned_inst.clone();
        insert_gen_arg(&mut floor_trait_seg, indexes, || {
            Ok(GenericArgument::Type(floor_counter))
        })?;

        let floor_next_inst: Type =
            parse_quote!(<#floor_inst as #floor_trait_seg>::#next_inst_ident);
        let floor_next_counter: Type =
            parse_quote!(<#floor_inst as #floor_trait_seg>::#next_counter_ident);

        let [floor_next_inst, floor_next_counter] = make_type(
            [&floor_next_inst, &floor_next_counter],
            &mut border_type_aliases,
            counter,
            generics,
        );

        let mut floor_next_trait_seg = cleaned_inst.clone();
        insert_gen_arg(&mut floor_next_trait_seg, indexes, || {
            Ok(GenericArgument::Type(floor_next_counter))
        })?;

        border.node = floor_next_inst;
        border.inst = floor_next_trait_seg;
        border.aliases = border_type_aliases;

        Ok(())
    }

    /// Derives the type aliases, accessor expressions, and border aliases required
    /// to construct the final dynamic-counter expression.
    ///
    /// The derivation walks the counter arguments from the innermost counter toward
    /// the outer counters. For each counter, it builds the corresponding border
    /// and then follows the `NextAffiliateInstance` / `NextAffiliateCounters`
    /// chain to produce the accessor expression for every possible position.
    ///
    /// The resulting [`DerivedItems`] contains:
    ///
    /// - `ident_checkers` - expressions that resolve each dynamic identifier to
    ///   its position within the corresponding final counter collection;
    /// - `aliases` - type aliases required by the generated intermediate instance
    ///   and counter types;
    /// - `exprs` - accessor expressions arranged by counter position;
    /// - `borders` - type aliases required by the derived counter borders.
    ///
    /// The `projection` determines what is projected from each derived instance
    /// state. The generated expressions are retained separately from the aliases
    /// and borders because [`Self::derive_expr`] later assembles them into the nested
    /// runtime index-selection logic.
    fn derive_items<'a>(
        &self,
        mut count: &mut usize,
        projection: &Projection<'a>,
    ) -> Result<DerivedItems, TokenStream> {
        let ident_checkers = self.ident_checkers()?;

        let mut aliases: Vec<Vec<ItemType>> = Vec::new();
        let mut exprs: Vec<Vec<Expr>> = Vec::new();
        let mut borders: Vec<Vec<ItemType>> = Vec::new();

        let (inner, outer) = self.depths;

        if inner == 0 {
            return Err(ResolveBugs::DepthIsZero {}.into());
        };

        let next_inst_ident = gen_type_ident::<NextAffiliateInstance>();
        let next_counter_ident = gen_type_ident::<NextAffiliateCounters>();

        let rev_indexes = self.indexes.ints.iter().rev();
        let rev_idents = self.idents.exprs.iter().rev();

        let mut args_iter = rev_indexes.clone().zip(rev_idents.clone());

        let (last, _) = args_iter.next().unwrap();

        let mut given = parse_pos_usize(last)?;

        let node = &self.neighbours.node;
        let mut self_type: Type = parse_quote!(#node);
        let initial_instance = &self.instances.0;
        let mut trait_seg = initial_instance.clone();

        for _ in 0..outer + 1 {
            let archive_self_type = self_type.clone();
            let archive_trait_seg = trait_seg.clone();

            let mut next_exists = None;

            let mut in_aliases = Vec::new();
            let mut in_exprs = Vec::new();
            let mut in_borders = Vec::new();

            in_borders.push(Vec::new());

            // CURRENT ARGUMENT BLOCK
            for _ in 0..inner {
                let mut border = BorderInfo {
                    index: given,
                    node: self_type.clone(),
                    inst: trait_seg.clone(),
                    aliases: Vec::new(),
                };
                self.take_border(&mut border, &mut count)?;

                let mut aliases = Vec::new();
                let mut accessors = Vec::new();

                // CHAIN
                for i in 0..inner {
                    if i > 0 {
                        let next_inst: Type = parse_quote!(
                            <#self_type as #trait_seg>::#next_inst_ident
                        );

                        let next_counter: Type = parse_quote!(
                            <#self_type as #trait_seg>::#next_counter_ident
                        );

                        let [next_inst, next_counter] = make_type(
                            [&next_inst, &next_counter],
                            &mut aliases,
                            &mut count,
                            &self.generics,
                        );

                        self_type = next_inst;

                        let mut bound = self.inst.clone();
                        insert_gen_arg(&mut bound, &self.indexes, || {
                            Ok(GenericArgument::Type(next_counter))
                        })?;

                        trait_seg = bound;
                    }

                    let pre_project = quote::quote! {<#self_type as #trait_seg>};

                    let expr = projection.project(&pre_project);

                    accessors.push(expr);
                }

                in_aliases.push(aliases);
                in_exprs.push(accessors);
                in_borders.push(border.aliases.clone());

                // Pick the next argument exactly as the old implementation did.
                if next_exists.is_none() {
                    if let Some((next, _)) = args_iter.next() {
                        self_type = border.node.clone();
                        trait_seg = border.inst.clone();

                        next_exists = Some(next);
                    } else {
                        break;
                    }
                }
            }

            // SAVE CURRENT BLOCK
            aliases.append(&mut in_aliases);
            exprs.append(&mut in_exprs);
            borders.append(&mut in_borders);

            // NO MORE ARGUMENTS
            let Some(next) = next_exists else {
                break;
            };

            // MOVE TO NEXT ARGUMENT
            //
            // Restore the state from the beginning of this block, then construct
            // the border corresponding to the argument we just selected.
            given = parse_pos_usize(next)?;

            self_type = archive_self_type;
            trait_seg = archive_trait_seg;

            let mut border = BorderInfo {
                index: given,
                node: self_type.clone(),
                inst: trait_seg.clone(),
                aliases: Vec::new(),
            };
            self.take_border(&mut border, &mut count)?;

            self_type = border.node.clone();
            trait_seg = border.inst.clone();

            let Some(hanging_border) = borders.last_mut() else {
                return Err(ResolveBugs::HangingBordersNotAvailable {}.into());
            };

            *hanging_border = border.aliases.clone();
        }

        Ok(DerivedItems {
            ident_checkers,
            aliases,
            exprs,
            borders,
        })
    }

    /// Assembles the derived accessors and identifier checkers into the final
    /// dynamic-counter expression.
    ///
    /// Each identifier checker produces an `Option<usize>`. A successful checker
    /// supplies the index used to select the corresponding accessor expression;
    /// `None` produces `Error::OutOfRange`.
    ///
    /// For example, for a single counter:
    ///
    /// ```text
    /// ident_check(...) -> Some(idx)
    ///                       |
    ///                       ▼
    ///                  match idx
    ///                  |---0 -> exprs[0]
    ///                  |---1 -> exprs[1]
    ///                  |---n -> Exhausted
    /// ```
    ///
    /// With multiple counters, the resulting matches are nested, with each
    /// identifier checker selecting the next dimension:
    ///
    /// ```text
    /// first identifier
    ///        |
    ///    counter index
    ///        |
    /// second identifier
    ///        |
    ///    counter index
    ///        |
    ///   final accessor
    /// ```
    ///
    /// The generated border and type aliases are emitted before the selection
    /// expression so that all intermediate types required by the accessors are
    /// available in the generated item.
    ///
    /// `OutOfRange` indicates that an identifier does not exist in its counter
    /// collection, while `Exhausted` indicates that the identifier exists but
    /// that resolved numeric position exceeds the available derived accessors.
    fn derive_expr(&self, derives: DerivedItems) -> Result<Expr, TokenStream> {
        let DerivedItems {
            ident_checkers,
            aliases,
            exprs,
            borders,
        } = &derives;

        let all_borders = borders.into_iter().flatten();
        let all_aliases = aliases.into_iter().flatten();
        let prefixes = quote::quote! {
            #(#all_borders)*
            #(#all_aliases)*
        };

        let crate_of = Instance::support_crate();

        let rev_indexes = self.indexes.ints.iter().rev();

        let mut err_indexes = rev_indexes.clone();
        let inner_idx = err_indexes.next().unwrap();

        let mut matches = Vec::new();
        for exprs in exprs.iter() {
            let arms = exprs
                .into_iter()
                .enumerate()
                .map(|(i, expr)| {
                    quote::quote! {
                        #i => {
                            Ok({#expr}.into())
                        },
                    }
                })
                .collect::<Vec<_>>();

            let expr = quote::quote! {
                match idx {
                    #(#arms)*
                    n => {
                        Err(#crate_of::Error::Exhausted { depth: n, index: #inner_idx, expect: idx })
                    },
                }
            };
            matches.push(expr);
        }

        let mut derives_iter = ident_checkers.iter();
        let first_derive = derives_iter.next().unwrap();
        let mut matches_with_last_derive = Vec::new();
        for matches in matches.iter() {
            let tokens = quote::quote! {
                if let Some(idx) = #first_derive {
                    #matches
                } else {
                    Err(#crate_of::Error::OutOfRange {})
                }
            };
            matches_with_last_derive.push(tokens)
        }

        let Some(second_derive) = derives_iter.next() else {
            if matches_with_last_derive.len() > 1 {
                return Err(ResolveBugs::OnlyCounterHaveMultiDimensionExprs {}.into());
            };
            let matched = &matches_with_last_derive[0];
            return Ok(parse_quote!(
                {
                    #prefixes
                    #matched
                }
            ));
        };

        if matches_with_last_derive.len() <= 1 {
            return Err(ResolveBugs::MultiCounterHaveLessThanOneDimensionExprs {}.into());
        };

        let Some(second_err_idx) = err_indexes.next() else {
            return Err(ResolveBugs::MultiCounterButIndexesInsufficient {}.into());
        };

        let arms = matches_with_last_derive
            .into_iter()
            .enumerate()
            .map(|(i, expr)| {
                quote::quote! {
                    #i => {
                        {#expr}.into()
                    },
                }
            })
            .collect::<Vec<_>>();

        let final_expr = quote::quote! {
            if let Some(idx) = #second_derive {
                match idx {
                    #(#arms)*
                    n => {
                        Err(#crate_of::Error::Exhausted { depth: n, index: #second_err_idx, expect: idx })
                    }
                }
            } else {
                Err(#crate_of::Error::OutOfRange {})
            }
        };

        if derives_iter.next().is_some() {
            return Err(ResolveBugs::TralingIdentCheckersUnnessary {}.into());
        }

        let finish: Expr = parse_quote!(
            {
                #prefixes
                #final_expr
            }
        );

        Ok(finish)
    }
}

/// Materializes types as generated aliases and returns paths to those aliases.
///
/// Each input type is assigned a fresh `T{count}` alias, with the original
/// generic parameters and where-clause of `generics` attached to the alias.
///
/// For example:
///
/// ```text
/// input:  [Foo, Bar]
/// output aliases:
///     type T0<...> = Foo;
///     type T1<...> = Bar;
/// ```
///
/// The returned types reference the generated aliases using the generic
/// arguments appropriate for use at the alias site. The generated aliases are
/// appended to `collect`, and `count` is advanced for each type.
fn make_type<const N: usize>(
    tys: [&Type; N],
    collect: &mut Vec<ItemType>,
    count: &mut usize,
    generics: &Generics,
) -> [Type; N] {
    let id = std::array::from_fn(|i| {
        let ty = tys[i];
        let ident = format_ident!("T{}", *count);

        let (decl, ty_use, where_bounds) = generics.split_for_impl();
        collect.push(parse_quote!(
            #[allow(type_alias_bounds)]
            type #ident #decl #where_bounds = #ty;
        ));

        *count += 1;

        parse_quote!(#ident #ty_use)
    });

    id
}

/// Builds the complete subscriber call expression from the supplied instance
/// type and access arguments.
///
/// The access arguments are first resolved into [`ExprCallEssentials`], which
/// derives the identifier checkers, counter-chain accessors, and required type
/// aliases. Those derived items are then assembled into the final expression
/// that dynamically resolves the requested instance.
fn subscriber_call_expr<'a>(
    self_ty: &TypePath,
    instance: &mut PathSegment,
    projection: &Projection<'a>,
    generics: &mut Generics,
    access: &AccessArgs,
) -> Result<Expr, TokenStream> {
    let essence = ExprCallEssentials::get(self_ty, instance, access, generics)?;
    let mut count = 0usize;
    let derives = essence.derive_items(&mut count, projection)?;
    let expr = essence.derive_expr(derives)?;
    Ok(expr)
}
