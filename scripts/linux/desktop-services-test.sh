#!/usr/bin/env bash
set -euo pipefail

stop_services() {
  for pid in ${SERVICE_PIDS:-}; do
    kill "$pid" 2>/dev/null || true
  done
  pkill -P "${FEED_PID:-0}" 2>/dev/null || true
  rm -rf "$STATE"
}

use_private_folders() {
  STATE=$(mktemp -d "${TMPDIR:-/tmp}/ty-linux-desktop-services.XXXXXX")
  export STATE
  trap stop_services EXIT
  mkdir -m 700 "$STATE/runtime"
  export XDG_RUNTIME_DIR="$STATE/runtime"
  export XDG_DATA_HOME="$STATE/data"
  export XDG_CACHE_HOME="$STATE/cache"
  export XDG_STATE_HOME="$STATE/state"
  export XDG_CONFIG_HOME="$STATE/config"
  unset WAYLAND_DISPLAY
}

in_the_live_desktop() {
  [[ ${TAC_LINUX_DISPLAY:-x11} == desktop ]]
}

use_the_live_desktop() {
  local exports
  exports=$(node "$(dirname "$0")/linux-display-session.mjs" desktop-env)
  eval "$exports"
  echo "==> running in the live $XDG_SESSION_TYPE desktop session (${XDG_CURRENT_DESKTOP:-unnamed desktop} on ${WAYLAND_DISPLAY:-$DISPLAY}, desktop services: ${TAC_DESKTOP_SERVICES:-none})"
}

desktop_has() {
  [[ " ${TAC_DESKTOP_SERVICES:-} " == *" $1 "* ]]
}

find_geoclue() {
  GEOCLUE=""
  for candidate in /usr/libexec/geoclue /usr/lib/geoclue-2.0/geoclue; do
    if [[ -x "$candidate" ]]; then GEOCLUE="$candidate"; fi
  done
  GEOCLUE_AGENT=/usr/libexec/geoclue-2.0/demos/agent
  local programs=("$GEOCLUE" "$GEOCLUE_AGENT")
  if ! in_the_live_desktop; then programs+=("$(command -v dunst)" "$(command -v dunstctl)"); fi
  for program in "${programs[@]}"; do
    if [[ ! -x "$program" ]]; then
      echo "Missing GeoClue, its demo agent or dunst. On Ubuntu: apt install geoclue-2.0 dunst" >&2
      exit 1
    fi
  done
  export GEOCLUE_AGENT
}

start_system_bus() {
  echo '==> starting a private system bus'
  cat >"$STATE/system-bus.conf" <<CONF
<!DOCTYPE busconfig PUBLIC "-//freedesktop//DTD D-BUS Bus Configuration 1.0//EN" "http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd">
<busconfig>
  <type>system</type>
  <listen>unix:path=$STATE/system-bus</listen>
  <auth>EXTERNAL</auth>
  <policy context="default">
    <allow user="*"/>
    <allow own="*"/>
    <allow send_destination="*"/>
    <allow receive_type="*"/>
  </policy>
</busconfig>
CONF
  SERVICE_PIDS="$(dbus-daemon --config-file="$STATE/system-bus.conf" --fork --print-pid)"
  export DBUS_SYSTEM_BUS_ADDRESS="unix:path=$STATE/system-bus"
}

start_geoclue() {
  echo "==> starting GeoClue $("$GEOCLUE" --version) with a GPS fix from an NMEA socket"
  python3 "$(dirname "$0")/nmea-feed.py" "$STATE/nmea.sock" &
  FEED_PID=$!
  SERVICE_PIDS="$SERVICE_PIDS $FEED_PID"
  for _ in $(seq 40); do
    if [[ -S "$STATE/nmea.sock" ]]; then break; fi
    sleep 0.25
  done
  G_MESSAGES_DEBUG=Geoclue "$GEOCLUE" --timeout=0 --nmea-socket="$STATE/nmea.sock" >"$STATE/geoclue.log" 2>&1 &
  SERVICE_PIDS="$SERVICE_PIDS $!"
  for _ in $(seq 40); do
    if gdbus introspect --system -d org.freedesktop.GeoClue2 -o /org/freedesktop/GeoClue2/Manager >/dev/null 2>&1; then return; fi
    sleep 0.25
  done
  cat "$STATE/geoclue.log" >&2
  exit 1
}

