//! The token types of a parsed document.

use std::hash::{Hash, Hasher};

use pulldown_cmark::CowStr;

/// One element of a parsed document.
#[derive(Debug, Clone)]
pub enum Token<'s> {
  /// A line break: a hard break, or the end of a block.
  Newline,
  /// A run of text with one style.
  Text {
    /// The text content.
    text: CowStr<'s>,
    /// Inline styling, such as bold and italic.
    style: TokenStyle,
  },
  /// A fenced or indented code block.
  CodeBlock {
    /// The code content.
    text: CowStr<'s>,
    /// The language hint, such as `"rust"`, when the fence gives one.
    language: Option<CowStr<'s>>,
  },
  /// A link, with its text, its URL and an optional title.
  Link {
    /// The text of the link.
    text: CowStr<'s>,
    /// The link destination URL.
    href: CowStr<'s>,
    /// The title of the link, such as `"Docs"` in `[text](url "Docs")`.
    title: Option<CowStr<'s>>,
    /// The inline style around the link, such as the heading level of a link inside a heading.
    style: TokenStyle,
  },
  /// The bullet or the number of a list item. An empty marker sets the x of a later paragraph or of a code
  /// block in a list item.
  ListMarker {
    /// The marker string, such as `"• "` or `"1. "`.
    marker: CowStr<'s>,
    /// The nesting depth. A top-level item has depth 1.
    indent_level: usize,
  },
  /// An image, with its alt text, its URL and an optional title. Markdown `![alt](url)` and an HTML `<img>` tag
  /// both give this token.
  Image {
    /// The alt text of the image.
    alt: CowStr<'s>,
    /// The URL of the image.
    url: CowStr<'s>,
    /// The title of the image.
    title: Option<CowStr<'s>>,
    /// The width in points from the `width` attribute of an `<img>` tag.
    width: Option<u16>,
    /// The height in points from the `height` attribute of an `<img>` tag.
    height: Option<u16>,
    /// The link around the image, such as `https://site` in `[![logo](logo.png)](https://site)`, or `None`
    /// for an image that is not in a link. A click on a linked image opens the link, not the image.
    link: Option<Box<ImageLink<'s>>>,
  },
  /// A table, with its header, its rows and the alignment of its columns.
  Table(TableData<'s>),
  /// Horizontal rule (`---`).
  HorizontalRule,
  /// The start of a blockquote.
  BlockquoteStart,
  /// The end of a blockquote.
  BlockquoteEnd,
  /// The checkbox of a task list item (`- [x]` or `- [ ]`).
  TaskListMarker {
    /// Whether the checkbox is checked.
    checked: bool,
    /// The nesting depth. A top-level item has depth 1.
    indent_level: usize,
  },
  /// A footnote reference, such as `[^1]`.
  FootnoteRef {
    /// The footnote label.
    label: CowStr<'s>,
  },
  /// The start of a footnote definition, such as `[^1]: ...`.
  FootnoteDef {
    /// The footnote label.
    label: CowStr<'s>,
  },
}

/// The link around a [`Token::Image`].
#[derive(Debug, Clone)]
pub struct ImageLink<'s> {
  /// The link destination URL.
  pub href: CowStr<'s>,
  /// The title of the link, such as `"Site"` in `[![logo](logo.png)](https://site "Site")`.
  pub title: Option<CowStr<'s>>,
}

impl<'s> Hash for ImageLink<'s> {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.href.as_ref().hash(state);
    self.title.as_ref().map(|t| t.as_ref()).hash(state);
  }
}

/// The inline style of a [`Token::Text`], and the style of the text around a [`Token::Link`].
#[derive(Debug, Clone, Default, Hash)]
pub struct TokenStyle {
  /// Bold weight.
  pub bold: bool,
  /// Italic slant.
  pub italic: bool,
  /// Strikethrough line.
  pub strikethrough: bool,
  /// Inline code, between backticks.
  pub inline_code: bool,
  /// The heading level, from 1 to 6, or `None` for text that is not in a heading.
  pub heading: Option<u8>,
}

impl TokenStyle {
  /// Returns `true` when the text has no inline style.
  pub fn is_plain(&self) -> bool {
    !self.bold && !self.italic && !self.strikethrough && !self.inline_code && self.heading.is_none()
  }
}

/// The content of a table: the alignment of each column, the header cells and the body rows.
#[derive(Debug, Clone)]
pub struct TableData<'s> {
  /// The alignment of each column.
  pub alignments: Vec<Alignment>,
  /// The header cells. Each cell is a list of tokens.
  pub headers: Vec<Vec<Token<'s>>>,
  /// Body rows. Each row is a `Vec` of cells, and each cell is a `Vec<Token>`.
  pub rows: Vec<Vec<Vec<Token<'s>>>>,
}

