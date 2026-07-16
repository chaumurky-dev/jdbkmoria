#!/usr/bin/env python3
"""PTY smoke test for jdbkmoria: version check, character creation, movement, quit."""
import os
import pty
import select
import subprocess
import sys
import tempfile
import time
import shutil

# Resolve paths relative to script location (repo root = parent of tools/)
SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
REPO_ROOT = os.path.dirname(SCRIPT_DIR)

# Allow override via JDBKMORIA_BIN env var
BIN = os.environ.get("JDBKMORIA_BIN", os.path.join(REPO_ROOT, "target/debug/jdbkmoria"))

def check_binary_exists():
    """Check if binary exists; if not, print hint and exit."""
    if not os.path.exists(BIN):
        print(f"FAIL: Binary not found at {BIN}")
        print(f"Hint: Run 'cargo build' from {REPO_ROOT}")
        sys.exit(1)

def spawn(args, cwd):
    """Spawn game process with PTY and set window size."""
    master, slave = pty.openpty()
    os.environ["TERM"] = "xterm"
    # set window size 24x80
    import fcntl, termios, struct
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 24, 80, 0, 0))
    proc = subprocess.Popen(
        [BIN] + args, stdin=slave, stdout=slave, stderr=slave,
        cwd=cwd, close_fds=True,
    )
    os.close(slave)
    return proc, master

def read_all(fd, timeout=2.0):
    """Read all available output from fd with timeout."""
    out = b""
    end = time.time() + timeout
    while time.time() < end:
        r, _, _ = select.select([fd], [], [], 0.2)
        if r:
            try:
                chunk = os.read(fd, 65536)
            except OSError:
                break
            if not chunk:
                break
            out += chunk
        elif out:
            # got something and stream went quiet
            if time.time() > end - 1.0:
                break
    return out.decode("latin-1", errors="replace")

def send(fd, s, delay=0.15):
    """Send keystrokes with delay between each character."""
    for ch in s:
        try:
            os.write(fd, ch.encode("latin-1"))
            time.sleep(delay)
        except OSError:
            # PTY closed, process likely exited
            break

