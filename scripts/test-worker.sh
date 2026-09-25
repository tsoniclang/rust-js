#!/usr/bin/env bash
set -euo pipefail
exec node "${TSONIC_ROOT:-../tsonic}/scripts/certification/capture-tests.mjs" cargo cargo test --locked --workspace "$@"
