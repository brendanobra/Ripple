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

cargo build --features local_dev || exit
echo "Cleaning up manifest folder in target directory"
mkdir -p target/manifests
rm -rf ./target/manifests/firebolt-extn-manifest.json
rm -rf ./target/manifests/firebolt-device-manifest.json
rm -rf ./target/manifests/firebolt-app-library.json
echo "Copying to target directory"
cp firebolt-devices/"$partner_type"/"$device_type"/app-library.json target/manifests/firebolt-app-library.json

echo "Copying mock manifests"
cp mock/manifest.json target/manifests/firebolt-device-manifest.json
cp mock/extn.json target/manifests/firebolt-extn-manifest.json
cp mock/mock-thunder-device.json target/manifests/mock-thunder-device.json

## Update firebolt-extn-manifest.json
default_extension=$(get_default_extension)
extnManifestJson=$(<target/manifests/firebolt-extn-manifest.json)
new_extnJson=$(echo "$extnManifestJson" | jq '.default_path = "'$workspace_dir'/target/debug/" |
    .default_extension = "'$default_extension'" |
    .extns[].symbols[] |= if has("config") and (.config | type == "object" and has("mock_data_file")) then .config.mock_data_file |= "'$workspace_dir'/target/manifests/mock-thunder-device.json" else . end')
echo "$new_extnJson" > target/manifests/firebolt-extn-manifest.json


## Update firebolt-device-manifest.json
deviceManifestJson=$(<target/manifests/firebolt-device-manifest.json)
new_deviceJson=$(echo "$deviceManifestJson" | jq '.applications.distribution.library = "'$workspace_dir'/target/manifests/firebolt-app-library.json"')
echo "$new_deviceJson" > target/manifests/firebolt-device-manifest.json

export EXTN_MANIFEST=${workspace_dir}/target/manifests/firebolt-extn-manifest.json
export DEVICE_MANIFEST=${workspace_dir}/target/manifests/firebolt-device-manifest.json

echo ""
echo "Environment variables for manifests set"
echo ""
echo "DEVICE_MANIFEST=${DEVICE_MANIFEST}"
echo "EXTN_MANIFEST=${EXTN_MANIFEST}"
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

