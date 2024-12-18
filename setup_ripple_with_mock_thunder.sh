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
HAS_SUDO=`which sudo`

echo "*****      Welcome to Eos-Ripple script to setup mock thunder       *****"
echo ""
echo "Note: Always run this in the eos-ripple folder"
echo "Current working directory: ${workspace_dir}"

echo ""
echo "Before we proceed lets setup the manifest"

device_type="mock"
echo "Device Type selected ${device_type}"

echo "Configuring ripple to use the mock device extension"

is_mock=true
device_type=puck
partner_type=cert
HERE=`pwd`
CONFIG_ROOT="/etc/"
RIPPLE_CONFIG_DIR="/etc/ripple"
MANIFESTS_PATH="${CONFIG_ROOT}"
RULES_PATH="${RIPPLE_CONFIG_DIR}/rules"
OPEN_RPC_PATH="${TARGET_DIR}/openrpc"
OPEN_RPC_ETC_PATH="${RIPPLE_CONFIG_DIR}/openrpc"

source "${HERE}/utils.sh"
backup_ripple_configs
prepare_ripple_etc

echo "Copying to target directory"
do_sudo 'ln -s ${HERE}/firebolt-devices/"$partner_type"/"$device_type"/app-library.json "${MANIFESTS_PATH}/firebolt-app-library.json"'

echo "Copying mock manifests, rules and open-rpc to target directory"
do_sudo 'ln -s  ${HERE}/mock/manifest.json "${MANIFESTS_PATH}/firebolt-device-manifest.json"'
do_sudo 'ln -s ${HERE}/mock/extn.json "${MANIFESTS_PATH}/firebolt-extn-manifest.json"'
do_sudo 'ln -s  ${HERE}/mock/mock-thunder-device.json "${MANIFESTS_PATH}/mock-thunder-device.json"'


do_sudo 'cp mock/*rpc*.json "${OPEN_RPC_ETC_PATH}/"'


do_sudo 'cp mock/rules/* "${RULES_PATH}/"'

do_sudo 'ln -s ${HERE}/firebolt-devices/openrpc/extns/firebolt-players-open-rpc.json "${OPEN_RPC_ETC_PATH}/"'
setup_ripple_extensions "${MANIFESTS_PATH}/firebolt-extn-manifest.json"

