#!/bin/bash
shopt -s nocasematch
workspace_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)

function get_default_extension() {
      case "$(uname -s)" in
      Darwin)
          echo "dylib"
          ;;
      CYGWIN* | MINGW32* | MSYS* | MINGW*)
          echo "dll"
          ;;
      Linux)
          echo "so"
          ;;
      esac
}

echo "*****      Welcome to Eos-Ripple Run script        *****"
echo ""
echo "Note: Always run this in the eos-ripple folder"
echo "Current working directory: ${workspace_dir}"

echo ""
echo "Before we proceed lets setup the manifest"

device_type="mock"
echo "Device Type selected ${device_type}"

echo "Initializing mock mode"
is_mock=true
device_type=puck
partner_type=cert
HERE=`pwd`
TARGET_DIR="${workspace_dir}/target"
MANIFESTS_PATH="${TARGET_DIR}/manifests"
RULES_PATH="${TARGET_DIR}/rules"
cargo build --quiet --features local_dev || exit
echo "Cleaning up manifest folder in target directory"
mkdir -p "${MANIFESTS_PATH}"
mkdir -p "${RULES_PATH}"
rm -rf "${MANIFESTS_PATH}/firebolt-extn-manifest.json"
rm -rf "${MANIFESTS_PATH}/firebolt-device-manifest.json"
rm -rf "${MANIFESTS_PATH}/firebolt-app-library.json"
echo "Copying to target directory"
cp firebolt-devices/"$partner_type"/"$device_type"/app-library.json "${MANIFESTS_PATH}/firebolt-app-library.json"

echo "Copying mock manifests and rules to target directory"
cp mock/manifest.json "${MANIFESTS_PATH}/firebolt-device-manifest.json"
cp mock/extn.json "${MANIFESTS_PATH}/firebolt-extn-manifest.json"
cp mock/mock-thunder-device.json "${MANIFESTS_PATH}/mock-thunder-device.json"
cp mock/rules/* "${RULES_PATH}/"
ls -la "${MANIFESTS_PATH}"
ls -la "${RULES_PATH}"
#create list of rules in json format for loading into manifest 
RULES=$(find "${RULES_PATH}" -maxdepth 1 -type f | jq -R -s -c 'split("\n")[:-1]' )
echo "Rules path: ${RULES}"

## Update firebolt-extn-manifest.json
default_extension=$(get_default_extension)
extnManifestJson=$(<target/manifests/firebolt-extn-manifest.json)
new_extnJson=$(echo "$extnManifestJson" | jq '.default_path = "'$workspace_dir'/target/debug/" |
    .default_extension = "'$default_extension'" |
    .rules_path = '$RULES' |
    .extns[].symbols[] |= if has("config") and (.config | type == "object" and has("mock_data_file")) then .config.mock_data_file |= "'$workspace_dir'/target/manifests/mock-thunder-device.json" else . end')
echo "$new_extnJson" > target/manifests/firebolt-extn-manifest.json
## update rules_path to target/debug/rules




export EXTN_MANIFEST=${workspace_dir}/target/manifests/firebolt-extn-manifest.json
export DEVICE_MANIFEST=${workspace_dir}/target/manifests/firebolt-device-manifest.json
export APP_LIBRARY=${workspace_dir}/target/manifests/firebolt-app-library.json

echo ""
echo "Environment variables for manifests set"
echo ""
echo "DEVICE_MANIFEST=${DEVICE_MANIFEST}"
echo "EXTN_MANIFEST=${EXTN_MANIFEST}"
echo "APP_LIBRARY=${APP_LIBRARY}"
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
echo "ripple log after 10s: "
tail -n 50 /tmp/ripple.stdout.log

