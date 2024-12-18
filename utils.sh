function backup_ripple_configs() {
    backup_name="$(date +"%F-%H-%M")"
    echo "backups will be created in etc with suffix ${backup_name}"
    if [ -f "/etc/firebolt-device-manifest.json" ]; 
    then
        if [ -L "/etc/firebolt-device-manifest.json" ];
        then 
            echo "/etc/firebolt-device-manifest.json is a symlink , deleting"
            do_sudo 'rm -f /etc/firebolt-device-manifest.json'
        else
            echo "backup up /etc/firebolt-device-manifest.json to /etc/firebolt-device-manifest.json.${backup_name}"
            do_sudo 'mv /etc/firebolt-device-manifest.json "/etc/firebolt-device-manifest.json.${backup_name}"'
        fi
   fi;
    if [ -f "/etc/firebolt-app-library.json" ]; 
    then
        if [ -L "/etc/firebolt-app-library.json" ];
        then 
            echo "/etc/firebolt-app-library.json is a symlink , deleting"
            do_sudo 'rm -f /etc/firebolt-app-library.json'
        else
            echo "backup up /etc/firebolt-app-library.json to /etc/firebolt-app-library.json.${backup_name}"
            do_sudo 'mv /etc/firebolt-app-library.json "/etc/firebolt-app-library.json.${backup_name}"'
        fi
    fi;

    if [ -f "/etc/firebolt-extn-manifest.json" ]; 
    then
        if [ -L "/etc/firebolt-extn-manifest.json" ];
        then 
            echo "/etc/firebolt-extn-manifest.json is a symlink , deleting"
            do_sudo 'rm -f /etc/firebolt-extn-manifest.json'
        else
            echo "backup up /etc/firebolt-extn-manifest.json to /etc/firebolt-extn-manifest.json.${backup_name}"
            do_sudo 'mv /etc/firebolt-extn-manifest.json "/etc/firebolt-extn-manifest.json.${backup_name}"'
        fi
    fi;
}
function prepare_ripple_etc() {
    do_sudo 'mkdir -p /etc/ripple'
    do_sudo 'rm -f /etc/firebolt-device-manifest.json'
    do_sudo 'rm -f /etc/firebolt-app-library.json'
    do_sudo 'rm -f /etc/firebolt-extn-manifest.json'
    do_sudo 'rm -f /etc/mock-thunder-device.json'
    do_sudo 'rm -rf /etc/ripple/openrpc'
    do_sudo 'rm -rf /etc/ripple/rules'
    do_sudo 'mkdir -p /etc/ripple/openrpc'
    do_sudo 'mkdir -p /etc/ripple/rules'
    
}
setup_ripple_extensions() {
    echo "setting up ripple extensions from file $1"
    HERE=`pwd`
    EXTENSIONS=$(cat $1)
    LIB_EXTENSION=${2-so}
    RIPPLE_DIR=${3-$HERE} 
    LIB_DIR=$(echo $EXTENSIONS| jq -r .default_path)
    echo "libraries will be symlinked in ${LIB_DIR}"
    do_sudo 'mkdir -p "${LIB_DIR}"'
    do_sudo 'rm -rf "${LIB_DIR}/*"'
    echo "${EXTENSIONS}" | jq  -r .extns[].path| while read extension ; do

        extension_path=$(find "${RIPPLE_DIR}/target/debug/" -name "$extension"."${LIB_EXTENSION}" -print -quit)
        echo "linking ${extension_path} in ${RIPPLE_DIR}/target/debug/ to ${LIB_DIR}"
        do_sudo 'rm -rf "${LIB_DIR}/${extension}.${LIB_EXTENSION}"'
        do_sudo 'ln -s "${extension_path}" "${LIB_DIR}${extension}.${LIB_EXTENSION}"'
    done
}


