import assert from 'node:assert/strict'
import { mkdirSync } from 'node:fs'
import path from 'node:path'

export class LinuxDisplaySession {
  static waylandDisplay = 'tachyon-app-test'

  /** @param {string} script @param {string} size */
  static x11(script, size) {
    return {
      command: 'xvfb-run',
      args: ['--auto-servernum', `--server-args=-screen 0 ${size}x24`, 'dbus-run-session', '--', 'bash', '-c', `set -euo pipefail\n${script}`],
      env: { GDK_BACKEND: 'x11' },
    }
  }

  /** @param {string} script @param {string} size @param {string} folder */
  static wayland(script, size, folder) {
    const runtime = path.join(folder, 'runtime')
    mkdirSync(runtime, { mode: 0o700, recursive: true })
    const compositor = `set -euo pipefail
unset DISPLAY
mutter --headless --wayland --no-x11 --wayland-display="$WAYLAND_DISPLAY" --virtual-monitor ${size} >"$XDG_RUNTIME_DIR/mutter.log" 2>&1 &
compositor=$!
for _ in $(seq 60); do
  if [[ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ]]; then break; fi
  kill -0 "$compositor" 2>/dev/null || { cat "$XDG_RUNTIME_DIR/mutter.log" >&2; exit 1; }
  sleep 0.5
done
echo "$(mutter --version | head -n 1) is the Wayland compositor on $WAYLAND_DISPLAY"
`
    return {
      command: 'dbus-run-session',
      args: ['--', 'bash', '-c', `${compositor}${script}`],
      env: { GDK_BACKEND: 'wayland', WAYLAND_DISPLAY: this.waylandDisplay, XDG_RUNTIME_DIR: runtime },
    }
  }

  /**
   * The command, arguments and environment that run a bash script beside a display: Xvfb for x11, headless mutter for wayland.
   * @param {string} display @param {string} script @param {string} size @param {string} folder
   */
  static of(display, script, size, folder) {
    const sessions = { x11: () => this.x11(script, size), wayland: () => this.wayland(script, size, folder) }
    assert.ok(sessions[display], `TAC_LINUX_DISPLAY must be x11 or wayland, not ${display}`)
    return sessions[display]()
  }
}
