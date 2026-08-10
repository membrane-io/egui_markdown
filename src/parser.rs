//! CommonMark markdown parser with streaming-friendly healing.

use std::borrow::Cow;

use pulldown_cmark::{CowStr, Event, Options, Parser, Tag, TagEnd, TextMergeStream};

use crate::types::{Alignment, Markdown, TableData, Token, TokenStyle};

#[derive(Clone)]
struct ListInfo {
  is_ordered: bool,
  number: u64,
}

fn trim_end_newlines<'s>(s: CowStr<'s>) -> CowStr<'s> {
  match s {
    CowStr::Borrowed(s) => CowStr::Borrowed(s.trim_end_matches("\n")),
    CowStr::Boxed(s) => CowStr::Boxed(s.trim_end_matches("\n").into()),
    CowStr::Inlined(s) => CowStr::Inlined(s),
  }
}

/// Auto-close unclosed markdown constructs for streaming/incomplete input.
///
/// When markdown is being streamed (e.g. from an LLM), unclosed constructs cause
/// pulldown-cmark to render raw syntax characters (`**`, `[`, etc.) as literal text.
/// This function detects unclosed code fences, bold, italic, strikethrough, inline
/// code, and links, and appends the necessary closing markers.
///
/// Returns `Cow::Borrowed` when no healing is needed (zero-cost).
///
/// ```
/// use egui_markdown::heal;
///
/// assert_eq!(heal("hello"), "hello");
/// assert_eq!(heal("```rust\nlet x = 1;"), "```rust\nlet x = 1;\n```");
/// assert_eq!(heal("~~~python\nprint()"), "~~~python\nprint()\n```");
/// assert_eq!(heal("**bold text"), "**bold text**");
/// assert_eq!(heal("_italic"), "_italic_");
/// assert_eq!(heal("[link text"), "[link text]()");
/// assert_eq!(heal("\\*escaped"), "\\*escaped");
/// ```
pub fn heal(s: &str) -> Cow<'_, str> {
  let mut in_fence = false;
  for line in s.lines() {
    let trimmed = line.trim();
    if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
      in_fence = !in_fence;
    }
  }
  if in_fence {
    let mut healed = String::with_capacity(s.len() + 4);
    healed.push_str(s);
    if !s.ends_with('\n') {
      healed.push('\n');
    }
    healed.push_str("```");
    return Cow::Owned(healed);
  }

  let table_suffix = heal_table(s);
  // Skip inline healing when a table suffix was generated; the inline scanner
  // doesn't understand table structure and can produce conflicting closings.
  let inline_suffix = if table_suffix.is_empty() { heal_inline(s) } else { String::new() };

  if table_suffix.is_empty() && inline_suffix.is_empty() {
    Cow::Borrowed(s)
  } else {
    let mut healed = String::with_capacity(s.len() + table_suffix.len() + inline_suffix.len());
    healed.push_str(s);
    healed.push_str(&table_suffix);
    healed.push_str(&inline_suffix);
    Cow::Owned(healed)
  }
}

fn heal_inline(s: &str) -> String {
  let mut in_fence = false;
  let mut in_inline_code = false;
  let mut open_star_bold = false;
  let mut open_star_italic = false;
  let mut open_under_bold = false;
  let mut open_under_italic = false;
  let mut open_strike = false;
  let mut in_link_text = false;
  let mut in_link_url = false;
  let mut in_link_title = false;
  let mut link_paren_depth: u32 = 0;

  for line in s.lines() {
    let trimmed = line.trim();
    if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
      in_fence = !in_fence;
      continue;
    }
    if in_fence {
      continue;
    }

    let bytes = line.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
      let c = bytes[i];

      // Backslash escape: skip the next character.
      if c == b'\\' && i + 1 < len {
        i += 2;
        continue;
      }

      // Inside inline code, only backtick can close it.
      if in_inline_code {
        if c == b'`' {
          in_inline_code = false;
        }
        i += 1;
        continue;
      }

      if c == b'`' {
        in_inline_code = true;
        i += 1;
        continue;
      }

      // Inside link URL, track paren nesting and title quotes.
      if in_link_url {
        if c == b'"' {
          in_link_title = !in_link_title;
        } else if !in_link_title {
          if c == b'(' {
            link_paren_depth += 1;
          } else if c == b')' {
            if link_paren_depth > 0 {
              link_paren_depth -= 1;
            } else {
              in_link_url = false;
              in_link_title = false;
            }
          }
        }
        i += 1;
        continue;
      }

      // Asterisk emphasis: ***, **, *.
      if c == b'*' {
        if i + 2 < len && bytes[i + 1] == b'*' && bytes[i + 2] == b'*' {
          open_star_bold = !open_star_bold;
          open_star_italic = !open_star_italic;
          i += 3;
          continue;
        }
        if i + 1 < len && bytes[i + 1] == b'*' {
          open_star_bold = !open_star_bold;
          i += 2;
          continue;
        }
        open_star_italic = !open_star_italic;
        i += 1;
        continue;
      }

      // Underscore emphasis: ___, __, _.
      // CommonMark: underscores flanked by alphanumeric on both sides are intra-word and not emphasis.
      if c == b'_' {
        let preceded_by_alnum = i > 0 && bytes[i - 1].is_ascii_alphanumeric();
        if i + 2 < len && bytes[i + 1] == b'_' && bytes[i + 2] == b'_' {
          let followed_by_alnum = i + 3 < len && bytes[i + 3].is_ascii_alphanumeric();
          if preceded_by_alnum && followed_by_alnum {
            i += 3;
            continue;
          }
          open_under_bold = !open_under_bold;
          open_under_italic = !open_under_italic;
          i += 3;
          continue;
        }
        if i + 1 < len && bytes[i + 1] == b'_' {
          let followed_by_alnum = i + 2 < len && bytes[i + 2].is_ascii_alphanumeric();
          if preceded_by_alnum && followed_by_alnum {
            i += 2;
            continue;
          }
          open_under_bold = !open_under_bold;
          i += 2;
          continue;
        }
        let followed_by_alnum = i + 1 < len && bytes[i + 1].is_ascii_alphanumeric();
        if preceded_by_alnum && followed_by_alnum {
          i += 1;
          continue;
        }
        open_under_italic = !open_under_italic;
        i += 1;
        continue;
      }

      // Strikethrough: ~~.
      if c == b'~' && i + 1 < len && bytes[i + 1] == b'~' {
        open_strike = !open_strike;
        i += 2;
        continue;
      }

      // Links: [text](url), but not footnote refs [^label].
      if c == b'[' && !in_link_text {
        // Detect footnote reference [^ and skip it.
        if i + 1 < len && bytes[i + 1] == b'^' {
          // Skip past the closing ] if present.
          i += 2;
          while i < len && bytes[i] != b']' {
            i += 1;
          }
          if i < len {
            i += 1; // skip ]
          }
          continue;
        }
        in_link_text = true;
        i += 1;
        continue;
      }
      if c == b']' && in_link_text {
        if i + 1 < len && bytes[i + 1] == b'(' {
          in_link_text = false;
          in_link_url = true;
          link_paren_depth = 0;
          i += 2;
          continue;
        }
        in_link_text = false;
        i += 1;
        continue;
      }

      i += 1;
    }
  }

  let mut suffix = String::new();
  if in_inline_code {
    suffix.push('`');
  }
  if in_link_url {
    if in_link_title {
      suffix.push('"');
    }
    suffix.push(')');
  }
  if in_link_text {
    suffix.push_str("]()");
  }
  if open_star_italic {
    suffix.push('*');
  }
  if open_star_bold {
    suffix.push_str("**");
  }
  if open_under_italic {
    suffix.push('_');
  }
  if open_under_bold {
    suffix.push_str("__");
  }
  if open_strike {
    suffix.push_str("~~");
  }
  suffix
}

