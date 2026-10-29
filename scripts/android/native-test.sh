#!/usr/bin/env bash
set -euo pipefail
here="$(dirname "${BASH_SOURCE[0]}")"
step() {
  local script="$1"
  shift
  local started="$SECONDS"
  local status=0
  node "$here/$script" "$@" || status=$?
  node "$here/chain-state.mjs" between "$script" "$status" "$((SECONDS - started))"
  return "$status"
}
node "$here/chain-state.mjs" prepare
trap 'node "$here/chain-state.mjs" restore' EXIT
step native-test.mjs "$@"
step window-controls-test.mjs
step window-settings-test.mjs
step browser-apis-test.mjs
step yon-origin-test.mjs
step rust-services-test.mjs
step menus-test.mjs
step application-lifetime-test.mjs
step page-checks-test.mjs
step python-service-test.mjs
step accessibility-test.mjs
step system-back-test.mjs
