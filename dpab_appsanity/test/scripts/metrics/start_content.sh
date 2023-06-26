#!/bin/sh
PAYLOAD='{"jsonrpc":"2.0","id":1,"method":"metrics.startContent", "params": {"entity_id" : "a_clockwork_orange" }}'
TOKEN=""
URL="ws://localhost:3474/jsonrpc?appId=refui&session=123"
echo "$PAYLOAD" | websocat  "$URL"

