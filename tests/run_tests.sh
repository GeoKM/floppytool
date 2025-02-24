#!/bin/bash
ROOT_DIR=/Users/keith/src/floppytool
TEST_DIR=/Users/keith/src/floppytool/tests
BIN=/Users/keith/src/floppytool/target/release/floppytool
TEMP_DIR=/Users/keith/src/floppytool/tests/temp
SIZE_360k=368640
SIZE_720k=737280
SIZE_1_2M=1228800
SIZE_1_44M=1474560

cargo build --release
mkdir -p $TEMP_DIR
cd $TEMP_DIR

test_conversion() {
    local size=$1
    local geometry=$2
    local imd=$TEST_DIR/$size/$size.imd
    local img=$TEST_DIR/$size/$size.img
    local out_img=$TEMP_DIR/${size}_out.img
    local out_imd=$TEMP_DIR/${size}_out.imd
    local meta=$TEST_DIR/$size/$size.imd.meta
    local expected_size

    case "$size" in
        "360k") expected_size=$SIZE_360k ;;
        "720k") expected_size=$SIZE_720k ;;
        "1.2M") expected_size=$SIZE_1_2M ;;
        "1.44M") expected_size=$SIZE_1_44M ;;
    esac

    echo "Testing $size..."
    rm -f $out_img $out_imd $meta

    echo "  .imd -> .img"
    $BIN --input $imd convert --format img --output $out_img --validate
    [ -f $out_img ] || exit 1
    actual_size=$(wc -c < $out_img | tr -d ' ')
    [ $actual_size -eq $expected_size ] || { echo "Size mismatch: $actual_size != $expected_size"; exit 1; }
    cmp $img $out_img && echo "    OK: Matches reference .img"

    echo "  .img -> .imd (no meta)"
    $BIN --input $img convert --format imd --output $out_imd --geometry $geometry --validate
    [ -f $out_imd ] || exit 1

    echo "  Roundtrip: .imd -> .img -> .imd"
    $BIN --input $imd convert --format img --output $out_img --validate
    $BIN --input $out_img convert --format imd --output $out_imd --geometry $geometry --imdmeta $meta --validate
    cmp $imd $out_imd && echo "    OK: Roundtrip matches original"
}

test_conversion 360k 40,2,9,512,4   # 5.25-inch DD, 250 kbps
test_conversion 720k 80,2,9,512,5   # 3.5-inch DD, 500 kbps (should be 250 kbps)
test_conversion 1.2M 80,2,15,512,3  # 5.25-inch HD, 500 kbps
test_conversion 1.44M 80,2,18,512,3 # 3.5-inch HD, 500 kbps (should be mode 5)

echo "Cleaning up..."
rm -rf $TEMP_DIR
echo "All tests completed successfully!"
