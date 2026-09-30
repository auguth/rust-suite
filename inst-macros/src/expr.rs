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
// ``````````````````````````````````` INST EXPR `````````````````````````````````
// ===============================================================================

//! Provides expression transformations for inst-direct and inst-get access.
//!
//! The module transforms instance expressions by resolving instance-trait
//! and instance-node associated accesses and lowering them to the corresponding
//! internal instance access support function macros.
//!
//! Direct instance expressions resolve associated accesses against explicitly
//! supplied instance arguments, while instance-get expressions resolve
//! associated paths relative to an instance-node associated type and forward
//! the supplied input arguments.
//!
//! The transformed instance-get representation may also introduce a scoped
//! import when a qualified associated item requires its path qualifier to be
//! brought into scope.

// ===============================================================================
// ``````````````````````````````````` IMPORTS ```````````````````````````````````
// ===============================================================================

// --- Proc Macro Crates ---
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    Expr, ExprPath, LitByteStr, Path, PathArguments, Type, parse_quote,
    punctuated::Punctuated,
    spanned::Spanned,
    token::{Comma, PathSep},
    visit_mut::VisitMut,
};

// --- Local Crate ---
use crate::{
    Inst, InstDirectArgs, InstGetArgs, Transformation,
    args::{BStrInputList, InstanceIdent, InstanceIdents, InstanceModel},
    errors::{ExprBug, ExprError},
    node::AccessArgsRef,
    traits::append_placeholder_args_to_trait,
};

// --- Proc Suite ---
use proc_suite::{IntList, SupportCrate};

// ===============================================================================
// ``````````````````````````````` INST DIRECT EXPR ``````````````````````````````
// ===============================================================================

/// Transforms direct instance expressions by resolving instance-trait
/// associated accesses against the explicitly supplied instance arguments.
#[derive(Debug, Clone)]
pub struct InstDirectExpr;

/// Visits paths within a direct instance expression and resolves
/// instance-trait associated accesses to concrete instance arguments.
struct VistDirectExprPath<'a> {
    /// Stores the first transformation error encountered during traversal.
    err: Option<TokenStream>,

    /// Instance model used to resolve instance arguments for associated accesses.
    model: &'a InstanceModel,

    /// Instance argument indexes generated while resolving the associated access.
    indexes: Option<IntList>,
}

impl<'a> VisitMut for VistDirectExprPath<'a> {
    /// Visits supported expression forms recursively while resolving
    /// instance-trait associated items.
    ///
    /// Call arguments are deliberately not traversed because their argument
    /// expressions are not part of the direct instance path being resolved.
    fn visit_expr_mut(&mut self, i: &mut syn::Expr) {
        match i {
            Expr::Call(expr_call) => {
                self.visit_expr_mut(&mut expr_call.func);
                // deliberately don't visit node.args
            }
            Expr::Path(expr_path) => {
                self.visit_expr_path_mut(expr_path);
            }
            Expr::Reference(expr_reference) => {
                self.visit_expr_mut(&mut expr_reference.expr);
            }
            Expr::Return(expr_return) => {
                if let Some(expr) = &mut expr_return.expr {
                    self.visit_expr_mut(&mut *expr);
                }
            }
            Expr::Try(expr_try) => {
                self.visit_expr_mut(&mut expr_try.expr);
            }
            Expr::Tuple(expr_tuple) => {
                for expr in &mut expr_tuple.elems {
                    self.visit_expr_mut(expr);
                }
            }
            Expr::Group(expr_group) => {
                self.visit_expr_mut(&mut expr_group.expr);
            }
            expr => {
                self.err = Some(ExprError::UnsupportedExprYet { expr: expr.clone() }.into());
                return;
            }
        }
    }

