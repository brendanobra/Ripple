#!/bin/sh

cd Ripple && echo "Building OSS" && cargo build && cd .. && cd ripple_comcast_extns && echo "Building Comcast Extns" && cargo build && cd ..
