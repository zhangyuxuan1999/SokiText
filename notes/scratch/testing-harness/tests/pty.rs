use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

struct Pty { writer: Box<dyn Write + Send>, screen: Arc<Mutex<vt100::Parser>>,
             _child: Box<dyn portable_pty::Child + Send + Sync> }

impl Pty {
    fn spawn(args: &[&str], rows: u16, cols: u16) -> Self {
        let pty = native_pty_system();
        let pair = pty.openpty(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 }).unwrap();
        let mut cmd = CommandBuilder::new(env!("CARGO_BIN_EXE_probe"));
        for a in args { cmd.arg(a); }
        cmd.env("TERM", "xterm-256color");
        let child = pair.slave.spawn_command(cmd).unwrap();
        drop(pair.slave);
        let mut reader = pair.master.try_clone_reader().unwrap();
        let writer = pair.master.take_writer().unwrap();
        let screen = Arc::new(Mutex::new(vt100::Parser::new(rows, cols, 0)));
        let s2 = screen.clone();
        std::thread::spawn(move || {
            let mut buf = [0u8; 8192];
            while let Ok(n) = reader.read(&mut buf) { if n == 0 { break } s2.lock().unwrap().process(&buf[..n]); }
        });
        Pty { writer, screen, _child: child }
    }
    fn send(&mut self, s: &str) { self.writer.write_all(s.as_bytes()).unwrap(); self.writer.flush().unwrap(); }
    /// Deterministic wait: poll the emulated screen until predicate holds or timeout.
    fn wait_for(&self, pred: impl Fn(&str) -> bool) -> String {
        let start = Instant::now();
        loop {
            let c = self.screen.lock().unwrap().screen().contents();
            if pred(&c) { return c }
            if start.elapsed() > Duration::from_secs(10) { panic!("timeout; screen was:\n{c}") }
            std::thread::sleep(Duration::from_millis(20));
        }
    }
    fn contents(&self) -> String { self.screen.lock().unwrap().screen().contents() }
    fn cursor(&self) -> (u16, u16) { self.screen.lock().unwrap().screen().cursor_position() }
}

#[test]
fn e2e_type_and_save() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("a.txt");
    std::fs::write(&f, "").unwrap();
    let mut p = Pty::spawn(&[f.to_str().unwrap()], 10, 40, );
    p.wait_for(|s| s.contains("jio"));
    p.send("hello");
    p.wait_for(|s| s.contains("hello"));
    p.send("\u{13}"); // Ctrl-S
    // wait for the file on disk, not for a sleep
    let start = Instant::now();
    loop {
        if std::fs::read_to_string(&f).unwrap() == "hello" { break }
        assert!(start.elapsed() < Duration::from_secs(10), "file never saved");
        std::thread::sleep(Duration::from_millis(20));
    }
    p.send("\u{11}"); // Ctrl-Q
    println!("FINAL SCREEN:\n{}", p.contents());
    println!("CURSOR: {:?}", p.cursor());
}
