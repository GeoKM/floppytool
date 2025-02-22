#!/bin/bash
set -x  # Enable debug output

# Root directory
ROOT_DIR="$(pwd)"
TEST_DIR="$ROOT_DIR/tests"
BIN="$ROOT_DIR/target/release/floppytool"
TEMP_DIR="$TEST_DIR/temp"

# Expected sizes (bytes)
declare -A EXPECTED_SIZES=(
    ["360k"]=368640   # 40×2×9×512
    ["720k"]=737280   # 80×2×9×512
    ["1.2M"]=1228800  # 80×2×15×512
    ["1.44M"]=1474560 # 80×2×18×512
)

# Build the tool
cargo build --release || { echo "Build failed"; exit 1; }

# Create temp directory
mkdir -p "$TEMP_DIR"
cd "$TEMP_DIR" || exit 1

# Test cases
test_conversion() {
    local size="$1"
    local geometry="$2"
    local imd="$TEST_DIR/${size}/${size}.imd"
    local img="$TEST_DIR/${size}/${size}.img"
    local out_img="$TEMP_DIR/${size}_out.img"
    local out_imd="$TEMP_DIR/${size}_out.imd"
    local meta="$TEMP_DIR/${size}.imd.meta"

    echo "Testing $size..."

    # Clean up previous runs
    rm -f "$out_img" "$out_imd" "$meta"

    # .imd to .img
    echo "  .imd -> .img"
    "$BIN" --input "$imd" convert --format img --output "$out_img" --verbose --validate
    [ -f "$out_img" ] || { echo "Failed: $out_img not created"; exit 1; }
    actual_size=$(stat -c %s "$out_img")
    [ "$actual_size" -eq "${EXPECTED_SIZES[$size]}" ] || echo "    Warning: Size $actual_size != expected ${EXPECTED_SIZES[$size]}"
    cmp "$img" "$out_img" && echo "    OK: Matches reference .img" || echo "    Warning: Differs from reference .img"
    [ -f "$meta" ] || { echo "Failed: $meta not created"; exit 1; }

    # .img to .imd (no metadata)
    echo "  .img -> .imd (no meta)"
    "$BIN" --input "$img" convert --format imd --output "$out_imd" --geometry "$geometry" --verbose --validate
    [ -f "$out_imd" ] || { echo "Failed: $out_imd not created"; exit 1; }
    grep "No metadata found" test.log >/dev/null 2>&1 && echo "    OK: Used default header" || echo "    Warning: Metadata behavior unexpected"

    # Roundtrip with metadata
    echo "  Roundtrip: .imd -> .img -> .imd"
    "$BIN" --input "$imd" convert --format img --output "$out_img" --verbose --validate
    "$BIN" --input "$out_img" convert --format imd --output "$out_imd" --geometry "$geometry" --imdmeta "$meta" --verbose --validate
    cmp "$imd" "$out_imd" && echo "    OK: Roundtrip matches original" || { echo "Failed: Roundtrip differs"; diff -u "$imd" "$out_imd"; exit 1; }
}

# Run tests for each size
test_conversion "360k" "40,2,9,512,4"
test_conversion "720k" "80,2,9,512,5"
test_conversion "1.2M" "80,2,15,512,4"
test_conversion "1.44M" "80,2,18,512,5"

# Clean up
echo "Cleaning up..."
rm -rf "$TEMP_DIR"

echo "All tests completed successfully!"
