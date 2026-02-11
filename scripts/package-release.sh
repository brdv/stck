#!/usr/bin/env bash
set -euo pipefail

if [[ "$#" -ne 4 ]]; then
  echo "usage: $0 <version> <target> <binary_path> <output_dir>" >&2
  exit 2
fi

version="$1"
target="$2"
binary_path="$3"
output_dir="$4"
archive_name="stck-v${version}-${target}.tar.gz"
archive_path="${output_dir}/${archive_name}"
checksum_path="${archive_path}.sha256"

if [[ ! -f "$binary_path" ]]; then
  echo "binary not found: $binary_path" >&2
  exit 1
fi

mkdir -p "$output_dir"

# Build reproducible tarball with the binary at archive root.
tar -C "$(dirname "$binary_path")" -czf "$archive_path" "$(basename "$binary_path")"

if command -v sha256sum >/dev/null 2>&1; then
  sha256sum "$archive_path" | awk '{print $1}' > "$checksum_path"
elif command -v shasum >/dev/null 2>&1; then
  shasum -a 256 "$archive_path" | awk '{print $1}' > "$checksum_path"
else
  echo "no SHA256 tool found (expected sha256sum or shasum)" >&2
  exit 1
fi

echo "packaged: $archive_path"
echo "sha256: $(cat "$checksum_path")"
