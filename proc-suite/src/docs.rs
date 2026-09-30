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
// `````````````````````````` PROC-MACRO DOCUMENTATIONS ``````````````````````````
// ===============================================================================

//! Utilities for generating and attaching structured Rustdoc attributes to
//! [`syn`] syntax nodes.
//!
//! This module provides [`InsertDocs`], [`DocAttr`], and [`DocCodeBlock`] for
//! constructing documentation programmatically during proc-macro
//! transformations. Instead of manually constructing `#[doc = "..."]`
//! attributes, documentation can be expressed as structured values and then
//! attached directly to the relevant syntax node.
//!
//! The primary entry point is [`InsertDocs`]. It provides a common interface
//! for inserting, replacing, appending, prepending, and clearing documentation
//! on supported [`syn`] nodes.
//!
//! [`InsertDocs`] is built on the lower-level attribute operation traits.
//! A syntax-tree type becomes documentation-capable by providing the required
//! attribute operations, while [`InsertDocs`] supplies the higher-level
//! documentation behavior through their default implementations.
//!
//! For example, documentation can be attached to a trait item using:
//!
//! ```ignore
//! item.insert_docs(vec![
//!     DocAttr::Raw("Associated instance counter type for instance trait ".into()),
//!     DocAttr::Ref("InstanceTrait".into()),
//!     DocAttr::LineBreak,
//!     DocAttr::Bullet {
//!         level: 1,
//!         content: vec![
//!             DocAttr::Raw("Must be the same unsigned type used by all instance counters: ".into()),
//!             DocAttr::Inline("Counter".into()),
//!         ],
//!     },
//!     DocAttr::Bullet {
//!         level: 1,
//!         content: vec![
//!             DocAttr::Raw("Expected type: ".into()),
//!             DocAttr::Inline("usize".into()),
//!         ],
//!     },
//! ]);
//! ```
//!
//! The structured values are rendered into ordinary Rustdoc attributes such
//! as:
//!
//! ```ignore
//! #[doc = "Associated instance counter type for instance trait [`InstanceTrait`]"]
//! #[doc = ""]
//! #[doc = "  - Must be the same unsigned type used by all instance counters: `Counter`"]
//! #[doc = "  - Expected type: `usize`"]
//! ```
//!
//! ## Documentation support for `syn` nodes
//!
//! [`InsertDocs`] is available for syntax-tree types that provide the
//! attribute operations required by the trait. These operations provide the
//! primitive mechanisms for manipulating a node's `Vec<Attribute>`:
//!
//! - inserting and pushing attributes
//! - prepending attributes
//! - locating documentation attributes
//! - removing documentation attributes
//!
//! [`InsertDocs`] builds the higher-level documentation API from these
//! operations. Its default methods are therefore shared by every supported
//! syntax-tree type, keeping documentation rendering and document-management
//! semantics consistent across nodes.
//!
//! ## Documentation representation
//!
//! [`DocAttr`] provides the structured documentation primitives used by the
//! transformation code. It supports raw Markdown, line breaks, headings,
//! bullet lists, numbered lists, code blocks, external links, Rustdoc
//! references, inline code, and blockquotes.
//!
//! [`DocCodeBlock`] provides the supported code-block representations,
//! including Rust syntax trees, ignored Rust examples, manually specified
//! fenced blocks, and plain-text blocks.
//!
//! Nested [`DocAttr`] values can be used to compose a single documentation
//! element. For example:
//!
//! ```ignore
//! DocAttr::Bullet {
//!     level: 1,
//!     content: vec![
//!         DocAttr::Raw("Expected type: ".into()),
//!         DocAttr::Inline(expected_ty.into()),
//!     ],
//! }
//! ```
//!
//! ## Rendering
//!
//! [`DocAttr`] values are converted into one or more Rustdoc lines before they
//! are attached to the syntax node. Each rendered line becomes an individual
//! `#[doc = "..."]` attribute.
//!
//! Structured block elements such as headings and lists are rendered as
//! Markdown, while inline elements can be combined recursively to construct
//! a single documentation line.
//!
//! This separation between documentation construction, rendering, and
//! attribute manipulation allows proc-macro transformations to generate
//! consistent Rustdoc while keeping the syntax-tree operations independent
//! from the specific documentation content.

// ===============================================================================
// ``````````````````````````````````` IMPORTS ```````````````````````````````````
// ===============================================================================

// --- Proc-Macro Utils ---
use quote::quote;
use syn::{Attribute, File, parse_quote};

// --- Local Crate ---
use crate::syns::*;

// ===============================================================================
// ````````````````````````````` DOC ATTRIBUTE CASES `````````````````````````````
// ===============================================================================

