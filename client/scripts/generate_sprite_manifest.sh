#!/bin/bash
# Generate sprite manifest for Android builds
# Run this from the client directory whenever sprites are added/removed

set -e

cd "$(dirname "$0")/.."

MANIFEST="assets/sprite_manifest.json"

echo "Generating sprite manifest..."

# Helper function to generate JSON array from find results
generate_array() {
    local dir="$1"
    local strip_prefix="$2"

    find "$dir" -name "*.png" 2>/dev/null | \
        sed "s|${strip_prefix}/||;s|\.png$||" | \
        sort | \
        while IFS= read -r line; do
            echo "    \"$line\""
        done | \
        paste -sd ',' - | \
        sed 's/,/,\n/g'
}

{
    echo '{'

    echo '  "enemies": ['
    generate_array "assets/sprites/enemies" "assets/sprites/enemies"
    echo '  ],'

    echo '  "equipment": ['
    generate_array "assets/sprites/equipment" "assets/sprites"
    echo '  ],'

    echo '  "weapons": ['
    generate_array "assets/sprites/weapons" "assets/sprites/weapons"
    echo '  ],'

    echo '  "inventory": ['
    generate_array "assets/sprites/inventory" "assets/sprites/inventory"
    echo '  ],'

    echo '  "objects": ['
    generate_array "assets/sprites/objects" "assets/sprites/objects"
    echo '  ],'

    echo '  "walls": ['
    generate_array "assets/sprites/walls" "assets/sprites/walls"
    echo '  ]'

    echo '}'
} > "$MANIFEST"

echo "Generated $MANIFEST"
echo "  - enemies: $(find assets/sprites/enemies -name '*.png' 2>/dev/null | wc -l | tr -d ' ')"
echo "  - equipment: $(find assets/sprites/equipment -name '*.png' 2>/dev/null | wc -l | tr -d ' ')"
echo "  - weapons: $(find assets/sprites/weapons -name '*.png' 2>/dev/null | wc -l | tr -d ' ')"
echo "  - inventory: $(find assets/sprites/inventory -name '*.png' 2>/dev/null | wc -l | tr -d ' ')"
echo "  - objects: $(find assets/sprites/objects -name '*.png' 2>/dev/null | wc -l | tr -d ' ')"
echo "  - walls: $(find assets/sprites/walls -name '*.png' 2>/dev/null | wc -l | tr -d ' ')"
