#!/bin/sh
PAYLOAD='{"jsonrpc":"2.0","id":1,"method":"metrics.signout", "params": {}}'
TOKEN=""
URL="ws://localhost:3474/jsonrpc?appId=refui&session=123"
echo "$PAYLOAD" | websocat  "$URL"