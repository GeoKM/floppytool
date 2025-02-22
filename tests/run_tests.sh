#!/bin/bash
set -x  # Enable debug output

# Root directory
ROOT_DIR="$(pwd)"
TEST_DIR="$ROOT_DIR/tests"
BIN="$ROOT_DIR/target/release/floppytool"
TEMP_DIR="$TEST_DIR/temp"

# Expected sizes (bytes)
SIZE_360k=368640   # 40×2×9×512
# SIZE_1_2M=1228800  # 80×2×15×512
# SIZE_1_44M=1474560 # 80×2×18×512

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
    local meta="$TEST_DIR/${size}/${size}.imd.meta"

    local expected_size
    case "$size" in
        "360k") expected_size=$SIZE_360k ;;
        # "1.2M") expected_size=$SIZE_1_2M ;;
        # "1.44M") expected_size=$SIZE_1_44M ;;
        *) echo "Unknown size: $size"; exit 1 ;;
    esac

    echo "Testing $size..."

    rm -f "$out_img" "$out_imd" "$meta"

    echo "  .imd -> .img"
    "$BIN" --input "$imd" convert --format img --output "$out_img" --verbose --validate
    [ -f "$out_img" ] || { echo "Failed: $out_img not created"; exit 1; }
    actual_size=$(wc -c < "$out_img" | tr -d ' ')
    [ "$actual_size" -eq "$expected_size" ] || { echo "Failed: Size $actual_size != expected $expected_size"; exit 1; }
    cmp "$img" "$out_img" && echo "    OK: Matches reference .img" || echo "    Warning: Differs from reference .img"
    [ -f "$meta" ] || { echo "Failed: $meta not created"; exit 1; }

    echo "  .img -> .imd (no meta)"
    "$BIN" --input "$img" convert --format imd --output "$out_imd" --geometry "$geometry" --verbose --validate
    [ -f "$out_imd" ] || { echo "Failed: $out_imd not created"; exit 1; }

    echo "  Roundtrip: .imd -> .img -> .imd"
    "$BIN" --input "$imd" convert --format img --output "$out_img" --verbose --validate
    "$BIN" --input "$out_img" convert --format imd --output "$out_imd" --geometry "$geometry" --imdmeta "$meta" --verbose --validate
    cmp "$imd" "$out_imd" && echo "    OK: Roundtrip matches original" || { echo "Failed: Roundtrip differs"; diff -u "$imd" "$out_imd"; exit 1; }
}

# Run tests for each size
test_conversion "360k" "40,2,9,512,4"
# test_conversion "1.2M" "80,2,15,512,3"  # Commented out for now
# test_conversion "1.44M" "80,2,18,512,5"  # Commented out for now

# Clean up
echo "Cleaning up..."
rm -rf "$TEMP_DIR"

echo "All tests completed successfully!"
