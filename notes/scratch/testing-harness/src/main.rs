use crossterm::event::{self, Event, KeyCode, KeyModifiers, KeyEventKind};
use crossterm::{execute, terminal::*};
use probe::{Core, Key, Cmd, Effect, keymap, render};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io::stdout;

fn to_key(k: event::KeyEvent) -> Option<Key> {
    if k.kind != KeyEventKind::Press { return None }
    Some(match (k.code, k.modifiers) {
        (KeyCode::Char('s'), KeyModifiers::CONTROL) => Key::CtrlS,
        (KeyCode::Char('q'), KeyModifiers::CONTROL) => Key::CtrlQ,
        (KeyCode::Char('y'), KeyModifiers::CONTROL) => { osc52("jio-clip-payload"); return None }
        (KeyCode::Char(c), _) => Key::Char(c),
        (KeyCode::Left, _) => Key::Left,
        (KeyCode::Right, _) => Key::Right,
        (KeyCode::Backspace, _) => Key::Backspace,
        (KeyCode::Enter, _) => Key::Enter,
        _ => return None,
    })
}

fn osc52(s: &str) {
    use base64ct::{Base64, Encoding};
    let b = Base64::encode_string(s.as_bytes());
    use std::io::Write;
    let mut o = stdout();
    let _ = write!(o, "\x1b]52;c;{b}\x07");
    let _ = o.flush();
}

fn main() -> std::io::Result<()> {
    let path = std::env::args().nth(1);
    let init = path.as_ref().and_then(|p| std::fs::read_to_string(p).ok()).unwrap_or_default();
    let mut core = Core::new(&init);
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;
    let mut term = Terminal::new(CrosstermBackend::new(stdout()))?;
    loop {
        term.draw(|f| render::draw(f, &core))?;
        if let Event::Key(k) = event::read()? {
            if let Some(key) = to_key(k) {
                if let Some(cmd) = keymap(key) {
                    match core.apply(cmd) {
                        Some(Effect::Quit) => break,
                        Some(Effect::WriteFile) => {
                            if let Some(p) = &path { std::fs::write(p, core.text.to_string())?; core.dirty = false; }
                        }
                        None => {}
                    }
                }
            }
        }
    }
    execute!(stdout(), LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}
