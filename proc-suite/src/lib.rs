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
// `````````````````````````````` PROC-MACRO SUITE ```````````````````````````````
// ===============================================================================

//! The Proc Macro Suite
//! 
//! For Serious Proc-Macro Builders
//! 
//! Module Doc (TBD)

// ===============================================================================
// `````````````````````````````````` MODULES ````````````````````````````````````
// ===============================================================================

pub mod delim;
pub mod docs;
pub mod errors;
pub mod idents;
pub mod keys;
pub mod lists;
pub mod misc;
pub mod pipeline;
pub(crate) mod space;
pub mod syns;

// ===============================================================================
// `````````````````````````````````` IMPORTS ````````````````````````````````````
// ===============================================================================

// --- Proc Macro Crates ---
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::parse_quote;

// ===============================================================================
// `````````````````````````````````` RE-EXPORTS `````````````````````````````````
// ===============================================================================

pub use delim::*;
pub use docs::*;
pub use errors::*;
pub use idents::*;
pub use keys::*;
pub use lists::*;
pub use misc::*;
pub use pipeline::*;

// ===============================================================================
// `````````````````````````````` PUBLIC FN UTILS ````````````````````````````````
// ===============================================================================

/// Mutates an item through a fallible operation and returns its final
/// [`TokenStream`] representation.
///
/// The supplied function receives the item by mutable reference and performs
/// the transformation in place. This is useful for proc-macro transformation
/// stages where an existing [`syn`] syntax-tree item must be modified and then
/// emitted as generated tokens.
///
/// When the `dev` feature is enabled with `--features dev`, the item can be
/// rendered with `prettyplease` before and after the mutation for development
/// debugging. The `pre_debug` and `post_debug` flags control whether the
/// corresponding representations are printed, while `_label` identifies the
/// transformation in the debug output.
///
/// For example, an item can be mutated by passing a closure that modifies it:
///
/// ```ignore
/// let tokens = mutate_item(
///     "InstanceAccess",
///     &mut item,
///     |item| {
///         item.attrs.push(parse_quote!(#[doc(hidden)]));
///         Ok(())
///     },
///     true,
///     true,
/// );
/// ```
///
/// The closure may perform any transformation supported by the item type and
/// returns `Ok(())` when the mutation succeeds:
///
/// ```ignore
/// |item| {
///     transform(item)?;
///     validate(item)?;
///     Ok(())
/// }
/// ```
///
/// If a transformation fails, the closure can return the generated error
/// [`TokenStream`]. The error is returned immediately without emitting the
/// partially transformed item:
///
/// ```ignore
/// |item| {
///     if invalid(item) {
///         return Err(compile_error!("invalid item"));
///     }
///
///     Ok(())
/// }
/// ```
///
/// On success, the mutated item is converted into a [`TokenStream`] through
/// [`ToTokens`]:
///
/// ```ignore
/// let tokens: TokenStream = mutate_item(
///     "MyTransform",
///     &mut item,
///     |item| {
///         transform(item)?;
///         Ok(())
///     },
///     false,
///     false,
/// );
/// ```
///
/// With `--features dev`, enabling `pre_debug` and `post_debug` produces
/// labeled representations similar to:
///
/// ```text
/// +==========+ PRE-MUTATE (MyTransform) +==========+
/// ...
///
/// +==========+ POST-MUTATE (MyTransform) +==========+
/// ...
/// ```
///
/// The debug rendering uses `prettyplease` when the item can be parsed as a
/// [`syn::File`]. If parsing fails, its [`ToTokens`] representation is used
/// instead.
///
/// This function therefore provides a common wrapper for the pattern of
/// mutating an item, optionally inspecting the transformation during
/// development, propagating transformation errors, and finally emitting the
/// transformed item.
pub fn mutate_item<T: ToTokens, F>(
    _label: &str,
    item: &mut T,
    f: F,
    pre_debug: bool,
    post_debug: bool,
) -> TokenStream
where
    F: FnOnce(&mut T) -> Result<(), TokenStream>,
{
    if pre_debug {
        // ------- Debugging (Pre) -------
        #[cfg(feature = "dev")]
        {
            let label = _label;
            let pretty = syn::parse2::<syn::File>(quote!(#item))
                .map(|f| prettyplease::unparse(&f))
                .unwrap_or_else(|_| quote!(#item).to_string());
            eprintln!(
                "\n +==========+ PRE-MUTATE ({}) +==========+ \n {}\n",
                label, pretty
            );
        }
    }

    // Run embedded logic
    if let Err(err) = f(item) {
        return err;
    }

    if post_debug {
        // ------- Debugging (Post) -------
        #[cfg(feature = "dev")]
        {
            let label = _label;
            let pretty = syn::parse2::<syn::File>(quote!(#item))
                .map(|f| prettyplease::unparse(&f))
                .unwrap_or_else(|_| quote!(#item).to_string());

            eprintln!(
                "\n +==========+ POST-MUTATE ({}) +==========+ \n\n{}\n",
                label, pretty,
            );
        }
    }

    // Final expansion
    quote!(#item)
}

/// Creates an item through a fallible construction operation and returns its
/// final [`TokenStream`] representation.
///
/// Unlike [`mutate_item`], this function does not operate on an existing item.
/// The supplied function constructs the item from scratch and returns it on
/// success. This is useful for proc-macro stages that need to generate a new
/// [`syn`] syntax-tree item rather than transform an existing one.
///
/// The construction function is executed exactly once. If it succeeds, the
/// resulting item is converted into a [`TokenStream`] through [`ToTokens`].
/// If it fails, the returned error [`TokenStream`] is propagated immediately.
///
/// For example, a new item can be constructed directly inside the closure:
///
/// ```ignore
/// let tokens = create_item(
///     "GeneratedFunction",
///     || {
///         let item: syn::ItemFn = parse_quote! {
///             fn generated() -> u8 {
///                 42
///             }
///         };
///
///         Ok(item)
///     },
///     false,
/// );
/// ```
///
/// The construction operation can also perform fallible processing before
/// returning the item:
///
/// ```ignore
/// let tokens = create_item(
///     "GeneratedType",
///     || {
///         let item = create_type()?;
///         validate_type(&item)?;
///         Ok(item)
///     },
///     false,
/// );
/// ```
///
/// When the `debug` flag is enabled and the `dev` feature is enabled with
/// `--features dev`, the created item is rendered using `prettyplease` and
/// printed with the supplied `_label`:
///
/// ```ignore
/// let tokens = create_item(
///     "GeneratedFunction",
///     || {
///         Ok(parse_quote! {
///             fn generated() {}
///         })
///     },
///     true,
/// );
/// ```
///
/// The debug output has the form:
///
/// ```text
/// +==========+ CREATED-ITEM (GeneratedFunction) +==========+
///
/// fn generated() {}
/// ```
///
/// If the created tokens cannot be parsed as a [`syn::File`] for pretty
/// printing, the function falls back to the item's [`ToTokens`] representation.
///
/// An empty or otherwise valid item is still emitted normally; the `debug`
/// flag only controls development-time inspection and does not affect the
/// generated [`TokenStream`].
///
/// This function therefore provides a common wrapper for the pattern of
/// constructing a new item, propagating construction errors, optionally
/// inspecting the generated item during development, and finally emitting
/// its token representation.
pub fn create_item<T: ToTokens, F>(_label: &str, f: F, debug: bool) -> TokenStream
where
    F: FnOnce() -> Result<T, TokenStream>,
{
    // Run embedded logic
    let item = match f() {
        Ok(i) => i,
        Err(err) => return err,
    };

    if debug {
        // ------- Debugging (Post) -------
        #[cfg(feature = "dev")]
        {
            let label = _label;
            let pretty = syn::parse2::<syn::File>(quote!(#item))
                .map(|f| prettyplease::unparse(&f))
                .unwrap_or_else(|_| quote!(#item).to_string());

            eprintln!(
                "\n +==========+ CREATED-ITEM ({}) +==========+ \n\n{}\n",
                label, pretty,
            );
        }
    }

    // Final expansion
    quote!(#item)
}

/// Creates multiple items through a fallible construction operation and
/// combines their token representations into a single [`TokenStream`].
///
/// Unlike [`create_item`], which constructs a single item, this function
/// allows the construction operation to produce any [`IntoIterator`] of items.
/// Each item must implement [`ToTokens`], and all successfully created items
/// are emitted consecutively in their iteration order.
///
/// The construction function is executed exactly once. If it succeeds, the
/// returned collection is consumed and collected into a [`Vec`] so that the
/// generated items can be inspected for debugging and emitted afterwards. If
/// it fails, the returned error [`TokenStream`] is propagated immediately.
///
/// For example, multiple functions can be constructed as a collection:
///
/// ```ignore
/// let tokens = create_items(
///     "GeneratedFunctions",
///     || {
///         let items = vec![
///             parse_quote! {
///                 fn first() {}
///             },
///             parse_quote! {
///                 fn second() {}
///             },
///         ];
///
///         Ok(items)
///     },
///     false,
/// );
/// ```
///
/// The construction operation does not need to return a [`Vec`] directly. Any
/// type implementing [`IntoIterator`] can be returned:
///
/// ```ignore
/// let tokens = create_items(
///     "GeneratedItems",
///     || {
///         let items = create_items_from_source()?;
///         Ok(items)
///     },
///     false,
/// );
/// ```
///
/// The generated items are emitted in the same order in which the iterator
/// yields them:
///
/// ```text
/// first item
/// second item
/// third item
/// ```
///
/// When `debug` is enabled and the `dev` feature is enabled with
/// `--features dev`, all created items are rendered together using
/// `prettyplease` and printed with the supplied `_label`:
///
/// ```ignore
/// let tokens = create_items(
///     "GeneratedFunctions",
///     || {
///         Ok(vec![
///             parse_quote! { fn first() {} },
///             parse_quote! { fn second() {} },
///         ])
///     },
///     true,
/// );
/// ```
///
/// The debug output represents the complete generated sequence:
///
/// ```text
/// +==========+ CREATED-ITEMS (GeneratedFunctions) +==========+
///
/// fn first() {}
/// fn second() {}
/// ```
///
/// If the combined tokens cannot be parsed as a [`syn::File`] for pretty
/// printing, the function falls back to their [`ToTokens`] representation.
///
/// If the construction operation produces no items, an empty [`TokenStream`]
/// is returned.
///
/// This function therefore provides a common wrapper for the pattern of
/// constructing multiple items, propagating construction errors, optionally
/// inspecting the complete generated sequence during development, and finally
/// emitting all items as one [`TokenStream`].
pub fn create_items<T, I, F>(_label: &str, f: F, debug: bool) -> TokenStream
where
    T: ToTokens,
    I: IntoIterator<Item = T>,
    F: FnOnce() -> Result<I, TokenStream>,
{
    let items = match f() {
        Ok(items) => items,
        Err(err) => return err,
    };

    let items: Vec<T> = items.into_iter().collect();

    if debug {
        #[cfg(feature = "dev")]
        {
            let label = _label;

            let pretty = syn::parse2::<syn::File>(quote! {
                #(#items)*
            })
            .map(|f| prettyplease::unparse(&f))
            .unwrap_or_else(|_| {
                quote! {
                    #(#items)*
                }
                .to_string()
            });

            eprintln!(
                "\n +==========+ CREATED-ITEMS ({}) +==========+ \n\n{}\n",
                label, pretty,
            );
        }
    }

    if items.is_empty() {
        return TokenStream::new();
    }

    quote! {
        #(#items)*
    }
}

/// Creates paired documentation and non-documentation representations of an
/// item and emits them conditionally according to `cfg(doc)`.
///
/// This function is intended for proc-macro transformations where the item
/// used by the compiler during normal compilation must differ from the item
/// presented by generated Rustdoc. A clone of the original item is created
/// before any mutation so that the two representations can be transformed
/// independently.
///
/// The transformation is performed in three stages:
///
/// 1. `actual` transforms the non-documentation representation.
/// 2. `doc` transforms the cloned documentation representation.
/// 3. `post_op` receives both representations and can perform operations that
///    need to coordinate the two versions.
///
/// The resulting expansion contains the transformed `item` under
/// `cfg(not(doc))` and the transformed `for_doc` under `cfg(doc)`. The
/// non-documentation representation is additionally marked with
/// `#[doc(hidden)]`, so the generated implementation does not appear as the
/// authoritative Rustdoc representation.
///
/// For example, an item can be transformed differently for normal compilation
/// and documentation:
///
/// ```ignore
/// let tokens = doc_twin(
///     "GeneratedFunction",
///     &mut item,
///     |item| {
///         // Transformation used during normal compilation.
///         transform_actual(item)?;
///         Ok(())
///     },
///     |item| {
///         // Transformation used when generating documentation.
///         transform_doc(item)?;
///         Ok(())
///     },
///     |actual, doc| {
///         // Coordinate the two representations after both transformations.
///         synchronize(actual, doc)?;
///         Ok(())
///     },
///     false,
///     false,
///     false,
///     false,
/// );
/// ```
///
/// Conceptually, if the original item is:
///
/// ```ignore
/// fn example() {}
/// ```
///
/// the two closures may produce different representations:
///
/// ```text
/// Normal compilation:
///     transformed implementation
///
/// Documentation:
///     documentation-friendly representation
/// ```
///
/// The final expansion is structurally equivalent to:
///
/// ```ignore
/// #[cfg(not(doc))]
/// #[doc(hidden)]
/// /* actual representation */
///
/// #[cfg(doc)]
/// /* documentation representation */
/// ```
///
/// The documentation representation is created by cloning the original item
/// before `actual` mutates the non-documentation representation. Consequently,
/// changes made by `actual` do not automatically appear in `for_doc`, and
/// changes made by `doc` do not automatically appear in `item`.
///
/// `post_op` is useful when some information must be transferred or compared
/// between the two representations after their independent transformations:
///
/// ```ignore
/// |actual, doc| {
///     copy_required_metadata(actual, doc)?;
///     validate_pair(actual, doc)?;
///     Ok(())
/// }
/// ```
///
/// Before the documentation representation is generated, this function also
/// prepends a Rust-analyzer notice to the non-documentation representation.
/// The notice explains how to enable `cfg(doc)` analysis and directs users to
/// generated rustdoc or docs.rs as the authoritative documentation when
/// rust-analyzer displays the internal expanded form.
///
/// When the `dev` feature is enabled with `--features dev`, the intermediate
/// representations can optionally be rendered using `prettyplease`:
///
/// - `pre_debug` prints the original item before either transformation.
/// - `post_debug` prints the non-documentation representation after `actual`.
/// - `post_doc_debug` prints the documentation representation after `doc`.
/// - `post_op_debug` prints both representations after `post_op`.
///
/// For example:
///
/// ```ignore
/// let tokens = doc_twin(
///     "InstanceAccess",
///     &mut item,
///     actual,
///     doc,
///     post_op,
///     true,  // pre_debug
///     true,  // post_debug
///     true,  // post_doc_debug
///     true,  // post_op_debug
/// );
/// ```
///
/// If any of `actual`, `doc`, or `post_op` returns an error, that error
/// [`TokenStream`] is returned immediately and no final conditional expansion
/// is produced.
///
/// This function therefore provides a common transformation boundary for
/// proc-macro code that must maintain distinct compiler and documentation
/// representations of the same source item while allowing both representations
/// to be coordinated before final expansion.
pub fn doc_twin<T, FActual, FDoc, FPost>(
    _label: &str,
    item: &mut T,
    actual: FActual,
    doc: FDoc,
    post_op: FPost,
    pre_debug: bool,
    post_debug: bool,
    post_doc_debug: bool,
    post_op_debug: bool,
) -> TokenStream
where
    T: ToTokens + Clone + InsertDocs,
    FActual: FnOnce(&mut T) -> Result<(), TokenStream>,
    FDoc: FnOnce(&mut T) -> Result<(), TokenStream>,
    FPost: FnOnce(&mut T, &mut T) -> Result<(), TokenStream>,
{
    if pre_debug {
        // ------- Debugging (Pre) -------
        #[cfg(feature = "dev")]
        {
            let label = _label;
            let pretty = syn::parse2::<syn::File>(quote!(#item))
                .map(|f| prettyplease::unparse(&f))
                .unwrap_or_else(|_| quote!(#item).to_string());
            eprintln!(
                "\n +==========+ PRE-DOC-TWIN ({}) +==========+ \n\n{}\n",
                label, pretty
            );
        }
    }

    // Before mutation for actual item repr
    let mut for_doc = item.clone();

    if let Err(err) = actual(item) {
        return err;
    }

    cargo_not_doc_disclaimer(item);

    if post_debug {
        // ------- Debugging (Post) -------
        #[cfg(feature = "dev")]
        {
            let label = _label;
            let pretty = syn::parse2::<syn::File>(quote!(#item))
                .map(|f| prettyplease::unparse(&f))
                .unwrap_or_else(|_| quote!(#item).to_string());

            eprintln!(
                "\n +==========+ POST-NOT-DOC ({}) +==========+ \n\n{}\n",
                label, pretty,
            );
        }
    }

    if let Err(err) = doc(&mut for_doc) {
        return err;
    }

    if post_doc_debug {
        // ------- Debugging (Doc) -------
        #[cfg(feature = "dev")]
        {
            let label = _label;
            let pretty = syn::parse2::<syn::File>(quote!(#for_doc))
                .map(|f| prettyplease::unparse(&f))
                .unwrap_or_else(|_| quote!(#for_doc).to_string());

            eprintln!(
                "\n +==========+ POST-DOC ({}) +==========+ \n\n{}\n",
                label, pretty,
            );
        }
    }

    if let Err(err) = post_op(item, &mut for_doc) {
        return err;
    }

    if post_op_debug {
        // ------- Debugging Post (NotDoc + Doc) -------
        #[cfg(feature = "dev")]
        {
            let label = _label;
            let pretty = syn::parse2::<syn::File>(quote!(#item))
                .map(|f| prettyplease::unparse(&f))
                .unwrap_or_else(|_| quote!(#item).to_string());

            eprintln!(
                "\n +==========+ POST-OP-NOT-DOC ({}) +==========+ \n\n{}\n",
                label, pretty,
            );

            let pretty = syn::parse2::<syn::File>(quote!(#for_doc))
                .map(|f| prettyplease::unparse(&f))
                .unwrap_or_else(|_| quote!(#for_doc).to_string());

            eprintln!(
                "\n +==========+ POST-OP-DOC ({}) +==========+ \n\n{}\n",
                label, pretty,
            );
        }
    }

    // Final expansion
    quote!(
        #[cfg(not(doc))]
        #[doc(hidden)]
        #item

        #[cfg(doc)]
        #for_doc
    )
}

/// Combines multiple [`ToTokens`] expressions into a single
/// [`proc_macro2::TokenStream`].
///
/// Each expression can also be a function call whose result implements
/// [`ToTokens`]:
///
/// ```ignore
/// let tokens = collect!(
///     generate_struct(),
///     generate_impl(),
///     generate_function(),
/// );
/// ```
///
/// Conceptually, the macro expands the collected values through a `quote!`
/// repetition:
///
/// ```ignore
/// let items = vec![expr1, expr2, expr3];
/// quote! {
///     #(#items)*
/// }
/// ```
///
/// An empty invocation produces an empty [`proc_macro2::TokenStream`]:
///
/// ```ignore
/// let tokens = collect!();
/// ```
#[macro_export]
macro_rules! collect {
    ($($expr:expr),* $(,)?) => {{
        let items = vec![$($expr),*];
        quote::quote! {
            #(#items)*
        }
    }};
}

/// Returns attributes used to mark generated implementation items as internal.
///
/// These attributes hide generated implementation details from Rustdoc and
/// suppress the `dead_code` diagnostic that can occur when an item exists only
/// to support macro expansion.
///
/// The returned attributes are equivalent to:
///
/// ```ignore
/// #[doc(hidden)]
/// #[allow(dead_code)]
/// ```
pub fn internal_code() -> Vec<syn::Attribute> {
    vec![
        parse_quote!(#[doc(hidden)]),
        parse_quote!(#[allow(dead_code)]),
    ]
}
