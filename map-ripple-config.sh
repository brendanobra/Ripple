#!/bin/sh
# form is platform/form factor, i."cert/puck"
set -e
print_usage() {
  echo "Missing at least one arg"
  echo "Usage: $0 <plaform> <formfactor> "
  echo "example: ${0} cert puck "

}
ERROR_MSG=`print_usage`
#"Missing at least one arg: \n Usage: $0 please pass platform as 1st arg, and form factor as 2nd arg. For example: ${0} cert puck ~/eos-ripple  ."
: ${1?"$ERROR_MSG"}
: ${2?"${ERROR_MSG}"}


HERE=`pwd`
PLATFORM="${1}"
FORM_FACTOR="${2}"
RIPPLE_DIR="${HERE}"
FIREBOLT_OPENRPC="${3:-1.3.0}"
FIREBOLT_DEVICES_DIR="${HERE}/firebolt-devices"
LIB_EXTENSION="so"


if [ ! -d "${RIPPLE_DIR}" ]; then
  echo "The ripple dir: ${RIPPLE_DIR} does not exist, cannot continue. buh bye."
  exit 1;
fi

if [ ! -d "${FIREBOLT_DEVICES_DIR}" ]; then
  echo "The firebolt-devices dir: ${FIREBOLT_DEVICES_DIR} does not exist, cannot continue. buh bye."
  exit 1;
fi

echo "setting up with manifests in ${PLATFORM}/${FORM_FACTOR}"
HERE=`pwd`
FROM="${FIREBOLT_DEVICES_DIR}/${PLATFORM}/${FORM_FACTOR}"
RULES="${FIREBOLT_DEVICES_DIR}/rules"

backup_name="$(date +"%F-%H-%M")"

if [ -f "/etc/firebolt-device-manifest.json" ]; 
then
  if [ -L "/etc/firebolt-device-manifest.json" ];
  then 
    echo "/etc/firebolt-device-manifest.json is a symlink , deleting"
    sudo rm -f /etc/firebolt-device-manifest.json
  else
    echo "backup up /etc/firebolt-device-manifest.json to /etc/firebolt-device-manifest.json.${backup_name}"
    sudo mv /etc/firebolt-device-manifest.json "/etc/firebolt-device-manifest.json.${backup_name}"
  fi
   
fi;
if [ -f "/etc/firebolt-app-library.json" ]; 
then
  if [ -L "/etc/firebolt-app-library.json" ];
  then 
    echo "/etc/firebolt-app-library.json is a symlink , deleting"
    sudo rm -f /etc/firebolt-app-library.json
  else
    echo "backup up /etc/firebolt-app-library.json to /etc/firebolt-app-library.json.${backup_name}"
    sudo mv /etc/firebolt-app-library.json "/etc/firebolt-app-library.json.${backup_name}"
  fi
fi;

if [ -f "/etc/firebolt-extn-manifest.json" ]; 
then
  if [ -L "/etc/firebolt-extn-manifest.json" ];
  then 
    echo "/etc/firebolt-extn-manifest.json is a symlink , deleting"
    sudo rm -f /etc/firebolt-extn-manifest.json
  else
    echo "backup up /etc/firebolt-extn-manifest.json to /etc/firebolt-extn-manifest.json.${backup_name}"
    sudo mv /etc/firebolt-extn-manifest.json "/etc/firebolt-extn-manifest.json.${backup_name}"
  fi
fi;
sudo mkdir -p /etc/ripple

sudo rm  -f /etc/firebolt-device-manifest.json
sudo rm  -f /etc/firebolt-app-library.json
sudo rm  -f /etc/firebolt-extn-manifest.json
sudo rm  -Rf /etc/ripple/openrpc
sudo rm -Rf /etc/ripple/rules
echo "creating symlinks"
sudo ln -s "${FROM}/manifest.json" /etc/firebolt-device-manifest.json
sudo ln -s "${FROM}/app-library.json" /etc/firebolt-app-library.json
sudo ln -s "${FROM}/extn.json" /etc/firebolt-extn-manifest.json
sudo ln -s "${FIREBOLT_DEVICES_DIR}/openrpc/firebolt/${FIREBOLT_OPENRPC}" /etc/ripple/openrpc 

sudo ln -s "${FIREBOLT_DEVICES_DIR}/rules" /etc/ripple/rules
echo "symlinks created, figuring out socat parameters"
WS_GATEWAY=$(cat /etc/firebolt-device-manifest.json | jq .configuration.ws_configuration.gateway)
if [ "$(echo "$WS_GATEWAY" | grep 127.0.0.1 | wc -l)" -eq "1" ]; then
  echo ".configuration.ws_configuration.gateway references loopback, will generate proxy"
  echo "socat tcp-listen:3473,reuseaddr,fork tcp:"127.0.0.1":3473  &" > ./socat.sh
fi
INTERNAL_WS_GATEWAY=$(cat /etc/firebolt-device-manifest.json | jq .configuration.internal_ws_configuration.gateway)
if [ "$(echo "$INTERNAL_WS_GATEWAY" | grep 127.0.0.1 | wc -l)" -eq "1" ]; then
  echo ".configuration.internal_ws_configuration.gateway references loopback, will generate proxy"
  echo "socat tcp-listen:3474,reuseaddr,fork tcp:"127.0.0.1":3474  &" > ./socat.sh
fi
IFS='
'
EXTENSIONS=$(cat /etc/firebolt-extn-manifest.json | jq  -r .extns[].path )
LIB_DIR=$(cat /etc/firebolt-extn-manifest.json | jq -r .default_path)
echo "libraries will be symlinked in ${LIB_DIR}"
sudo mkdir -p "${LIB_DIR}"
ls -la "${LIB_DIR}"
sudo rm -rf "${LIB_DIR}/*"
ls -la "${LIB_DIR}"
cat /etc/firebolt-extn-manifest.json | jq  -r .extns[].path| while read extension ; do
  extension_path=$(find "$RIPPLE_DIR/target/debug" -name "$extension"."${LIB_EXTENSION}" -print -quit)
  echo "linking ${extension_path}"
  sudo rm -f "${LIB_DIR}/${extension}.${LIB_EXTENSION}"
  sudo ln -s "${extension_path}" "${LIB_DIR}/${extension}.${LIB_EXTENSION}"

done


echo "to port forward ripple (running locally) to the device (without changing device manifests on this machine), run the following command ./ripple-eos-test-utils/sbin/port-forward-to-device.sh "
