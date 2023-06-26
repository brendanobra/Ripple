#!/bin/sh
PAYLOAD='{"jsonrpc":"2.0","method":"localization.addAdditionalInfo","params": {"key":"test3","value":"asdf4"},"id":15}'
TOKEN=""
URL="ws://localhost:3474/jsonrpc?appId=refui&session=123"
echo "$PAYLOAD" | websocat  "$URL"

