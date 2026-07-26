#!/usr/bin/env bash
# Preview a HOCON device layout (.conf) file in a plain GTK window, with no
# MIDI/controller wiring. Thin wrapper around `cargo run --example preview`
#
# Usage:
#   ./preview.sh <path-to-conf-file>                  # open an interactive window
#   ./preview.sh <path-to-conf-file> <screenshot.png> # save a screenshot and exit

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

cargo run --manifest-path "$script_dir/Cargo.toml" --example preview -p pod-gtk -- "$@"