/// The alignment of the cells of a table column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Alignment {
  /// The separator gives no alignment. The cells are left-aligned.
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
      Token::Link { text, href, title, style } => {
        text.as_ref().hash(state);
        href.as_ref().hash(state);
        title.as_ref().map(|t| t.as_ref()).hash(state);
        style.hash(state);
      }
      Token::ListMarker { marker, indent_level } => {
        marker.as_ref().hash(state);
        indent_level.hash(state);
      }
      Token::Image { alt, url, title, width, height, link } => {
        alt.as_ref().hash(state);
        url.as_ref().hash(state);
        title.as_ref().map(|t| t.as_ref()).hash(state);
        width.hash(state);
        height.hash(state);
        link.hash(state);
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

/// The result of [`crate::parse`]: the source text and its tokens.
pub struct Markdown<'s> {
  /// The source text.
  pub s: &'s str,
  /// The tokens of the text.
  pub tokens: Vec<Token<'s>>,
}

impl<'s> Token<'s> {
  /// The text of the token: the text of a text, code block or link, the alt text of an image, the marker of
  /// a list item, or the label of a footnote. Other tokens have no text.
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

  /// The URL of a [`Token::Link`], or `None` for another token.
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

/// Copies of `tokens` that own their text, so that the label can keep them after the frame.
pub(crate) fn tokens_to_owned(tokens: &[Token<'_>]) -> Vec<Token<'static>> {
  tokens.iter().map(|t| token_to_owned(t)).collect()
}

fn cowstr_to_owned(s: &pulldown_cmark::CowStr<'_>) -> pulldown_cmark::CowStr<'static> {
  pulldown_cmark::CowStr::Boxed(s.to_string().into_boxed_str())
}

fn token_to_owned(t: &Token<'_>) -> Token<'static> {
  match t {
    Token::Newline => Token::Newline,
    Token::Text { text, style } => Token::Text { text: cowstr_to_owned(text), style: style.clone() },
    Token::CodeBlock { text, language } => {
      Token::CodeBlock { text: cowstr_to_owned(text), language: language.as_ref().map(|l| cowstr_to_owned(l)) }
    }
    Token::Link { text, href, title, style } => Token::Link {
      text: cowstr_to_owned(text),
      href: cowstr_to_owned(href),
      title: title.as_ref().map(|t| cowstr_to_owned(t)),
      style: style.clone(),
    },
    Token::ListMarker { marker, indent_level } => {
      Token::ListMarker { marker: cowstr_to_owned(marker), indent_level: *indent_level }
    }
    Token::Image { alt, url, title, width, height, link } => Token::Image {
      alt: cowstr_to_owned(alt),
      url: cowstr_to_owned(url),
      title: title.as_ref().map(|t| cowstr_to_owned(t)),
      width: *width,
      height: *height,
      link: link.as_ref().map(|link| {
        Box::new(ImageLink {
          href: cowstr_to_owned(&link.href),
          title: link.title.as_ref().map(|t| cowstr_to_owned(t)),
        })
      }),
    },
    Token::Table(data) => Token::Table(TableData {
      alignments: data.alignments.clone(),
      headers: data.headers.iter().map(|row| tokens_to_owned(row)).collect(),
      rows: data.rows.iter().map(|row| row.iter().map(|cell| tokens_to_owned(cell)).collect()).collect(),
    }),
    Token::HorizontalRule => Token::HorizontalRule,
    Token::BlockquoteStart => Token::BlockquoteStart,
    Token::BlockquoteEnd => Token::BlockquoteEnd,
    Token::TaskListMarker { checked, indent_level } => {
      Token::TaskListMarker { checked: *checked, indent_level: *indent_level }
    }
    Token::FootnoteRef { label } => Token::FootnoteRef { label: cowstr_to_owned(label) },
    Token::FootnoteDef { label } => Token::FootnoteDef { label: cowstr_to_owned(label) },
  }
}

#[cfg(test)]
mod size_tests {
  use super::*;

  #[test]
  fn token_enum_size() {
    let size = std::mem::size_of::<Token<'_>>();
    // The Link and Image variants set the size. Each holds 3 CowStr fields. Link also holds a TokenStyle,
    // and Image holds the size of an <img> tag and a Box of its link. A Box around these fields makes the
    // size smaller, but it changes the public API.
    eprintln!("Token size: {size} bytes");
    assert!(size <= 88, "Token grew unexpectedly: {size} bytes");
  }

  #[test]
  fn layout_result_size() {
    let size = std::mem::size_of::<crate::layout::LayoutResult>();
    eprintln!("LayoutResult size: {size} bytes");
    assert!(size <= 400, "LayoutResult grew unexpectedly: {size} bytes");
  }
}