/// Represents a structured documentation element used to generate Rustdoc.
///
/// [`DocAttr`] provides the documentation primitives used by proc-macro
/// transformations to construct generated documentation without manually
/// creating `#[doc = "..."]` attributes. Each variant represents a Markdown
/// or Rustdoc construct that can be rendered into one or more documentation
/// lines.
///
/// The variants can be composed recursively through their `content` fields,
/// allowing documentation to be constructed from both block-level and
/// inline-level elements. For example:
///
/// ```ignore
/// DocAttr::Bullet {
///     level: 1,
///     content: vec![
///         DocAttr::Raw("Expected type: ".into()),
///         DocAttr::Inline("usize".into()),
///     ],
/// }
/// ```
///
/// renders conceptually as:
///
/// ```text
///   - Expected type: `usize`
/// ```
///
/// [`DocAttr`] values are consumed by the documentation renderer and converted
/// into `#[doc = "..."]` attributes when used with [`InsertDocs`].
///
/// The enum is marked `non_exhaustive` so additional documentation
/// constructs can be introduced without making exhaustive matching outside
/// this crate a breaking change.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum DocAttr {
    /// Raw markdown inserted exactly as-is.
    ///
    /// This bypasses all structured formatting rules and should be
    /// used only when full manual control is required.
    ///
    /// Expands directly to:
    /// #[doc = "..."]
    Raw(String),

    /// Explicit markdown line break.
    ///
    /// Useful when rendering inline-composed content that should force
    /// a hard visual break.
    ///
    /// Expands to:
    /// #[doc = ""]
    LineBreak,

    /// Markdown heading.
    ///
    /// level:
    /// 1 => # Heading
    /// 2 => ## Heading
    /// 3 => ### Heading
    /// 4 => #### Heading
    ///
    /// Values above 6 should be clamped.
    Heading { level: u8, content: Vec<DocAttr> },

    /// Bullet list item.
    ///
    /// level:
    /// 0 => - item
    /// 1 =>   - nested item
    /// 2 =>     - deeper nesting
    Bullet { level: usize, content: Vec<DocAttr> },

    /// Numbered list item.
    ///
    /// level:
    /// 0 => 1. item
    /// 1 =>   1. nested item
    ///
    /// index controls rendered numbering.
    Numbered {
        level: usize,
        index: usize,
        content: Vec<DocAttr>,
    },

    /// Fenced markdown code block. Uses [`DocCodeBlock`]
    ///
    /// Supports:
    /// - Rust syntax trees stored as [`syn::File`]
    /// - Arbitrary fenced blocks with a custom language identifier
    /// - Plain text blocks
    ///
    /// ## Example
    ///
    /// ```text
    ///     ```rust
    ///     let x = 1;
    ///     ```
    /// ```
    ///
    /// ```text
    ///     ```bash
    ///     echo hello
    ///     ```
    /// ```
    /// 
    /// ```text
    ///     ```text
    ///     some plain text
    ///     ```
    /// ```
    CodeBlock(DocCodeBlock),

    /// External markdown link.
    ///
    /// Expands to:
    /// ```text
    /// [`title`](url)
    /// ```
    Link { title: String, url: String },

    /// Internal rustdoc reference.
    ///
    /// Expands to:
    /// ```text
    /// [`SomeType`]
    /// ```
    Ref(String),

    /// Inline code block
    ///
    /// Expands to:
    /// `SomeType`
    Inline(String),

    /// Markdown blockquote.
    ///
    /// Expands to:
    /// > quoted text
    Quote(Vec<DocAttr>),
}

