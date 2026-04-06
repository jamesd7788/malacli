#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DATA_DIR="$ROOT/data/raw"
mkdir -p "$DATA_DIR"

curl -L https://raw.githubusercontent.com/seven1m/open-bibles/master/eng-kjv.osis.xml \
  -o "$DATA_DIR/eng-kjv.osis.xml"

curl -L https://a.openbible.info/data/cross-references.zip \
  -o "$DATA_DIR/cross-references.zip"

unzip -o "$DATA_DIR/cross-references.zip" -d "$DATA_DIR" >/dev/null

echo "Fetched sources into $DATA_DIR"
ls -lh "$DATA_DIR"
