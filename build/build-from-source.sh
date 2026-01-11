#!/bin/bash
echo "Starting building from source"
cd source-code
cd star
python3.13 -m venv venv
source venv/bin/activate
pip install python-hsl2
cargo build --release 
decativate
sudo cp -r /target/release/star ../source-code
cd ..
cd virus
../source-code/star main.hcs
sudo mv cache/build/target/release/virus ../source-code
cd ..
cd hspm
../source-code/star main.hcs
sudo mv cache/build/target/release/hspm ../source-code
cd ..
echo "Build from source complete"
