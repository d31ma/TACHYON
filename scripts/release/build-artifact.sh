#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TARGET="${TAC_RELEASE_TARGET:-}"
export SOURCE_DATE_EPOCH="${SOURCE_DATE_EPOCH:-0}"

read_toolchain() {
  grep -oE '"[0-9]+\.[0-9]+\.[0-9]+"' "${REPO_ROOT}/rust-toolchain.toml" | tr -d '"' | head -1
}

detect_os() {
  if [[ "${TARGET}" == *apple-darwin ]]; then
    echo macos
    return
  fi
  if [[ "${TARGET}" == *linux* ]]; then
    echo linux
    return
  fi
  if [[ "${TARGET}" == *windows* ]]; then
    echo windows
    return
  fi
  case "$(uname -s)" in
    Darwin) echo macos ;;
    Linux) echo linux ;;
    *) echo windows ;;
  esac
}

detect_arch() {
  local machine="${TARGET:-$(uname -m)}"
  if [[ "${machine}" == aarch64* || "${machine}" == arm64* ]]; then
    echo aarch64
    return
  fi
  echo x86_64
}

build_binary() {
  local build_args=(build --release --locked -p tachyon-cli --bin ty --features storage-all)
  if [[ -n "${TARGET}" ]]; then
    build_args+=(--target "${TARGET}")
  fi
  if [[ "${TAC_AUDITABLE:-0}" == "1" ]]; then
    cargo auditable "${build_args[@]}"
    return
  fi
  cargo "${build_args[@]}"
}

copy_release_files() {
  local stage=$1
  local binary=$2
  local target_dir="${CARGO_TARGET_DIR:-target}"
  if [[ -n "${TARGET}" ]]; then
    target_dir="${target_dir}/${TARGET}"
  fi
  cp "${target_dir}/release/${binary}" "${stage}/bin/${binary}"
  chmod 0755 "${stage}/bin/${binary}"
  cp LICENSE NOTICE "${stage}/"
  cp docs/SUPPORT_TIERS.md "${stage}/SUPPORT_TIERS.md"
}

write_install_notes() {
  local stage=$1
  local version=$2
  local binary=$3
  cat > "${stage}/INSTALL.md" <<EOF
# Tachyon ${version}

Install by copying \`bin/${binary}\` onto your PATH.

See https://tachyon.del.ma/docs/installation for platform-specific steps.

Verify the artifact before installing:

    shasum -a 256 -c SHA256SUMS

Uninstall by removing that file. The CLI artifact writes nothing else.
See SUPPORT_TIERS.md for the supported target contract.
EOF
}

write_manifest() {
  local stage=$1
  local version=$2
  local os=$3
  local arch=$4
  local toolchain=$5
  local commit
  commit="$(git -C "${REPO_ROOT}" rev-parse HEAD 2>/dev/null || printf '%040d' 0)"
  cat > "${stage}/manifest.json" <<EOF
{
  "contract_version": 1,
  "release_version": "${version}",
  "commit": "${commit}",
  "source_date_epoch": ${SOURCE_DATE_EPOCH},
  "target": { "os": "${os}", "architecture": "${arch}" },
  "toolchains": [{ "name": "rust", "version": "${toolchain}" }]
}
EOF
}

write_checksums() {
  if command -v sha256sum > /dev/null 2>&1; then
    find . -type f ! -name SHA256SUMS -print0 | sort -z \
      | xargs -0 sha256sum > SHA256SUMS
    return
  fi
  find . -type f ! -name SHA256SUMS -print0 | sort -z \
    | xargs -0 shasum -a 256 > SHA256SUMS
}

archive_timestamp() {
  if date -u -r "${SOURCE_DATE_EPOCH}" +%Y%m%d%H%M.%S >/dev/null 2>&1; then
    date -u -r "${SOURCE_DATE_EPOCH}" +%Y%m%d%H%M.%S
    return
  fi
  date -u -d "@${SOURCE_DATE_EPOCH}" +%Y%m%d%H%M.%S
}

create_archive() {
  local out=$1
  local name=$2
  local timestamp
  timestamp="$(archive_timestamp)"
  TZ=UTC find "${name}" -type f -exec touch -t "${timestamp}" {} +
  local files
  files="$(cd "${out}" && find "${name}" -type f | LC_ALL=C sort)"
  if tar --version 2>/dev/null | grep -q GNU; then
    printf '%s\n' "${files}" | tar --format=ustar \
        --owner=0 --group=0 --numeric-owner \
        --mtime="@${SOURCE_DATE_EPOCH}" \
        -cf "${name}.tar" -T -
  else
    printf '%s\n' "${files}" | tar --format=ustar \
        --uid 0 --gid 0 --numeric-owner \
        -cf "${name}.tar" -T -
  fi
  gzip -9 -n -f "${name}.tar"
}

print_checksum() {
  if command -v sha256sum > /dev/null 2>&1; then
    sha256sum "$1"
    return
  fi
  shasum -a 256 "$1"
}

main() {
  local out="${1:-${REPO_ROOT}/target/release-artifact}"
  local version
  version="$(tr -d '[:space:]' < "${REPO_ROOT}/VERSION")"
  local toolchain
  toolchain="$(read_toolchain)"
  local os
  os="$(detect_os)"
  local arch
  arch="$(detect_arch)"
  local name="tachyon-${version}-${arch}-${os}"
  local stage="${out}/${name}"
  local binary=ty
  if [[ "${os}" == windows ]]; then
    binary=ty.exe
  fi

  echo "==> building ty ${version} for ${arch}-${os}"
  cd "${REPO_ROOT}"
  build_binary

  rm -rf "${out}"
  mkdir -p "${stage}/bin"
  copy_release_files "${stage}" "${binary}"
  write_install_notes "${stage}" "${version}" "${binary}"

  echo "==> recording the manifest"
  write_manifest "${stage}" "${version}" "${os}" "${arch}" "${toolchain}"

  echo "==> checksumming every published file"
  cd "${stage}"
  write_checksums

  cd "${out}"
  create_archive "${out}" "${name}"

  echo "==> artifact"
  echo "${out}/${name}.tar.gz"
  print_checksum "${name}.tar.gz"
}

main "$@"
