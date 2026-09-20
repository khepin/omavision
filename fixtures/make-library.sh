#!/bin/sh
# Materializes fixtures/library/ from fixtures/library.txt as empty files, so the
# app can be developed without the real media mount. Idempotent.
set -e
cd "$(dirname "$0")"
rm -rf library
while IFS= read -r rel; do
  [ -z "$rel" ] && continue
  mkdir -p "library/$(dirname "$rel")"
  : > "library/$rel"
done < library.txt
echo "fixtures/library: $(find library -type f | wc -l | tr -d ' ') files"
