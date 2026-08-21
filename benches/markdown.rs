use criterion::{black_box, criterion_group, criterion_main, Criterion};

use egui_markdown::parse;

fn generate_document(sections: usize) -> String {
  let mut doc = String::new();
  for i in 0..sections {
    match i % 5 {
      0 => {
        doc.push_str(&format!("## Heading {i}\n\n"));
        doc.push_str("Some **bold** text and *italic* text with `inline code`.\n\n");
      }
      1 => {
        doc.push_str("```rust\nfn example() {\n    let x = 42;\n    println!(\"{x}\");\n}\n```\n\n");
      }
      2 => {
        doc.push_str("- Item one\n- Item two\n  - Nested item\n- Item three\n\n");
      }
      3 => {
        doc.push_str("| Col A | Col B | Col C |\n|-------|-------|-------|\n");
        doc.push_str("| cell  | cell  | cell  |\n| cell  | cell  | cell  |\n\n");
      }
      4 => {
        doc.push_str("> Blockquote with **bold** and a [link](https://example.com).\n\n");
        doc.push_str("---\n\n");
      }
      _ => unreachable!(),
    }
  }
  doc
}

fn bench_parse(c: &mut Criterion) {
  let doc = generate_document(100);
  c.bench_function("parse_100_sections", |b| {
    b.iter(|| {
      let md = parse(black_box(&doc));
      black_box(&md.tokens);
    });
  });
}

fn bench_hash_text(c: &mut Criterion) {
  use std::collections::hash_map::DefaultHasher;
  use std::hash::{Hash, Hasher};

  let doc = generate_document(100);

  c.bench_function("hash_text_100_sections", |b| {
    b.iter(|| {
      let mut hasher = DefaultHasher::new();
      black_box(&doc).hash(&mut hasher);
      black_box(hasher.finish());
    });
  });
}

fn bench_hash_token_slice(c: &mut Criterion) {
  use std::collections::hash_map::DefaultHasher;
  use std::hash::{Hash, Hasher};

  let doc = generate_document(100);
  let md = parse(&doc);
  let tokens = &md.tokens;

  c.bench_function("hash_token_slice_100_sections", |b| {
    b.iter(|| {
      let mut hasher = DefaultHasher::new();
      tokens.hash(&mut hasher);
      black_box(hasher.finish());
    });
  });
}

fn bench_arc_clone(c: &mut Criterion) {
  use std::sync::Arc;

  let doc = generate_document(100);
  let md = parse(&doc);
  let tokens: Vec<egui_markdown::Token<'static>> = md
    .tokens
    .iter()
    .map(|t| {
      // Simple owned clone for benchmarking
      match t {
        egui_markdown::Token::Newline => egui_markdown::Token::Newline,
        egui_markdown::Token::Text { text, style } => egui_markdown::Token::Text {
          text: pulldown_cmark::CowStr::Boxed(text.to_string().into_boxed_str()),
          style: style.clone(),
        },
        other => egui_markdown::Token::Text {
          text: pulldown_cmark::CowStr::Boxed(other.text().to_string().into_boxed_str()),
          style: Default::default(),
        },
      }
    })
    .collect();
  let arc_tokens = Arc::new(tokens);

  c.bench_function("arc_clone_tokens", |b| {
    b.iter(|| {
      let cloned = Arc::clone(black_box(&arc_tokens));
      black_box(&cloned);
    });
  });
}

