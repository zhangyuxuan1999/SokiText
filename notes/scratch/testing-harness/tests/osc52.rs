use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Default)]
struct Clip(Arc<Mutex<Vec<String>>>);
impl vt100::Callbacks for Clip {
    fn copy_to_clipboard(&mut self, _: &mut vt100::Screen, _kind: &[u8], data: &[u8]) {
        self.0.lock().unwrap().push(String::from_utf8_lossy(data).into_owned());
    }
}

#[test]
fn osc52_clipboard_is_observable_in_ci() {
    let pty = native_pty_system();
    let pair = pty.openpty(PtySize{rows:10, cols:40, pixel_width:0, pixel_height:0}).unwrap();
    let mut cmd = CommandBuilder::new(env!("CARGO_BIN_EXE_probe"));
    cmd.env("TERM", "xterm-256color");
    let _c = pair.slave.spawn_command(cmd).unwrap();
    drop(pair.slave);
    let mut reader = pair.master.try_clone_reader().unwrap();
    let mut writer = pair.master.take_writer().unwrap();
    let clips = Arc::new(Mutex::new(Vec::new()));
    let p = Arc::new(Mutex::new(vt100::Parser::new_with_callbacks(10, 40, 0, Clip(clips.clone()))));
    let p2 = p.clone();
    std::thread::spawn(move || { let mut b=[0u8;8192]; while let Ok(n)=reader.read(&mut b){ if n==0 {break} p2.lock().unwrap().process(&b[..n]); }});
    std::thread::sleep(Duration::from_millis(200));
    writer.write_all(b"\x19").unwrap(); writer.flush().unwrap(); // Ctrl-Y
    let t = Instant::now();
    loop {
        if !clips.lock().unwrap().is_empty() { break }
        assert!(t.elapsed() < Duration::from_secs(5), "no OSC52 seen");
        std::thread::sleep(Duration::from_millis(20));
    }
    let got = clips.lock().unwrap().clone();
    println!("OSC52 base64 payloads: {got:?}");
    use base64ct::{Base64, Encoding};
    let decoded = String::from_utf8(Base64::decode_vec(&got[0]).unwrap()).unwrap();
    assert_eq!(decoded, "jio-clip-payload");
    println!("decoded = {decoded}");
}
