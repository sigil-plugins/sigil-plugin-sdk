#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: $0 /path/to/sigil/checkout" >&2
  exit 2
fi

cmp --silent \
  "$1/wit/sigil-host/1.0.0/host.wit" \
  "wit/sigil-host/1.0.0/host.wit"
echo "sigil:host@1.0.0 WIT is byte-identical"

cmp --silent \
  "$1/wit/sigil-host/1.1.0/host.wit" \
  "wit/sigil-host/1.1.0/host.wit"
echo "sigil:host@1.1.0 WIT is byte-identical"
