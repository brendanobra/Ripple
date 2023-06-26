#!/bin/sh
URL="ws://localhost:3474/jsonrpc?appId=refui&session=123"
PAYLOAD='{"jsonrpc":"2.0","method":"accessibility.onVoiceGuidanceSettingsChanged","params":{"listen":true},"id":11} '
echo "$PAYLOAD" | websocat  "$URL"

