# stck

[![CI](https://github.com/brdv/stck/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/brdv/stck/actions/workflows/ci.yml) [![Release](https://github.com/brdv/stck/actions/workflows/release.yml/badge.svg)](https://github.com/brdv/stck/actions/workflows/release.yml) [![GitHub release](https://img.shields.io/github/v/release/brdv/stck)](https://github.com/brdv/stck/releases) [![License](https://img.shields.io/github/license/brdv/stck)](https://github.com/brdv/stck/blob/main/LICENSE) [![GitHub stars](https://img.shields.io/github/stars/brdv/stck?style=social)](https://github.com/brdv/stck/stargazers)

`stck` is a Rust CLI for working with stacked GitHub pull requests.

Stacked PRs improve review quality and throughput, but day-to-day maintenance can be tedious and error-prone. `stck` focuses on automating the repetitive mechanics while staying close to native `git` and `gh` behavior.

## Status

First public release (`v0.1.0`).

## Basic Usage

Command surface:

```bash
stck new <branch>
stck status
stck sync
stck push
```

Git subcommand entrypoint is also installed (when installed via homebrew):

```bash
git stck <command>
```

## Installation

### Homebrew (recommended)

```bash
brew tap brdv/stck https://github.com/brdv/stck
brew install brdv/stck/stck
```

Verify installation:

```bash
stck --version
stck --help
git stck --help
```

### From source

```bash
cargo build --release --all-features
./target/release/stck --help
```

Homebrew release details are documented in [`docs/release-homebrew.md`](./docs/release-homebrew.md).

## Goal

`stck` aims to make stacked PR workflows predictable and low-friction by providing a small set of commands to:

- create the next branch in a stack,
- inspect stack and PR state,
- restack/rebase locally after upstream changes,
- push rewritten branches and update PR base relationships.

## Usage

For a step-by-step tutorial and command behavior details, see [`USAGE.md`](./USAGE.md).

## Contributing

Development and validation commands live in [`CONTRIBUTING.md`](./CONTRIBUTING.md).
