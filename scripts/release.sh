#!/usr/bin/env bash
cargo fmt --all
cargo clippy --fix
git-cliff -o CHANGELOG.md
embedmd -w README.md