fn heal_table(s: &str) -> String {
  let lines: Vec<&str> = s.lines().collect();
  if lines.is_empty() {
    return String::new();
  }

  // Find last non-empty line.
  let mut last_idx = lines.len() - 1;
  while last_idx > 0 && lines[last_idx].trim().is_empty() {
    last_idx -= 1;
  }
  let last = lines[last_idx].trim();

  // Case 1: Last line is a complete table header row - needs a separator.
  if is_table_row(last) && !is_separator_like(last) {
    // Walk backwards to see if there's already a separator in this table block.
    for i in (0..last_idx).rev() {
      let line = lines[i].trim();
      if line.is_empty() {
        break;
      }
      if is_full_separator(line) {
        return String::new(); // body row in recognized table
      }
    }
    let cols = count_table_columns(last);
    if cols >= 2 {
      return format!("\n|{}", "---|".repeat(cols));
    }
  }

  // Case 2: Last line is a partial separator, previous line is a header.
  if is_separator_like(last) && last.contains('-') && last_idx > 0 {
    let header = lines[last_idx - 1].trim();
    if is_table_row(header) && !is_separator_like(header) {
      let header_pipes = header.matches('|').count();
      let sep_pipes = last.matches('|').count();
      if sep_pipes < header_pipes {
        let missing = header_pipes - sep_pipes;
        return if last.ends_with('|') {
          "---|".repeat(missing)
        } else {
          format!("|{}", "---|".repeat(missing.saturating_sub(1)))
        };
      }
    }
  }

  String::new()
}

/// A line that looks like a table row: starts and ends with `|`, has at least 2 cells.
fn is_table_row(line: &str) -> bool {
  line.starts_with('|') && line.ends_with('|') && line.matches('|').count() >= 3
}

/// A line containing only table-separator characters (`|`, `-`, `:`, space).
fn is_separator_like(line: &str) -> bool {
  !line.is_empty() && line.starts_with('|') && line.chars().all(|c| matches!(c, '|' | '-' | ':' | ' '))
}

/// A complete separator line (separator-like with dashes and enough pipes for a table).
fn is_full_separator(line: &str) -> bool {
  is_separator_like(line) && line.contains('-') && line.matches('|').count() >= 3
}

fn count_table_columns(row: &str) -> usize {
  row.split('|').filter(|s| !s.trim().is_empty()).count()
}

