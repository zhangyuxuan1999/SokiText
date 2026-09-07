pub mod render;
use ropey::Rope;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key { Char(char), Left, Right, Backspace, Enter, CtrlS, CtrlQ }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cmd { Insert(char), MoveLeft, MoveRight, DeleteBack, NewLine, Save, Quit }

/// Pure key -> command mapping (the "keymap" seam).
pub fn keymap(k: Key) -> Option<Cmd> {
    Some(match k {
        Key::Char(c) => Cmd::Insert(c),
        Key::Left => Cmd::MoveLeft,
        Key::Right => Cmd::MoveRight,
        Key::Backspace => Cmd::DeleteBack,
        Key::Enter => Cmd::NewLine,
        Key::CtrlS => Cmd::Save,
        Key::CtrlQ => Cmd::Quit,
    })
}

/// Effects the core asks the shell to perform. The shell is the only impure part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect { WriteFile, Quit }

#[derive(Debug, Clone)]
pub struct Core { pub text: Rope, pub cursor: usize, pub dirty: bool }

impl Core {
    pub fn new(s: &str) -> Self { Core { text: Rope::from_str(s), cursor: 0, dirty: false } }
    pub fn apply(&mut self, c: Cmd) -> Option<Effect> {
        match c {
            Cmd::Insert(ch) => { self.text.insert_char(self.cursor, ch); self.cursor += 1; self.dirty = true; None }
            Cmd::NewLine => { self.text.insert_char(self.cursor, '\n'); self.cursor += 1; self.dirty = true; None }
            Cmd::MoveLeft => { self.cursor = self.cursor.saturating_sub(1); None }
            Cmd::MoveRight => { self.cursor = (self.cursor + 1).min(self.text.len_chars()); None }
            Cmd::DeleteBack => {
                if self.cursor > 0 { self.text.remove(self.cursor - 1..self.cursor); self.cursor -= 1; self.dirty = true; }
                None
            }
            Cmd::Save => Some(Effect::WriteFile),
            Cmd::Quit => Some(Effect::Quit),
        }
    }
}

