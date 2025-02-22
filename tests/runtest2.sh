#!/bin/bash
set -x
./target/release/floppytool --input ../test_floppytool/360/360.IMD convert --format img --output test.img --verbose --validate
./target/release/floppytool --input test.img convert --format imd --output test2.imd --verbose --validate --geometry 40,2,9,512,4 --imdmeta ../test_floppytool/360/360.imd.meta
cmp -l ../test_floppytool/360/360.IMD test2.imd
