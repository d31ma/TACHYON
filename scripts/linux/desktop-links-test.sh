#!/usr/bin/env bash
set -euo pipefail

write_fixture() {
  local root=$1
  mkdir -p "$root/client/pages/help" "$root/client/pages/orders" \
    "$root/client/components/home/panel" "$root/client/components/help/panel" "$root/client/components/orders/panel"
  cat > "$root/client/manifest.json" <<'JSON'
{
  "name": "Link Gate",
  "start_url": "/",
  "tac": {
    "application": { "id": "dev.tachyon.link-gate", "version": "0.0.1", "build": 3, "links": { "schemes": ["linkgate"] } }
  }
}
JSON
  cat > "$root/client/components/home/panel/tac.html" <<'HTML'
<main aria-label="Home route"><h1>Home route</h1>
<a href="https://example.com/a?b=1">Open example</a>
<a href="mailto:someone@example.com">Write mail</a>
</main>
HTML
  echo '<main aria-label="Help route"><h1>Help route</h1></main>' > "$root/client/components/help/panel/tac.html"
  echo '<main aria-label="Orders route"><h1>Orders route</h1></main>' > "$root/client/components/orders/panel/tac.html"
  echo '<home-panel />' > "$root/client/pages/tac.html"
  echo '<help-panel />' > "$root/client/pages/help/tac.html"
  echo '<orders-panel />' > "$root/client/pages/orders/tac.html"
}

