#!/bin/bash
#set -e
touch ${COMMENT_FILE}
echo "executing PR job for Dpab Core"
TASKS_DIR="${CODEBASE}/ci/scripts"
source "${TASKS_DIR}/utils.sh"
setup_ssh id_rsa "${REPO_PRIVATE_KEY}"
git_config
export FAIL_ON_DECREASED_UNIT_COVERAGE=${FAIL_ON_DECREASED_UNIT_COVERAGE:-true}
cd codebase
git submodule update --init --recursive
check_format
check_build
run_unit_tests
check_unit_test_coverage_decreases
