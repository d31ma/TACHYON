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
  timeout 150s xvfb-run --auto-servernum --server-args='-screen 0 1024x1400x24' \
    dbus-run-session -- bash -c '
      set -euo pipefail
      "$APP/bin/NativeGate" &
      host=$!
      trap '\''kill "$host" 2>/dev/null || true; wait "$host" 2>/dev/null || true'\'' EXIT
      timeout 120s python3 scripts/linux/a11y-probe.py
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

main() {
  FIXTURE=$(mktemp -d "${TMPDIR:-/tmp}/ty-linux-native.XXXXXX")
  trap 'rm -rf "$FIXTURE"' EXIT
  local out="${FIXTURE}/dist/linux"
  build_native_app "$FIXTURE"
  check_artifacts "$out"
  check_host_descriptor "$out"
  drive_accessibility_probe "$FIXTURE" "${out}/NativeGate"
  check_lifecycle_log
  echo 'PASS: Linux WebView, shared JS/CSS, native ABI, publish, dynamic routes, and route isolation'
}

main "$@"
