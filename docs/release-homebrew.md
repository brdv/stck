# Homebrew Release Notes

This repository includes the release automation and a Homebrew formula scaffold for Milestone 8.

## What happens on tag push

Pushing a tag like `v0.1.0` triggers `.github/workflows/release.yml`, which:

1. Builds `stck` on Apple Silicon macOS (`aarch64-apple-darwin`).
2. Packages release archive:
   - `stck-vX.Y.Z-aarch64-apple-darwin.tar.gz`
3. Generates SHA256 files per archive and a combined `checksums.txt`.
4. Publishes/updates the GitHub Release for that tag with these assets.

## Version guard

The release workflow validates that:

- tag `vX.Y.Z` matches `Cargo.toml` `version = "X.Y.Z"`.

## Homebrew formula

`Formula/stck.rb` is a tap-style formula scaffold that:

- installs `stck`
- installs `git-stck` symlink to support `git stck ...`

Before publishing for real, update `Formula/stck.rb`:

1. Set `version`.
2. Set macOS arm64 asset URL.
3. Replace placeholder SHA256 with the real checksum from the release asset.

## Manual post-release checklist

1. Verify `stck --version` matches tag version.
2. Push release tag: `git tag vX.Y.Z && git push origin vX.Y.Z`.
3. Confirm release assets + checksums uploaded.
4. Update `Formula/stck.rb` SHA256 values.
5. Test install from tap on Apple Silicon:
   - `brew install <tap>/stck`
   - `stck --help`
   - `git stck --help`
