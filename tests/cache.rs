use egui::{vec2, Color32, Context, FontId, Id, RawInput, Rect, UiBuilder};
use egui_markdown::{layout, MarkdownLabel, MarkdownStyle, OverflowWrap};

/// Render `text` into `ctx` and return every string painted as text.
fn painted_text(ctx: &Context, text: &str) -> Vec<String> {
  painted_text_with(ctx, text, false)
}

fn painted_text_with(ctx: &Context, text: &str, scroll_code_blocks: bool) -> Vec<String> {
  let screen = Rect::from_min_size(egui::pos2(0.0, 0.0), vec2(500.0, 2000.0));
  let mut output = ctx.run_ui(RawInput { screen_rect: Some(screen), ..Default::default() }, |ui| {
    let mut child = ui.new_child(UiBuilder::new().max_rect(screen));
    MarkdownLabel::new(Id::NULL.with("test"), text).scroll_code_blocks(scroll_code_blocks).show(&mut child);
  });
  output.textures_delta.clear();

  let mut out = Vec::new();
  for clipped in &output.shapes {
    collect(&clipped.shape, &mut out);
  }
  out
}

fn collect(shape: &egui::epaint::Shape, out: &mut Vec<String>) {
  match shape {
    egui::epaint::Shape::Text(t) => out.push(t.galley.text().to_owned()),
    egui::epaint::Shape::Vec(v) => v.iter().for_each(|s| collect(s, out)),
    _ => {}
  }
}

/// Editing a word must be reflected even when the edit does not change the token count.
#[test]
fn edit_within_segmented_doc_is_reflected() {
  // A table forces the segmented render path, which caches each flush range separately.
  let doc = |word: &str| format!("Intro {word} paragraph.\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\nOutro.");

  let ctx = Context::default();
  let before = painted_text(&ctx, &doc("alpha"));
  assert!(before.iter().any(|t| t.contains("alpha")), "first render missing 'alpha': {before:?}");

  let after = painted_text(&ctx, &doc("bravo"));
  assert!(after.iter().any(|t| t.contains("bravo")), "edited text not re-rendered: {after:?}");
  assert!(!after.iter().any(|t| t.contains("alpha")), "stale galley still painted: {after:?}");
}

/// Same, for an edit after the block element (a later flush range).
#[test]
fn edit_after_block_is_reflected() {
  let doc = |word: &str| format!("Intro.\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\nOutro {word} here.");

  let ctx = Context::default();
  let before = painted_text(&ctx, &doc("alpha"));
  assert!(before.iter().any(|t| t.contains("alpha")), "first render missing 'alpha': {before:?}");

  let after = painted_text(&ctx, &doc("bravo"));
  assert!(after.iter().any(|t| t.contains("bravo")), "edited text not re-rendered: {after:?}");
  assert!(!after.iter().any(|t| t.contains("alpha")), "stale galley still painted: {after:?}");
}

/// Editing the body of a scrolling fence must invalidate the code-block galley cache.
#[test]
fn edit_within_scrolling_code_block_is_reflected() {
  let doc = |word: &str| format!("Intro.\n\n```rust\nfn main() {{\n    let x = \"{word}\";\n}}\n```\n\nOutro.");

  let ctx = Context::default();
  let before = painted_text_with(&ctx, &doc("alpha"), true);
  assert!(before.iter().any(|t| t.contains("alpha")), "first render missing 'alpha': {before:?}");

  let after = painted_text_with(&ctx, &doc("bravo"), true);
  assert!(after.iter().any(|t| t.contains("bravo")), "edited fence not re-rendered: {after:?}");
  assert!(!after.iter().any(|t| t.contains("alpha")), "stale code galley still painted: {after:?}");
}

