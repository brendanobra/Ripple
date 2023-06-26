#!/bin/bash
#set -e
touch ${COMMENT_FILE}
echo "executing PR job for Dpab Appsanity"

TASKS_DIR="codebase/ci/scripts"
source "${TASKS_DIR}/utils.sh"
echo "setup_ssh id_rsa ${REPO_PRIVATE_KEY}"
setup_ssh id_rsa "${REPO_PRIVATE_KEY}"
echo "git_config"
git_config
export FAIL_ON_DECREASED_UNIT_COVERAGE=${FAIL_ON_DECREASED_UNIT_COVERAGE:-true}
git clone $DPAB_CORE_REPO_URI
cd codebase

git submodule update --init --recursive


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

cd ..
cd codebase
check_format
check_build
run_unit_tests
check_unit_test_coverage_decreases