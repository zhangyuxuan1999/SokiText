use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[test]
fn resize_does_not_crash_and_reflows() {
    let pty = native_pty_system();
    let pair = pty.openpty(PtySize { rows: 10, cols: 40, pixel_width: 0, pixel_height: 0 }).unwrap();
    let mut cmd = CommandBuilder::new(env!("CARGO_BIN_EXE_probe"));
    cmd.env("TERM", "xterm-256color");
    let _child = pair.slave.spawn_command(cmd).unwrap();
    drop(pair.slave);
    let mut reader = pair.master.try_clone_reader().unwrap();
    let mut writer = pair.master.take_writer().unwrap();
    let screen = Arc::new(Mutex::new(vt100::Parser::new(10, 40, 0)));
    let s2 = screen.clone();
    std::thread::spawn(move || { let mut b=[0u8;8192]; while let Ok(n)=reader.read(&mut b){ if n==0 {break} s2.lock().unwrap().process(&b[..n]); }});
    let wait = |pred: &dyn Fn(&str)->bool| { let t=Instant::now(); loop {
        let c = screen.lock().unwrap().screen().contents();
        if pred(&c) { return c } assert!(t.elapsed()<Duration::from_secs(10), "timeout:\n{c}");
        std::thread::sleep(Duration::from_millis(20)); } };
    wait(&|s| s.contains("jio"));
    writer.write_all(b"abc").unwrap(); writer.flush().unwrap();
    wait(&|s| s.contains("abc"));
    // shrink hard: 1 row x 1 col is the classic panic case
    for (r, c) in [(1u16,1u16),(3,5),(60,200),(24,80)] {
        pair.master.resize(PtySize{rows:r, cols:c, pixel_width:0, pixel_height:0}).unwrap();
        screen.lock().unwrap().screen_mut().set_size(r, c);
        std::thread::sleep(Duration::from_millis(120));
        writer.write_all(b"x").unwrap(); writer.flush().unwrap();
    }
    let final_screen = wait(&|s| s.contains("abc"));
    println!("survived resizes; final:\n{final_screen}");
}