    /// Resolves an instance-trait associated path by inserting the
    /// corresponding placeholder instance arguments into the trait path.
    fn visit_expr_path_mut(&mut self, i: &mut syn::ExprPath) {
        let Some(qself) = &i.qself else {
            self.err =
                Some(ExprError::InstTypeInstTraitAssociatedAccess { expr: i.clone() }.into());
            return;
        };

        if qself.as_token.is_none() {
            self.err = Some(
                ExprError::InstTypeQSelfButNotInstTraitAssocAccess {
                    ty: *qself.ty.clone(),
                }
                .into(),
            );
            return;
        }

        let Type::Path(ty) = &*qself.ty else {
            self.err = Some(
                ExprError::InstTypeShouldBeConcrete {
                    ty: *qself.ty.clone(),
                }
                .into(),
            );
            return;
        };

        if ty.qself.is_some() {
            self.err = Some(
                ExprError::InstTypeShouldBeConcrete {
                    ty: *qself.ty.clone(),
                }
                .into(),
            );
            return;
        }

        let pos = qself.position;
        let trait_ = i
            .path
            .segments
            .iter()
            .take(pos)
            .cloned()
            .collect::<Punctuated<_, PathSep>>();
        let assoc = i.path.segments.iter().skip(pos);

        let mut path = Path {
            leading_colon: None,
            segments: trait_,
        };

        match append_placeholder_args_to_trait(&mut path, &self.model, true) {
            Ok(indexes) => self.indexes = Some(indexes),
            Err(err) => {
                self.err = Some(err);
                return;
            }
        };

        for seg in assoc {
            path.segments.push(seg.clone());
        }

        i.path = path;

        return;
    }
}

impl Transformation<TokenStream, InstDirectArgs> for InstDirectExpr {
    fn raw_transform(
        &self,
        transform: &mut TokenStream,
        context: &InstDirectArgs,
    ) -> Result<(), proc_macro2::TokenStream> {
        let mut params: Punctuated<InstanceIdent, Comma> = Punctuated::new();
        let span = context.idents.span();
        for ident in &context.idents.idents {
            params.push(InstanceIdent::Compile(ident.clone()));
        }

        let model = InstanceModel::Simple(InstanceIdents {
            span,
            paren_token: Default::default(),
            params,
        });

        let mut expr = context.expr.clone();
        let mut visitor = VistDirectExprPath {
            err: None,
            indexes: None,
            model: &model,
        };
        visitor.visit_expr_mut(&mut expr);
        if let Some(err) = &visitor.err {
            return Err(err.clone());
        };

        let Some(indexes) = &visitor.indexes else {
            unreachable!()
        };

        let access = AccessArgsRef(model);
        let Some(args) = access.try_leaf(indexes) else {
            unreachable!()
        };

        let support_crate = Inst::support_crate();

        *transform = quote! {
            #support_crate::__instance_direct_internal_only!([#args] #expr)
        };

        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &TokenStream,
        _: Option<&InstDirectArgs>,
    ) -> Result<(), proc_macro2::TokenStream> {
        #[allow(unused)]
        struct DirectExpr {
            path: syn::Path,
            bang: syn::Token![!],
            bracket: syn::token::Bracket,
            bracket_content: TokenStream,
            expr: syn::Expr,
        }

        impl syn::parse::Parse for DirectExpr {
            fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
                let path: syn::Path = input.parse()?;
                let bang: syn::Token![!] = input.parse()?;

                let content;
                syn::parenthesized!(content in input);

                let bracket_content;
                syn::bracketed!(bracket_content in content);

                let bracket_content_ts = bracket_content.parse()?;
                let expr: syn::Expr = content.parse()?;

                if !content.is_empty() {
                    return Err(content.error("unexpected tokens after expression"));
                }

                Ok(Self {
                    path,
                    bang,
                    bracket: syn::token::Bracket::default(),
                    bracket_content: bracket_content_ts,
                    expr,
                })
            }
        }

        let Ok(_) = syn::parse2::<DirectExpr>(transform.clone()) else {
            return Err(ExprBug::DirectExprFailed {}.into());
        };

        Ok(())
    }
}

// ===============================================================================
// ````````````````````````````````` INST GET EXPR ```````````````````````````````
// ===============================================================================

/// Transforms instance-get expressions by resolving instance-node associated
/// accesses and lowering them to the internal instance-get support macro.
#[derive(Debug, Clone)]
pub struct InstGetExpr;

