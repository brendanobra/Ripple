#!/bin/sh
PAYLOAD='{"jsonrpc":"2.0","id":1,"method":"metrics.mediaRenditionChange", "params": { "entityId" : "a_clockwork_orange", "bitrate" : 2048, "width" : 1024, "height": 768, "profile": "SVGA" }}'
TOKEN=""
URL="ws://localhost:3474/jsonrpc?appId=refui&session=123"
echo "$PAYLOAD" | websocat  "$URL"

