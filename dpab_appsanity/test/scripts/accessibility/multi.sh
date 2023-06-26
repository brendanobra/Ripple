#!/bin/sh
websocat -E -t http-post-sse:tcp-l:127.0.0.1:3474 reuse:ws://127.0.0.1:8888/websock
URL="ws://localhost:3474/jsonrpc?appId=refui&session=123"
PAYLOAD='{"jsonrpc":"2.0","method":"accessibility.onVoiceGuidanceSettingsChanged","params":{"listen":true},"id":11} '
echo "$PAYLOAD" | websocat -E -t http-post-sse:tcp-l:127.0.0.1:3475 reuse:ws://127.0.0.1:3474/jsonrpc?appId=refui&session=123


URL="ws://localhost:3474/jsonrpc?appId=refui&session=123"
PAYLOAD='{"jsonrpc":"2.0","method":"voiceguidance.setEnabled","params":{"value" : true},"id":11} '
echo "$PAYLOAD" | websocat  "$URL"

#!/bin/sh
URL="ws://localhost:3474/jsonrpc?appId=refui&session=123"
PAYLOAD='{"jsonrpc":"2.0","method":"voiceguidance.setEnabled","params":{"value" : false},"id":11} '
echo "$PAYLOAD" | websocat  "$URL"