/// Growing a scrolling fence mid-line then completing the line must paint the new tail.
#[test]
fn streaming_append_scrolling_code_block_is_reflected() {
  let ctx = Context::default();

  let partial = "Intro.\n\n```rust\nfn main() {\n    let x = \"alp\n```\n";
  // heal closes the fence; body is still the incomplete string inside.
  let mid = painted_text_with(&ctx, partial, true);
  assert!(mid.iter().any(|t| t.contains("alp")), "partial stream missing 'alp': {mid:?}");
  assert!(!mid.iter().any(|t| t.contains("alpha")), "partial stream should not yet contain 'alpha': {mid:?}");

  let grown = "Intro.\n\n```rust\nfn main() {\n    let x = \"alpha\";\n}\n```\n";
  let after = painted_text_with(&ctx, grown, true);
  assert!(after.iter().any(|t| t.contains("alpha")), "appended stream missing 'alpha': {after:?}");
}

/// A non-prefix edit of a scrolling fence must rebuild rather than keep a stale galley.
#[test]
fn non_prefix_edit_scrolling_code_block_is_reflected() {
  let ctx = Context::default();

  let first = "```rust\nfn alpha() {}\n```\n";
  let before = painted_text_with(&ctx, first, true);
  assert!(before.iter().any(|t| t.contains("alpha")), "first render missing 'alpha': {before:?}");

  let second = "```rust\nfn bravo() {}\n```\n";
  let after = painted_text_with(&ctx, second, true);
  assert!(after.iter().any(|t| t.contains("bravo")), "non-prefix edit missing 'bravo': {after:?}");
  assert!(!after.iter().any(|t| t.contains("alpha")), "stale fence still painted: {after:?}");
}

/// `needs_segmentation` decides the render path without laying out; it must agree with the
/// segment breaks `build_layout` would have produced.
#[test]
fn needs_segmentation_matches_build_layout() {
  let docs = [
    "Just **plain** text with `code` and a [link](https://example.com).",
    "# Heading\n\nParagraph.\n\n---\n\nAfter the rule.",
    "- one\n- two\n  - nested\n\n1. first\n2. second",
    "Intro.\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\nOutro.",
    "> quoted **text**\n>\n> > nested",
    "Text\n\n```rust\nfn main() {}\n```\n\nMore.",
    "![alt](https://example.com/x.png)",
    "A footnote[^1].\n\n[^1]: body.",
    "- [x] done\n- [ ] todo",
  ];

  let ctx = Context::default();
  let screen = Rect::from_min_size(egui::pos2(0.0, 0.0), vec2(500.0, 2000.0));
  let style = MarkdownStyle::default();

  let mut output = ctx.run_ui(RawInput { screen_rect: Some(screen), ..Default::default() }, |ui| {
    for doc in docs {
      for scroll_code_blocks in [false, true] {
        let md = egui_markdown::parse(doc);
        let predicted = layout::needs_segmentation(&md.tokens, scroll_code_blocks, None);
        let built = layout::build_layout(
          ui,
          &md.tokens,
          FontId::proportional(14.0),
          Color32::WHITE,
          None,
          ui.available_width(),
          false,
          None,
          scroll_code_blocks,
          &style,
          Default::default(),
        );
        assert_eq!(
          predicted,
          !built.segment_breaks.is_empty(),
          "scroll_code_blocks={scroll_code_blocks} disagreement on:\n{doc}"
        );
      }
    }
  });
  output.textures_delta.clear();
}

/// A label that saw a reply stream in, one piece at a time with heal on, keeps tokens from earlier
/// parses. It must paint what a label that sees only the final text paints.
#[test]
fn a_streamed_reply_paints_what_a_fresh_label_paints() {
  let reply = "# Plan\n\nFirst **step** with `code`.\n\n- one\n- two\n\n  more of two\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\n```rust\nfn main() {}\n\nlet x = 1;\n```\n\n> quoted\n\nDone, with a [link](https://example.com) and *emphasis*.\n";
  let render = |ctx: &Context, text: &str| {
    let screen = Rect::from_min_size(egui::pos2(0.0, 0.0), vec2(500.0, 2000.0));
    let mut output = ctx.run_ui(RawInput { screen_rect: Some(screen), ..Default::default() }, |ui| {
      let mut child = ui.new_child(UiBuilder::new().max_rect(screen));
      MarkdownLabel::new(Id::NULL.with("stream"), text).heal(true).show(&mut child);
    });
    output.textures_delta.clear();
    let mut out = Vec::new();
    for clipped in &output.shapes {
      collect(&clipped.shape, &mut out);
    }
    out
  };

  let streamed = Context::default();
  let mut end = 0;
  while end < reply.len() {
    end = (end + 5).min(reply.len());
    while !reply.is_char_boundary(end) {
      end += 1;
    }
    render(&streamed, &reply[..end]);
  }
  let from_stream = render(&streamed, reply);
  let fresh = render(&Context::default(), reply);
  assert_eq!(from_stream, fresh);
}