build_tests() {
  echo '==> building the Linux host tests'
  cargo test --locked -p tachyon-core --lib --no-run
}

run_with_desktop_services() {
  echo '==> running the browser API test with dunst and the GeoClue agent in a session bus'
  GDK_BACKEND=x11 LIBGL_ALWAYS_SOFTWARE=1 WEBKIT_DMABUF_RENDERER_FORCE_SHM=1 GDK_DEBUG=no-portals \
    dbus-run-session -- xvfb-run --auto-servernum --server-args='-screen 0 1280x1000x24' bash -c '
      set -euo pipefail
      dunst >"$STATE/dunst.log" 2>&1 &
      services=$!
      trap '\''kill $services 2>/dev/null || true'\'' EXIT
      for _ in $(seq 40); do
        if gdbus call --session -d org.freedesktop.DBus -o /org/freedesktop/DBus -m org.freedesktop.DBus.NameHasOwner org.freedesktop.Notifications | grep -q true; then break; fi
        sleep 0.25
      done
      echo "notification server: $(gdbus call --session -d org.freedesktop.Notifications -o /org/freedesktop/Notifications -m org.freedesktop.Notifications.GetServerInformation)"
      "$GEOCLUE_AGENT" >"$STATE/agent.log" 2>&1 &
      services="$services $!"
      cargo test --locked -p tachyon-core --lib linux_browser_apis_reach_geoclue_and_the_notification_server -- --ignored --test-threads=1 --nocapture
      dunstctl close-all
      sleep 0.5
      dunstctl history >"$STATE/history.json"
    '
}

run_with_the_desktop_notification_server() {
  desktop_has notifications || { echo 'the desktop has no notification server on its session bus (org.freedesktop.Notifications)' >&2; exit 1; }
  echo "==> running the browser API test with the desktop's own notification server and the GeoClue agent"
  echo "notification server: $(gdbus call --session -d org.freedesktop.Notifications -o /org/freedesktop/Notifications -m org.freedesktop.Notifications.GetServerInformation)"
  dbus-monitor --session "interface='org.freedesktop.Notifications'" "interface='org.gtk.Notifications'" >"$STATE/notifications.log" 2>&1 &
  SERVICE_PIDS="$SERVICE_PIDS $!"
  "$GEOCLUE_AGENT" >"$STATE/agent.log" 2>&1 &
  SERVICE_PIDS="$SERVICE_PIDS $!"
  LIBGL_ALWAYS_SOFTWARE=1 WEBKIT_DMABUF_RENDERER_FORCE_SHM=1 GDK_DEBUG=no-portals \
    cargo test --locked -p tachyon-core --lib linux_browser_apis_reach_geoclue_and_the_notification_server -- --ignored --test-threads=1 --nocapture
  take_screenshot notification
}

check_the_desktop_notification() {
  local calls
  calls=$(grep -A20 -E 'member=(Notify|AddNotification)' "$STATE/notifications.log" || true)
  echo "notification calls on the desktop's session bus: $(grep -cE 'member=(Notify|AddNotification)' <<<"$calls" || true)"
  grep -q '"Order"' <<<"$calls" || { echo "the page's notification never reached the desktop's notification server" >&2; exit 1; }
  grep -q '"Shipped"' <<<"$calls" || { echo "the notification reached the desktop without its body" >&2; exit 1; }
}

take_screenshot() {
  if [[ -z "${TAC_LINUX_SCREENSHOTS:-}" ]]; then return; fi
  node "$(dirname "$0")/linux-display-session.mjs" screenshot "$TAC_LINUX_SCREENSHOTS/services-$1.png"
}

write_tray_fixture() {
  local root=$1
  mkdir -p "$root/client/pages/help" "$root/client/components/tray/home" "$root/client/components/tray/help" "$root/client/shared"
  cat >"$root/client/manifest.json" <<'JSON'
{
  "name": "Tray Gate",
  "start_url": "/",
  "tac": {
    "application": {
      "id": "dev.tachyon.tray-gate",
      "version": "0.0.1",
      "build": 1,
      "tray": {
        "icon": "/shared/tray.png",
        "tooltip": "Tray Gate in the desktop tray",
        "items": [{ "id": "tray-help", "label": "Tray Help", "route": "/help" }],
        "closeHidesWindow": true
      }
    }
  }
}
JSON
  echo '<main aria-label="Tray home"><h1>Tray home</h1></main>' >"$root/client/components/tray/home/tac.html"
  echo '<main aria-label="Tray help"><h1>Tray help</h1></main>' >"$root/client/components/tray/help/tac.html"
  echo '<tray-home />' >"$root/client/pages/tac.html"
  echo '<tray-help />' >"$root/client/pages/help/tac.html"
  cp "$(dirname "$0")/../../website/client/shared/assets/logo-192.png" "$root/client/shared/tray.png"
}

