#!/bin/bash
set -x

# Step 1: .imd -> .img
./target/release/floppytool --input ../test_floppytool/360/360.IMD convert --format img --output test.img --verbose --validate
hexdump -C -s 184320 test.img | head -n 33  # Middle
hexdump -C -s 368128 test.img | head -n 33  # End

# Step 2: .img -> .imd
./target/release/floppytool --input test.img convert --format imd --output test2.imd --verbose --validate --geometry 40,2,9,512,4
hexdump -C -s 184320 test2.imd | head -n 33  # Middle
hexdump -C -s 368128 test2.imd | head -n 33  # End

# Compare
cmp -l ../test_floppytool/360/360.IMD test2.imd | head -n 20
hexdump -C -s 184320 ../test_floppytool/360/360.IMD | head -n 33  # Middle reference
hexdump -C -s 368128 ../test_floppytool/360/360.IMD | head -n 33  # End reference