function run_interactive_sanity_on_mock_device() {
  local ws_server="$1"
  local ws_port="$2"
  local app_id="$3"
  local app_base_url="$4"
  local app_port="$5"
  local report_base_url="$6"
  local excluded_modules="$7"
  local device_model="$8"
  local device_partner="$9"
  local fb_version="${10}"
  local headless="${11:-true}"
  local verbose="${12:-true}"
  #bobra200: this, sadly, still needs to be run here
  npm i puppeteer
  npx puppeteer browsers install firefox

  echo "Going to run sanity suite...."

  # WebSocket connection
  WEBSOCKET_URL="ws://${ws_server}:${ws_port}?appId=${app_id}"
  PUBSUB_URL="ws://${ws_server}:${ws_port}"

  # Execution Id and Report URL
  executionId=$(uuidgen)
  echo $executionId
  reportIdArr=(${executionId//-/ })
  reportId=${reportIdArr[0]}
  REPORT_URL="${report_base_url}/${reportId}/report.html"
  echo "Report URL at the start: $REPORT_URL"

  # Send LifecycleManagement.session message to get sessionId
  MESSAGE='{"jsonrpc": "2.0", "id": 1, "method": "LifecycleManagement.session", "params": {"session": {"app": {"id": "'${app_id}'","url": "'${app_base_url}'?systemui=true&mf=ws://'${ws_server}':'${ws_port}'?appId='${app_id}'"},"runtime": {"id": "WebBrowser-1"},"launch": {"intent": {"action": "search","data": {"query": "{\"task\":\"runTest\",\"params\":{\"macaddress\":\"'${executionId}'\",\"certification\":true,\"modulesToBeExcluded\":'${excluded_modules}'},\"action\":\"CORE\",\"context\":{\"communicationMode\":\"SDK\"},\"metadata\":{\"target\":\"RIPPLE\",\"targetVersion\":\"TBC\",\"fireboltVersion\":\"1.1.0\",\"deviceModel\":\"'${device_model}'\",\"devicePartner\":\"'${device_partner}'\",\"fbVersion\":\"'${fb_version}'\"},\"asynchronous\":false,\"appType\":\"firebolt\",\"reportingId\":\"'${executionId}'\",\"standalone\":\"true\",\"standalonePrefix\":\"rippleMockDeviceSanity\"}"},"context": {"source": "device"}}}}}}'
  NOTMESSAGE='{"jsonrpc": "2.0", "id": 1, "method": "LifecycleManagement.session", "params": {"session": {"app": {"id": "'${app_id}'","url": "'${app_base_url}'?systemui=true"}}}}'
  
  echo "Sending priming message to websocket server to get session Id Message: $MESSAGE  url: $WEBSOCKET_URL"
  response=$(wscat -c $WEBSOCKET_URL -w 1 -x "$MESSAGE")
  echo "Received session response from webocket server: $response"

  # Extract the sessionId
  sessionId=$(echo "$response" | jq -r '.result.sessionId')

  if [ "$sessionId" = "null" ]; then
    echo "Session ID is empty. Exiting..."
    echo "last 100 lines of ripple.logs are: "
    tail -n 100 /tmp/ripple.stdout.log
    exit 1
  fi
  echo "Session ID: $sessionId"
  
  # This is will run headless in docker container - mainly for running in CI/CD pipeline
  XVFB=`which Xvfb`
  FOUND=$?
  if [ "$FOUND" -eq 0 ]; then
    echo "Xvfb installed. will run tests headless using Xvfb"
    echo "Starting xvfb..."
    Xvfb :99 -screen 0 1024x768x24 > /dev/null 2>&1 &
    export DISPLAY=:99
    headless=true
  else
    echo "Xvfb not installed. will run tests using the display, and use the headless flag which is set to $headless"
  fi
  

  echo "Running sanity script with ripple-mock-device-extn. Vebosity is set to $verbose"
  node -e '
  const puppeteer = require("puppeteer");
  const fs = require("fs");
  (async () => {
    const browser = await puppeteer.launch({ product: "firefox", headless: '${headless}', args: ["--no-sandbox", "--disable-gpu"] });
    const page = await browser.newPage();
    console.log(await browser.version());

    // Enable console logging
    page.on("console", (msg) => {
      let logMessage = msg.text();
      
      if ('${verbose}') {
        console.log("CONSOLE message from FCA: " + logMessage);
      }
      if (logMessage.includes("Response String:")) {
        const jsonStringMatch = logMessage.match(/Response String:(.*)/);
        if (jsonStringMatch && jsonStringMatch[1]) {
          try {
            const jsonString = jsonStringMatch[1].trim();
            const responseString = JSON.parse(jsonString);
            const filePath="report.json"
            fs.writeFileSync(filePath, JSON.stringify(responseString), "utf-8");
          } catch (error) {
            console.error("Error parsing JSON:", error);
          }
        }
      }

      if (logMessage.includes("Response on load:")) {
        // Exit the Node.js script
        process.exit(0);
      }
    });

    const url = "'${app_base_url}'?systemui=true&mf=ws://'${ws_server}':'${app_port}'?appId='${app_id}'%26session='${sessionId}'";

    console.log("Test client: Navigating to:", url);

    await page.goto(url);
    await new Promise(resolve => setTimeout(resolve, 120000));
    await browser.close();

    console.log("Execution for Sanity Suite is completed...")
  })();'

  echo ""
  echo "************* Sanity Suite Report **************"
  echo "$REPORT_URL"
  echo "Core Sanity report can be found here :: $REPORT_URL" 

}

function parse_result_and_log() {
  echo "************* Sanity Suite Result Summary **************"
  echo "running in $(pwd)"
  if [ -e report.json ]; then
    totalTests=$(jq -r '.tests' report.json)
    pass=$(jq -r '.passes' report.json)
    skip=$(jq -r '.skipped' report.json)
    pending=$(jq -r '.pending' report.json)
    failures=$(jq -r '.failures' report.json)
    echo -e "Total Tests: $totalTests \nPasses: $pass \nFailures: $failures \nSkipped: $skip \nPending: $pending"
    echo "Failure threshold is : $1"
    if [ "$1" -gt 0 ]; then
        if [ "$failures" -gt "$1" ]; then
            echo "Failures are more than threshold of ${1}. Failing and exiting nonzero"
            exit 1
        else
            echo "There were ${failures} failures, which are less than or equal to ${1}, so exiting with 0 (success)"
            exit 0
        fi
    else
      if [ "$failures" -eq 0 ]; then
          echo "No failures detected."
          exit 0
      else
        echo "There are failures in Sanity... Review the report and resolve the errors..."
        exit 1 
      fi
    fi
  else
    echo "No report.json file found. Exiting with 1"
    exit 1
  fi
}
function do_sudo() {
  DOSUDO=`which sudo`
  FOUND=$?
  if [ "$FOUND" -eq 0 ]; then
    echo "sudo installed. will run command: $1 with sudo"
    eval "sudo $1"
  else
    echo "sudo not installed. will run command: $1 without sudo"
    eval "$1"
  fi
}
function start_debug_ripple() {
  target/debug/ripple > /tmp/ripple.stdout.log 2>&1 &
  sleep 10s
  pgrep ripple
  RIPPLE_RUNNING=$?
  if [ "${RIPPLE_RUNNING}" -ne "0" ]; then
    echo "ripple is not running, halting. please see logfile output for clues as to what went wrong:"
    cat /tmp/ripple.stdout.log
    exit "${RIPPLE_RUNNING}"
  else
    echo "ripple is running, proceeeding"
  fi
}
function stop_debug_ripple() {
  killall -9 ripple
}
