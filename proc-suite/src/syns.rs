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
// ````````````````````````` SYN COMMON TRANSFORMATIONS ``````````````````````````
// ===============================================================================

//! Shared utilities for working with [`syn`] syntax nodes.
//!
//! The module is deliberately kept as a collection of general-purpose
//! utilities so that common syntax-tree functionality can be shared throughout
//! the crate rather than reimplemented by individual transformations.
//!
//! These utilities are intended to be reusable across transformations and
//! can be extended or adapted as additional common syntax-tree operations
//! are needed.
//!
//! This module currently provides utilities for manipulating path qualifiers,
//! identifiers and `Vec<Attribute>` collections.

// ===============================================================================
// ``````````````````````````````````` IMPORTS ```````````````````````````````````
// ===============================================================================

// --- Proc Macro Crates ---
use syn::{Attribute, Ident, visit_mut::VisitMut};

// ===============================================================================
// `````````````````````````````````` SEGMENTS ```````````````````````````````````
// ===============================================================================

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ``````````````````````````````````` TRAITS ````````````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// Provides the ability to replace path segment qualifiers throughout a syntax node.
pub trait PathQualifierReplace {
    fn replace_path_qualifiers(&mut self, visitor: &mut ReplacePathQualifier);
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ``````````````````````````````````` STRUCT ````````````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// Visitor that replaces matching path qualifiers with a replacement identifier.
///
/// The final path segment is never considered a qualifier and is therefore
/// excluded from replacement. Paths containing only a single segment are
/// consequently left unchanged.
pub struct ReplacePathQualifier {
    /// Qualifier to search for.
    pub from: Ident,

    /// Identifier to substitute for each matching qualifier.
    pub to: Ident,
}

