#!/bin/bash
WS_SERVER="127.0.0.1"
WS_PORT="3474"
APP_ID="refui"
PROD_APP_BASE_URL="https://firecertapp.firecert.comcast.com/prod/index.html"
LOCAL_APP_BASE_URL="http://localhost:8888/index.html"
APP_BASE_URL="$PROD_APP_BASE_URL"
APP_PORT="3473"
REPORT_BASE_URL="https://reporting.firecert.comcast.com/rippleMockDeviceSanity"
EXCLUDED_MODULES="[\\\"keyboard\\\",\\\"acknowledgechallenge\\\",\\\"pinchallenge\\\"]"
DEVICE_MODEL="Mock"
DEVICE_PARTNER="Mock"
FB_VERSION="1.1.0"
source ./utils.sh
HEADLESS=true
VERBOSE=true
. ./ci-params.sh
rm -f report.json
start_debug_ripple
./setup_ripple_with_mock_thunder.sh
run_interactive_sanity_on_mock_device "$WS_SERVER" "$WS_PORT" "$APP_ID" "$APP_BASE_URL" "$APP_PORT" "$REPORT_BASE_URL" "$EXCLUDED_MODULES" "$DEVICE_MODEL" "$DEVICE_PARTNER" "$FB_VERSION" "$HEADLESS" "$VERBOSE"
parse_result_and_log $FAILURE_THRESHOLD
stop_debug_ripple 
