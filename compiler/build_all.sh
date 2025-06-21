#!/bin/bash

# Build script for Mist programs

echo "Building all Mist programs..."
echo "=============================="

# Array of Mist source files
MIST_FILES=(
    "src/dice.mist"
    "src/restaurant.mist"
    "src/weather.mist"
)

# Compile each file
for mist_file in "${MIST_FILES[@]}"
do
    echo
    echo "Compiling: $mist_file"
    
    # Get base name for output
    base_name=$(basename "$mist_file" .mist)
    
    # Compile to C
    cargo run -- "$mist_file" --output "build/${base_name}.c"
    
    # If successful, compile C to executable
    if [ $? -eq 0 ]; then
        gcc -o "build/${base_name}" "build/${base_name}.c" -Wall -Wextra -std=c99
        if [ $? -eq 0 ]; then
            echo "✓ Created executable: build/${base_name}"
        fi
    fi
done

echo
echo "Build complete! Executables are in the build/ directory"
echo
echo "Run them with:"
for mist_file in "${MIST_FILES[@]}"
do
    base_name=$(basename "$mist_file" .mist)
    echo "  ./build/${base_name}"
done