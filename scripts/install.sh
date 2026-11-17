#!/usr/bin/env bash
set -euo pipefail
umask 077

fail() { printf 'airgap: %s\n' "$*" >&2; exit 1; }
command -v curl >/dev/null || fail 'curl is required.'
command -v tar >/dev/null || fail 'tar is required.'
case "$(uname -s)" in
  Linux) platform=unknown-linux-musl ;;
  Darwin) platform=apple-darwin ;;
  *) fail 'Supported systems: 64-bit Linux and macOS.' ;;
esac
case "$(uname -m)" in
  x86_64|amd64) arch=x86_64 ;;
  aarch64|arm64) arch=aarch64 ;;
  *) fail 'Supported architectures: x86_64 and aarch64.' ;;
esac
if [[ "$platform" == apple-darwin && "$arch" == x86_64 ]] && command -v sysctl >/dev/null && [[ "$(sysctl -n hw.optional.arm64 2>/dev/null || true)" == 1 ]]; then
  arch=aarch64
fi
base=https://github.com/dhruvil-codes/airgap/releases
fetch() { curl --fail --silent --show-error --location --connect-timeout 10 --max-time 120 --proto '=https' --proto-redir '=https' --tlsv1.2 "$@"; }
if [[ -n "${AIRGAP_VERSION:-}" ]]; then
  tag="v${AIRGAP_VERSION#v}"
else
  resolved=$(fetch --head --output /dev/null --write-out '%{url_effective}' "$base/latest") || fail 'No published release could be resolved. See the repository for source installation.'
  [[ "$resolved" == "$base/tag/"* ]] || fail 'Unexpected latest-release URL.'
  tag=${resolved##*/}
fi
[[ "$tag" =~ ^v[0-9][0-9A-Za-z.+-]*$ ]] || fail 'Invalid release version.'
archive="airgap-$tag-$arch-$platform.tar.gz"
tmpdir=$(mktemp -d "${TMPDIR:-/tmp}/airgap.XXXXXXXX")
staged=''
cleanup() { [[ -z "$staged" ]] || rm -f -- "$staged"; rm -rf -- "$tmpdir"; }
trap cleanup EXIT
fetch --output "$tmpdir/$archive" "$base/download/$tag/$archive" || fail "Release asset unavailable for $arch-$platform."
fetch --output "$tmpdir/$archive.sha256" "$base/download/$tag/$archive.sha256" || fail 'Release checksum is unavailable.'
read -r expected checksum_name extra < "$tmpdir/$archive.sha256"
[[ "$expected" =~ ^[0-9a-f]{64}$ && "$checksum_name" == "$archive" && -z "$extra" ]] || fail 'Invalid checksum file.'
if command -v sha256sum >/dev/null; then
  actual=$(sha256sum "$tmpdir/$archive"); actual=${actual%% *}
elif command -v shasum >/dev/null; then
  actual=$(shasum -a 256 "$tmpdir/$archive"); actual=${actual%% *}
else
  fail 'sha256sum or shasum is required.'
fi
[[ "$actual" == "$expected" ]] || fail 'Checksum mismatch; the installed binary was not changed.'
members=$(tar -tzf "$tmpdir/$archive" -- airgap)
listing=$(tar -tvzf "$tmpdir/$archive" -- airgap)
[[ "$members" == airgap && "${listing:0:1}" == '-' && "$listing" != *$'\n'* ]] || fail 'Archive must contain one regular airgap executable.'
tar -xzf "$tmpdir/$archive" -C "$tmpdir" -- airgap
[[ -f "$tmpdir/airgap" && ! -L "$tmpdir/airgap" ]] || fail 'Invalid executable in archive.'
chmod 755 "$tmpdir/airgap"
[[ "$("$tmpdir/airgap" --version)" == "airgap ${tag#v}" ]] || fail 'Downloaded executable version does not match the release.'
if [[ -n "${AIRGAP_INSTALL_DIR:-}" ]]; then
  install_dir=$AIRGAP_INSTALL_DIR
elif [[ -d /usr/local/bin && -w /usr/local/bin ]]; then
  install_dir=/usr/local/bin
elif [[ -d "${HOME:?}/.cargo/bin" ]]; then
  install_dir="$HOME/.cargo/bin"
else
  install_dir="${HOME:?}/.local/bin"
fi
[[ "$install_dir" == /* ]] || fail 'AIRGAP_INSTALL_DIR must be an absolute path.'
mkdir -p "$install_dir"
[[ ! -d "$install_dir/airgap" ]] || fail 'Install destination is a directory.'
staged=$(mktemp "$install_dir/.airgap.XXXXXXXX")
install -m 755 "$tmpdir/airgap" "$staged"
mv -f "$staged" "$install_dir/airgap"
staged=''
if [[ ":$PATH:" != *":$install_dir:"* && "${AIRGAP_NO_PATH_UPDATE:-0}" != 1 ]]; then
  printf -v path_line 'export PATH=%q:"$PATH" # airgap installer' "$install_dir"
  case "${SHELL:-}" in
    */zsh) profiles=("$HOME/.zshrc") ;;
    */bash) profiles=("$HOME/.bashrc" "$HOME/.bash_profile") ;;
    *) profiles=("$HOME/.profile") ;;
  esac
  for profile in "${profiles[@]}"; do
    if [[ -L "$profile" ]]; then printf 'PATH profile is a symlink; leaving it untouched: %s\n' "$profile" >&2; continue; fi
    if [[ ! -f "$profile" ]] || ! grep -Fqx -- "$path_line" "$profile"; then
      if ! printf '\n%s\n' "$path_line" >> "$profile"; then printf 'Could not update PATH in %s. Editor setup uses the full executable path.\n' "$profile" >&2; fi
    fi
  done
  printf 'Added %s to your shell profile. Open a new terminal to use airgap by name.\n' "$install_dir"
fi
"$install_dir/airgap" setup --yes
printf 'Installed %s to %s\n' "$tag" "$install_dir/airgap"
