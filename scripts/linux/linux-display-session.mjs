#!/usr/bin/env node
import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { existsSync, mkdirSync, readdirSync, readFileSync, statSync } from 'node:fs'
import path from 'node:path'
import { pathToFileURL } from 'node:url'

export class LinuxDisplaySession {
  static waylandDisplay = 'tachyon-app-test'
  static desktopKeys = ['DISPLAY', 'WAYLAND_DISPLAY', 'XAUTHORITY', 'XDG_RUNTIME_DIR', 'DBUS_SESSION_BUS_ADDRESS', 'XDG_SESSION_TYPE', 'XDG_SESSION_ID', 'XDG_CURRENT_DESKTOP', 'XDG_SESSION_DESKTOP', 'DESKTOP_SESSION']
  static desktopTypes = ['x11', 'wayland']
  static desktopServices = {
    tray: 'org.kde.StatusNotifierWatcher',
    notifications: 'org.freedesktop.Notifications',
    'remote-desktop': 'org.gnome.Mutter.RemoteDesktop',
    portal: 'org.freedesktop.portal.Desktop',
    accessibility: 'org.a11y.Bus',
  }
  static screenshotTools = {
    x11: [['import', '-window', 'root'], ['gnome-screenshot', '-f']],
    wayland: [['gnome-screenshot', '-f'], ['grim']],
  }

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

  /** @param {string} script */
  static desktop(script) {
    const live = this.liveDesktop()
    const unset = live.env.XDG_SESSION_TYPE === 'x11' ? ['-u', 'WAYLAND_DISPLAY'] : []
    const announce = `echo "the live ${live.env.XDG_SESSION_TYPE} desktop session ${live.env.XDG_SESSION_ID ?? ''} (${live.env.XDG_CURRENT_DESKTOP ?? 'unnamed desktop'}) is the display"\n`
    return {
      command: 'env',
      args: [...unset, 'bash', '-c', `set -euo pipefail\n${announce}${script}`],
      env: live.env,
    }
  }

  /**
   * The command, arguments and environment that run a bash script beside a display: Xvfb for x11, headless mutter for wayland, and the signed-in desktop session for desktop.
   * @param {string} display @param {string} script @param {string} size @param {string} folder
   */
  static of(display, script, size, folder) {
    const sessions = { x11: () => this.x11(script, size), wayland: () => this.wayland(script, size, folder), desktop: () => this.desktop(script) }
    assert.ok(sessions[display], `TAC_LINUX_DISPLAY must be x11, wayland or desktop, not ${display}`)
    return sessions[display]()
  }

  static desktopUser() {
    const name = process.env.TAC_LINUX_DESKTOP_USER
    if (!name) return process.getuid?.() ?? 0
    const found = spawnSync('id', ['-u', name], { encoding: 'utf8' })
    assert.equal(found.status, 0, `TAC_LINUX_DESKTOP_USER names no user: ${name}`)
    return Number(found.stdout.trim())
  }

  /**
   * The command and arguments that run a tool as the desktop's user, because a root shell cannot use that user's session bus.
   * @param {string} command @param {string[]} args
   */
  static asDesktopUser(command, args) {
    const name = process.env.TAC_LINUX_DESKTOP_USER
    if (!name || process.getuid?.() !== 0) return [command, args]
    return ['runuser', ['-u', name, '--', command, ...args]]
  }

  /** @param {string} pid */
  static environmentOf(pid) {
    try {
      const entries = readFileSync(`/proc/${pid}/environ`, 'latin1').split('\0').filter(Boolean)
      return Object.fromEntries(entries.map(entry => [entry.slice(0, entry.indexOf('=')), entry.slice(entry.indexOf('=') + 1)]))
    } catch {
      return {}
    }
  }

  /** @param {string} pid @param {number} user */
  static ownedBy(pid, user) {
    try {
      return statSync(`/proc/${pid}`).uid === user
    } catch {
      return false
    }
  }

  /** @param {Record<string, string>} environment */
  static belongsToDesktop(environment) {
    const wanted = process.env.TAC_LINUX_DESKTOP_SESSION
    const shown = Boolean(environment.DISPLAY || environment.WAYLAND_DISPLAY)
    const typed = this.desktopTypes.includes(environment.XDG_SESSION_TYPE)
    return typed && shown && Boolean(environment.DBUS_SESSION_BUS_ADDRESS) && (!wanted || environment.XDG_SESSION_ID === wanted)
  }

  static desktopProcesses() {
    const pinned = process.env.TAC_LINUX_DESKTOP_PID
    if (pinned) return [this.environmentOf(pinned)]
    const user = this.desktopUser()
    const pids = readdirSync('/proc').filter(name => /^\d+$/.test(name) && this.ownedBy(name, user))
    return pids.map(pid => this.environmentOf(pid)).filter(environment => this.belongsToDesktop(environment))
  }

  /** @param {Array<Record<string, string>>} environments @param {string} key */
  static commonValue(environments, key) {
    const counts = new Map()
    for (const value of environments.map(environment => environment[key]).filter(Boolean)) counts.set(value, (counts.get(value) ?? 0) + 1)
    return [...counts].sort((left, right) => right[1] - left[1])[0]?.[0]
  }

