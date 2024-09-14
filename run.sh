#!/bin/bash
shopt -s nocasematch
workspace_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)

echo "*****      Welcome to Eos-Ripple Run script        *****"
echo ""
echo "Note: Always run this in the eos-ripple folder"
echo "Current working directory: ${workspace_dir}"

echo ""
echo "Before we proceed lets setup the manifest"

if [ -z "$1" ]; then
    echo "Please enter the type of device you are on. Options: Panel, Puck, Mock"
    read -r device_type
else
    device_type=$1
fi
device_type=$(echo "$device_type" | tr '[:upper:]' '[:lower:]')

echo "Device Type selected ${device_type}"

if [ "$device_type" == "mock" ]; then
    echo "Initializing mock mode"
    is_mock=true
    device_type=puck
    partner_type=cert

else
    if [ -z "$2" ]; then
        echo "Please enter the type of Partner you are on. Options: Cert, Sky-UK, Xumo"
        read -r partner_type
    else
        partner_type=$2
    fi

    partner_type=$(echo "$partner_type" | tr '[:upper:]' '[:lower:]')

    echo "Partner Type selected ${partner_type}"

    if [ -z "$3" ]; then
        echo "Please enter the ip address of the device"
        read -r device_ip
    else
        device_ip=$3
    fi

    echo "Device ip entered ${device_ip}"
fi

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

cargo build --features local_dev || exit
echo "Cleaning up manifest folder in target directory"
mkdir -p target/manifests
mkdir -p target/debug/rules
mkdir -p target/openrpc
rm -rf target/openrpc/*
rm -rf ./target/manifests/firebolt-extn-manifest.json
echo "Copying to target directory"
cp firebolt-devices/"$partner_type"/"$device_type"/app-library.json target/manifests/firebolt-app-library.json
cp firebolt-devices/openrpc/* target/openrpc

if [ "$is_mock" ]; then
    echo "Copying mock manifests"
    cp mock/manifest.json target/manifests/firebolt-device-manifest.json
    cp mock/extn.json target/manifests/firebolt-extn-manifest.json
    cp mock/mock-thunder-device.json target/manifests/mock-thunder-device.json
    cp mock/rules/* target/debug/rules
    sed -i "" "s@\"mock_data_file\": \"mock-device.json\"@\"mock_data_file\": \"$workspace_dir/target/manifests/mock-thunder-device.json\"@" target/manifests/firebolt-extn-manifest.json
else
    cp firebolt-devices/rules/* target/debug/rules
    cp firebolt-devices/"$partner_type"/"$device_type"/manifest.json target/manifests/firebolt-device-manifest.json
    cp firebolt-devices/"$partner_type"/"$device_type"/extn.json target/manifests/firebolt-extn-manifest.json
fi

sed -i "" "s@\"default_path\": \"/usr/lib/rust/\"@\"default_path\": \"$workspace_dir/target/debug/\"@" target/manifests/firebolt-extn-manifest.json
default_extension=$(get_default_extension)
sed -i "" "s@\"default_extension\": \"so\"@\"default_extension\": \"$default_extension\"@" target/manifests/firebolt-extn-manifest.json

## Update firebolt-device-manifest.json
sed -i "" "s@\"library\": \"/etc/firebolt-app-library.json\"@\"library\": \"$workspace_dir/target/manifests/firebolt-app-library.json\"@" target/manifests/firebolt-device-manifest.json

rules="\/etc\/ripple\/rules"
rules_relative="${workspace_dir}/target/debug/rules"
rules_relative="${rules_relative//\//\\/}"
echo "${rules} ||| ${rules_relative}"
sed -i -e "s/${rules}/${rules_relative}/g" target/manifests/firebolt-extn-manifest.json


openrpc="\/etc\/ripple\/openrpc"
openrpc_relative="${workspace_dir}/target/openrpc"
openrpc_relative="${openrpc_relative//\//\\/}"
echo "${openrpc} ||| ${openrpc_relative}"
sed -i -e "s/${openrpc}/${openrpc_relative}/g" target/manifests/firebolt-extn-manifest.json

tail -10 target/manifests/firebolt-extn-manifest.json
export EXTN_MANIFEST=${workspace_dir}/target/manifests/firebolt-extn-manifest.json
export DEVICE_MANIFEST=${workspace_dir}/target/manifests/firebolt-device-manifest.json
export APP_LIBRARY=${workspace_dir}/target/manifests/firebolt-app-library.json
export FIREBOLT_OPEN_RPC=${workspace_dir}/target/openrpc/firebolt-open-rpc.json

echo ""
echo "Environment variables for manifests set"
echo ""
echo "DEVICE_MANIFEST=${DEVICE_MANIFEST}"
echo "EXTN_MANIFEST=${EXTN_MANIFEST}"
echo "APP_LIBRARY=${APP_LIBRARY}"

DEVICE_HOST=${device_ip} target/debug/ripple
