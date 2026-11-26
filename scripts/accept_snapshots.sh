#!/bin/sh
# This script moves all {name}.new.png files to {name}.png.
# scripts/update_snapshots_from_ci.sh uses it, and you can also use it locally.

set -eu

# rename the .new.png files to .png
find . -type d -path "*/tests/snapshots*" | while read dir; do
    find "$dir" -type f -name "*.new.png" | while read file; do
        mv -f "$file" "${file%.new.png}.png"
    done
done

echo "Done!"