/// Parse a markdown string into a [`Markdown`] token stream.
///
/// Supports CommonMark with extensions: tables, strikethrough, footnotes, and task lists.
pub fn parse<'s>(s: &'s str) -> Markdown<'s> {
  let mut tokens = Vec::new();
  let mut options = Options::empty();
  options.insert(Options::ENABLE_STRIKETHROUGH);
  options.insert(Options::ENABLE_TABLES);
  options.insert(Options::ENABLE_FOOTNOTES);
  options.insert(Options::ENABLE_TASKLISTS);

  let parser = TextMergeStream::new(Parser::new_ext(s, options));

  let mut current_style = TokenStyle::default();
  let mut in_link = false;
  let mut in_code_block = false;
  let mut in_image = false;
  let mut current_code_language: Option<CowStr> = None;
  let mut link_text = None;
  let mut link_href = None;
  let mut link_title: Option<CowStr> = None;
  let mut image_alt = None;
  let mut image_url = None;
  let mut image_title: Option<CowStr> = None;
  let mut list_stack: Vec<ListInfo> = Vec::new();
  let mut in_list_item = false;

  // Table collection state.
  let mut in_table = false;
  let mut table_alignments: Vec<Alignment> = Vec::new();
  let mut table_headers: Vec<Vec<Token<'s>>> = Vec::new();
  let mut table_rows: Vec<Vec<Vec<Token<'s>>>> = Vec::new();
  let mut current_row: Vec<Vec<Token<'s>>> = Vec::new();
  let mut current_cell: Vec<Token<'s>> = Vec::new();
  let mut in_table_head = false;

  // Footnote definition state.
  let mut in_footnote_def = false;

  let ensure_double_newline = |tokens: &mut Vec<Token>| {
    if tokens.is_empty() {
      return;
    }
    if !matches!(tokens.last(), Some(Token::Newline)) {
      tokens.push(Token::Newline);
      tokens.push(Token::Newline);
    } else if tokens.len() > 1 && !matches!(tokens.get(tokens.len() - 2), Some(Token::Newline)) {
      tokens.push(Token::Newline);
    }
  };

  let ensure_newline = |tokens: &mut Vec<Token>| {
    if tokens.is_empty() || !matches!(tokens.last(), Some(Token::Newline)) {
      tokens.push(Token::Newline);
    }
  };

  let is_last_token_list_marker =
    |tokens: &[Token]| -> bool { matches!(tokens.last(), Some(Token::ListMarker { .. })) };

  for event in parser {
    // When collecting table cells, route text into the cell buffer.
    if in_table {
      match &event {
        Event::Start(Tag::TableHead) => {
          in_table_head = true;
          current_row = Vec::new();
          continue;
        }
        Event::End(TagEnd::TableHead) => {
          in_table_head = false;
          table_headers = std::mem::take(&mut current_row);
          continue;
        }
        Event::Start(Tag::TableRow) => {
          current_row = Vec::new();
          continue;
        }
        Event::End(TagEnd::TableRow) => {
          if !in_table_head {
            table_rows.push(std::mem::take(&mut current_row));
          } else {
            current_row.clear();
          }
          continue;
        }
        Event::Start(Tag::TableCell) => {
          current_cell = Vec::new();
          continue;
        }
        Event::End(TagEnd::TableCell) => {
          current_row.push(std::mem::take(&mut current_cell));
          continue;
        }
        Event::End(TagEnd::Table) => {
          in_table = false;
          ensure_double_newline(&mut tokens);
          tokens.push(Token::Table(TableData {
            alignments: std::mem::take(&mut table_alignments),
            headers: std::mem::take(&mut table_headers),
            rows: std::mem::take(&mut table_rows),
          }));
          continue;
        }
        Event::Text(text) => {
          if in_link {
            link_text = Some(text.clone());
          } else {
            current_cell.push(Token::Text { text: text.clone(), style: current_style.clone() });
          }
          continue;
        }
        Event::Code(text) => {
          if in_link {
            link_text = Some(text.clone());
          } else {
            current_cell
              .push(Token::Text { text: text.clone(), style: TokenStyle { inline_code: true, ..Default::default() } });
          }
          continue;
        }
        Event::Start(Tag::Strong) => {
          current_style.bold = true;
          continue;
        }
        Event::End(TagEnd::Strong) => {
          current_style.bold = false;
          continue;
        }
        Event::Start(Tag::Emphasis) => {
          current_style.italic = true;
          continue;
        }
        Event::End(TagEnd::Emphasis) => {
          current_style.italic = false;
          continue;
        }
        Event::Start(Tag::Strikethrough) => {
          current_style.strikethrough = true;
          continue;
        }
        Event::End(TagEnd::Strikethrough) => {
          current_style.strikethrough = false;
          continue;
        }
        Event::SoftBreak => {
          current_cell.push(Token::Text { text: CowStr::Borrowed(" "), style: current_style.clone() });
          continue;
        }
        Event::HardBreak => {
          current_cell.push(Token::Newline);
          continue;
        }
        Event::Start(Tag::Link { dest_url, title, .. }) => {
          in_link = true;
          link_href = Some(dest_url.clone());
          link_title = if title.is_empty() { None } else { Some(title.clone()) };
          continue;
        }
        Event::End(TagEnd::Link) => {
          in_link = false;
          let text = link_text.take().unwrap_or_else(|| CowStr::Boxed("".into()));
          if let Some(url) = link_href.take() {
            current_cell.push(Token::Link { text, href: url, title: link_title.take() });
          }
          continue;
        }
        _ => {
          continue;
        }
      }
    }

    match event {
      Event::Text(text) => {
        if in_image {
          image_alt = Some(text);
        } else if in_link {
          link_text = Some(text);
        } else if in_code_block {
          tokens.push(Token::CodeBlock { text: trim_end_newlines(text), language: current_code_language.clone() });
        } else {
          tokens.push(Token::Text { text, style: current_style.clone() });
        }
      }

      Event::Html(html) => {
        tokens.push(Token::Text { text: html, style: current_style.clone() });
      }

      Event::InlineHtml(html) => {
        tokens.push(Token::Text { text: html, style: current_style.clone() });
      }

      Event::Code(text) => {
        if in_link {
          link_text = Some(text);
        } else {
          let code_style = TokenStyle { inline_code: true, ..Default::default() };
          tokens.push(Token::Text { text, style: code_style });
        }
      }

      Event::SoftBreak => {
        tokens.push(Token::Text { text: CowStr::Borrowed(" "), style: current_style.clone() });
      }
      Event::HardBreak => {
        tokens.push(Token::Newline);
      }

      Event::Rule => {
        ensure_double_newline(&mut tokens);
        tokens.push(Token::HorizontalRule);
      }

      Event::TaskListMarker(checked) => {
        let indent_level = list_stack.len();
        tokens.push(Token::TaskListMarker { checked, indent_level });
      }

      Event::FootnoteReference(label) => {
        tokens.push(Token::FootnoteRef { label });
      }

      Event::Start(tag) => match tag {
        Tag::Paragraph => {
          let after_header_in_list = in_list_item
            && tokens.last().is_some_and(|token| matches!(token, Token::Text { style, .. } if style.heading.is_some()));

          let last = tokens.last();
          if matches!(last, Some(Token::ListMarker { .. })) || matches!(last, Some(Token::TaskListMarker { .. })) {
            // Don't add newlines right after a list/task marker.
          } else if after_header_in_list {
            ensure_newline(&mut tokens);
          } else if !in_list_item && !in_footnote_def {
            ensure_double_newline(&mut tokens);
          }
        }
        Tag::CodeBlock(code_block_kind) => {
          let after_header_in_list = in_list_item
            && tokens.last().is_some_and(|token| matches!(token, Token::Text { style, .. } if style.heading.is_some()));

          let last = tokens.last();
          if matches!(last, Some(Token::ListMarker { .. })) {
          } else if after_header_in_list {
            ensure_newline(&mut tokens);
          } else if !in_list_item {
            ensure_double_newline(&mut tokens);
          }

          current_code_language = match code_block_kind {
            pulldown_cmark::CodeBlockKind::Fenced(lang) => {
              if lang.is_empty() {
                None
              } else {
                Some(lang)
              }
            }
            pulldown_cmark::CodeBlockKind::Indented => None,
          };
          in_code_block = true;
        }
        Tag::List(start) => {
          let is_ordered = start.is_some();
          let number = start.unwrap_or(1);
          list_stack.push(ListInfo { is_ordered, number });

          if !is_last_token_list_marker(&tokens) {
            if list_stack.len() == 1 {
              ensure_double_newline(&mut tokens);
            } else {
              ensure_newline(&mut tokens);
            }
          }
        }
        Tag::Item => {
          if !tokens.is_empty() {
            ensure_newline(&mut tokens);
          }
          in_list_item = true;
          let indent_level = list_stack.len();
          let marker: CowStr = if let Some(list_info) = list_stack.last_mut() {
            if list_info.is_ordered {
              let marker = format!("{}. ", list_info.number);
              list_info.number += 1;
              CowStr::Boxed(marker.into())
            } else {
              CowStr::Borrowed("• ")
            }
          } else {
            CowStr::Borrowed("  • ")
          };
          tokens.push(Token::ListMarker { marker, indent_level });
        }
        Tag::Heading { level, .. } => {
          let after_list_marker = is_last_token_list_marker(&tokens);
          if !after_list_marker && !in_list_item {
            ensure_double_newline(&mut tokens);
          }
          current_style.heading = Some(level as u8);
        }
        Tag::Emphasis => {
          current_style.italic = true;
        }
        Tag::Strong => {
          current_style.bold = true;
        }
        Tag::Strikethrough => {
          current_style.strikethrough = true;
        }
        Tag::Link { dest_url, title, .. } => {
          in_link = true;
          link_href = Some(dest_url);
          link_title = if title.is_empty() { None } else { Some(title) };
        }
        Tag::Image { dest_url, title, .. } => {
          in_image = true;
          image_url = Some(dest_url);
          image_title = if title.is_empty() { None } else { Some(title) };
        }
        Tag::BlockQuote(_) => {
          tokens.push(Token::BlockquoteStart);
        }
        Tag::Table(alignments) => {
          in_table = true;
          table_alignments = alignments
            .into_iter()
            .map(|a| match a {
              pulldown_cmark::Alignment::None => Alignment::None,
              pulldown_cmark::Alignment::Left => Alignment::Left,
              pulldown_cmark::Alignment::Center => Alignment::Center,
              pulldown_cmark::Alignment::Right => Alignment::Right,
            })
            .collect();
          table_headers = Vec::new();
          table_rows = Vec::new();
        }
        Tag::FootnoteDefinition(label) => {
          in_footnote_def = true;
          ensure_double_newline(&mut tokens);
          tokens.push(Token::FootnoteDef { label });
        }
        _ => {}
      },

      Event::End(tag) => match tag {
        TagEnd::CodeBlock => {
          in_code_block = false;
          current_code_language = None;
        }
        TagEnd::Paragraph => {}
        TagEnd::Item => {
          in_list_item = false;
        }
        TagEnd::List(_) => {
          list_stack.pop();
        }
        TagEnd::Heading(_) => {
          current_style.heading = None;
        }
        TagEnd::Emphasis => {
          current_style.italic = false;
        }
        TagEnd::Strong => {
          current_style.bold = false;
        }
        TagEnd::Strikethrough => {
          current_style.strikethrough = false;
        }
        TagEnd::Link => {
          in_link = false;
          let text = link_text.take().unwrap_or_else(|| CowStr::Boxed("".into()));
          if let Some(url) = link_href.take() {
            tokens.push(Token::Link { text, href: url, title: link_title.take() });
          }
        }
        TagEnd::Image => {
          in_image = false;
          let alt = image_alt.take().unwrap_or_else(|| CowStr::Boxed("".into()));
          if let Some(url) = image_url.take() {
            tokens.push(Token::Image { alt, url, title: image_title.take() });
          }
        }
        TagEnd::BlockQuote(_) => {
          tokens.push(Token::BlockquoteEnd);
        }
        TagEnd::FootnoteDefinition => {
          in_footnote_def = false;
        }
        _ => {}
      },

      _ => {}
    }
  }

  Markdown { s, tokens }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn empty() {
    let md = parse("");
    assert!(md.tokens.is_empty());
  }

  #[test]
  fn text() {
    let md = parse("Some text");
    let mut it = md.tokens.iter();
    assert_eq!(it.next().unwrap().text(), "Some text");
    assert!(it.next().is_none());
  }

  #[test]
  fn test_html_tags_preserved() {
    let md = parse("This is a <tag>test</tag> with <br/> tags");
    assert!(md.tokens.iter().any(|token| matches!(token, Token::Text { text, .. } if text.contains("<tag>"))));
    assert!(md.tokens.iter().any(|token| matches!(token, Token::Text { text, .. } if text.contains("</tag>"))));
    assert!(md.tokens.iter().any(|token| matches!(token, Token::Text { text, .. } if text.contains("<br/>"))));
  }

  #[test]
  fn test_empty_list_items() {
    let md = parse("- \n- Item with content\n- ");
    let mut it = md.tokens.iter();
    let first_marker = it.next().unwrap();
    assert!(first_marker.is_list_marker());
    if let Token::ListMarker { marker, indent_level } = first_marker {
      assert_eq!(marker.as_ref(), "• ");
      assert_eq!(*indent_level, 1);
    }
  }

  #[test]
  fn test_headers_in_list_items() {
    let md = parse("- # Heading in list\n  Some text under heading");
    let tokens: Vec<_> = md.tokens.iter().collect();
    assert!(tokens.iter().any(|t| matches!(t, Token::ListMarker { .. })));
    assert!(tokens.iter().any(|t| matches!(t, Token::Text { style, .. } if style.heading.is_some())));
  }

  #[test]
  fn test_nested_lists() {
    let md = parse("- Item 1\n  - Nested item\n- Item 2");
    let markers: Vec<_> = md.tokens.iter().filter(|t| t.is_list_marker()).collect();
    assert_eq!(markers.len(), 3);
    if let Token::ListMarker { indent_level, .. } = markers[1] {
      assert_eq!(*indent_level, 2);
    }
  }

  #[test]
  fn soft_break() {
    let md = parse("Line 1\nLine 2");
    let mut it = md.tokens.iter();
    assert_eq!(it.next().unwrap().text(), "Line 1");
    // SoftBreak becomes a space per CommonMark spec.
    assert_eq!(it.next().unwrap().text(), " ");
    assert_eq!(it.next().unwrap().text(), "Line 2");
    assert!(it.next().is_none());
  }

  #[test]
  fn hard_break() {
    // Two trailing spaces before newline = hard break.
    let md = parse("Line 1  \nLine 2");
    let mut it = md.tokens.iter();
    assert_eq!(it.next().unwrap().text(), "Line 1");
    assert!(it.next().unwrap().is_newline());
    assert_eq!(it.next().unwrap().text(), "Line 2");
    assert!(it.next().is_none());
  }

  #[test]
  fn bold() {
    let md = parse("**bold**");
    let mut it = md.tokens.iter();
    let token = it.next().unwrap();
    assert_eq!(token.text(), "bold");
    if let Token::Text { style, .. } = token {
      assert!(style.bold);
    }
  }

  #[test]
  fn italic() {
    let md = parse("*italic*");
    let mut it = md.tokens.iter();
    let token = it.next().unwrap();
    assert_eq!(token.text(), "italic");
    if let Token::Text { style, .. } = token {
      assert!(style.italic);
    }
  }

  #[test]
  fn inline_code() {
    let md = parse("`code`");
    let mut it = md.tokens.iter();
    let token = it.next().unwrap();
    assert_eq!(token.text(), "code");
    if let Token::Text { style, .. } = token {
      assert!(style.inline_code);
    }
  }

  #[test]
  fn heading() {
    let md = parse("# Heading");
    let mut it = md.tokens.iter();
    let token = it.next().unwrap();
    assert_eq!(token.text(), "Heading");
    if let Token::Text { style, .. } = token {
      assert_eq!(style.heading, Some(1));
    }
  }

  #[test]
  fn heading_levels() {
    for level in 1..=6u8 {
      let input = format!("{} H{level}", "#".repeat(level as usize));
      let md = parse(&input);
      let token = md.tokens.iter().find(|t| matches!(t, Token::Text { style, .. } if style.heading.is_some()));
      assert!(token.is_some(), "Expected heading token for H{level}");
      if let Some(Token::Text { style, .. }) = token {
        assert_eq!(style.heading, Some(level), "Wrong level for H{level}");
      }
    }
  }

  #[test]
  fn code_block() {
    let md = parse("```rust\nlet x = 1;\n```");
    let mut it = md.tokens.iter();
    let token = it.next().unwrap();
    if let Token::CodeBlock { text, language } = token {
      assert_eq!(text.as_ref(), "let x = 1;");
      assert_eq!(language.as_deref(), Some("rust"));
    } else {
      panic!("Expected CodeBlock");
    }
  }

  #[test]
  fn link() {
    let md = parse("[click](https://example.com)");
    let mut it = md.tokens.iter();
    let token = it.next().unwrap();
    assert_eq!(token.text(), "click");
    assert_eq!(token.href(), Some("https://example.com"));
  }

  #[test]
  fn ordered_list() {
    let md = parse("1. First\n2. Second\n3. Third");
    let markers: Vec<_> = md.tokens.iter().filter(|t| t.is_list_marker()).collect();
    assert_eq!(markers.len(), 3);
    if let Token::ListMarker { marker, .. } = markers[0] {
      assert_eq!(marker.as_ref(), "1. ");
    }
    if let Token::ListMarker { marker, .. } = markers[1] {
      assert_eq!(marker.as_ref(), "2. ");
    }
  }

  #[test]
  fn strikethrough() {
    let md = parse("~~struck~~");
    let mut it = md.tokens.iter();
    let token = it.next().unwrap();
    assert_eq!(token.text(), "struck");
    if let Token::Text { style, .. } = token {
      assert!(style.strikethrough);
    }
  }

  #[test]
  fn horizontal_rule() {
    let md = parse("above\n\n---\n\nbelow");
    assert!(md.tokens.iter().any(|t| matches!(t, Token::HorizontalRule)));
  }

  #[test]
  fn blockquote() {
    let md = parse("> quoted text");
    let tokens: Vec<_> = md.tokens.iter().collect();
    assert!(tokens.iter().any(|t| matches!(t, Token::BlockquoteStart)));
    assert!(tokens.iter().any(|t| matches!(t, Token::BlockquoteEnd)));
    assert!(tokens.iter().any(|t| matches!(t, Token::Text { text, .. } if text.as_ref() == "quoted text")));
  }

  #[test]
  fn task_list() {
    let md = parse("- [x] Done\n- [ ] Not done");
    let task_markers: Vec<_> = md.tokens.iter().filter(|t| matches!(t, Token::TaskListMarker { .. })).collect();
    assert_eq!(task_markers.len(), 2);
    if let Token::TaskListMarker { checked, .. } = task_markers[0] {
      assert!(*checked);
    }
    if let Token::TaskListMarker { checked, .. } = task_markers[1] {
      assert!(!*checked);
    }
  }

  #[test]
  fn table() {
    let md = parse("| A | B |\n|---|---|\n| 1 | 2 |\n| 3 | 4 |");
    let table = md.tokens.iter().find(|t| matches!(t, Token::Table(_)));
    assert!(table.is_some());
    if let Some(Token::Table(data)) = table {
      assert_eq!(data.headers.len(), 2);
      assert_eq!(data.rows.len(), 2);
      assert_eq!(data.rows[0].len(), 2);
    }
  }

  #[test]
  fn image() {
    let md = parse("![alt text](https://example.com/image.png)");
    let img = md.tokens.iter().find(|t| matches!(t, Token::Image { .. }));
    assert!(img.is_some());
    if let Some(Token::Image { alt, url, .. }) = img {
      assert_eq!(alt.as_ref(), "alt text");
      assert_eq!(url.as_ref(), "https://example.com/image.png");
    }
  }

  #[test]
  fn mixed_bold_italic() {
    let md = parse("***bold italic***");
    let mut it = md.tokens.iter();
    let token = it.next().unwrap();
    if let Token::Text { style, .. } = token {
      assert!(style.bold);
      assert!(style.italic);
    }
  }

  #[test]
  fn unicode() {
    let md = parse("Hello 🌍 world");
    let token = md.tokens.first().unwrap();
    assert_eq!(token.text(), "Hello 🌍 world");
  }

  #[test]
  fn nested_formatting() {
    let md = parse("**bold *and italic* text**");
    let tokens: Vec<_> = md.tokens.iter().collect();
    assert!(tokens.iter().any(|t| matches!(t, Token::Text { style, .. } if style.bold && !style.italic)));
    assert!(tokens.iter().any(|t| matches!(t, Token::Text { style, .. } if style.bold && style.italic)));
  }

  #[test]
  fn empty_link() {
    let md = parse("[](https://example.com)");
    let link = md.tokens.iter().find(|t| matches!(t, Token::Link { .. }));
    assert!(link.is_some());
  }

  #[test]
  fn multi_level_list() {
    let md = parse("- Level 1\n  - Level 2\n    - Level 3");
    let markers: Vec<_> = md
      .tokens
      .iter()
      .filter_map(|t| if let Token::ListMarker { indent_level, .. } = t { Some(*indent_level) } else { None })
      .collect();
    assert_eq!(markers, vec![1, 2, 3]);
  }

  #[test]
  fn paragraphs_separated() {
    let md = parse("Para 1\n\nPara 2");
    let newlines = md.tokens.iter().filter(|t| t.is_newline()).count();
    assert!(newlines >= 2);
  }

  #[test]
  fn code_block_no_language() {
    let md = parse("```\ncode\n```");
    if let Some(Token::CodeBlock { language, .. }) = md.tokens.first() {
      assert!(language.is_none());
    }
  }

  #[test]
  fn link_with_code_text() {
    let md = parse("[`code`](https://example.com)");
    let link = md.tokens.iter().find(|t| matches!(t, Token::Link { .. }));
    assert!(link.is_some());
    if let Some(Token::Link { text, href, .. }) = link {
      assert_eq!(text.as_ref(), "code");
      assert_eq!(href.as_ref(), "https://example.com");
    }
  }

  #[test]
  fn nested_blockquotes() {
    let md = parse("> outer\n> > inner");
    let starts = md.tokens.iter().filter(|t| matches!(t, Token::BlockquoteStart)).count();
    assert!(starts >= 2);
  }

  #[test]
  fn table_with_alignment() {
    let md = parse("| Left | Center | Right |\n|:-----|:------:|------:|\n| a | b | c |");
    if let Some(Token::Table(data)) = md.tokens.iter().find(|t| matches!(t, Token::Table(_))) {
      assert_eq!(data.alignments, vec![Alignment::Left, Alignment::Center, Alignment::Right]);
    }
  }

  #[test]
  fn strikethrough_in_bold() {
    let md = parse("**~~struck bold~~**");
    let token = md.tokens.iter().find(|t| matches!(t, Token::Text { style, .. } if style.strikethrough));
    assert!(token.is_some());
    if let Some(Token::Text { style, .. }) = token {
      assert!(style.bold);
      assert!(style.strikethrough);
    }
  }

  #[test]
  fn no_panic_on_malformed() {
    // These should not panic.
    parse("**unclosed bold");
    parse("```unclosed code block");
    parse("[broken link](");
    parse("| broken | table\n|");
    parse("> > > deeply nested");
    parse("- [ unclosed task");
    parse("[^broken footnote");
    parse("");
    parse("   ");
    parse("\n\n\n");
  }

  #[test]
  fn heal_no_fences() {
    assert!(matches!(super::heal("hello world"), std::borrow::Cow::Borrowed(_)));
  }

  #[test]
  fn heal_closed_fence() {
    let input = "```rust\nlet x = 1;\n```";
    assert!(matches!(super::heal(input), std::borrow::Cow::Borrowed(_)));
  }

  #[test]
  fn heal_unclosed_fence() {
    let input = "```rust\nlet x = 1;";
    let healed = super::heal(input);
    assert!(matches!(healed, std::borrow::Cow::Owned(_)));
    assert!(healed.ends_with("```"));
    let md = parse(&healed);
    assert!(md.tokens.iter().any(|t| matches!(t, Token::CodeBlock { .. })));
  }

  #[test]
  fn heal_multiple_fences() {
    let input = "```\nfirst\n```\n\n```python\nprint('hi')";
    let healed = super::heal(input);
    assert!(healed.ends_with("```"));
  }

  #[test]
  fn heal_trailing_newline() {
    let input = "```\ncode\n";
    let healed = super::heal(input);
    assert!(healed.ends_with("\n```"));
    assert!(!healed.ends_with("\n\n```"));
  }

  #[test]
  fn heal_unclosed_bold() {
    let healed = super::heal("**bold text");
    assert_eq!(&*healed, "**bold text**");
    let md = parse(&healed);
    assert!(md.tokens.iter().any(|t| matches!(t, Token::Text { style, .. } if style.bold)));
  }

  #[test]
  fn heal_unclosed_italic() {
    let healed = super::heal("*italic text");
    assert_eq!(&*healed, "*italic text*");
    let md = parse(&healed);
    assert!(md.tokens.iter().any(|t| matches!(t, Token::Text { style, .. } if style.italic)));
  }

  #[test]
  fn heal_unclosed_strikethrough() {
    let healed = super::heal("~~struck");
    assert_eq!(&*healed, "~~struck~~");
    let md = parse(&healed);
    assert!(md.tokens.iter().any(|t| matches!(t, Token::Text { style, .. } if style.strikethrough)));
  }

  #[test]
  fn heal_unclosed_inline_code() {
    let healed = super::heal("`code");
    assert_eq!(&*healed, "`code`");
    let md = parse(&healed);
    assert!(md.tokens.iter().any(|t| matches!(t, Token::Text { style, .. } if style.inline_code)));
  }

  #[test]
  fn heal_unclosed_link_text() {
    let healed = super::heal("[link text");
    assert_eq!(&*healed, "[link text]()");
    let md = parse(&healed);
    assert!(md.tokens.iter().any(|t| matches!(t, Token::Link { .. })));
  }

  #[test]
  fn heal_unclosed_link_url() {
    let healed = super::heal("[text](https://example.com");
    assert_eq!(&*healed, "[text](https://example.com)");
    let md = parse(&healed);
    if let Some(Token::Link { href, .. }) = md.tokens.iter().find(|t| matches!(t, Token::Link { .. })) {
      assert_eq!(href.as_ref(), "https://example.com");
    } else {
      panic!("Expected Link token");
    }
  }

  #[test]
  fn heal_closed_constructs_no_change() {
    let input = "**bold** and *italic* and `code` and [link](url)";
    assert!(matches!(super::heal(input), std::borrow::Cow::Borrowed(_)));
  }

  #[test]
  fn heal_inline_inside_code_fence_ignored() {
    // Inline markers inside a closed code block should not trigger healing.
    let input = "```\n**not bold\n```\n\nRegular text";
    assert!(matches!(super::heal(input), std::borrow::Cow::Borrowed(_)));
  }

  #[test]
  fn heal_bold_after_code_block() {
    let input = "```\ncode\n```\n\n**bold text";
    let healed = super::heal(input);
    assert!(healed.ends_with("**"));
  }

  #[test]
  fn heal_table_header_gets_separator() {
    let input = "| A | B | C |";
    let healed = super::heal(input);
    assert!(healed.contains("|---|---|---|"), "Expected separator, got: {healed}");
    let md = parse(&healed);
    assert!(md.tokens.iter().any(|t| matches!(t, Token::Table(_))));
  }

  #[test]
  fn heal_table_partial_separator() {
    let input = "| A | B | C |\n|:--";
    let healed = super::heal(input);
    // Should complete the separator to have 3 columns.
    let md = parse(&healed);
    assert!(md.tokens.iter().any(|t| matches!(t, Token::Table(_))), "Expected table, got tokens: {:?}", md.tokens);
  }

  #[test]
  fn heal_table_complete_separator_no_change() {
    let input = "| A | B |\n|---|---|";
    let healed = super::heal(input);
    let md = parse(&healed);
    assert!(md.tokens.iter().any(|t| matches!(t, Token::Table(_))));
  }

  #[test]
  fn heal_table_body_row_no_extra_separator() {
    let input = "| A | B |\n|---|---|\n| 1 | 2 |";
    // Body row in recognized table, no healing needed.
    assert!(matches!(super::heal(input), std::borrow::Cow::Borrowed(_)));
  }

  #[test]
  fn heal_table_partial_separator_ends_with_pipe() {
    let input = "| A | B | C |\n|---|";
    let healed = super::heal(input);
    let md = parse(&healed);
    assert!(md.tokens.iter().any(|t| matches!(t, Token::Table(_))), "Expected table, got: {healed}");
  }

  #[test]
  fn heal_not_a_table() {
    let input = "some text | with a pipe";
    assert!(matches!(super::heal(input), std::borrow::Cow::Borrowed(_)));
  }

  #[test]
  fn heal_tilde_fence() {
    let input = "~~~python\nprint('hi')";
    let healed = super::heal(input);
    assert!(healed.ends_with("```"));
    let md = parse(&healed);
    assert!(md.tokens.iter().any(|t| matches!(t, Token::CodeBlock { .. })));
  }

  #[test]
  fn heal_tilde_fence_closed() {
    let input = "~~~\ncode\n~~~";
    assert!(matches!(super::heal(input), std::borrow::Cow::Borrowed(_)));
  }

  #[test]
  fn heal_backslash_escape_star() {
    // \* is an escaped asterisk - should NOT trigger italic healing
    let input = "\\*not italic";
    assert!(matches!(super::heal(input), std::borrow::Cow::Borrowed(_)));
  }

  #[test]
  fn heal_backslash_escape_bracket() {
    let input = "\\[not a link";
    assert!(matches!(super::heal(input), std::borrow::Cow::Borrowed(_)));
  }

  #[test]
  fn heal_underscore_italic() {
    let healed = super::heal("_italic text");
    assert_eq!(&*healed, "_italic text_");
  }

  #[test]
  fn heal_underscore_bold() {
    let healed = super::heal("__bold text");
    assert_eq!(&*healed, "__bold text__");
  }

  #[test]
  fn heal_underscore_bold_italic() {
    let healed = super::heal("___bold italic");
    assert!(healed.ends_with("___"));
  }

  #[test]
  fn heal_link_url_with_parens() {
    // URL with balanced parens like Wikipedia links.
    let input = "[wiki](https://en.wikipedia.org/wiki/Foo_(bar)";
    let healed = super::heal(input);
    // The inner () should be tracked, only the outer ) closes the link.
    assert_eq!(&*healed, "[wiki](https://en.wikipedia.org/wiki/Foo_(bar))");
    let md = parse(&healed);
    if let Some(Token::Link { href, .. }) = md.tokens.iter().find(|t| matches!(t, Token::Link { .. })) {
      assert_eq!(href.as_ref(), "https://en.wikipedia.org/wiki/Foo_(bar)");
    } else {
      panic!("Expected Link token");
    }
  }

  #[test]
  fn heal_footnote_ref_not_link() {
    // [^1] should not be treated as a link
    let input = "text[^1";
    let healed = super::heal(input);
    // Should NOT produce [^1]() - footnote refs are not links.
    assert!(!healed.contains("]("), "Footnote ref should not be healed as link: {healed}");
  }

  #[test]
  fn heal_footnote_ref_complete() {
    let input = "text[^1]";
    assert!(matches!(super::heal(input), std::borrow::Cow::Borrowed(_)));
  }

  #[test]
  fn heal_image_unclosed_url() {
    let input = "![alt text](https://example.com/img.png";
    let healed = super::heal(input);
    assert_eq!(&*healed, "![alt text](https://example.com/img.png)");
  }

  #[test]
  fn heal_image_unclosed_alt() {
    let input = "![alt text";
    let healed = super::heal(input);
    assert_eq!(&*healed, "![alt text]()");
  }

  #[test]
  fn heal_closed_underscores_no_change() {
    let input = "__bold__ and _italic_";
    assert!(matches!(super::heal(input), std::borrow::Cow::Borrowed(_)));
  }

  #[test]
  fn heal_inline_code_contains_stars() {
    // Stars inside inline code should not trigger emphasis healing.
    let input = "`**not bold**`";
    assert!(matches!(super::heal(input), std::borrow::Cow::Borrowed(_)));
  }

  #[test]
  fn heal_multiple_links_second_unclosed() {
    let input = "[a](url1) and [b";
    let healed = super::heal(input);
    assert_eq!(&*healed, "[a](url1) and [b]()");
  }

  #[test]
  fn heal_tilde_fence_inline_ignored() {
    // Inline markers inside a closed tilde fence should not trigger healing.
    let input = "~~~\n**not bold\n~~~\n\nRegular text";
    assert!(matches!(super::heal(input), std::borrow::Cow::Borrowed(_)));
  }

  #[test]
  fn large_document() {
    let mut doc = String::new();
    for i in 0..100 {
      doc.push_str(&format!("# Heading {i}\n\nParagraph {i} with **bold** and *italic*.\n\n"));
      doc.push_str("```rust\nfn test() {}\n```\n\n");
      doc.push_str(&format!("- Item {i}\n"));
    }
    let md = parse(&doc);
    assert!(!md.tokens.is_empty());
  }

  #[test]
  fn link_with_title() {
    let md = parse("[click](https://example.com \"Example Title\")");
    let link = md.tokens.iter().find(|t| matches!(t, Token::Link { .. }));
    assert!(link.is_some());
    if let Some(Token::Link { text, href, title }) = link {
      assert_eq!(text.as_ref(), "click");
      assert_eq!(href.as_ref(), "https://example.com");
      assert_eq!(title.as_deref(), Some("Example Title"));
    }
  }

  #[test]
  fn link_without_title() {
    let md = parse("[click](https://example.com)");
    let link = md.tokens.iter().find(|t| matches!(t, Token::Link { .. }));
    if let Some(Token::Link { title, .. }) = link {
      assert!(title.is_none());
    }
  }

  #[test]
  fn links_in_table() {
    let md = parse("| Col |\n|-----|\n| [link](https://example.com) |");
    let table = md.tokens.iter().find(|t| matches!(t, Token::Table(_)));
    assert!(table.is_some());
    if let Some(Token::Table(data)) = table {
      assert!(!data.rows.is_empty());
      let cell = &data.rows[0][0];
      let has_link = cell.iter().any(|t| matches!(t, Token::Link { .. }));
      assert!(has_link, "Expected a Link token in table cell, got: {cell:?}");
    }
  }
}

