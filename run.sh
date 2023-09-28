#!/bin/bash
cd Ripple
cargo build
cd ../ripple_comcast_extns
cargo build
cd ../Ripple
./ripple run