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

find_geoclue() {
  GEOCLUE=""
  for candidate in /usr/libexec/geoclue /usr/lib/geoclue-2.0/geoclue; do
    if [[ -x "$candidate" ]]; then GEOCLUE="$candidate"; fi
  done
  GEOCLUE_AGENT=/usr/libexec/geoclue-2.0/demos/agent
  for program in "$GEOCLUE" "$GEOCLUE_AGENT" "$(command -v dunst)" "$(command -v dunstctl)"; do
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
  export -f check_services_answered
  run_with_desktop_services
  check_services_answered
  echo 'PASS: the Linux host reached GeoClue (a GPS fix from an NMEA socket, allowed by the GeoClue agent) and a desktop notification server (dunst) from the page'
}

main "$@"
