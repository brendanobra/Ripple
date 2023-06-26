#!/bin/bash
#set -e
echo "Executing get-coverage-from-main job for dpab appsanity"
TASKS_DIR="codebase/ci/scripts"
source "${TASKS_DIR}/utils.sh"
echo "setup_ssh id_rsa ${REPO_PRIVATE_KEY}"
setup_ssh id_rsa "${REPO_PRIVATE_KEY}"
echo "git_config"
git_config
git clone $DPAB_CORE_REPO_URI
cd codebase
current_branch=$(get_current_branch)
echo "dpab appsanity current branch is ${current_branch}"

cd ../dpab_core
if is_in_remote $current_branch; then
    git checkout $current_branch
    git config pull.rebase false 
    git pull origin $current_branch
else 
    git checkout main
    git pull origin main
fi
get_coverage_from_main
cd ..