/// Represents the supported fenced code-block forms used by [`DocAttr`].
///
/// [`DocCodeBlock`] separates the representation of code from the surrounding
/// documentation structure. Each variant defines how the code is fenced and
/// rendered, while [`DocAttr::CodeBlock`] embeds that representation into a
/// larger documentation sequence.
///
/// Code blocks may contain a parsed Rust syntax tree, structured
/// [`DocAttr`] values, or plain structured text depending on the required
/// documentation form.
///
/// For example, Rust code can be represented directly as a [`syn::File`]:
///
/// ```ignore
/// DocCodeBlock::Rust(file)
/// ```
///
/// while an illustrative Rust example that should not be compiled as a
/// doctest can use:
///
/// ```ignore
/// DocCodeBlock::Ignore(vec![
///     DocAttr::Raw("let value = example();".into()),
/// ])
/// ```
///
/// [`DocCodeBlock::Manual`] allows the fenced language to be selected
/// explicitly, making it suitable for languages or formats other than Rust.
/// [`DocCodeBlock::Text`] provides a dedicated `text` fenced block for
/// non-source-code content.
///
/// The resulting code block is rendered as Markdown and subsequently emitted
/// as Rustdoc through [`InsertDocs`].
#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum DocCodeBlock {
    /// Rust code stored as a parsed [`syn::File`].
    ///
    /// The syntax tree is rendered through `quote!()` when generating
    /// documentation, preserving proc-macro ergonomics and formatting.
    ///
    /// Renders as:
    ///
    /// ```text
    ///     ```rust
    ///     <rust code>
    ///     ```
    /// ```
    Rust(File),

    /// Rust code block excluded from doctests.
    ///
    /// Useful for examples that are illustrative but not intended to
    /// compile or execute during documentation testing.
    ///
    /// Renders as:
    ///
    /// ```text
    ///     ```ignore
    ///     ...
    ///     ```
    /// ```
    Ignore(Vec<DocAttr>),

    /// A fenced code block with an explicit language identifier.
    ///
    /// The block contents are composed from nested [`DocAttr`] values,
    /// allowing structured formatting within the code block.
    ///
    /// Common language identifiers include:
    /// - `rust`
    /// - `ignore`
    /// - `text`
    /// - `bash`
    /// - `toml`
    /// - `json`
    ///
    /// Renders as:
    ///
    /// ```text
    ///     ```<lang>
    ///     ...
    ///     ```
    /// ```
    Manual {
        /// Language identifier used after the opening fence.
        lang: String,

        /// Structured contents of the code block.
        block: Vec<DocAttr>,
    },

    /// Plain text code block.
    ///
    /// The block contents are composed from nested [`DocAttr`] values and
    /// rendered using a `text` fenced code block.
    ///
    /// Renders as:
    ///
    /// ```text
    ///     ```text
    ///     ...
    ///     ```
    /// ```
    Text(Vec<DocAttr>),
}

/// Inserts structured rustdoc attributes onto a syntax node.
///
/// This trait is intended to be implemented for `syn` item types.
///
/// It provides a unified API for attaching generated documentation
/// without manually constructing:
///
/// `#[doc = "..."]`
///
/// attributes by hand.
///
/// The implementation should:
///
/// 1. Convert [`DocAttr`] into one or more `#[doc = "..."]` attributes
/// 2. Preserve existing attributes where appropriate
/// 3. Insert generated docs in rustdoc-correct order
///
/// Example:
///
/// ```ignore
/// item.insert_docs([
///     DocAttr::Heading {
///         level: 1,
///         content: vec![DocAttr::Line(vec![
///             DocAttr::Raw("Overview".into())
///         ])],
///     },
///     DocAttr::Line(vec![
///         DocAttr::Raw("Provides structured diagnostics.".into())
///     ]),
/// ]);
/// ```
pub trait InsertDocs:
    PushAttribute + PrependAttributes + FirstDocAttribute + ClearDocAttributes
{
    /// Returns whether rustdoc attributes already exist.
    ///
    /// Used internally for append behavior.
    fn has_docs(&mut self) -> bool {
        let mut op = AttributesFirstDoc { index: None };

        self.first_doc_attribute(&mut op);

        op.index.is_some()
    }

    /// Inserts documentation attributes onto `self`.
    ////
    /// Accepts any iterator of [`DocAttr`] so callers can pass:
    ///
    /// - arrays
    /// - `Vec<DocAttr>`
    /// - iterators
    /// - macro-generated pipelines
    ///
    /// without friction.
    ///
    /// Implementors are responsible for converting the structured
    /// syntax tree into emitted rustdoc attributes.
    fn insert_docs<I>(&mut self, docs: I)
    where
        I: IntoIterator<Item = DocAttr>,
    {
        for doc in docs {
            for line in render_doc_attr(doc) {
                let attr: Attribute = parse_quote! {
                    #[doc = #line]
                };

                let mut op = AttributePush { attr };

                self.push_attribute(&mut op);
            }
        }
    }

    /// Removes previously attached rustdoc attributes.
    ///
    /// This should typically remove:
    ///
    /// `#[doc = "..."]`
    ///
    /// while preserving unrelated attributes.
    ///
    /// Useful for regeneration workflows where docs are rebuilt
    /// from scratch.
    fn clear_docs(&mut self) {
        let mut op = AttributesDocsClear;

        self.clear_doc_attributes(&mut op);
    }

    /// Replaces all existing documentation with new generated docs.
    ///
    /// Default behavior:
    ///
    /// 1. clear existing docs
    /// 2. insert new docs
    fn replace_docs<I>(&mut self, docs: I)
    where
        I: IntoIterator<Item = DocAttr>,
    {
        self.clear_docs();
        self.insert_docs(docs)
    }

    /// Appends documentation after existing rustdoc attributes.
    ///
    /// Generated docs are placed below existing docs while preserving
    /// unrelated attributes and relative ordering.
    fn append_docs<I>(&mut self, docs: I)
    where
        I: IntoIterator<Item = DocAttr>,
    {
        if self.has_docs() {
            self.insert_docs([DocAttr::Raw(String::new())]);
        }

        self.insert_docs(docs)
    }

    /// Prepends documentation attributes before existing rustdoc attributes.
    ///
    /// Generated docs are placed above existing docs while preserving
    /// unrelated attributes and relative ordering.
    fn prepend_docs<I>(&mut self, docs: I)
    where
        I: IntoIterator<Item = DocAttr>,
    {
        let mut generated = Vec::<Attribute>::new();

        for doc in docs {
            for line in render_doc_attr(doc) {
                let attr: Attribute = parse_quote! {
                    #[doc = #line]
                };

                generated.push(attr);
            }
        }

        let mut first_doc = AttributesFirstDoc { index: None };

        self.first_doc_attribute(&mut first_doc);

        // If docs already exist, insert a blank doc line between
        // the new prepended docs and the existing ones.
        if first_doc.index.is_some() {
            let line_break: Attribute = parse_quote! {
                #[doc = ""]
            };

            generated.push(line_break);
        }

        let mut op = AttributesPrepend { attrs: generated };

        self.prepend_attributes(&mut op);
    }
}

