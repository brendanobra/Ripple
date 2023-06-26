#!/bin/sh
PAYLOAD='{"jsonrpc":"2.0","id":1,"method":"metrics.error", "params": {"type": "other" , "code" : "666", "description" : "superbad", "visible": false }}'
TOKEN=""
URL="ws://localhost:3474/jsonrpc?appId=refui&session=123"
echo "$PAYLOAD" | websocat  "$URL"

