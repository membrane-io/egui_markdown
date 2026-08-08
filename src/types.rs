//! Parsed markdown token types and data structures.

use std::hash::{Hash, Hasher};

use pulldown_cmark::CowStr;

/// A single parsed markdown token.
#[derive(Debug, Clone)]
pub enum Token<'s> {
  /// Line break (hard break or paragraph separator).
  Newline,
  /// Styled text span.
  Text {
    /// The text content.
    text: CowStr<'s>,
    /// Inline styling (bold, italic, etc.).
    style: TokenStyle,
  },
  /// Fenced or indented code block.
  CodeBlock {
    /// The code content.
    text: CowStr<'s>,
    /// The language hint (e.g. `"rust"`), if any.
    language: Option<CowStr<'s>>,
  },
  /// Hyperlink with display text, URL, and optional title.
  Link {
    /// Display text of the link.
    text: CowStr<'s>,
    /// The link destination URL.
    href: CowStr<'s>,
    /// Optional hover title.
    title: Option<CowStr<'s>>,
  },
  /// List item bullet or number (e.g. `"• "` or `"1. "`).
  ListMarker {
    /// The marker string (e.g. `"• "`, `"1. "`).
    marker: CowStr<'s>,
    /// Nesting depth (1 = top-level).
    indent_level: usize,
  },
  /// Inline image with alt text, URL, and optional title.
  Image {
    /// Alt text for the image.
    alt: CowStr<'s>,
    /// Image source URL.
    url: CowStr<'s>,
    /// Optional hover title.
    title: Option<CowStr<'s>>,
  },
  /// Table with headers, rows, and column alignments.
  Table(TableData<'s>),
  /// Horizontal rule (`---`).
  HorizontalRule,
  /// Start of a blockquote region.
  BlockquoteStart,
  /// End of a blockquote region.
  BlockquoteEnd,
  /// Task list checkbox (`- [x]` or `- [ ]`).
  TaskListMarker {
    /// Whether the checkbox is checked.
    checked: bool,
    /// Nesting depth (1 = top-level).
    indent_level: usize,
  },
  /// Footnote reference (e.g. `[^1]`).
  FootnoteRef {
    /// The footnote label.
    label: CowStr<'s>,
  },
  /// Footnote definition (e.g. `[^1]: ...`).
  FootnoteDef {
    /// The footnote label.
    label: CowStr<'s>,
  },
}

/// Inline text styling applied to a [`Token::Text`].
#[derive(Debug, Clone, Default, Hash)]
pub struct TokenStyle {
  /// Bold weight.
  pub bold: bool,
  /// Italic slant.
  pub italic: bool,
  /// Strikethrough line.
  pub strikethrough: bool,
  /// Inline code span (backtick-delimited).
  pub inline_code: bool,
  /// Heading level (1–6), or `None` for non-heading text.
  pub heading: Option<u8>,
}

impl TokenStyle {
  /// Returns `true` if no styling is applied.
  pub fn is_plain(&self) -> bool {
    !self.bold && !self.italic && !self.strikethrough && !self.inline_code && self.heading.is_none()
  }
}

/// Parsed table data: column alignments, header cells, and body rows.
#[derive(Debug, Clone)]
pub struct TableData<'s> {
  /// Column alignment directives.
  pub alignments: Vec<Alignment>,
  /// Header cells, each containing a token stream.
  pub headers: Vec<Vec<Token<'s>>>,
  /// Body rows. Each row is a `Vec` of cells; each cell is a `Vec<Token>`.
  pub rows: Vec<Vec<Vec<Token<'s>>>>,
}

/// Column alignment for table cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Alignment {
  /// No alignment specified.
  None,
  /// Left-aligned.
  Left,
  /// Center-aligned.
  Center,
  /// Right-aligned.
  Right,
}

impl<'s> Hash for TableData<'s> {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.alignments.hash(state);
    self.headers.hash(state);
    self.rows.hash(state);
  }
}

impl<'s> Hash for Token<'s> {
  fn hash<H: Hasher>(&self, state: &mut H) {
    std::mem::discriminant(self).hash(state);
    match self {
      Token::Newline | Token::HorizontalRule | Token::BlockquoteStart | Token::BlockquoteEnd => {}
      Token::Text { text, style } => {
        text.as_ref().hash(state);
        style.hash(state);
      }
      Token::CodeBlock { text, language } => {
        text.as_ref().hash(state);
        language.as_ref().map(|l| l.as_ref()).hash(state);
      }
      Token::Link { text, href, title } => {
        text.as_ref().hash(state);
        href.as_ref().hash(state);
        title.as_ref().map(|t| t.as_ref()).hash(state);
      }
      Token::ListMarker { marker, indent_level } => {
        marker.as_ref().hash(state);
        indent_level.hash(state);
      }
      Token::Image { alt, url, title } => {
        alt.as_ref().hash(state);
        url.as_ref().hash(state);
        title.as_ref().map(|t| t.as_ref()).hash(state);
      }
      Token::Table(data) => data.hash(state),
      Token::TaskListMarker { checked, indent_level } => {
        checked.hash(state);
        indent_level.hash(state);
      }
      Token::FootnoteRef { label } => label.as_ref().hash(state),
      Token::FootnoteDef { label } => label.as_ref().hash(state),
    }
  }
}

/// The result of parsing a markdown string: the source text and its token stream.
pub struct Markdown<'s> {
  /// The original source text.
  pub s: &'s str,
  /// Parsed token stream.
  pub tokens: Vec<Token<'s>>,
}

impl<'s> Token<'s> {
  /// Returns the primary text content of this token.
  pub fn text(&self) -> &str {
    match self {
      Token::Newline => "",
      Token::CodeBlock { text, .. } => text,
      Token::Text { text, .. } => text,
      Token::Link { text, .. } => text,
      Token::ListMarker { marker, .. } => marker,
      Token::Image { alt, .. } => alt,
      Token::HorizontalRule => "",
      Token::BlockquoteStart | Token::BlockquoteEnd => "",
      Token::TaskListMarker { .. } => "",
      Token::FootnoteRef { label } => label,
      Token::FootnoteDef { label } => label,
      Token::Table(_) => "",
    }
  }

  /// Returns the link URL if this is a `Link` token.
  pub fn href(&self) -> Option<&str> {
    match self {
      Token::Link { href, .. } => Some(href),
      _ => None,
    }
  }

  /// Returns `true` if this is a `Newline` token.
  pub fn is_newline(&self) -> bool {
    matches!(self, Token::Newline)
  }

  /// Returns `true` if this is a `ListMarker` token.
  pub fn is_list_marker(&self) -> bool {
    matches!(self, Token::ListMarker { .. })
  }
}

#[cfg(test)]
mod size_tests {
  use super::*;

  #[test]
  fn token_enum_size() {
    let size = std::mem::size_of::<Token<'_>>();
    // Token is currently ~80 bytes due to Link/Image variants with 3 CowStr fields.
    // Boxing those would reduce it to ~24 bytes but breaks the public API - note for
    // future semver bump.
    eprintln!("Token size: {size} bytes");
    assert!(size <= 88, "Token grew unexpectedly: {size} bytes");
  }

  #[test]
  fn layout_result_size() {
    let size = std::mem::size_of::<crate::layout::LayoutResult>();
    eprintln!("LayoutResult size: {size} bytes");
    // LayoutResult contains LayoutJob + 5 Vecs + u32, expected ~200-300 bytes.
    assert!(size <= 400, "LayoutResult grew unexpectedly: {size} bytes");
  }
}
