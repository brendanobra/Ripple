#!/bin/sh
# SETPAYLOAD='{"jsonrpc":"2.0","id":1,"method":"voiceguidance.setSpeed", "params":{ "value" : 1.0 } }'
# TOKEN=""
URL="ws://localhost:3474/jsonrpc?appId=refui&session=123"
# echo "$SETPAYLOAD" | websocat  "$URL"
# sleep 1s

# GETPAYLOAD='{"jsonrpc":"2.0","id":1,"method":"voiceguidance.speed" }'
# echo "$GETPAYLOAD" | websocat "$URL"

SETPAYLOAD='{"jsonrpc":"2.0","id":1,"method":"voiceguidance.setSpeed", "params":{ "value" : 0.5 } }'
TOKEN=""
echo "$SETPAYLOAD" | websocat  "$URL"

# GETPAYLOAD='{"jsonrpc":"2.0","id":1,"method":"voiceguidance.speed" }'
# echo "$GETPAYLOAD" | websocat "$URL"

# SETPAYLOAD='{"jsonrpc":"2.0","id":1,"method":"voiceguidance.setSpeed", "params":{ "value" : 2.0 } }'
# TOKEN=""
# echo "$SETPAYLOAD" | websocat  "$URL"

# sleep 1s
# echo "$GETPAYLOAD" | websocat "$URL"

# SETPAYLOAD='{"jsonrpc":"2.0","id":1,"method":"voiceguidance.setSpeed", "params":{ "value" : 4.0 } }'
# TOKEN=""
# echo "$SETPAYLOAD" | websocat  "$URL"

# sleep 1s
# echo "$GETPAYLOAD" | websocat "$URL"

# SETPAYLOAD='{"jsonrpc":"2.0","id":1,"method":"voiceguidance.setSpeed", "params":{ "value" : 2.0 } }'
# TOKEN=""
# URL="ws://localhost:3474/jsonrpc?appId=refui&session=123"
# echo "$SETPAYLOAD" | websocat  "$URL"

# sleep 1s
# echo "$GETPAYLOAD" | websocat "$URL"
