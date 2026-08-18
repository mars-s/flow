#!/usr/bin/env bash

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

target_dir="${CARGO_TARGET_DIR:-target}"
version="$(cargo metadata --no-deps --format-version 1 | sed -n 's/.*"name":"flow","version":"\([^"]*\)".*/\1/p')"
target_triple="$(rustc -vV | sed -n 's/^host: //p')"
package="flow-${version}-${target_triple}"
archive="$target_dir/release/$package.tar.gz"
staging="$(mktemp -d)"
trap 'rm -rf -- "$staging"' EXIT

cargo build --locked --release --package flow --bin flow --package flow-daemon --bin flow-daemon

package_dir="$staging/$package"
install -Dm755 "$target_dir/release/flow" "$package_dir/bin/flow"
install -Dm755 "$target_dir/release/flow-daemon" "$package_dir/bin/flow-daemon"
install -Dm644 resources/linux/sh.flow.desktop \
  "$package_dir/share/applications/sh.flow.desktop"
install -Dm644 website/public/app-icon.png \
  "$package_dir/share/icons/hicolor/256x256/apps/sh.flow.png"
install -Dm644 LICENSE "$package_dir/share/licenses/flow/LICENSE"

mkdir -p "$(dirname "$archive")"
tar -C "$staging" -czf "$archive" "$package"
printf 'Created %s\n' "$archive"
