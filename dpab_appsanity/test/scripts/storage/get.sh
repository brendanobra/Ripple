#!/bin/sh
PAYLOAD='{"jsonrpc":"2.0","method":"securestorage.get","params":{"scope":"device","key":"authRefreshToken"},"id":15}'
TOKEN=""
URL="ws://localhost:3474/jsonrpc?appId=refui&session=123"
echo "$PAYLOAD" | websocat  "$URL"