#[cfg(test)]
mod bench_tests {
  use super::*;

  fn large_markdown() -> String {
    let mut doc = String::new();
    for i in 0..100 {
      doc.push_str(&format!("# Heading {i}\n\nParagraph {i} with **bold** and *italic* and `code`.\n\n"));
      doc.push_str(&format!("Visit [link {i}](https://example.com/{i}) for info.\n\n"));
      doc.push_str("```rust\nfn test() { println!(\"hello\"); }\n```\n\n");
      doc.push_str(&format!("- Item {i}\n  - Nested item\n"));
      doc.push_str(&format!("1. Ordered {i}\n2. Second\n\n"));
      doc.push_str("> Blockquote text\n\n");
      doc.push_str("| A | B |\n|---|---|\n| 1 | 2 |\n\n");
    }
    doc
  }

  #[test]
  fn bench_parse() {
    let doc = large_markdown();
    eprintln!("Document size: {} bytes, {} chars", doc.len(), doc.chars().count());

    // Warmup.
    for _ in 0..10 {
      std::hint::black_box(parse(&doc));
    }

    let iterations = 1000;
    let start = std::time::Instant::now();
    for _ in 0..iterations {
      std::hint::black_box(parse(&doc));
    }
    let elapsed = start.elapsed();
    eprintln!("parse(): {:?} per iter ({iterations} iterations)", elapsed / iterations);
  }
}

#[cfg(test)]
mod debug_tests {
  use super::*;
  #[test]
  fn debug_nested_bq() {
    let md = "> This is a blockquote.\n>\n> > Nested blockquotes work too.";
    let parsed = parse(md);
    for (i, token) in parsed.tokens.iter().enumerate() {
      eprintln!("{i}: {token:?}");
    }
  }
}