/// Visits paths within an instance-get expression and rewrites them relative
/// to the instance-node associated type used by the expression.
///
/// The visitor also records a required import path when a qualified item
/// requires its qualifier to be brought into scope.
struct VistGetExprPath<'a> {
    /// Stores the first transformation error encountered during traversal.
    err: Option<TokenStream>,

    /// Instance-node associated type used as the prefix for resolved paths.
    prefix: &'a ExprPath,

    /// Input arguments supplied to the instance-get expression.
    list: &'a BStrInputList,

    /// Import path required when resolving a qualified associated item.
    import: Option<Path>,
}

impl<'a> VisitMut for VistGetExprPath<'a> {
    fn visit_expr_mut(&mut self, i: &mut syn::Expr) {
        match i {
            Expr::Call(expr_call) => {
                self.visit_expr_mut(&mut expr_call.func);
                // deliberately don't visit node.args
            }
            Expr::Path(expr_path) => {
                if let Some(qself) = &expr_path.qself {
                    self.err = Some(
                        ExprError::PathQualifiedNotSelfQualified {
                            qself: *qself.ty.clone(),
                        }
                        .into(),
                    );
                    return;
                }

                if self.list.exprs.is_empty() {
                    let len = expr_path.path.segments.len();
                    if len > 1 {
                        self.err = Some(
                            ExprError::PathQualifierUnsupported {
                                seg: expr_path.path.segments[len - 2].clone(),
                            }
                            .into(),
                        );
                        return;
                    }
                }

                self.visit_expr_path_mut(expr_path);
            }
            Expr::Reference(expr_reference) => {
                self.visit_expr_mut(&mut expr_reference.expr);
            }
            Expr::Return(expr_return) => {
                if let Some(expr) = &mut expr_return.expr {
                    self.visit_expr_mut(&mut *expr);
                }
            }
            Expr::Try(expr_try) => {
                self.visit_expr_mut(&mut expr_try.expr);
            }
            Expr::Tuple(expr_tuple) => {
                for expr in &mut expr_tuple.elems {
                    self.visit_expr_mut(expr);
                }
            }
            Expr::Group(expr_group) => {
                self.visit_expr_mut(&mut expr_group.expr);
            }
            expr => {
                self.err = Some(ExprError::UnsupportedExprYet { expr: expr.clone() }.into());
                return;
            }
        }
    }
    fn visit_expr_path_mut(&mut self, i: &mut syn::ExprPath) {
        let mut path = i.path.clone();
        let Some(last_pair) = path.segments.pop() else {
            self.err = Some(ExprError::ExpectedPathQualifiedItem { path: path.clone() }.into());
            return;
        };
        path.segments.pop_punct();
        let last_seg = last_pair.value();

        if !path.segments.is_empty() {
            for seg in &path.segments {
                if !matches!(seg.arguments, PathArguments::None) {
                    self.err = Some(ExprError::ExpectedPathQualifier { seg: seg.clone() }.into());
                    return;
                }
            }

            let mut item = last_seg.clone();
            item.arguments = PathArguments::None;
            path.segments.push(item);
            self.import = Some(path);
        }

        let prefix = self.prefix;
        *i = parse_quote!(#prefix::#last_seg);

        return;
    }
}

/// Represents either a byte-string literal or a Rust expression.
///
/// Byte-string literals are used for identifier-based inputs, while
/// expressions are used for raw input values.
enum BStrOrExpr {
    /// A byte-string literal representing an identifier input.
    BStr(LitByteStr),

    /// A Rust expression representing a raw input value.
    Expr(Expr),
}

impl syn::parse::Parse for BStrOrExpr {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if input.peek(LitByteStr) {
            Ok(Self::BStr(input.parse()?))
        } else {
            Ok(Self::Expr(input.parse()?))
        }
    }
}

impl ToTokens for BStrOrExpr {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::BStr(lit) => lit.to_tokens(tokens),
            Self::Expr(expr) => expr.to_tokens(tokens),
        }
    }
}