/// A document that the label renders as one galley.
const WHOLE: &str = "Hello **bold** [link](https://example.com) and `code`.\n\n- item";
/// The same document with a table, which makes the label render each range of text on its own.
const SEGMENTED: &str = "Hello **bold** [link](https://example.com) and `code`.\n\n- item\n\n| a |\n|---|\n| 1 |";

/// The job and the size of each galley that a frame paints.
type Painted = Vec<(egui::text::LayoutJob, egui::Vec2)>;

fn painted_jobs(ctx: &Context, show: &dyn Fn(&mut egui::Ui)) -> Painted {
  painted_galleys(ctx, show).iter().map(|galley| ((*galley.job).clone(), galley.size())).collect()
}

/// The galleys that a frame paints.
fn painted_galleys(ctx: &Context, show: &dyn Fn(&mut egui::Ui)) -> Vec<std::sync::Arc<egui::Galley>> {
  let screen = Rect::from_min_size(egui::pos2(0.0, 0.0), vec2(500.0, 2000.0));
  let mut output = ctx.run_ui(RawInput { screen_rect: Some(screen), ..Default::default() }, |ui| {
    let mut child = ui.new_child(UiBuilder::new().max_rect(screen));
    show(&mut child);
  });
  output.textures_delta.clear();
  let mut galleys = Vec::new();
  for clipped in &output.shapes {
    collect_galleys(&clipped.shape, &mut galleys);
  }
  galleys
}

fn collect_galleys(shape: &egui::epaint::Shape, out: &mut Vec<std::sync::Arc<egui::Galley>>) {
  match shape {
    egui::epaint::Shape::Text(t) => out.push(t.galley.clone()),
    egui::epaint::Shape::Vec(v) => v.iter().for_each(|s| collect_galleys(s, out)),
    _ => {}
  }
}

/// Two frames of `show`. The last frame is what the label paints when nothing changes.
fn settled_jobs(ctx: &Context, show: &dyn Fn(&mut egui::Ui)) -> Painted {
  painted_jobs(ctx, show);
  painted_jobs(ctx, show)
}

/// Render `before` in a context, apply `change` to the context, and render `after`. A fresh context that gets
/// only `change` and `after` builds each layout again. The two contexts must paint the same jobs at the same
/// sizes, so no cache of the label keeps a layout from before the change. The change must also change what the
/// label paints, so that the test checks something.
fn assert_follows_change(
  name: &str,
  before: impl Fn(&mut egui::Ui),
  change: impl Fn(&Context),
  after: impl Fn(&mut egui::Ui),
) {
  let ctx = Context::default();
  let painted_before = settled_jobs(&ctx, &before);
  change(&ctx);
  let cached = settled_jobs(&ctx, &after);

  let fresh_ctx = Context::default();
  change(&fresh_ctx);
  let fresh = settled_jobs(&fresh_ctx, &after);

  assert_ne!(painted_before, fresh, "{name}: the change does not change what the label paints");
  assert_eq!(cached, fresh, "{name}: the label paints a layout from before the change");
}

fn show_doc(doc: &'static str, font: FontId, style: MarkdownStyle) -> impl Fn(&mut egui::Ui) {
  move |ui| {
    MarkdownLabel::new(Id::NULL.with("label"), doc).font(font.clone()).style(&style).show(ui);
  }
}

