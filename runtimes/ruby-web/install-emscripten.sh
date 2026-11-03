#!/usr/bin/env bash
set -euo pipefail

runtime_folder() {
  cd "$(dirname "${BASH_SOURCE[0]}")" && pwd
}

source_field() {
  local name="$1"
  local field="$2"
  awk -v name="$name" -v field="$field" '$1 == name { print $field }' "$(runtime_folder)/sources.txt"
}

fail() {
  echo "install-emscripten.sh: $1" >&2
  exit 1
}

check_platform() {
  if [ "$(uname -s)" != "Darwin" ] || [ "$(uname -m)" != "arm64" ]; then
    fail "the pinned toolchain archives are for macOS on Apple silicon"
  fi
}

download_verified() {
  local name="$1"
  local target="$2"
  local checksum
  local url
  checksum="$(source_field "$name" 3)"
  url="$(source_field "$name" 4)"
  curl --fail --location --silent --show-error --output "$target" "$url"
  echo "${checksum}  ${target}" | shasum -a 256 --check --status || fail "$name archive has the wrong SHA-256"
}

unpack_emsdk() {
  local emsdk_folder="$1"
  mkdir -p "$emsdk_folder"
  if [ -n "$(ls -A "$emsdk_folder")" ]; then
    fail "the emsdk folder must be empty: $emsdk_folder"
  fi
  download_verified emsdk "$emsdk_folder/emsdk.tar.gz"
  tar -xzf "$emsdk_folder/emsdk.tar.gz" -C "$emsdk_folder" --strip-components=1
  rm "$emsdk_folder/emsdk.tar.gz"
}

download_toolchain_archives() {
  local downloads="$1/downloads"
  local release_url
  local release_hash
  mkdir -p "$downloads"
  release_url="$(source_field emscripten 4)"
  release_hash="$(basename "$(dirname "$release_url")")"
  download_verified emscripten "$downloads/${release_hash}-$(basename "$release_url")"
  download_verified node "$downloads/$(basename "$(source_field node 4)")"
  download_verified python "$downloads/$(basename "$(source_field python 4)")"
}

install_emscripten() {
  local emsdk_folder="$1"
  local version
  version="$(source_field emscripten 2)"
  (
    cd "$emsdk_folder"
    EMSDK_KEEP_DOWNLOADS=1 ./emsdk install "$version"
    ./emsdk activate "$version"
  )
}

main() {
  [ "$#" -eq 1 ] || fail "usage: install-emscripten.sh <empty folder for emsdk>"
  check_platform
  unpack_emsdk "$1"
  local emsdk_folder
  emsdk_folder="$(cd "$1" && pwd)"
  download_toolchain_archives "$emsdk_folder"
  install_emscripten "$emsdk_folder"
  echo "run: source $emsdk_folder/emsdk_env.sh"
}

main "$@"