impl Transformation<TokenStream, InstGetArgs> for InstGetExpr {
    fn raw_transform(
        &self,
        transform: &mut TokenStream,
        context: &InstGetArgs,
    ) -> Result<(), proc_macro2::TokenStream> {
        let expr_path = &context.path;

        let Some(qself) = &expr_path.qself else {
            return Err(ExprError::InstNodeTraitAssociatedType {
                expr: expr_path.clone(),
            }
            .into());
        };

        if qself.as_token.is_none() {
            return Err(ExprError::InstNodeQSelfButNotTraitAssoc {
                ty: *qself.ty.clone(),
            }
            .into());
        };

        let mut expr = context.expr.clone();

        let list = &context.inputs;
        let mut visitor = VistGetExprPath {
            err: None,
            prefix: &expr_path,
            import: None,
            list,
        };
        visitor.visit_expr_mut(&mut expr);

        if let Some(err) = &visitor.err {
            return Err(err.clone());
        };

        let support_crate = Inst::support_crate();

        let mut collect: Punctuated<BStrOrExpr, Comma> = Punctuated::new();
        for l in &list.exprs {
            match l {
                crate::args::BStrInput::Raw(expr) => collect.push(BStrOrExpr::Expr(expr.clone())),
                crate::args::BStrInput::Ident(ident) => {
                    let ident_str = ident.to_string();
                    let ident_bytes = ident_str.as_bytes();
                    collect.push(BStrOrExpr::BStr(LitByteStr::new(ident_bytes, ident.span())))
                }
            }
        }

        let Some(import_path) = &visitor.import else {
            *transform = quote! {
                #support_crate::__instance_get_internal_only!(#expr , #collect)
            };
            return Ok(());
        };

        *transform = quote! {
            {
                use #import_path;

                #support_crate::__instance_get_internal_only!(#expr , #collect)
            }
        };

        Ok(())
    }

    fn validate_transform(
        &self,
        transform: &TokenStream,
        _: Option<&InstGetArgs>,
    ) -> Result<(), proc_macro2::TokenStream> {
        #[allow(unused)]
        enum GetExpr {
            Direct {
                path: syn::Path,
                expr: syn::Expr,
                inputs: syn::punctuated::Punctuated<BStrOrExpr, syn::Token![,]>,
            },
            Scoped {
                use_path: syn::Path,
                path: syn::Path,
                expr: syn::Expr,
                inputs: syn::punctuated::Punctuated<BStrOrExpr, syn::Token![,]>,
            },
        }

        impl syn::parse::Parse for GetExpr {
            fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
                if input.peek(syn::token::Brace) {
                    let content;
                    syn::braced!(content in input);

                    content.parse::<syn::Token![use]>()?;
                    let use_path = content.parse::<syn::Path>()?;
                    content.parse::<syn::Token![;]>()?;

                    let path = content.parse::<syn::Path>()?;
                    content.parse::<syn::Token![!]>()?;

                    let args;
                    syn::parenthesized!(args in content);

                    let expr = args.parse::<syn::Expr>()?;
                    args.parse::<syn::Token![,]>()?;

                    let inputs =
                        syn::punctuated::Punctuated::<BStrOrExpr, syn::Token![,]>::parse_terminated(
                            &args,
                        )?;

                    if !content.is_empty() {
                        return Err(content.error("unexpected tokens"));
                    }

                    Ok(Self::Scoped {
                        use_path,
                        path,
                        expr,
                        inputs,
                    })
                } else {
                    let path = input.parse::<syn::Path>()?;
                    input.parse::<syn::Token![!]>()?;

                    let args;
                    syn::parenthesized!(args in input);

                    let expr = args.parse::<syn::Expr>()?;
                    args.parse::<syn::Token![,]>()?;

                    let inputs =
                        syn::punctuated::Punctuated::<BStrOrExpr, syn::Token![,]>::parse_terminated(
                            &args,
                        )?;

                    if !input.is_empty() {
                        return Err(input.error("unexpected tokens"));
                    }

                    Ok(Self::Direct { path, expr, inputs })
                }
            }
        }

        let Ok(_) = syn::parse2::<GetExpr>(transform.clone()) else {
            return Err(ExprBug::AccessExprFailed {}.into());
        };

        Ok(())
    }
}