check_tools() {
  local missing=() tool
  for tool in xvfb-run dbus-run-session dbus-monitor gdbus gio xdg-mime xdg-open dpkg-deb python3; do
    command -v "$tool" >/dev/null || missing+=("$tool")
  done
  if (( ${#missing[@]} )); then
    echo "missing tools: ${missing[*]}" >&2
    exit 1
  fi
  python3 -c 'import gi; gi.require_version("Atspi", "2.0")' || { echo 'python3 has no gi with Atspi 2.0' >&2; exit 1; }
}

build_native_app() {
  local fixture=$1
  if [[ -z "${TAC_BIN:-}" ]]; then
    cargo build --locked --bin ty
    TAC_BIN="${CARGO_TARGET_DIR:-target}/debug/ty"
  fi
  echo '==> building the Linux app and its release .deb'
  timeout 240s "$TAC_BIN" build "$fixture" --target linux
  timeout 300s "$TAC_BIN" bundle "$fixture" --target linux --release --out-dir release
  local deb
  deb=$(find "$fixture/release/linux" -maxdepth 1 -name 'linkgate_0.0.1-3_*.deb' -print -quit)
  test -f "$deb"
  dpkg-deb -x "$deb" "$fixture/installed"
  test -x "$fixture/dist/linux/LinkGate/bin/LinkGate"
  grep -qx 'MimeType=x-scheme-handler/linkgate;' "$fixture/dist/linux/LinkGate/dev.tachyon.link-gate.desktop"
}

use_a_writable_runtime_folder() {
  if [[ -d "${XDG_RUNTIME_DIR:-}" && -w "${XDG_RUNTIME_DIR:-}" ]]; then return; fi
  echo '==> using a private XDG_RUNTIME_DIR, because this shell has no writable one'
  PRIVATE_RUNTIME=$(mktemp -d "${TMPDIR:-/tmp}/ty-linux-runtime.XXXXXX")
  export XDG_RUNTIME_DIR="$PRIVATE_RUNTIME"
}

use_the_x11_display_only() {
  if [[ -z "${WAYLAND_DISPLAY:-}" ]]; then return; fi
  echo '==> unsetting WAYLAND_DISPLAY, because AT-SPI sends keys to the X11 display only without it'
  unset WAYLAND_DISPLAY
}

in_the_live_desktop() {
  [[ ${TAC_LINUX_DISPLAY:-x11} == desktop ]]
}

use_the_live_desktop() {
  local exports
  exports=$(node "$(dirname "$SCRIPT")/linux-display-session.mjs" desktop-env)
  eval "$exports"
  echo "==> running in the live $XDG_SESSION_TYPE desktop session (${XDG_CURRENT_DESKTOP:-unnamed desktop} on ${WAYLAND_DISPLAY:-$DISPLAY}, desktop services: ${TAC_DESKTOP_SERVICES:-none})"
}

read_desktop_default_apps() {
  local scheme
  local found=()
  for scheme in https mailto; do
    found+=("$scheme: $(xdg-mime query default "x-scheme-handler/$scheme" 2>/dev/null || true)")
  done
  DESKTOP_DEFAULT_APPS=$(IFS=,; echo "${found[*]}")
}

use_private_desktop_folders() {
  local fixture=$1
  export XDG_DATA_HOME="$fixture/desktop/data"
  export XDG_CONFIG_HOME="$fixture/desktop/config"
  export XDG_CACHE_HOME="$fixture/desktop/cache"
  export XDG_STATE_HOME="$fixture/desktop/state"
  mkdir -p "$XDG_DATA_HOME/applications" "$XDG_CONFIG_HOME" "$XDG_CACHE_HOME" "$XDG_STATE_HOME"
  unset BROWSER
  if ! in_the_live_desktop; then
    unset XDG_CURRENT_DESKTOP DESKTOP_SESSION
    export GDK_BACKEND=x11
  fi
  export GTK_A11Y=atspi
  export LIBGL_ALWAYS_SOFTWARE=1
  export WEBKIT_DMABUF_RENDERER_FORCE_SHM=1
}

install_link_recorder() {
  local fixture=$1
  echo '==> making a link recorder the default handler for https, http and mailto'
  export RECORDS="$fixture/desktop/opened-links"
  local recorder="$fixture/desktop/link-recorder"
  printf '#!/bin/sh\nprintf "%%s\\n" "$1" >> %q\n' "$RECORDS" > "$recorder"
  chmod +x "$recorder"
  cat > "$XDG_DATA_HOME/applications/link-recorder.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Name=Link Recorder
Exec=$recorder %u
MimeType=x-scheme-handler/https;x-scheme-handler/http;x-scheme-handler/mailto;
NoDisplay=true
DESKTOP
  xdg-mime default link-recorder.desktop x-scheme-handler/https x-scheme-handler/http x-scheme-handler/mailto
  local scheme
  for scheme in https mailto; do
    test "$(xdg-mime query default "x-scheme-handler/$scheme")" = link-recorder.desktop
  done
}

install_app_desktop_file() {
  local fixture=$1
  echo '==> installing the .deb desktop file into the private applications folder'
  local installed="$fixture/installed"
  local source="$installed/usr/share/applications/dev.tachyon.link_gate.desktop"
  local target="$XDG_DATA_HOME/applications/dev.tachyon.link_gate.desktop"
  grep -qx 'Exec=linkgate %u' "$source"
  grep -qx 'MimeType=x-scheme-handler/linkgate;' "$source"
  sed "s#^Exec=linkgate %u\$#Exec=$installed/usr/bin/linkgate %u#" "$source" > "$target"
  grep -qx "Exec=$installed/usr/bin/linkgate %u" "$target"
  if command -v desktop-file-validate >/dev/null; then desktop-file-validate "$target"; fi
  if command -v update-desktop-database >/dev/null; then
    update-desktop-database "$XDG_DATA_HOME/applications"
    grep -q '^x-scheme-handler/linkgate=dev.tachyon.link_gate.desktop' "$XDG_DATA_HOME/applications/mimeinfo.cache"
  else
    echo '==> update-desktop-database is missing, so the scheme is made the default in mimeapps.list instead'
    xdg-mime default dev.tachyon.link_gate.desktop x-scheme-handler/linkgate
  fi
  gio mime x-scheme-handler/linkgate | grep -q 'dev.tachyon.link_gate.desktop'
}

turn_off_desktop_portals() {
  local services="$XDG_DATA_HOME/dbus-1/services"
  mkdir -p "$services"
  local name
  for name in org.freedesktop.portal.Desktop org.freedesktop.portal.Documents; do
    printf '[D-BUS Service]\nName=%s\nExec=/bin/false\n' "$name" > "$services/$name.service"
  done
}

turn_on_desktop_portals() {
  rm -rf "$XDG_DATA_HOME/dbus-1"
}

portal_is_installed() {
  local folder
  local folders
  IFS=: read -ra folders <<< "${XDG_DATA_DIRS:-/usr/local/share:/usr/share}"
  for folder in "${folders[@]}"; do
    if [[ -f "$folder/dbus-1/services/org.freedesktop.portal.Desktop.service" ]]; then return 0; fi
  done
  return 1
}

run_in_desktop_session() {
  if in_the_live_desktop; then
    timeout 300s bash "$SCRIPT" "$@"
    return
  fi
  timeout 300s xvfb-run --auto-servernum --server-args='-screen 0 1024x1400x24' \
    dbus-run-session -- bash "$SCRIPT" "$@"
}

probe() {
  local application=$1
  local steps=$2
  TAC_PROBE_APPLICATION="$application" timeout 120s python3 "$PROBE" "$steps"
}

wait_for_portal_calls() {
  local bus_log=$1
  local count=$2
  local _
  for _ in $(seq 100); do
    if (( $(grep -c 'member=OpenURI' "$bus_log" || true) >= count )); then return; fi
    sleep 0.2
  done
  echo "the OpenURI portal received $(grep -c 'member=OpenURI' "$bus_log" || true) calls, not $count" >&2
  exit 1
}

wait_for_links() {
  local path=$1
  local bus_log=$2
  local count=$3
  if [[ $path == desktop-portal ]]; then
    wait_for_portal_calls "$bus_log" "$count"
    return
  fi
  wait_for_records "$count"
}

wait_for_records() {
  local count=$1
  local _
  for _ in $(seq 100); do
    if [[ -f "$RECORDS" && $(wc -l < "$RECORDS") -ge $count ]]; then return; fi
    sleep 0.2
  done
  echo "the default handler received $(cat "$RECORDS" 2>/dev/null | wc -l) links, not $count" >&2
  exit 1
}

stop_process() {
  local process=$1
  kill "$process" 2>/dev/null || true
  local _
  for _ in $(seq 50); do
    kill -0 "$process" 2>/dev/null || return 0
    sleep 0.2
  done
  kill -9 "$process" 2>/dev/null || true
}

open_external_links() {
  local path=$1
  local bus_log="$FIXTURE/desktop/bus-$path.log"
  local log="$XDG_STATE_HOME/tachyon/dev.tachyon.link-gate.jsonl"
  rm -f "$RECORDS" "$log"
  dbus-monitor --session > "$bus_log" 2>&1 &
  local monitor=$!
  "$FIXTURE/dist/linux/LinkGate/bin/LinkGate" > "$FIXTURE/desktop/app-$path.log" 2>&1 &
  local host=$!
  trap "kill $host $monitor 2>/dev/null || true" EXIT
  probe LinkGate '[["wait",["Home route","Open example","Write mail"]],["click","Open example"]]'
  wait_for_links "$path" "$bus_log" 1
  probe LinkGate '[["click","Write mail"]]'
  wait_for_links "$path" "$bus_log" 2
  probe LinkGate '[["wait",["Home route","Open example","Write mail"]]]'
  kill -0 "$host"
  if [[ -n "${TAC_LINUX_SCREENSHOTS:-}" ]]; then node "$(dirname "$SCRIPT")/linux-display-session.mjs" screenshot "$TAC_LINUX_SCREENSHOTS/links-$path.png"; fi
  if [[ $path != desktop-portal ]]; then test "$(cat "$RECORDS")" = $'https://example.com/a?b=1\nmailto:someone@example.com'; fi
  test -f "$log"
  if grep -q '"event":"link.external.failed"' "$log"; then echo 'the host recorded a failed launch' >&2; exit 1; fi
  if grep -v '"route":"/"' "$log"; then echo 'the app left its first page' >&2; exit 1; fi
  stop_process "$host"
  stop_process "$monitor"
  check_link_path "$path" "$bus_log"
  echo "OK: the $path path handed both links to the default handler and the app stayed on Home route"
}

check_link_path() {
  local path=$1
  local bus_log=$2
  local calls
  calls=$(grep -c 'interface=org.freedesktop.portal.OpenURI; member=OpenURI' "$bus_log" || true)
  if [[ $path == direct ]]; then
    test "$calls" = 0
    return
  fi
  test "$calls" = 2
  local link
  for link in 'https://example.com/a?b=1' 'mailto:someone@example.com'; do
    grep -A2 'member=OpenURI' "$bus_log" | grep -qF "string \"$link\""
  done
}

application_process() {
  local _
  for _ in $(seq 150); do
    local answer
    answer=$(gdbus call --session --dest org.freedesktop.DBus --object-path /org/freedesktop/DBus \
      --method org.freedesktop.DBus.GetConnectionUnixProcessID dev.tachyon.link_gate 2>/dev/null || true)
    if [[ $answer =~ \(uint32\ ([0-9]+),\) ]]; then
      echo "${BASH_REMATCH[1]}"
      return
    fi
    sleep 0.2
  done
  echo 'dev.tachyon.link_gate never joined the session bus' >&2
  exit 1
}

open_scheme_links() {
  local log="$XDG_STATE_HOME/tachyon/dev.tachyon.link-gate.jsonl"
  rm -f "$log"
  echo '==> gio open linkgate://help?tab=2 starts the app at its route'
  gio open 'linkgate://help?tab=2'
  local host
  host=$(application_process)
  trap "kill $host 2>/dev/null || true" EXIT
  probe linkgate '[["wait",["Help route"]]]'
  echo '==> gio open linkgate://orders reaches the running app'
  gio open 'linkgate://orders'
  probe linkgate '[["wait",["Orders route"]],["gone",["Help route"]]]'
  test "$(application_process)" = "$host"
  echo '==> xdg-open linkgate://help reaches the running app'
  xdg-open 'linkgate://help'
  probe linkgate '[["wait",["Help route"]],["gone",["Orders route"]]]'
  echo '==> gio open linkgate://missing/page opens the start route'
  gio open 'linkgate://missing/page'
  probe linkgate '[["wait",["Home route"]],["gone",["Help route"]]]'
  test "$(application_process)" = "$host"
  test "$(grep -c '"event":"controller.created"' "$log")" = 1
  test "$(grep -c '"event":"link.opened"' "$log")" = 4
  stop_process "$host"
  echo "OK: one app process ($host) opened all four scheme links"
}

open_links_through_the_desktop_portal() {
  echo "==> clicking external links in the live desktop: its xdg-desktop-portal answers OpenURI and opens the desktop's default apps ($DESKTOP_DEFAULT_APPS)"
  run_in_desktop_session external-links desktop-portal
  echo '==> opening scheme links through the installed desktop file'
  run_in_desktop_session scheme-links
}

run_session_step() {
  case $1 in
    external-links) open_external_links "$2" ;;
    scheme-links) open_scheme_links ;;
    *) echo "unknown step $1" >&2; exit 1 ;;
  esac
}