#[test]
fn a_label_follows_a_change_of_its_font() {
  for doc in [WHOLE, SEGMENTED] {
    let fixed = MarkdownStyle { code_font_size: Some(11.0), ..Default::default() };
    assert_follows_change(
      &format!("size, {doc:?}"),
      show_doc(doc, FontId::proportional(20.0), fixed.clone()),
      |_| {},
      show_doc(doc, FontId::proportional(24.0), fixed),
    );
    assert_follows_change(
      &format!("family, {doc:?}"),
      show_doc(doc, FontId::proportional(14.0), MarkdownStyle::default()),
      |_| {},
      show_doc(doc, FontId::monospace(14.0), MarkdownStyle::default()),
    );
  }
}

#[test]
fn a_label_follows_a_change_of_the_egui_body_style() {
  for doc in [WHOLE, SEGMENTED] {
    let show = move |ui: &mut egui::Ui| {
      MarkdownLabel::new(Id::NULL.with("label"), doc).show(ui);
    };
    let change = |ctx: &Context| {
      ctx.all_styles_mut(|style| {
        style.text_styles.insert(egui::TextStyle::Body, FontId::proportional(22.0));
      });
    };
    assert_follows_change(&format!("{doc:?}"), show, change, show);
  }
}

#[test]
fn a_label_follows_a_change_of_the_visuals() {
  type Change = fn(&mut egui::Visuals);
  let changes: [(&str, Change); 3] = [
    ("light mode", |visuals| *visuals = egui::Visuals::light()),
    ("hyperlink color", |visuals| visuals.hyperlink_color = Color32::from_rgb(1, 2, 3)),
    ("code corner radius", |visuals| visuals.code.corner_radius = egui::CornerRadius::same(9)),
  ];
  for doc in [WHOLE, SEGMENTED] {
    for (name, change) in changes {
      let show = show_doc(doc, FontId::proportional(14.0), MarkdownStyle::default());
      assert_follows_change(
        &format!("{name}, {doc:?}"),
        &show,
        |ctx| ctx.all_styles_mut(|s| change(&mut s.visuals)),
        &show,
      );
    }
  }
}

#[test]
fn a_label_follows_a_bold_family_that_the_app_registers_later() {
  for doc in [WHOLE, SEGMENTED] {
    let show = show_doc(doc, FontId::proportional(14.0), MarkdownStyle::default());
    let change = |ctx: &Context| {
      let mut fonts = egui::FontDefinitions::default();
      let proportional = fonts.families[&egui::FontFamily::Proportional].clone();
      fonts.families.insert(egui::FontFamily::Name("bold".into()), proportional);
      ctx.set_fonts(fonts);
    };
    assert_follows_change(&format!("{doc:?}"), &show, change, &show);
  }
}

#[test]
fn a_label_follows_a_change_of_max_lines() {
  let show = |max_lines: u32| {
    move |ui: &mut egui::Ui| {
      MarkdownLabel::new(Id::NULL.with("label"), "one\n\ntwo\n\nthree\n\nfour").max_lines(max_lines).show(ui);
    }
  };
  assert_follows_change("max_lines", show(1), |_| {}, show(3));
}

#[test]
fn a_label_follows_a_change_of_scroll_code_blocks() {
  let show = |scroll: bool| {
    move |ui: &mut egui::Ui| {
      MarkdownLabel::new(Id::NULL.with("label"), "Intro.\n\n```rust\nfn main() {}\n```")
        .scroll_code_blocks(scroll)
        .show(ui);
    }
  };
  assert_follows_change("scroll_code_blocks", show(false), |_| {}, show(true));
}

#[cfg(feature = "syntax_highlighting")]
#[test]
fn a_label_follows_a_code_theme_that_the_app_installs_later() {
  let show = |ui: &mut egui::Ui| {
    MarkdownLabel::new(Id::NULL.with("label"), "Intro.\n\n```rust\nfn main() {}\n```").show(ui);
  };
  let change = |ctx: &Context| {
    let theme = syntect::highlighting::ThemeSet::load_defaults().themes["InspiredGitHub"].clone();
    egui_markdown::set_code_themes(ctx, theme.clone(), theme);
  };
  assert_follows_change("code theme", show, change, show);
}

