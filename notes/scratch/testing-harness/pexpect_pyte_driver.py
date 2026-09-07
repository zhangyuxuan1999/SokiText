import sys, os, time, pexpect, pyte
cmd, args = sys.argv[1], sys.argv[2:]
rows, cols = 12, 46
screen = pyte.Screen(cols, rows); stream = pyte.Stream(screen)
env = dict(os.environ, TERM="xterm-256color", WINEDEBUG="-all")
c = pexpect.spawn(cmd, args, dimensions=(rows, cols), env=env, timeout=20, encoding=None)
def pump(t=1.0):
    end = time.time()+t
    while time.time() < end:
        try: data = c.read_nonblocking(4096, timeout=0.15)
        except Exception: continue
        if data: stream.feed(data.decode("utf-8","replace"))
def show(tag):
    print(f"--- {tag} ---")
    for l in screen.display: print(repr(l))
pump(3.0); show("startup")
c.send("hi"); pump(2.0); show("after typing 'hi'")
c.send("\x13"); pump(1.5)   # Ctrl-S
c.send("\x11"); pump(1.5)   # Ctrl-Q
show("final")
try: c.expect(pexpect.EOF, timeout=5); print("EOF ok")
except Exception as e: print("no EOF:", type(e).__name__)