/// End-to-end frame cost of the widget: at a constant width every cache hits, while a
/// resize changes the wrap width each frame and forces re-shaping.
fn bench_render(c: &mut Criterion) {
  use egui::{vec2, Context, Id, RawInput, Rect, UiBuilder};
  use egui_markdown::MarkdownLabel;

  let doc = generate_document(20);

  let frame = |ctx: &Context, width: f32| {
    let screen = Rect::from_min_size(egui::pos2(0.0, 0.0), vec2(width, 700.0));
    let _ = ctx.run_ui(RawInput { screen_rect: Some(screen), ..Default::default() }, |ui| {
      let mut child = ui.new_child(UiBuilder::new().max_rect(screen));
      MarkdownLabel::new(Id::new("md"), black_box(&doc)).show(&mut child);
    });
  };

  let ctx = Context::default();
  for _ in 0..10 {
    frame(&ctx, 700.0);
  }
  c.bench_function("render_steady_state", |b| b.iter(|| frame(&ctx, 700.0)));

  let ctx = Context::default();
  let mut width = 600.0f32;
  c.bench_function("render_resizing", |b| {
    b.iter(|| {
      width = if width >= 800.0 { 600.0 } else { width + 1.0 };
      frame(&ctx, width);
    });
  });
}

/// Steady-state cost of a large scrolling fence after the galley cache is warm.
fn bench_render_scroll_code(c: &mut Criterion) {
  use egui::{vec2, Context, Id, RawInput, Rect, UiBuilder};
  use egui_markdown::MarkdownLabel;
  use std::fmt::Write as _;

  let mut fence = String::from("```rust\nfn claim(log: &Log, seq: u64) -> Result<(), ClaimError> {\n");
  for idx in 0..200 {
    let _ = writeln!(fence, "    let step_{idx} = log.tail()?;");
  }
  fence.push_str("    log.insert(seq)\n}\n```\n");

  let frame = |ctx: &Context| {
    let screen = Rect::from_min_size(egui::pos2(0.0, 0.0), vec2(700.0, 900.0));
    let _ = ctx.run_ui(RawInput { screen_rect: Some(screen), ..Default::default() }, |ui| {
      let mut child = ui.new_child(UiBuilder::new().max_rect(screen));
      MarkdownLabel::new(Id::new("md"), black_box(&fence)).scroll_code_blocks(true).show(&mut child);
    });
  };

  let ctx = Context::default();
  for _ in 0..10 {
    frame(&ctx);
  }
  c.bench_function("render_scroll_code_steady_state", |b| b.iter(|| frame(&ctx)));
}

/// Cost of appending one complete line to a warm scrolling fence (should stay near O(line)).
fn bench_render_scroll_code_streaming(c: &mut Criterion) {
  use egui::{vec2, Context, Id, RawInput, Rect, UiBuilder};
  use egui_markdown::MarkdownLabel;
  use std::fmt::Write as _;

  let mut body = String::from("fn claim(log: &Log, seq: u64) -> Result<(), ClaimError> {\n");
  for idx in 0..100 {
    let _ = writeln!(body, "    let step_{idx} = log.tail()?;");
  }

  let ctx = Context::default();
  let screen = Rect::from_min_size(egui::pos2(0.0, 0.0), vec2(700.0, 900.0));
  // Warm with the initial fence.
  let warm = format!("```rust\n{body}```\n");
  let _ = ctx.run_ui(RawInput { screen_rect: Some(screen), ..Default::default() }, |ui| {
    let mut child = ui.new_child(UiBuilder::new().max_rect(screen));
    MarkdownLabel::new(Id::new("md"), &warm).scroll_code_blocks(true).show(&mut child);
  });

  let mut line_idx = 100usize;
  c.bench_function("render_scroll_code_streaming_append", |b| {
    b.iter(|| {
      let _ = writeln!(body, "    let step_{line_idx} = log.tail()?;");
      line_idx += 1;
      let fence = format!("```rust\n{body}```\n");
      let _ = ctx.run_ui(RawInput { screen_rect: Some(screen), ..Default::default() }, |ui| {
        let mut child = ui.new_child(UiBuilder::new().max_rect(screen));
        MarkdownLabel::new(Id::new("md"), black_box(&fence)).scroll_code_blocks(true).show(&mut child);
      });
    });
  });
}

criterion_group!(
  benches,
  bench_parse,
  bench_hash_text,
  bench_hash_token_slice,
  bench_arc_clone,
  bench_render,
  bench_render_scroll_code,
  bench_render_scroll_code_streaming
);
criterion_main!(benches);
