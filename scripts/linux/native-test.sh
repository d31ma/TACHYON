#!/usr/bin/env bash
set -euo pipefail

build_native_app() {
  local fixture=$1
  python3 scripts/native/desktop-fixture.py "$fixture" linux
  if [[ -z "${TAC_BIN:-}" ]]; then
    cargo build --locked --bin ty
    TAC_BIN="${CARGO_TARGET_DIR:-target}/debug/ty"
  fi
  echo '==> building the Linux WebView host and two-route Rust companion'
  timeout 240s "$TAC_BIN" build "$fixture" --target linux
}

check_artifacts() {
  local out=$1
  local app="${out}/NativeGate"
  test -x "$app/bin/NativeGate"
  test -f "$app/bin/libtachyoncompanion.so"
  test -f "$app/resources/NativeIndex.json"
  test -f "$app/resources/WebBundle/index.html"
  test -f "$app/resources/WebBundle/items/_id/index.html"
  test -f "$app/resources/WebBundle/shared/native-gate.css"
  test -f "$app/resources/WebBundle/.tachyon/services/GateRootService.js"
  test -f "$out/tachyon.host.json"
  test -f "$out/artifact-manifest.json"
  cmp "$out/web/index.html" "$app/resources/WebBundle/index.html"
  grep -qx 'X-Tachyon-Version=0.0.1' "$app/dev.tachyon.desktop-gate.desktop"
  grep -qx 'X-Tachyon-Build=7' "$app/dev.tachyon.desktop-gate.desktop"
}

check_host_descriptor() {
  local out=$1
  python3 - "$out/tachyon.host.json" <<'PY'
import json, sys
with open(sys.argv[1], encoding='utf-8') as source:
    host = json.load(source)
assert host['schemaVersion'] == 3 and host['renderMode'] == 'bundle', host
assert len(host['companions']) == 2, host
PY
}

drive_accessibility_probe() {
  local fixture=$1
  local app=$2
  echo '==> driving rendered web controls through the real accessibility bus'
  export XDG_STATE_HOME="$fixture/state"
  export XDG_DATA_HOME="$fixture/data"
  export GDK_BACKEND=x11
  export GTK_A11Y=atspi
  export LIBGL_ALWAYS_SOFTWARE=1
  export WEBKIT_DMABUF_RENDERER_FORCE_SHM=1
  export APP="$app"
  timeout 240s xvfb-run --auto-servernum --server-args='-screen 0 1024x1400x24' \
    dbus-run-session -- bash -c '
      set -euo pipefail
      "$APP/bin/NativeGate" &
      host=$!
      trap '\''kill "$host" 2>/dev/null || true; wait "$host" 2>/dev/null || true'\'' EXIT
      timeout 210s python3 scripts/linux/a11y-probe.py
    '
}

check_lifecycle_log() {
  local log="$XDG_STATE_HOME/tachyon/dev.tachyon.desktop-gate.jsonl"
  test -f "$log"
  local event
  for event in controller.created controller.mounted controller.active companion.loaded; do
    grep -q "\"event\":\"$event\"" "$log"
  done
}

launch_until_mounted() {
  local label=$1
  shift
  echo "==> launching the $label and waiting for its first page and Rust library"
  local log="$XDG_STATE_HOME/tachyon/dev.tachyon.desktop-gate.jsonl"
  rm -f "$log"
  export LOG="$log"
  timeout 150s xvfb-run --auto-servernum --server-args='-screen 0 1024x1400x24' \
    dbus-run-session -- bash -c '
      set -euo pipefail
      "$@" &
      host=$!
      trap '\''kill "$host" 2>/dev/null || true; wait "$host" 2>/dev/null || true'\'' EXIT
      for _ in $(seq 120); do
        if grep -q "\"event\":\"controller.mounted\"" "$LOG" 2>/dev/null && grep -q "\"event\":\"companion.loaded\"" "$LOG"; then exit 0; fi
        kill -0 "$host" 2>/dev/null || { echo "the app quit before its first page" >&2; exit 1; }
        sleep 1
      done
      echo "the app showed no page in 120 seconds" >&2
      exit 1
    ' launch "$@"
}