main() {
  if (( $# )); then
    run_session_step "$@"
    return
  fi
  local started=$SECONDS
  check_tools
  FIXTURE=$(mktemp -d "${TMPDIR:-/tmp}/ty-desktop-links.XXXXXX")
  PRIVATE_RUNTIME=
  trap 'rm -rf "$FIXTURE" ${PRIVATE_RUNTIME:+"$PRIVATE_RUNTIME"}' EXIT
  SCRIPT=$(realpath "${BASH_SOURCE[0]}")
  PROBE="$(dirname "$SCRIPT")/desktop-links-probe.py"
  export FIXTURE SCRIPT PROBE
  use_a_writable_runtime_folder
  use_the_x11_display_only
  if in_the_live_desktop; then
    use_the_live_desktop
    read_desktop_default_apps
  fi
  write_fixture "$FIXTURE"
  build_native_app "$FIXTURE"
  use_private_desktop_folders "$FIXTURE"
  install_link_recorder "$FIXTURE"
  install_app_desktop_file "$FIXTURE"
  if in_the_live_desktop && [[ " $TAC_DESKTOP_SERVICES " == *' portal '* ]]; then
    open_links_through_the_desktop_portal
    echo "PASS: in the live $XDG_SESSION_TYPE desktop session (${XDG_CURRENT_DESKTOP:-unnamed desktop}) GtkUriLauncher handed https and mailto links to the desktop's OpenURI portal and its default apps ($DESKTOP_DEFAULT_APPS), and the installed .desktop file opened linkgate:// links in one running app over the desktop's session bus ($((SECONDS - started)) s)"
    return
  fi
  turn_off_desktop_portals
  echo '==> clicking external links with no desktop portal: GtkUriLauncher falls back to GIO'
  run_in_desktop_session external-links direct
  echo '==> opening scheme links through the installed desktop file'
  run_in_desktop_session scheme-links
  turn_on_desktop_portals
  local paths='GIO launch'
  if portal_is_installed; then
    echo '==> clicking external links with xdg-desktop-portal on the bus: GtkUriLauncher calls OpenURI'
    run_in_desktop_session external-links portal
    paths='GIO launch and the OpenURI portal'
  else
    echo 'SKIP: xdg-desktop-portal is not installed, so the OpenURI portal path did not run'
  fi
  echo "PASS: GtkUriLauncher opened https and mailto links in the default handler through $paths, and the installed .desktop file opened linkgate:// links in one running app ($((SECONDS - started)) s)"
}

main "$@"
