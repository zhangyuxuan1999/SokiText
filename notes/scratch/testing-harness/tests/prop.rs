use probe::{Core, Cmd};
use proptest::prelude::*;

// Naive reference implementation: a plain String + char index cursor.
#[derive(Debug, Clone, Default)]
struct Naive { s: Vec<char>, cur: usize }
impl Naive {
    fn apply(&mut self, c: Cmd) {
        match c {
            Cmd::Insert(ch) => { self.s.insert(self.cur, ch); self.cur += 1 }
            Cmd::NewLine => { self.s.insert(self.cur, '\n'); self.cur += 1 }
            Cmd::MoveLeft => { self.cur = self.cur.saturating_sub(1) }
            Cmd::MoveRight => { self.cur = (self.cur + 1).min(self.s.len()) }
            Cmd::DeleteBack => { if self.cur > 0 { self.s.remove(self.cur - 1); self.cur -= 1 } }
            _ => {}
        }
    }
    fn text(&self) -> String { self.s.iter().collect() }
}

fn cmd_strategy() -> impl Strategy<Value = Cmd> {
    prop_oneof![
        4 => any::<char>().prop_map(Cmd::Insert),
        1 => Just(Cmd::NewLine),
        2 => Just(Cmd::MoveLeft),
        2 => Just(Cmd::MoveRight),
        2 => Just(Cmd::DeleteBack),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 500, ..ProptestConfig::default() })]
    #[test]
    fn rope_matches_naive(cmds in prop::collection::vec(cmd_strategy(), 0..200)) {
        let mut core = Core::new("");
        let mut naive = Naive::default();
        for c in cmds { core.apply(c); naive.apply(c); }
        prop_assert_eq!(core.text.to_string(), naive.text());
        prop_assert_eq!(core.cursor, naive.cur);
        // cursor must always be a valid char index
        prop_assert!(core.cursor <= core.text.len_chars());
    }
}
