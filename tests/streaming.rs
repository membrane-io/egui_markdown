//! Each fixture in `tests/fixtures` holds the text deltas of one streamed reply, as a JSON array of strings.
//! The test heals and parses each prefix of deltas, and it compares prefix `n` with prefix `n + 1`. A reader
//! sees each prefix for at least one frame, so a change that the next delta takes back shows as a defect.

use egui_markdown::{heal, parse, Token};

/// The parts of a parsed prefix that a reader can see.
#[derive(Default)]
struct Visible {
  text: String,
  /// The character offset in `text` of each rule.
  rules: Vec<usize>,
  /// The character offset in `text` and the href of each link.
  links: Vec<(usize, String)>,
  /// The heading level of each character in `text`.
  headings: Vec<Option<u8>>,
  /// The character offset in `text` and the column count of each table.
  tables: Vec<(usize, usize)>,
}

impl Visible {
  fn of(prefix: &str) -> Self {
    let healed = heal(prefix);
    let md = parse(&healed);
    let mut visible = Visible::default();
    visible.extend(&md.tokens);
    visible
  }

  fn extend(&mut self, tokens: &[Token]) {
    for token in tokens {
      match token {
        Token::Text { text, style } => self.push(text, style.heading),
        Token::CodeBlock { text, .. } => self.push(text, None),
        Token::ListMarker { marker, .. } => self.push(marker, None),
        Token::Newline => self.push("\n", None),
        Token::Link { text, href, .. } => {
          self.links.push((self.len(), href.to_string()));
          self.push(text, None);
        }
        Token::HorizontalRule => self.rules.push(self.len()),
        Token::Table(table) => {
          self.tables.push((self.len(), table.headers.len()));
          for cell in table.headers.iter().chain(table.rows.iter().flatten()) {
            self.extend(cell);
          }
        }
        _ => {}
      }
    }
  }

  fn len(&self) -> usize {
    self.headings.len()
  }

  fn push(&mut self, text: &str, heading: Option<u8>) {
    self.text.push_str(text);
    self.headings.extend(text.chars().map(|_| heading));
  }
}

/// The first rule that `after` breaks, compared with `before`.
fn defect(before: &Visible, after: &Visible) -> Option<String> {
  if !after.text.starts_with(&before.text) {
    return Some(format!("the visible text {:?} became {:?}", before.text, after.text));
  }
  for rule in &before.rules {
    if !after.rules.contains(rule) {
      return Some(format!("the rule at offset {rule} went away"));
    }
  }
  for (offset, href) in before.links.iter().filter(|(_, href)| !href.is_empty()) {
    if !after.links.iter().any(|(at, next)| at == offset && next == href) {
      return Some(format!("the link at offset {offset} with href {href:?} changed"));
    }
  }
  for table in &before.tables {
    if !after.tables.contains(table) {
      return Some(format!("the table at offset {} with {} columns changed", table.0, table.1));
    }
  }
  for (offset, level) in before.headings.iter().enumerate() {
    if level.is_some() && after.headings.get(offset) != Some(level) {
      return Some(format!("the heading level {level:?} at offset {offset} changed"));
    }
  }
  None
}

#[test]
fn a_streamed_prefix_does_not_take_back_what_it_showed() {
  let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
  let mut paths: Vec<_> = std::fs::read_dir(&dir).unwrap().map(|entry| entry.unwrap().path()).collect();
  paths.sort();
  let mut failures = Vec::new();
  for path in paths.iter().filter(|path| path.extension().is_some_and(|ext| ext == "json")) {
    let name = path.file_stem().unwrap().to_string_lossy();
    let deltas: Vec<String> = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let prefixes: Vec<String> = (1..=deltas.len()).map(|n| deltas[..n].concat()).collect();
    for (n, pair) in prefixes.windows(2).enumerate() {
      if let Some(defect) = defect(&Visible::of(&pair[0]), &Visible::of(&pair[1])) {
        failures.push(format!(
          "{name}, n = {}: {defect}\n  prefix n:     {:?}\n  prefix n + 1: {:?}",
          n + 1,
          pair[0],
          pair[1]
        ));
      }
    }
  }
  assert!(failures.is_empty(), "{} defects:\n{}", failures.len(), failures.join("\n"));
}