/// Blanket implementation providing the complete structured-rustdoc API
/// for every syntax node supporting the required primitive attribute
/// operations.
impl<T> InsertDocs for T where
    T: PushAttribute + PrependAttributes + FirstDocAttribute + ClearDocAttributes
{
}

/// Converts structured DocAttr into one or more rustdoc lines.
///
/// Each returned String becomes:
///
/// #[doc = "..."]
fn render_doc_attr(doc: DocAttr) -> Vec<String> {
    let mut out = Vec::<String>::new();
    match doc {
        DocAttr::Raw(s) => out.push(s),

        DocAttr::LineBreak => out.push(String::new()),

        DocAttr::Heading { level, content } => {
            let level = level.clamp(1, 6);
            let hashes = "#".repeat(level as usize);
            if let Some(body) = render_inline(content) {
                out.push(format!("{hashes} {body}"));
            }
        }

        DocAttr::Bullet { level, content } => {
            let indent = "  ".repeat(level);
            if let Some(body) = render_inline(content) {
                out.push(format!("{indent}- {body}"));
            }
        }

        DocAttr::Numbered {
            level,
            index,
            content,
        } => {
            let indent = "  ".repeat(level);
            if let Some(body) = render_inline(content) {
                out.push(format!("{indent}{index}. {body}"))
            }
        }

        DocAttr::CodeBlock(block) => match block {
            DocCodeBlock::Rust(code) => {
                out.push("```rust".into());
                out.push(quote!(#code).to_string());
                out.push("```".into());
            }

            DocCodeBlock::Manual { lang, block } => {
                out.push(format!("```{lang}"));
                out.extend(render_inline(block));
                out.push("```".into());
            }

            DocCodeBlock::Text(block) => {
                out.push("```text".into());
                out.extend(render_inline(block));
                out.push("```".into());
            }

            DocCodeBlock::Ignore(block) => {
                out.push("```ignore".into());
                out.extend(render_inline(block));
                out.push("```".into());
            }
        },

        DocAttr::Link { title, url } => out.push(format!("[`{title}`]({url})")),

        DocAttr::Ref(name) => out.push(format!("[`{name}`]")),

        DocAttr::Quote(content) => {
            if let Some(body) = render_inline(content) {
                out.push(format!("> {body}"))
            }
        }
        DocAttr::Inline(content) => out.push(format!("`{content}`")),
    };

    out
}

/// Renders inline recursive content into a single line.
fn render_inline(items: Vec<DocAttr>) -> Option<String> {
    if items.is_empty() {
        return None;
    }

    let mut out = String::new();
    for item in items {
        match item {
            DocAttr::Raw(s) => {
                out.push_str(&s);
            }

            DocAttr::LineBreak => {
                out.push('\n');
            }

            DocAttr::Ref(name) => {
                out.push_str(&format!("[`{}`]", name));
            }

            DocAttr::Link { title, url } => {
                out.push_str(&format!("[`{}`]({})", title, url));
            }

            DocAttr::Inline(content) => {
                out.push_str(&format!("`{content}`",));
            }

            // These should generally not appear inline.
            // Explicitly ignore or panic depending on strictness.
            DocAttr::Heading { .. }
            | DocAttr::Bullet { .. }
            | DocAttr::Numbered { .. }
            | DocAttr::CodeBlock(_)
            | DocAttr::Quote(_) => return None,
        }
    }

    Some(out)
}