def main():
    """Run smoke test: version check, character creation, movement, quit."""
    failed = False

    # Check binary exists
    check_binary_exists()

    # --- Test 1: version flag ---
    print("=== TEST 1: Version check ===")
    try:
        v = subprocess.run([BIN, "-v"], capture_output=True, text=True, timeout=10)
        print(f"VERSION OUTPUT: {v.stdout.strip()}")
        if "0.1.0" not in v.stdout:
            print("FAIL: version check failed (expected 0.1.0)")
            failed = True
        else:
            print("PASS: version check")
    except Exception as e:
        print(f"FAIL: version check error: {e}")
        failed = True

    # --- Test 2: full game session in temp directory ---
    print("\n=== TEST 2: Full game session ===")
    temp_dir = None
    try:
        # Create temp working directory
        temp_dir = tempfile.mkdtemp(prefix="jdbkmoria_test_")
        print(f"Created temp dir: {temp_dir}")

        # Copy scores.dat
        scores_src = os.path.join(REPO_ROOT, "scores.dat")
        if os.path.exists(scores_src):
            shutil.copy2(scores_src, os.path.join(temp_dir, "scores.dat"))

        # Symlink or copy data directory
        data_src = os.path.join(REPO_ROOT, "data")
        data_dst = os.path.join(temp_dir, "data")
        if os.path.exists(data_src):
            # Try symlink first (faster), fall back to copy
            try:
                os.symlink(data_src, data_dst)
            except OSError:
                shutil.copytree(data_src, data_dst)

        # Spawn game
        proc, fd = spawn(["-n", "-s", "12345"], temp_dir)

        transcript = []
        def step(keys, label, timeout=3.0):
            """Send keys and collect output."""
            if keys and proc.poll() is None:
                send(fd, keys)
            out = read_all(fd, timeout)
            transcript.append((label, out))
            return out

        # Game flow
        out = step("", "startup", 4.0)
        # splash screen -> any key
        out = step(" ", "after-splash", 3.0)
        # character creation: choose race 'a' (Human)
        out = step("a", "race-chosen", 2.0)
        # gender m
        out = step("m", "gender-chosen", 2.0)
        # stats + background shown; ESC accepts characteristics
        out = step("\x1b", "stats-accepted", 2.0)
        # class selection: 'a' (Warrior)
        out = step("a", "class-chosen", 3.0)
        # maybe more rolls: ESC to accept again if prompted
        out = step("\x1b", "class-stats-accepted", 2.0)
        # name prompt: accept default with RETURN
        out = step("\r", "name-accepted", 3.0)
        # possibly another key to continue (waitForContinueKey)
        out = step(" ", "post-creation", 5.0)

        full = "".join(o for _, o in transcript)

        # Now should be in town. Move around a bit.
        out = step("2", "move-south", 2.0)
        out = step("8", "move-north", 2.0)
        out = step("6", "move-east", 2.0)

        # Quit: ^K (Ctrl-K = \x0b) then y (for confirm)
        out = step("\x0b", "quit-prompt", 2.0)  # ^K = quit in original keys
        out = step("y", "quit-confirm", 3.0)
        # death/tomb screens: ESC to skip character record
        out = step("\x1b", "tomb-esc", 3.0)
        # high-score screen: space/ESC to exit
        out = step(" ", "scores-continue-1", 2.0)
        out = step("\x1b", "scores-continue-2", 2.0)

        # Wait for process to exit, poll for ~10 seconds
        print("Waiting for process to exit...")
        exit_deadline = time.time() + 10.0
        while proc.poll() is None and time.time() < exit_deadline:
            time.sleep(0.2)

        # If still running, try feeding more keys
        if proc.poll() is None:
            print("Process still running, feeding extra keys...")
            for k in [" ", "\x1b", " ", "\r"]:
                try:
                    os.write(fd, k.encode())
                except OSError:
                    break
                time.sleep(0.3)
                if proc.poll() is not None:
                    break

        # Final wait
        time.sleep(0.5)
        proc.poll()

        # Check exit code
        if proc.returncode is None:
            print("FAIL: Process did not exit within ~10 seconds, killing it")
            proc.kill()
            proc.wait()
            failed = True
        elif proc.returncode != 0:
            print(f"FAIL: Process exited with code {proc.returncode} (expected 0)")
            failed = True
        else:
            print("PASS: Process exited cleanly")

        # Collect all output
        full = "".join(o for _, o in transcript)

        # Validate game state checks
        checks = {
            "splash screen shown": ("Robert A. Koeneke" in full) or ("Koeneke" in full) or ("DUNGEONS" in full),
            "character creation shown": ("race" in full.lower()) or ("Race" in full),
            "class prompt": ("class" in full.lower()),
            "town/dungeon stats sidebar": ("LEV" in full and "GOLD" in full) or ("HP" in full),
        }
        for name, ok in checks.items():
            result = "PASS" if ok else "FAIL"
            print(f"{result}: {name}")
            if not ok:
                failed = True

    except Exception as e:
        print(f"FAIL: Unexpected error in game session: {e}")
        import traceback
        traceback.print_exc()
        failed = True

    finally:
        # Clean up temp directory
        if temp_dir and os.path.exists(temp_dir):
            try:
                shutil.rmtree(temp_dir)
                print(f"Cleaned up temp dir: {temp_dir}")
            except Exception as e:
                print(f"Warning: Could not clean up temp dir {temp_dir}: {e}")

    # Final result
    print()
    if failed:
        print("FAILED: Some checks did not pass")
        sys.exit(1)
    else:
        print("ALL CHECKS PASSED")
        sys.exit(0)

if __name__ == "__main__":
    main()