  /** @param {Record<string, string>} env */
  static checkSockets(env) {
    const wayland = env.WAYLAND_DISPLAY && path.resolve(env.XDG_RUNTIME_DIR ?? '/', env.WAYLAND_DISPLAY)
    const bus = /^unix:path=([^,;]+)/.exec(env.DBUS_SESSION_BUS_ADDRESS)?.[1]
    if (env.XDG_SESSION_TYPE === 'wayland') assert.ok(existsSync(wayland), `the desktop's Wayland socket ${wayland} is missing`)
    if (bus) assert.ok(existsSync(bus), `the desktop's session bus ${bus} is missing`)
  }

  /**
   * The environment of the signed-in x11 or wayland desktop session, read from its own processes.
   * TAC_LINUX_DESKTOP_SESSION picks a logind session, TAC_LINUX_DESKTOP_PID a process, and TAC_LINUX_DESKTOP_USER the user whose session it is.
   */
  static liveDesktop() {
    const environments = this.desktopProcesses()
    const sessions = [...new Set(environments.map(environment => environment.XDG_SESSION_ID).filter(Boolean))]
    assert.ok(environments.length > 0, 'no x11 or wayland desktop session is running for this user; sign in through the Windows App or the console first, or set TAC_LINUX_DESKTOP_PID to a process of that session')
    assert.ok(sessions.length <= 1, `more than one desktop session is running (${sessions.join(', ')}); set TAC_LINUX_DESKTOP_SESSION to one of them`)
    const env = Object.fromEntries(this.desktopKeys.map(key => [key, this.commonValue(environments, key)]).filter(([, value]) => value))
    env.XDG_SESSION_TYPE ??= env.WAYLAND_DISPLAY ? 'wayland' : 'x11'
    env.GDK_BACKEND = env.XDG_SESSION_TYPE
    this.checkSockets(env)
    env.TAC_DESKTOP_SERVICES = this.servicesOn(env).join(' ')
    return { env }
  }

  /** @param {Record<string, string>} env @param {string} method @param {string} [name] */
  static busCall(env, method, name) {
    const args = ['call', '--session', '--dest', 'org.freedesktop.DBus', '--object-path', '/org/freedesktop/DBus', '--method', `org.freedesktop.DBus.${method}`, ...(name ? [name] : [])]
    const found = spawnSync(...this.asDesktopUser('gdbus', args), { encoding: 'utf8', env: { ...process.env, ...env }, timeout: 10000 })
    assert.equal(found.status, 0, `gdbus could not reach the desktop's session bus: ${found.stderr}`)
    return found.stdout
  }

  /** @param {Record<string, string>} env */
  static servicesOn(env) {
    const activatable = this.busCall(env, 'ListActivatableNames')
    const present = ([, name]) => this.busCall(env, 'NameHasOwner', name).includes('true') || activatable.includes(`'${name}'`)
    return Object.entries(this.desktopServices).filter(present).map(([service]) => service)
  }

  /** @param {string} value */
  static shellQuoted(value) {
    return `'${value.replaceAll("'", "'\\''")}'`
  }

  static printShellEnvironment() {
    const { env } = this.liveDesktop()
    const unset = env.XDG_SESSION_TYPE === 'x11' ? 'unset WAYLAND_DISPLAY\n' : ''
    process.stdout.write(`${unset}${Object.entries(env).map(([key, value]) => `export ${key}=${this.shellQuoted(value)}`).join('\n')}\n`)
  }

  static describe() {
    const { env } = this.liveDesktop()
    const loginctl = spawnSync('loginctl', ['show-session', env.XDG_SESSION_ID ?? '', '-p', 'Type', '-p', 'Desktop', '-p', 'Remote', '-p', 'State', '-p', 'Name'], { encoding: 'utf8' })
    console.log(JSON.stringify({ ...env, loginctl: loginctl.stdout.trim().split('\n').filter(Boolean) }, null, 2))
  }

  /** @param {string} file */
  static screenshot(file) {
    const { env } = this.liveDesktop()
    mkdirSync(path.dirname(path.resolve(file)), { recursive: true })
    const failures = []
    for (const [tool, ...args] of this.screenshotTools[env.XDG_SESSION_TYPE]) {
      const attempt = spawnSync(...this.asDesktopUser(tool, [...args, file]), { encoding: 'utf8', env: { ...process.env, ...env }, timeout: 30000 })
      if (attempt.status === 0 && existsSync(file)) break
      failures.push(`${tool}: ${attempt.error?.message ?? attempt.stderr.trim()}`)
    }
    assert.ok(existsSync(file), `no screenshot tool could capture the ${env.XDG_SESSION_TYPE} desktop: ${failures.join('; ')}`)
    console.log(`SCREENSHOT: ${file}`)
  }

  /** @param {string[]} args */
  static main([command, ...args]) {
    const commands = {
      'desktop-env': () => this.printShellEnvironment(),
      describe: () => this.describe(),
      screenshot: () => this.screenshot(args[0]),
    }
    assert.ok(commands[command], 'usage: linux-display-session.mjs desktop-env | describe | screenshot <file.png>')
    commands[command]()
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) LinuxDisplaySession.main(process.argv.slice(2))
