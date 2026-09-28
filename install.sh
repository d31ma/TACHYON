#!/bin/sh
set -eu

REPO="d31ma/Tachyon"
BASE="${TAC_BASE_URL:-https://github.com/${REPO}/releases/latest/download}"
TACHYON_STEPS=5
tachyon_step=0

fail() {
    echo "$1" >&2
    exit 1
}

repeat_char() {
    char=$1
    count=$2
    out=""
    while [ "$count" -gt 0 ]; do
        out="${out}${char}"
        count=$((count - 1))
    done
    printf "%s" "$out"
}

tachyon_progress() {
    tachyon_step=$((tachyon_step + 1))
    percent=$((tachyon_step * 100 / TACHYON_STEPS))
    filled=$((tachyon_step * 24 / TACHYON_STEPS))
    empty=$((24 - filled))
    bar=$(repeat_char "#" "$filled")
    gap=$(repeat_char "-" "$empty")
    printf "TACHYON [%s%s] %3d%%  %s\n" "$bar" "$gap" "$percent" "$1"
}

print_banner() {
    printf "TACHYON installer\n"
    printf "Bringing the ty binary online...\n\n"
}

detect_os_tag() {
    os=$(uname -s)
    case "$os" in
        Darwin) os_tag="macos" ;;
        Linux) os_tag="linux" ;;
        *) fail "Unsupported OS: $os (use install.ps1 on Windows)" ;;
    esac
}

detect_arch_tag() {
    arch=$(uname -m)
    case "$arch" in
        x86_64 | amd64) arch_tag="x64" ;;
        arm64 | aarch64) arch_tag="arm64" ;;
        *) fail "Unsupported architecture: $arch" ;;
    esac
}

choose_install_dir() {
    if [ -n "${TAC_INSTALL_DIR:-}" ]; then
        dest="${TAC_INSTALL_DIR}"
        mkdir -p "$dest"
        return
    fi
    if [ -w /usr/local/bin ]; then
        dest="/usr/local/bin"
        return
    fi
    dest="${HOME}/.local/bin"
    mkdir -p "$dest"
}

choose_hash_command() {
    if command -v sha256sum >/dev/null 2>&1; then
        hash_cmd="sha256sum"
        return
    fi
    if command -v shasum >/dev/null 2>&1; then
        hash_cmd="shasum -a 256"
        return
    fi
    fail "No SHA-256 utility found. Aborting."
}

verify_checksum() {
    choose_hash_command
    curl -fsSL "${BASE}/SHA256SUMS" -o "$checksums"
    expected=$(awk -v asset="$asset" '$2 == asset { print $1; exit }' "$checksums")
    [ -n "$expected" ] || fail "No checksum published for ${asset}. Aborting."
    actual=$($hash_cmd "$tmp" | awk '{print $1}')
    [ "$expected" = "$actual" ] || fail "Checksum mismatch for ${asset}. Aborting."
}

install_binary() {
    chmod 0755 "$tmp"
    mv "$tmp" "$dest/ty"
    echo "Installed ty to ${dest}/ty"
}

print_path_hint() {
    case ":$PATH:" in
        *":$dest:"*) return ;;
    esac
    echo "Note: ${dest} is not on your PATH. Add it, e.g.:"
    echo "  export PATH=\"${dest}:\$PATH\""
}

main() {
    print_banner
    detect_os_tag
    detect_arch_tag
    tachyon_progress "Detected ${os_tag}/${arch_tag}"
    asset="ty-${os_tag}-${arch_tag}"

    choose_install_dir
    tachyon_progress "Selected install directory: ${dest}"

    tmp=$(mktemp "${dest}/.ty.download.XXXXXX")
    checksums=$(mktemp)
    trap 'rm -f "$tmp" "$checksums"' EXIT

    tachyon_progress "Downloading ${asset}"
    curl -fsSL "${BASE}/${asset}" -o "$tmp"

    tachyon_progress "Verifying release checksum"
    verify_checksum

    tachyon_progress "Installing ty"
    install_binary
    print_path_hint
    echo "Run 'ty --help' to get started."
}

main "$@"