show_the_tray_in_the_desktop() {
  if ! desktop_has tray; then
    echo 'SKIP: the desktop has no StatusNotifierItem host (org.kde.StatusNotifierWatcher), so the tray step did not run'
    TRAY_RESULT='no tray step (the desktop has no StatusNotifierItem host)'
    return
  fi
  if [[ -z "${TAC_BIN:-}" ]]; then
    cargo build --locked --bin ty
    TAC_BIN="${CARGO_TARGET_DIR:-target}/debug/ty"
  fi
  local fixture="$STATE/tray-gate"
  write_tray_fixture "$fixture"
  echo "==> building a Linux app with a tray icon and showing it in the desktop's own tray"
  timeout 300s "$TAC_BIN" build "$fixture" --target linux
  LIBGL_ALWAYS_SOFTWARE=1 WEBKIT_DMABUF_RENDERER_FORCE_SHM=1 GDK_DEBUG=no-portals \
    "$fixture/dist/linux/TrayGate/bin/TrayGate" >"$STATE/tray-gate.log" 2>&1 &
  local app=$!
  SERVICE_PIDS="$SERVICE_PIDS $app"
  timeout 300s python3 "$(dirname "$0")/desktop-tray-probe.py" "$app" "$XDG_STATE_HOME/tachyon/dev.tachyon.tray-gate.jsonl" | tee "$STATE/tray-probe.log" || { cat "$STATE/tray-gate.log" >&2; exit 1; }
  local closing='closing hid the window, Activate showed it'
  if grep -q '^OK close: Alt+F4 through Mutter' "$STATE/tray-probe.log"; then closing="closing (Alt+F4 through Mutter's RemoteDesktop keyboard) hid the window, Activate was answered"; fi
  if grep -q '^SKIP close' "$STATE/tray-probe.log"; then closing='closing was not checked (no Alt+F4 on this Wayland desktop), Activate was answered'; fi
  TRAY_RESULT="the desktop's own tray host showed the icon and its menu; $closing, a route item opened /help and Quit ended the app"
}

check_services_answered() {
  grep -E "allowed by agent|'[^']+' started\.|New location available from GClueNMEASource" "$STATE/geoclue.log" | sed 's/^.*DEBUG: [0-9:.]*: //' | sort -u
  grep -q "allowed by agent" "$STATE/geoclue.log" || { echo "the GeoClue agent was not asked" >&2; exit 1; }
  grep -q "New location available from GClueNMEASource" "$STATE/geoclue.log" || { echo "GeoClue gave no NMEA position" >&2; exit 1; }
  python3 - "$STATE/history.json" <<'PY'
import json
import sys
history = json.load(open(sys.argv[1]))["data"][0]
shown = [(entry["appname"]["data"], entry["summary"]["data"], entry["body"]["data"]) for entry in history]
print("notifications shown by dunst:", shown)
if not any(summary == "Order" and body == "Shipped" for _, summary, body in shown):
    sys.exit("dunst did not show the page's notification")
PY
}

main() {
  use_private_folders
  find_geoclue
  build_tests
  start_system_bus
  start_geoclue
  if in_the_live_desktop; then
    use_the_live_desktop
    run_with_the_desktop_notification_server
    check_the_desktop_notification
    show_the_tray_in_the_desktop
    echo "PASS: in the live $XDG_SESSION_TYPE desktop session (${XDG_CURRENT_DESKTOP:-unnamed desktop}) the Linux host reached GeoClue (a GPS fix from an NMEA socket, allowed by the GeoClue agent) and the desktop's own notification server; $TRAY_RESULT"
    return
  fi
  export -f check_services_answered
  run_with_desktop_services
  check_services_answered
  echo 'PASS: the Linux host reached GeoClue (a GPS fix from an NMEA socket, allowed by the GeoClue agent) and a desktop notification server (dunst) from the page'
}

main "$@"
