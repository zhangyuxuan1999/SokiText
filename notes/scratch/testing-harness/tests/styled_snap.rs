use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};
use probe::{Core, render};

/// TestBackend's Display drops ALL styling. This emits a second "style layer":
/// one legend char per distinct (fg,bg,modifier) triple, aligned to the text grid.
fn styled_view(buf: &Buffer) -> String {
    let mut styles: Vec<ratatui::style::Style> = Vec::new();
    let mut text = String::new();
    let mut mask = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            let c = &buf[(x, y)];
            text.push_str(c.symbol());
            let st = c.style();
            let idx = styles.iter().position(|s| *s == st).unwrap_or_else(|| { styles.push(st); styles.len() - 1 });
            mask.push((b'a' + idx as u8) as char);
        }
        text.push('\n'); mask.push('\n');
    }
    let mut out = String::from("--- text ---\n");
    out.push_str(&text);
    out.push_str("--- styles ---\n");
    out.push_str(&mask);
    out.push_str("--- legend ---\n");
    for (i, s) in styles.iter().enumerate() {
        out.push_str(&format!("{} = fg:{:?} bg:{:?} mod:{:?}\n", (b'a'+i as u8) as char, s.fg, s.bg, s.add_modifier));
    }
    out
}

#[test]
fn styled_snapshot_catches_colors() {
    let mut core = Core::new("ab");
    core.dirty = true;
    let mut term = Terminal::new(TestBackend::new(12, 4)).unwrap();
    term.draw(|f| render::draw(f, &core)).unwrap();
    insta::assert_snapshot!(styled_view(term.backend().buffer()));
}

#[test]
fn cjk_and_emoji_widths() {
    let core = Core::new("中文abc\n👨‍👩‍👧‍👦x");
    let mut term = Terminal::new(TestBackend::new(14, 5)).unwrap();
    term.draw(|f| render::draw(f, &core)).unwrap();
    insta::assert_snapshot!(term.backend());
}