fuse_is_usable() {
  [[ -c /dev/fuse && -r /dev/fuse && -w /dev/fuse ]] && { command -v fusermount3 >/dev/null || command -v fusermount >/dev/null; }
}

start_appimage() {
  local appimage=$1
  if fuse_is_usable; then
    launch_until_mounted 'AppImage through FUSE' "$appimage"
    return
  fi
  echo '==> this machine has no usable FUSE, so the AppImage extracts itself instead of mounting'
  APPIMAGE_EXTRACT_AND_RUN=1 launch_until_mounted 'AppImage' "$appimage"
}

check_release_packages() {
  local fixture=$1
  echo '==> building the Linux release packages'
  timeout 300s "$TAC_BIN" bundle "$fixture" --target linux --release --out-dir release
  local out="$fixture/release/linux"
  local deb
  deb=$(find "$out" -maxdepth 1 -name 'nativegate_0.0.1-7_*.deb' -print -quit)
  local appimage
  appimage=$(find "$out" -maxdepth 1 -name 'NativeGate-0.0.1-7-*.AppImage' -print -quit)
  test -f "$deb"
  test -x "$appimage"
  test "$(dpkg-deb --field "$deb" Package Version)" = $'Package: nativegate\nVersion: 0.0.1-7'
  local root="$fixture/installed"
  dpkg-deb -x "$deb" "$root"
  test "$(readlink "$root/usr/bin/nativegate")" = '../lib/nativegate/bin/NativeGate'
  test -f "$root/usr/lib/nativegate/bin/libtachyoncompanion.so"
  local desktop="$root/usr/share/applications/dev.tachyon.desktop_gate.desktop"
  local metainfo="$root/usr/share/metainfo/dev.tachyon.desktop_gate.metainfo.xml"
  grep -qx 'Exec=nativegate' "$desktop"
  if command -v desktop-file-validate >/dev/null; then desktop-file-validate "$desktop"; fi
  if command -v appstreamcli >/dev/null; then
    local report
    report=$(appstreamcli validate --no-net "$metainfo" || true)
    echo "$report"
    if grep -q '^E:' <<<"$report"; then exit 1; fi
  fi
  launch_until_mounted 'installed .deb command' "$root/usr/bin/nativegate"
  start_appimage "$appimage"
}

reach_yon() {
  local services
  for services in javascript rust; do
    echo "==> reaching Yon on its declared origin from the packaged Linux app (${services} services)"
    TAC_BIN="$TAC_BIN" TAC_YON_SERVICES="$services" YON_JAVASCRIPT_RUNTIME="${YON_JAVASCRIPT_RUNTIME:-bun}" \
      timeout 600s node scripts/linux/yon-origin-native-test.mjs
  done
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

release_document_portal() {
  if [[ -n "$PRIVATE_RUNTIME" ]] && mountpoint -q "$PRIVATE_RUNTIME/doc" 2>/dev/null; then
    fusermount3 -u "$PRIVATE_RUNTIME/doc" 2>/dev/null || fusermount -u "$PRIVATE_RUNTIME/doc" 2>/dev/null || true
  fi
}

remove_folders() {
  release_document_portal
  rm -rf "$FIXTURE" ${PRIVATE_RUNTIME:+"$PRIVATE_RUNTIME"}
}

main() {
  FIXTURE=$(mktemp -d "${TMPDIR:-/tmp}/ty-linux-native.XXXXXX")
  PRIVATE_RUNTIME=
  trap 'remove_folders' EXIT
  use_a_writable_runtime_folder
  use_the_x11_display_only
  local out="${FIXTURE}/dist/linux"
  build_native_app "$FIXTURE"
  check_artifacts "$out"
  check_host_descriptor "$out"
  drive_accessibility_probe "$FIXTURE" "${out}/NativeGate"
  check_lifecycle_log
  check_release_packages "$FIXTURE"
  reach_yon
  echo 'PASS: Linux WebView, shared JS/CSS, native ABI, publish, dynamic routes, route isolation, the AppImage and .deb release packages, and Yon reach'
}

main "$@"
