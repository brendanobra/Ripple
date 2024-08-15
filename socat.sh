#!/bin/sh
THUNDER_HOST="${1:-localhost}"
AS_HOST="${2:-localhost}"
killall -9 socat
sleep 3s
echo "Connecting to Thunder at ${THUNDER_HOST} and AS at ${AS_HOST}"
socat TCP4-LISTEN:9998,fork,reuseaddr TCP4:${THUNDER_HOST}:9998 &
socat TCP4-LISTEN:9005,fork,reuseaddr TCP4:${THUNDER_HOST}:9005 &