/// `calculate_size` measures in an invisible `Ui`, so it uses the cached size of each range of text that it
/// measured before. A change of the break flag changes the rows of a range, so it must measure the range again.
#[test]
fn calculate_size_follows_a_change_of_overflow_wrap() {
  let doc =
    "alpha bravissimo charlie deltadeltadelta echo foxtrotfoxtrot golf hotelhotelhotel india julietjuliet kilo \
             limalimalima mike novembernovember\n\n| a |\n|---|\n| 1 |";
  let size = |ctx: &Context, overflow_wrap: OverflowWrap| {
    let screen = Rect::from_min_size(egui::pos2(0.0, 0.0), vec2(120.0, 2000.0));
    let mut size = egui::Vec2::ZERO;
    let mut output = ctx.run_ui(RawInput { screen_rect: Some(screen), ..Default::default() }, |ui| {
      let mut child = ui.new_child(UiBuilder::new().max_rect(screen));
      size = MarkdownLabel::new(Id::NULL.with("label"), doc).overflow_wrap(overflow_wrap).calculate_size(&mut child);
    });
    output.textures_delta.clear();
    size
  };
  let settled = |ctx: &Context, overflow_wrap| {
    size(ctx, overflow_wrap);
    size(ctx, overflow_wrap)
  };

  let ctx = Context::default();
  let normal = settled(&ctx, OverflowWrap::Normal);
  let cached = settled(&ctx, OverflowWrap::BreakAll);
  let fresh = settled(&Context::default(), OverflowWrap::BreakAll);
  assert_ne!(normal, fresh, "the break flag does not change the size");
  assert_eq!(cached, fresh, "the label measures with the break flag from before the change");
}

/// A bold token of the code theme uses the "mono-bold" family when the app registers one, in a code block that
/// scrolls and in one that does not.
#[cfg(feature = "syntax_highlighting")]
#[test]
fn a_code_block_follows_a_mono_bold_family_that_the_app_registers_later() {
  let theme = syntect::highlighting::ThemeSet::load_defaults().themes["InspiredGitHub"].clone();
  for scroll in [true, false] {
    let show = |ui: &mut egui::Ui| {
      MarkdownLabel::new(Id::NULL.with("label"), "```rust\nstruct A;\nimpl A {}\n```")
        .scroll_code_blocks(scroll)
        .code_theme(&theme)
        .show(ui);
    };
    let change = |ctx: &Context| {
      let mut fonts = egui::FontDefinitions::default();
      let monospace = fonts.families[&egui::FontFamily::Monospace].clone();
      fonts.families.insert(egui::FontFamily::Name("mono-bold".into()), monospace);
      ctx.set_fonts(fonts);
    };
    assert_follows_change(&format!("scroll_code_blocks={scroll}"), show, change, show);
  }
}

/// A galley from before `set_fonts` has the glyph advances and the atlas positions of the old fonts, so a scrolling
/// code block lays out again.
#[test]
fn a_scrolling_code_block_follows_set_fonts() {
  let show = |ui: &mut egui::Ui| {
    MarkdownLabel::new(Id::NULL.with("label"), "```rust\nfn main() {}\n```").scroll_code_blocks(true).show(ui);
  };
  let change = |ctx: &Context| {
    let mut fonts = egui::FontDefinitions::default();
    let proportional = fonts.families[&egui::FontFamily::Proportional].clone();
    fonts.families.insert(egui::FontFamily::Monospace, proportional);
    ctx.set_fonts(fonts);
  };
  assert_follows_change("set_fonts", show, change, show);
}