impl VisitMut for ReplacePathQualifier {
    fn visit_path_mut(&mut self, i: &mut syn::Path) {
        if i.segments.is_empty() || i.segments.len() == 1 {
            return;
        }

        let segs = i.segments.iter_mut().rev().skip(1);

        for seg in segs {
            let ident = &mut seg.ident;

            if *ident == self.from {
                *ident = self.to.clone();
            }
        }
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ``````````````````````````````````` MACROS ````````````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

macro_rules! impl_path_seg_mut {
    ($($ty:ty => $method:ident),* $(,)?) => {
        $(
            impl PathQualifierReplace for $ty {
                fn replace_path_qualifiers(&mut self, visitor: &mut ReplacePathQualifier) {
                    visitor.$method(self);
                }
            }
        )*
    };
}

impl_path_seg_mut! {
    syn::Abi => visit_abi_mut,
    syn::AngleBracketedGenericArguments => visit_angle_bracketed_generic_arguments_mut,
    syn::Arm => visit_arm_mut,
    syn::AssocConst => visit_assoc_const_mut,
    syn::AssocType => visit_assoc_type_mut,
    syn::AttrStyle => visit_attr_style_mut,
    syn::Attribute => visit_attribute_mut,
    Vec<syn::Attribute> => visit_attributes_mut,
    syn::BareFnArg => visit_bare_fn_arg_mut,
    syn::BareVariadic => visit_bare_variadic_mut,
    syn::BinOp => visit_bin_op_mut,
    syn::Block => visit_block_mut,
    syn::BoundLifetimes => visit_bound_lifetimes_mut,
    syn::CapturedParam => visit_captured_param_mut,
    syn::ConstParam => visit_const_param_mut,
    syn::Constraint => visit_constraint_mut,
    syn::Data => visit_data_mut,
    syn::DataEnum => visit_data_enum_mut,
    syn::DataStruct => visit_data_struct_mut,
    syn::DataUnion => visit_data_union_mut,
    syn::DeriveInput => visit_derive_input_mut,
    syn::Expr => visit_expr_mut,
    syn::ExprArray => visit_expr_array_mut,
    syn::ExprAssign => visit_expr_assign_mut,
    syn::ExprAsync => visit_expr_async_mut,
    syn::ExprAwait => visit_expr_await_mut,
    syn::ExprBinary => visit_expr_binary_mut,
    syn::ExprBlock => visit_expr_block_mut,
    syn::ExprBreak => visit_expr_break_mut,
    syn::ExprCall => visit_expr_call_mut,
    syn::ExprCast => visit_expr_cast_mut,
    syn::ExprClosure => visit_expr_closure_mut,
    syn::ExprConst => visit_expr_const_mut,
    syn::ExprContinue => visit_expr_continue_mut,
    syn::ExprField => visit_expr_field_mut,
    syn::ExprForLoop => visit_expr_for_loop_mut,
    syn::ExprGroup => visit_expr_group_mut,
    syn::ExprIf => visit_expr_if_mut,
    syn::ExprIndex => visit_expr_index_mut,
    syn::ExprInfer => visit_expr_infer_mut,
    syn::ExprLet => visit_expr_let_mut,
    syn::ExprLit => visit_expr_lit_mut,
    syn::ExprLoop => visit_expr_loop_mut,
    syn::ExprMacro => visit_expr_macro_mut,
    syn::ExprMatch => visit_expr_match_mut,
    syn::ExprMethodCall => visit_expr_method_call_mut,
    syn::ExprParen => visit_expr_paren_mut,
    syn::ExprPath => visit_expr_path_mut,
    syn::ExprRange => visit_expr_range_mut,
    syn::ExprRawAddr => visit_expr_raw_addr_mut,
    syn::ExprReference => visit_expr_reference_mut,
    syn::ExprRepeat => visit_expr_repeat_mut,
    syn::ExprReturn => visit_expr_return_mut,
    syn::ExprStruct => visit_expr_struct_mut,
    syn::ExprTry => visit_expr_try_mut,
    syn::ExprTryBlock => visit_expr_try_block_mut,
    syn::ExprTuple => visit_expr_tuple_mut,
    syn::ExprUnary => visit_expr_unary_mut,
    syn::ExprUnsafe => visit_expr_unsafe_mut,
    syn::ExprWhile => visit_expr_while_mut,
    syn::ExprYield => visit_expr_yield_mut,
    syn::Field => visit_field_mut,
    syn::FieldMutability => visit_field_mutability_mut,
    syn::FieldPat => visit_field_pat_mut,
    syn::FieldValue => visit_field_value_mut,
    syn::Fields => visit_fields_mut,
    syn::FieldsNamed => visit_fields_named_mut,
    syn::FieldsUnnamed => visit_fields_unnamed_mut,
    syn::File => visit_file_mut,
    syn::FnArg => visit_fn_arg_mut,
    syn::ForeignItem => visit_foreign_item_mut,
    syn::ForeignItemFn => visit_foreign_item_fn_mut,
    syn::ForeignItemMacro => visit_foreign_item_macro_mut,
    syn::ForeignItemStatic => visit_foreign_item_static_mut,
    syn::ForeignItemType => visit_foreign_item_type_mut,
    syn::GenericArgument => visit_generic_argument_mut,
    syn::GenericParam => visit_generic_param_mut,
    syn::Generics => visit_generics_mut,
    syn::Ident => visit_ident_mut,
    syn::ImplItem => visit_impl_item_mut,
    syn::ImplItemConst => visit_impl_item_const_mut,
    syn::ImplItemFn => visit_impl_item_fn_mut,
    syn::ImplItemMacro => visit_impl_item_macro_mut,
    syn::ImplItemType => visit_impl_item_type_mut,
    syn::ImplRestriction => visit_impl_restriction_mut,
    syn::Index => visit_index_mut,
    syn::Item => visit_item_mut,
    syn::ItemConst => visit_item_const_mut,
    syn::ItemEnum => visit_item_enum_mut,
    syn::ItemExternCrate => visit_item_extern_crate_mut,
    syn::ItemFn => visit_item_fn_mut,
    syn::ItemForeignMod => visit_item_foreign_mod_mut,
    syn::ItemImpl => visit_item_impl_mut,
    syn::ItemMacro => visit_item_macro_mut,
    syn::ItemMod => visit_item_mod_mut,
    syn::ItemStatic => visit_item_static_mut,
    syn::ItemStruct => visit_item_struct_mut,
    syn::ItemTrait => visit_item_trait_mut,
    syn::ItemTraitAlias => visit_item_trait_alias_mut,
    syn::ItemType => visit_item_type_mut,
    syn::ItemUnion => visit_item_union_mut,
    syn::ItemUse => visit_item_use_mut,
    syn::Label => visit_label_mut,
    syn::Lifetime => visit_lifetime_mut,
    syn::LifetimeParam => visit_lifetime_param_mut,
    syn::LitBool => visit_lit_bool_mut,
    syn::LitByte => visit_lit_byte_mut,
    syn::LitByteStr => visit_lit_byte_str_mut,
    syn::LitCStr => visit_lit_cstr_mut,
    syn::LitChar => visit_lit_char_mut,
    syn::LitInt => visit_lit_int_mut,
    syn::LitStr => visit_lit_str_mut,
    syn::Local => visit_local_mut,
    syn::LocalInit => visit_local_init_mut,
    syn::Macro => visit_macro_mut,
    syn::MacroDelimiter => visit_macro_delimiter_mut,
    syn::Member => visit_member_mut,
    syn::Meta => visit_meta_mut,
    syn::MetaList => visit_meta_list_mut,
    syn::MetaNameValue => visit_meta_name_value_mut,
    syn::ParenthesizedGenericArguments => visit_parenthesized_generic_arguments_mut,
    syn::Pat => visit_pat_mut,
    syn::PatIdent => visit_pat_ident_mut,
    syn::PatOr => visit_pat_or_mut,
    syn::PatParen => visit_pat_paren_mut,
    syn::PatReference => visit_pat_reference_mut,
    syn::PatRest => visit_pat_rest_mut,
    syn::PatSlice => visit_pat_slice_mut,
    syn::PatStruct => visit_pat_struct_mut,
    syn::PatTuple => visit_pat_tuple_mut,
    syn::PatTupleStruct => visit_pat_tuple_struct_mut,
    syn::PatType => visit_pat_type_mut,
    syn::PatWild => visit_pat_wild_mut,
    syn::Path => visit_path_mut,
    syn::PathArguments => visit_path_arguments_mut,
    syn::PathSegment => visit_path_segment_mut,
    syn::PointerMutability => visit_pointer_mutability_mut,
    syn::PreciseCapture => visit_precise_capture_mut,
    syn::PredicateLifetime => visit_predicate_lifetime_mut,
    syn::PredicateType => visit_predicate_type_mut,
    syn::QSelf => visit_qself_mut,
    syn::RangeLimits => visit_range_limits_mut,
    syn::Receiver => visit_receiver_mut,
    syn::ReturnType => visit_return_type_mut,
    syn::Signature => visit_signature_mut,
    syn::StaticMutability => visit_static_mutability_mut,
    syn::Stmt => visit_stmt_mut,
    syn::StmtMacro => visit_stmt_macro_mut,
    syn::TraitBound => visit_trait_bound_mut,
    syn::TraitBoundModifier => visit_trait_bound_modifier_mut,
    syn::TraitItem => visit_trait_item_mut,
    syn::TraitItemConst => visit_trait_item_const_mut,
    syn::TraitItemFn => visit_trait_item_fn_mut,
    syn::TraitItemMacro => visit_trait_item_macro_mut,
    syn::TraitItemType => visit_trait_item_type_mut,
    syn::Type => visit_type_mut,
    syn::TypeArray => visit_type_array_mut,
    syn::TypeBareFn => visit_type_bare_fn_mut,
    syn::TypeGroup => visit_type_group_mut,
    syn::TypeImplTrait => visit_type_impl_trait_mut,
    syn::TypeInfer => visit_type_infer_mut,
    syn::TypeMacro => visit_type_macro_mut,
    syn::TypeNever => visit_type_never_mut,
    syn::TypeParam => visit_type_param_mut,
    syn::TypeParamBound => visit_type_param_bound_mut,
    syn::TypeParen => visit_type_paren_mut,
    syn::TypePath => visit_type_path_mut,
    syn::TypePtr => visit_type_ptr_mut,
    syn::TypeReference => visit_type_reference_mut,
    syn::TypeSlice => visit_type_slice_mut,
    syn::TypeTraitObject => visit_type_trait_object_mut,
    syn::TypeTuple => visit_type_tuple_mut,
    syn::UnOp => visit_un_op_mut,
    syn::UseGlob => visit_use_glob_mut,
    syn::UseGroup => visit_use_group_mut,
    syn::UseName => visit_use_name_mut,
    syn::UsePath => visit_use_path_mut,
    syn::UseRename => visit_use_rename_mut,
    syn::UseTree => visit_use_tree_mut,
    syn::Variadic => visit_variadic_mut,
    syn::Variant => visit_variant_mut,
    syn::VisRestricted => visit_vis_restricted_mut,
    syn::Visibility => visit_visibility_mut,
    syn::WhereClause => visit_where_clause_mut,
    syn::WherePredicate => visit_where_predicate_mut,
}

// ===============================================================================
// ``````````````````````````````````` IDENTS ````````````````````````````````````
// ===============================================================================

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ``````````````````````````````````` TRAITS ````````````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// Provides the ability to replace identifiers throughout a syntax node.
pub trait IdentReplace {
    fn replace_ident(&mut self, visitor: &mut ReplaceIdent);
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ``````````````````````````````````` STRUCT ````````````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// Visitor that replaces occurrences of one identifier with another.
pub struct ReplaceIdent {
    /// Identifier to search for.
    pub from: Ident,

    /// Identifier to substitute for each matching occurrence.
    pub to: Ident,
}

impl VisitMut for ReplaceIdent {
    fn visit_ident_mut(&mut self, ident: &mut Ident) {
        if *ident == self.from {
            *ident = self.to.clone();
        }
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ``````````````````````````````````` MACROS ````````````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

macro_rules! impl_ident_mut {
    ($($ty:ty => $method:ident),* $(,)?) => {
        $(
            impl IdentReplace for $ty {
                fn replace_ident(&mut self, visitor: &mut ReplaceIdent) {
                    visitor.$method(self);
                }
            }
        )*
    };
}

impl_ident_mut! {
    syn::Abi => visit_abi_mut,
    syn::AngleBracketedGenericArguments => visit_angle_bracketed_generic_arguments_mut,
    syn::Arm => visit_arm_mut,
    syn::AssocConst => visit_assoc_const_mut,
    syn::AssocType => visit_assoc_type_mut,
    syn::AttrStyle => visit_attr_style_mut,
    syn::Attribute => visit_attribute_mut,
    Vec<syn::Attribute> => visit_attributes_mut,
    syn::BareFnArg => visit_bare_fn_arg_mut,
    syn::BareVariadic => visit_bare_variadic_mut,
    syn::BinOp => visit_bin_op_mut,
    syn::Block => visit_block_mut,
    syn::BoundLifetimes => visit_bound_lifetimes_mut,
    syn::CapturedParam => visit_captured_param_mut,
    syn::ConstParam => visit_const_param_mut,
    syn::Constraint => visit_constraint_mut,
    syn::Data => visit_data_mut,
    syn::DataEnum => visit_data_enum_mut,
    syn::DataStruct => visit_data_struct_mut,
    syn::DataUnion => visit_data_union_mut,
    syn::DeriveInput => visit_derive_input_mut,
    syn::Expr => visit_expr_mut,
    syn::ExprArray => visit_expr_array_mut,
    syn::ExprAssign => visit_expr_assign_mut,
    syn::ExprAsync => visit_expr_async_mut,
    syn::ExprAwait => visit_expr_await_mut,
    syn::ExprBinary => visit_expr_binary_mut,
    syn::ExprBlock => visit_expr_block_mut,
    syn::ExprBreak => visit_expr_break_mut,
    syn::ExprCall => visit_expr_call_mut,
    syn::ExprCast => visit_expr_cast_mut,
    syn::ExprClosure => visit_expr_closure_mut,
    syn::ExprConst => visit_expr_const_mut,
    syn::ExprContinue => visit_expr_continue_mut,
    syn::ExprField => visit_expr_field_mut,
    syn::ExprForLoop => visit_expr_for_loop_mut,
    syn::ExprGroup => visit_expr_group_mut,
    syn::ExprIf => visit_expr_if_mut,
    syn::ExprIndex => visit_expr_index_mut,
    syn::ExprInfer => visit_expr_infer_mut,
    syn::ExprLet => visit_expr_let_mut,
    syn::ExprLit => visit_expr_lit_mut,
    syn::ExprLoop => visit_expr_loop_mut,
    syn::ExprMacro => visit_expr_macro_mut,
    syn::ExprMatch => visit_expr_match_mut,
    syn::ExprMethodCall => visit_expr_method_call_mut,
    syn::ExprParen => visit_expr_paren_mut,
    syn::ExprPath => visit_expr_path_mut,
    syn::ExprRange => visit_expr_range_mut,
    syn::ExprRawAddr => visit_expr_raw_addr_mut,
    syn::ExprReference => visit_expr_reference_mut,
    syn::ExprRepeat => visit_expr_repeat_mut,
    syn::ExprReturn => visit_expr_return_mut,
    syn::ExprStruct => visit_expr_struct_mut,
    syn::ExprTry => visit_expr_try_mut,
    syn::ExprTryBlock => visit_expr_try_block_mut,
    syn::ExprTuple => visit_expr_tuple_mut,
    syn::ExprUnary => visit_expr_unary_mut,
    syn::ExprUnsafe => visit_expr_unsafe_mut,
    syn::ExprWhile => visit_expr_while_mut,
    syn::ExprYield => visit_expr_yield_mut,
    syn::Field => visit_field_mut,
    syn::FieldMutability => visit_field_mutability_mut,
    syn::FieldPat => visit_field_pat_mut,
    syn::FieldValue => visit_field_value_mut,
    syn::Fields => visit_fields_mut,
    syn::FieldsNamed => visit_fields_named_mut,
    syn::FieldsUnnamed => visit_fields_unnamed_mut,
    syn::File => visit_file_mut,
    syn::FnArg => visit_fn_arg_mut,
    syn::ForeignItem => visit_foreign_item_mut,
    syn::ForeignItemFn => visit_foreign_item_fn_mut,
    syn::ForeignItemMacro => visit_foreign_item_macro_mut,
    syn::ForeignItemStatic => visit_foreign_item_static_mut,
    syn::ForeignItemType => visit_foreign_item_type_mut,
    syn::GenericArgument => visit_generic_argument_mut,
    syn::GenericParam => visit_generic_param_mut,
    syn::Generics => visit_generics_mut,
    syn::Ident => visit_ident_mut,
    syn::ImplItem => visit_impl_item_mut,
    syn::ImplItemConst => visit_impl_item_const_mut,
    syn::ImplItemFn => visit_impl_item_fn_mut,
    syn::ImplItemMacro => visit_impl_item_macro_mut,
    syn::ImplItemType => visit_impl_item_type_mut,
    syn::ImplRestriction => visit_impl_restriction_mut,
    syn::Index => visit_index_mut,
    syn::Item => visit_item_mut,
    syn::ItemConst => visit_item_const_mut,
    syn::ItemEnum => visit_item_enum_mut,
    syn::ItemExternCrate => visit_item_extern_crate_mut,
    syn::ItemFn => visit_item_fn_mut,
    syn::ItemForeignMod => visit_item_foreign_mod_mut,
    syn::ItemImpl => visit_item_impl_mut,
    syn::ItemMacro => visit_item_macro_mut,
    syn::ItemMod => visit_item_mod_mut,
    syn::ItemStatic => visit_item_static_mut,
    syn::ItemStruct => visit_item_struct_mut,
    syn::ItemTrait => visit_item_trait_mut,
    syn::ItemTraitAlias => visit_item_trait_alias_mut,
    syn::ItemType => visit_item_type_mut,
    syn::ItemUnion => visit_item_union_mut,
    syn::ItemUse => visit_item_use_mut,
    syn::Label => visit_label_mut,
    syn::Lifetime => visit_lifetime_mut,
    syn::LifetimeParam => visit_lifetime_param_mut,
    syn::LitBool => visit_lit_bool_mut,
    syn::LitByte => visit_lit_byte_mut,
    syn::LitByteStr => visit_lit_byte_str_mut,
    syn::LitCStr => visit_lit_cstr_mut,
    syn::LitChar => visit_lit_char_mut,
    syn::LitInt => visit_lit_int_mut,
    syn::LitStr => visit_lit_str_mut,
    syn::Local => visit_local_mut,
    syn::LocalInit => visit_local_init_mut,
    syn::Macro => visit_macro_mut,
    syn::MacroDelimiter => visit_macro_delimiter_mut,
    syn::Member => visit_member_mut,
    syn::Meta => visit_meta_mut,
    syn::MetaList => visit_meta_list_mut,
    syn::MetaNameValue => visit_meta_name_value_mut,
    syn::ParenthesizedGenericArguments => visit_parenthesized_generic_arguments_mut,
    syn::Pat => visit_pat_mut,
    syn::PatIdent => visit_pat_ident_mut,
    syn::PatOr => visit_pat_or_mut,
    syn::PatParen => visit_pat_paren_mut,
    syn::PatReference => visit_pat_reference_mut,
    syn::PatRest => visit_pat_rest_mut,
    syn::PatSlice => visit_pat_slice_mut,
    syn::PatStruct => visit_pat_struct_mut,
    syn::PatTuple => visit_pat_tuple_mut,
    syn::PatTupleStruct => visit_pat_tuple_struct_mut,
    syn::PatType => visit_pat_type_mut,
    syn::PatWild => visit_pat_wild_mut,
    syn::Path => visit_path_mut,
    syn::PathArguments => visit_path_arguments_mut,
    syn::PathSegment => visit_path_segment_mut,
    syn::PointerMutability => visit_pointer_mutability_mut,
    syn::PreciseCapture => visit_precise_capture_mut,
    syn::PredicateLifetime => visit_predicate_lifetime_mut,
    syn::PredicateType => visit_predicate_type_mut,
    syn::QSelf => visit_qself_mut,
    syn::RangeLimits => visit_range_limits_mut,
    syn::Receiver => visit_receiver_mut,
    syn::ReturnType => visit_return_type_mut,
    syn::Signature => visit_signature_mut,
    syn::StaticMutability => visit_static_mutability_mut,
    syn::Stmt => visit_stmt_mut,
    syn::StmtMacro => visit_stmt_macro_mut,
    syn::TraitBound => visit_trait_bound_mut,
    syn::TraitBoundModifier => visit_trait_bound_modifier_mut,
    syn::TraitItem => visit_trait_item_mut,
    syn::TraitItemConst => visit_trait_item_const_mut,
    syn::TraitItemFn => visit_trait_item_fn_mut,
    syn::TraitItemMacro => visit_trait_item_macro_mut,
    syn::TraitItemType => visit_trait_item_type_mut,
    syn::Type => visit_type_mut,
    syn::TypeArray => visit_type_array_mut,
    syn::TypeBareFn => visit_type_bare_fn_mut,
    syn::TypeGroup => visit_type_group_mut,
    syn::TypeImplTrait => visit_type_impl_trait_mut,
    syn::TypeInfer => visit_type_infer_mut,
    syn::TypeMacro => visit_type_macro_mut,
    syn::TypeNever => visit_type_never_mut,
    syn::TypeParam => visit_type_param_mut,
    syn::TypeParamBound => visit_type_param_bound_mut,
    syn::TypeParen => visit_type_paren_mut,
    syn::TypePath => visit_type_path_mut,
    syn::TypePtr => visit_type_ptr_mut,
    syn::TypeReference => visit_type_reference_mut,
    syn::TypeSlice => visit_type_slice_mut,
    syn::TypeTraitObject => visit_type_trait_object_mut,
    syn::TypeTuple => visit_type_tuple_mut,
    syn::UnOp => visit_un_op_mut,
    syn::UseGlob => visit_use_glob_mut,
    syn::UseGroup => visit_use_group_mut,
    syn::UseName => visit_use_name_mut,
    syn::UsePath => visit_use_path_mut,
    syn::UseRename => visit_use_rename_mut,
    syn::UseTree => visit_use_tree_mut,
    syn::Variadic => visit_variadic_mut,
    syn::Variant => visit_variant_mut,
    syn::VisRestricted => visit_vis_restricted_mut,
    syn::Visibility => visit_visibility_mut,
    syn::WhereClause => visit_where_clause_mut,
    syn::WherePredicate => visit_where_predicate_mut,
}

// ===============================================================================
// ````````````````````````````````` ATTRIBUTES ``````````````````````````````````
// ===============================================================================

/// Provides the ability to append a single attribute to a syntax node.
pub trait PushAttribute {
    fn push_attribute(&mut self, op: &mut AttributePush);
}

/// Provides the ability to remove and retrieve the last attribute from a
/// syntax node.
pub trait PopAttribute {
    fn pop_attribute(&mut self, op: &mut AttributePop);
}

/// Provides the ability to insert a single attribute at a specified index
/// within a syntax node's attribute collection.
pub trait InsertAttribute {
    fn insert_attribute(&mut self, op: &mut AttributeInsert);
}

/// Provides the ability to prepend a single attribute to a syntax node.
pub trait PrependAttribute {
    fn prepend_attribute(&mut self, op: &mut AttributePrepend);
}

/// Provides the ability to prepend multiple attributes to a syntax node.
pub trait PrependAttributes {
    fn prepend_attributes(&mut self, op: &mut AttributesPrepend);
}

/// Provides the ability to retrieve an attribute at a specified index from a
/// syntax node.
pub trait GetAttribute {
    fn get_attribute(&mut self, op: &mut AttributeGet);
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ``````````````````````````````````` STRUCT ````````````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// Operation that appends a single attribute to an attribute collection.
pub struct AttributePush {
    pub attr: Attribute,
}

impl VisitMut for AttributePush {
    fn visit_attributes_mut(&mut self, i: &mut Vec<syn::Attribute>) {
        i.push(self.attr.clone())
    }
}

/// Operation that removes and retrieves the last attribute from an attribute
/// collection.
pub struct AttributePop {
    pub attr: Option<Attribute>,
}

impl VisitMut for AttributePop {
    fn visit_attributes_mut(&mut self, i: &mut Vec<syn::Attribute>) {
        self.attr = i.pop();
    }
}

/// Operation that prepends a single attribute to an attribute collection.
pub struct AttributePrepend {
    pub attr: Attribute,
}

impl VisitMut for AttributePrepend {
    fn visit_attributes_mut(&mut self, i: &mut Vec<syn::Attribute>) {
        i.insert(0, self.attr.clone())
    }
}

/// Operation that prepends multiple attributes to an attribute collection.
///
/// The attributes supplied to the operation are moved before the existing
/// attributes while preserving the relative order of both collections.
pub struct AttributesPrepend {
    pub attrs: Vec<Attribute>,
}

impl VisitMut for AttributesPrepend {
    fn visit_attributes_mut(&mut self, attrs: &mut Vec<syn::Attribute>) {
        let mut prepend = std::mem::take(&mut self.attrs);
        prepend.append(attrs);
        *attrs = prepend;
    }
}

/// Operation that inserts a single attribute at a specified index in an
/// attribute collection.
pub struct AttributeInsert {
    pub index: usize,
    pub attr: Attribute,
}

impl VisitMut for AttributeInsert {
    fn visit_attributes_mut(&mut self, attrs: &mut Vec<syn::Attribute>) {
        attrs.insert(self.index, self.attr.clone());
    }
}

/// Operation that retrieves an attribute at a specified index.
pub struct AttributeGet {
    pub index: usize,
    pub attr: Option<Attribute>,
}

impl VisitMut for AttributeGet {
    fn visit_attributes_mut(&mut self, attrs: &mut Vec<syn::Attribute>) {
        self.attr = attrs.get(self.index).cloned();
    }
}

// ===============================================================================
// ``````````````````````````````` DOC ATTRIBUTES ````````````````````````````````
// ===============================================================================

/// Provides the ability to retrieve all Rustdoc attributes from a syntax node.
///
/// The returned operation records each matching attribute together with its
/// original index in the attribute collection.
pub trait DocsAttributes {
    fn doc_attributes(&mut self, op: &mut AttributesDocs);
}

/// Provides the ability to locate the first Rustdoc attribute on a syntax
/// node.
///
/// The returned operation records the index of the first matching attribute,
/// if one exists.
pub trait FirstDocAttribute {
    fn first_doc_attribute(&mut self, op: &mut AttributesFirstDoc);
}

/// Provides the ability to remove all Rustdoc attributes from a syntax node.
pub trait ClearDocAttributes {
    fn clear_doc_attributes(&mut self, op: &mut AttributesDocsClear);
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ``````````````````````````````````` STRUCT ````````````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// Operation that retrieves all Rustdoc attributes from an attribute
/// collection.
///
/// Each matching attribute is returned together with its original index.
pub struct AttributesDocs {
    pub value: Option<Vec<(usize, Attribute)>>,
}

impl VisitMut for AttributesDocs {
    fn visit_attributes_mut(&mut self, attrs: &mut Vec<syn::Attribute>) {
        let docs: Vec<_> = attrs
            .iter()
            .enumerate()
            .filter(|(_, attr)| is_doc_attr(attr))
            .map(|(index, attr)| (index, attr.clone()))
            .collect();

        self.value = (!docs.is_empty()).then_some(docs);
    }
}

/// Operation that locates the first Rustdoc attribute in an attribute
/// collection.
///
/// The resulting index is `None` when the collection contains no Rustdoc
/// attributes.
pub struct AttributesFirstDoc {
    pub index: Option<usize>,
}

impl VisitMut for AttributesFirstDoc {
    fn visit_attributes_mut(&mut self, attrs: &mut Vec<syn::Attribute>) {
        self.index = attrs.iter().position(is_doc_attr);
    }
}

/// Operation that removes all Rustdoc attributes from an attribute
/// collection.
///
/// Non-documentation attributes are preserved in their original order.
pub struct AttributesDocsClear;

impl VisitMut for AttributesDocsClear {
    fn visit_attributes_mut(&mut self, attrs: &mut Vec<Attribute>) {
        attrs.retain(|attr| !is_doc_attr(attr));
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ```````````````````````````````````` FUNCS ````````````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// Detects whether an attribute is:
///
/// #[doc = "..."]
fn is_doc_attr(attr: &Attribute) -> bool {
    match &attr.meta {
        syn::Meta::NameValue(meta) => meta.path.is_ident("doc"),
        _ => false,
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ``````````````````````````````````` MACROS ````````````````````````````````````
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

macro_rules! impl_attribute {
    ($($ty:ty $(=> $field:tt)?),* $(,)?) => {
        $(
            impl PushAttribute for $ty {
                fn push_attribute(
                    &mut self,
                    op: &mut AttributePush,
                ) {
                    impl_attribute!(@call op, self $(.$field)?);
                }
            }

            impl PopAttribute for $ty {
                fn pop_attribute(
                    &mut self,
                    op: &mut AttributePop,
                ) {
                    impl_attribute!(@call op, self $(.$field)?);
                }
            }

            impl InsertAttribute for $ty {
                fn insert_attribute(
                    &mut self,
                    op: &mut AttributeInsert,
                ) {
                    impl_attribute!(@call op, self $(.$field)?);
                }
            }

            impl PrependAttribute for $ty {
                fn prepend_attribute(
                    &mut self,
                    op: &mut AttributePrepend,
                ) {
                    impl_attribute!(@call op, self $(.$field)?);
                }
            }

            impl PrependAttributes for $ty {
                fn prepend_attributes(
                    &mut self,
                    op: &mut AttributesPrepend,
                ) {
                    impl_attribute!(@call op, self $(.$field)?);
                }
            }

            impl DocsAttributes for $ty {
                fn doc_attributes(
                    &mut self,
                    op: &mut AttributesDocs,
                ) {
                    impl_attribute!(@call op, self $(.$field)?);
                }
            }

            impl FirstDocAttribute for $ty {
                fn first_doc_attribute(
                    &mut self,
                    op: &mut AttributesFirstDoc,
                ) {
                    impl_attribute!(@call op, self $(.$field)?);
                }
            }

            impl GetAttribute for $ty {
                fn get_attribute(
                    &mut self,
                    op: &mut AttributeGet,
                ) {
                    impl_attribute!(@call op, self $(.$field)?);
                }
            }

            impl ClearDocAttributes for $ty {
                fn clear_doc_attributes(
                    &mut self,
                    op: &mut AttributesDocsClear,
                ) {
                    impl_attribute!(@call op, self $(.$field)?);
                }
            }
        )*
    };

    (@call $visitor:ident, $self:ident) => {
        $visitor.visit_attributes_mut($self);
    };

    (@call $visitor:ident, $self:ident .$field:ident) => {
        $visitor.visit_attributes_mut(&mut $self.$field);
    };
}

impl_attribute! {
    syn::Arm => attrs,
    Vec<syn::Attribute>,
    syn::BareFnArg => attrs,
    syn::BareVariadic => attrs,
    syn::ConstParam => attrs,
    syn::DeriveInput => attrs,
    syn::ExprArray => attrs,
    syn::ExprAssign => attrs,
    syn::ExprAsync => attrs,
    syn::ExprAwait => attrs,
    syn::ExprBinary => attrs,
    syn::ExprBlock => attrs,
    syn::ExprBreak => attrs,
    syn::ExprCall => attrs,
    syn::ExprCast => attrs,
    syn::ExprClosure => attrs,
    syn::ExprConst => attrs,
    syn::ExprContinue => attrs,
    syn::ExprField => attrs,
    syn::ExprForLoop => attrs,
    syn::ExprGroup => attrs,
    syn::ExprIf => attrs,
    syn::ExprIndex => attrs,
    syn::ExprInfer => attrs,
    syn::ExprLet => attrs,
    syn::ExprLit => attrs,
    syn::ExprLoop => attrs,
    syn::ExprMacro => attrs,
    syn::ExprMatch => attrs,
    syn::ExprMethodCall => attrs,
    syn::ExprParen => attrs,
    syn::ExprPath => attrs,
    syn::ExprRange => attrs,
    syn::ExprRawAddr => attrs,
    syn::ExprReference => attrs,
    syn::ExprRepeat => attrs,
    syn::ExprReturn => attrs,
    syn::ExprStruct => attrs,
    syn::ExprTry => attrs,
    syn::ExprTryBlock => attrs,
    syn::ExprTuple => attrs,
    syn::ExprUnary => attrs,
    syn::ExprUnsafe => attrs,
    syn::ExprWhile => attrs,
    syn::ExprYield => attrs,
    syn::Field => attrs,
    syn::FieldPat => attrs,
    syn::FieldValue => attrs,
    syn::File => attrs,
    syn::ForeignItemFn => attrs,
    syn::ForeignItemMacro => attrs,
    syn::ForeignItemStatic => attrs,
    syn::ForeignItemType => attrs,
    syn::ImplItemConst => attrs,
    syn::ImplItemFn => attrs,
    syn::ImplItemMacro => attrs,
    syn::ImplItemType => attrs,
    syn::ItemConst => attrs,
    syn::ItemEnum => attrs,
    syn::ItemExternCrate => attrs,
    syn::ItemFn => attrs,
    syn::ItemForeignMod => attrs,
    syn::ItemImpl => attrs,
    syn::ItemMacro => attrs,
    syn::ItemMod => attrs,
    syn::ItemStatic => attrs,
    syn::ItemStruct => attrs,
    syn::ItemTrait => attrs,
    syn::ItemTraitAlias => attrs,
    syn::ItemType => attrs,
    syn::ItemUnion => attrs,
    syn::ItemUse => attrs,
    syn::LifetimeParam => attrs,
    syn::Local => attrs,
    syn::PatIdent => attrs,
    syn::PatOr => attrs,
    syn::PatParen => attrs,
    syn::PatReference => attrs,
    syn::PatRest => attrs,
    syn::PatSlice => attrs,
    syn::PatStruct => attrs,
    syn::PatTuple => attrs,
    syn::PatTupleStruct => attrs,
    syn::PatType => attrs,
    syn::PatWild => attrs,
    syn::Receiver => attrs,
    syn::StmtMacro => attrs,
    syn::TraitItemConst => attrs,
    syn::TraitItemFn => attrs,
    syn::TraitItemMacro => attrs,
    syn::TraitItemType => attrs,
    syn::TypeParam => attrs,
    syn::Variadic => attrs,
    syn::Variant => attrs,
}
