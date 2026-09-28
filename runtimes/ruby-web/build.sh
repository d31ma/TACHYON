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
  echo "build.sh: $1" >&2
  exit 1
}

emcc_version() {
  emcc --version | head -n 1 | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -n 1
}

check_tools() {
  local minimum_version
  minimum_version="$(source_field emscripten 2)"
  command -v ruby >/dev/null || fail "ruby is not on PATH"
  command -v emcc >/dev/null || fail "emcc is not on PATH; install Emscripten ${minimum_version} or newer and load emsdk_env.sh"
  ruby -e 'exit(Gem::Version.new(ARGV[0]) >= Gem::Version.new(ARGV[1]))' "$(emcc_version)" "$minimum_version" ||
    fail "emcc must be version ${minimum_version} or newer"
}

prepare_work_folder() {
  local work_folder="$1"
  mkdir -p "$work_folder"
  if [ -n "$(ls -A "$work_folder")" ]; then
    fail "the work folder must be empty: $work_folder"
  fi
}

download_source() {
  local name="$1"
  local archive="$2"
  local checksum
  local url
  checksum="$(source_field "$name" 3)"
  url="$(source_field "$name" 4)"
  curl --fail --location --silent --show-error --output "$archive" "$url"
  echo "${checksum}  ${archive}" | shasum -a 256 --check --status || fail "$name archive has the wrong SHA-256"
}

unpack_sources() {
  local work_folder="$1"
  local mruby_folder="$work_folder/mruby"
  local prism_folder="$mruby_folder/mrbgems/mruby-compiler/lib/prism"
  download_source mruby "$work_folder/mruby.tar.gz"
  download_source prism "$work_folder/prism.tar.gz"
  mkdir -p "$mruby_folder"
  tar -xzf "$work_folder/mruby.tar.gz" -C "$mruby_folder" --strip-components=1
  mkdir -p "$prism_folder"
  tar -xzf "$work_folder/prism.tar.gz" -C "$prism_folder" --strip-components=1
}

build_library() {
  local work_folder="$1"
  local jobs="${TAC_RUBY_BUILD_JOBS:-2}"
  local configuration="$(runtime_folder)/build-config.rb"
  (
    cd "$work_folder/mruby"
    TAC_RUBY_BUILD_DIR="$work_folder/build" nice -n 10 ruby ./minirake -j"$jobs" \
      MRUBY_CONFIG="$configuration" all
  )
}

link_module() {
  local work_folder="$1"
  nice -n 10 emcc -Oz -fwasm-exceptions -sSUPPORT_LONGJMP=wasm -sSTANDALONE_WASM --no-entry \
    -sALLOW_MEMORY_GROWTH=1 -sINITIAL_MEMORY=4MB -sSTACK_SIZE=1MB \
    -sEXPORTED_FUNCTIONS=_tac_boot,_tac_invoke,_tac_alloc,_tac_free \
    "$work_folder/build/tachyon-ruby/lib/libmruby.a" -o "$work_folder/tachyon-ruby.wasm"
}

main() {
  local work_folder="${1:-$(mktemp -d)}"
  check_tools
  prepare_work_folder "$work_folder"
  work_folder="$(cd "$work_folder" && pwd)"
  unpack_sources "$work_folder"
  build_library "$work_folder" >"$work_folder/build.log" 2>&1 || fail "the mruby build failed; see $work_folder/build.log"
  link_module "$work_folder"
  (cd "$work_folder" && shasum -a 256 tachyon-ruby.wasm)
  echo "built $work_folder/tachyon-ruby.wasm"
}

main "$@"
