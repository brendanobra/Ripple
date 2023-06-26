#!/bin/bash
#set -e
apt-get update
apt-get -y install cmake

echo "Executing get-coverage-from-main job for dpab_core"
TASKS_DIR="codebase/ci/scripts"
source "${TASKS_DIR}/utils.sh"
echo "setup_ssh id_rsa ${REPO_PRIVATE_KEY}"
setup_ssh id_rsa "${REPO_PRIVATE_KEY}"
echo "git_config"
git_config
cd codebase
git submodule update --init --recursive
get_coverage_from_main
cd ..
