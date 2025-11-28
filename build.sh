#!/bin/bash
# Build script for This Bitter Ground - Rust ECS Edition

set -e

echo "=== Building This Bitter Ground ==="

# Build Rust simulation library
echo ""
echo "--- Building Rust simulation (sim) ---"
cd sim
cargo build --release
cargo test
cd ..

# Build GDExtension
echo ""
echo "--- Building GDExtension (gdext) ---"
cd gdext
cargo build --release
cd ..

# Copy library to Godot project
echo ""
echo "--- Copying library to Godot project ---"
mkdir -p client/bin

# Detect platform and copy appropriate library
if [[ "$OSTYPE" == "darwin"* ]]; then
    cp gdext/target/release/libtbg_gdext.dylib client/bin/
    echo "Copied libtbg_gdext.dylib"
elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
    cp gdext/target/release/libtbg_gdext.so client/bin/
    echo "Copied libtbg_gdext.so"
elif [[ "$OSTYPE" == "msys" ]] || [[ "$OSTYPE" == "cygwin" ]]; then
    cp gdext/target/release/tbg_gdext.dll client/bin/
    echo "Copied tbg_gdext.dll"
fi

echo ""
echo "=== Build complete! ==="
echo ""
echo "To run the game:"
echo "  1. Open Godot 4.3+"
echo "  2. Import project from: client/project.godot"
echo "  3. Run the Main scene"
