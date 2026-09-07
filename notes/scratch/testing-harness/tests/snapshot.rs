use probe::{Core, Key, keymap, render};
use ratatui::{Terminal, backend::TestBackend};

/// tiny keystroke-script DSL: literal chars, <esc-like> names in angle brackets
fn parse_keys(s: &str) -> Vec<Key> {
    let mut out = Vec::new();
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        if c == '<' {
            let mut name = String::new();
            while let Some(&n) = it.peek() { it.next(); if n == '>' { break } name.push(n) }
            out.push(match name.as_str() {
                "left" => Key::Left, "right" => Key::Right,
                "bs" => Key::Backspace, "ret" => Key::Enter,
                "C-s" => Key::CtrlS, "C-q" => Key::CtrlQ,
                other => panic!("unknown key <{other}>"),
            });
        } else { out.push(Key::Char(c)) }
    }
    out
}

fn run(initial: &str, script: &str) -> Core {
    let mut core = Core::new(initial);
    for k in parse_keys(script) { if let Some(cmd) = keymap(k) { core.apply(cmd); } }
    core
}

#[test]
fn dsl_edits_buffer() {
    let c = run("", "hello<bs><bs>p!");
    similar_asserts::assert_eq!(c.text.to_string(), "help!");
}

#[test]
fn snapshot_screen() {
    let core = run("", "fn main() {<ret>    println!();<ret>}");
    let mut term = Terminal::new(TestBackend::new(30, 8)).unwrap();
    term.draw(|f| render::draw(f, &core)).unwrap();
    insta::assert_snapshot!(term.backend());
}