/// egui clears the font atlas when it is more than 80% full, and a galley from before the reset paints parts of
/// other glyphs. The size of the galley does not change, so the test compares the atlas position of each glyph
/// with a galley that egui lays out after the reset.
#[test]
fn a_scrolling_code_block_follows_an_atlas_reset() {
  let ctx = Context::default();
  let large = |ctx: &Context, text: &str, size: f32| {
    let job = egui::text::LayoutJob::simple(text.to_owned(), FontId::proportional(size), Color32::WHITE, f32::INFINITY);
    ctx.fonts_mut(|f| f.layout_job(job));
  };
  // Each frame shows the label. `fill` lays out large glyphs that fill the atlas.
  let frame = |fill: Option<usize>| {
    let show = |ui: &mut egui::Ui| {
      if let Some(n) = fill {
        for i in 0..20 {
          large(ui.ctx(), "ABCDEFGHIJKLMNOPQRSTUVWXYZ", 200.0 + (n * 20 + i) as f32);
        }
      }
      MarkdownLabel::new(Id::NULL.with("label"), "```rust\nfn main() {}\n```").scroll_code_blocks(true).show(ui);
    };
    let painted = painted_galleys(&ctx, &show);
    painted.into_iter().find(|galley| galley.text().contains("main")).expect("the code block is painted")
  };
  // Glyphs before the first code block, so that the code glyphs are not at the start of the atlas.
  painted_galleys(&ctx, &|ui| large(ui.ctx(), "abcdefghij", 150.0));
  frame(None);
  frame(None);

  let fill_ratio = || ctx.fonts(|f| f.font_atlas_fill_ratio());
  let mut n = 0;
  while fill_ratio() <= 0.8 {
    frame(Some(n));
    n += 1;
    assert!(n < 50, "the atlas does not fill");
  }
  // egui resets the atlas at the start of this frame.
  let painted = frame(None);
  assert!(fill_ratio() < 0.8, "egui did not reset the atlas");

  let fresh = ctx.fonts_mut(|f| f.layout_job((*painted.job).clone()));
  let uv = |galley: &egui::Galley| -> Vec<_> {
    galley.rows.iter().flat_map(|row| row.glyphs.iter().map(|g| g.uv_rect)).collect()
  };
  assert_eq!(uv(&painted), uv(&fresh), "the code block paints glyphs from before the atlas reset");
}

/// A font definition that swaps the proportional and the monospace families, so every glyph gets a new width.
fn swap_font_families(ctx: &Context) {
  let mut fonts = egui::FontDefinitions::default();
  let proportional = fonts.families[&egui::FontFamily::Proportional].clone();
  let monospace = fonts.families[&egui::FontFamily::Monospace].clone();
  fonts.families.insert(egui::FontFamily::Proportional, monospace);
  fonts.families.insert(egui::FontFamily::Monospace, proportional);
  ctx.set_fonts(fonts);
}

/// The label measures the width of a list marker with the fonts, and keeps the result in its layout.
#[test]
fn a_list_follows_set_fonts() {
  for doc in ["- item\n- other\n  - nested", "- item\n- other\n  - nested\n\n| a |\n|---|\n| 1 |"] {
    let show = move |ui: &mut egui::Ui| {
      MarkdownLabel::new(Id::NULL.with("label"), doc).show(ui);
    };
    assert_follows_change(&format!("{doc:?}"), show, swap_font_families, show);
  }
}

/// The label measures the indent of each line of a code block with the fonts, and keeps the result in its layout.
#[test]
fn an_indented_code_line_follows_set_fonts() {
  let show = |ui: &mut egui::Ui| {
    MarkdownLabel::new(Id::NULL.with("label"), "Intro.\n\n```rust\nfn main() {\n        x();\n}\n```").show(ui);
  };
  assert_follows_change("set_fonts", show, swap_font_families, show);
}

/// Without syntax highlighting, a scrolling code block has the color of `Visuals::code`.
#[cfg(not(feature = "syntax_highlighting"))]
#[test]
fn a_plain_scrolling_code_block_follows_the_code_color() {
  let show = |ui: &mut egui::Ui| {
    MarkdownLabel::new(Id::NULL.with("label"), "```rust\nfn main() {}\n```").scroll_code_blocks(true).show(ui);
  };
  let change = |ctx: &Context| ctx.all_styles_mut(|style| style.visuals.code.text_color = Some(Color32::RED));
  assert_follows_change("code color", show, change, show);
}
