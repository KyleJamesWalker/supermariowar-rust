# shellcheck shell=bash
# Sourced by the tools: cargo_target prints where cargo writes binaries, which a worktree may share with the main
# checkout (tools/share-target.sh). Each checkout's own outputs (packages, dist/) stay in its own tree.
cargo_target() {
    cargo metadata --format-version 1 --no-deps --manifest-path "$(dirname "${BASH_SOURCE[0]}")/../Cargo.toml" |
        sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p'
}
