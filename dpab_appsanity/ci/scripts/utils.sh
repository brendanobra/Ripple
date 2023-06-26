# The coverage.txt file from the dpab_core code-coverage branch stores a value of the current code coverage
# which is measured and updated every time when main branch changes are detected.
UNIT_TEST_COVERAGE_FILENAME="coverage.txt"
CODEBASE="${CODEBASE:-$PWD}"
COVERAGE="${COVERAGE:-../code-coverage}"
COMMENT_FILE="${COMMENT_FILE}"
function git_config() {
  if [ -n "${GIT_USER_EMAIL}" ]; then
    git config --global user.email "${GIT_USER_EMAIL}"
  fi

  if [ -n "${GIT_USER_NAME}" ]; then
    git config --global user.name "${GIT_USER_NAME}"
  fi
}

function check_format() {
    echo "checking format"
    rustup component add rustfmt
    cargo fmt -- --check
    fmt_result=$?
    if [[ ${fmt_result} -ne 0 ]] ; then
    echo ":rotating_light: Build failed due to Format errors" >> "${COMMENT_FILE}"
    echo "Try running **cargo fmt --all**" >> "${COMMENT_FILE}"
        exit 1
    else
    echo ":white_check_mark: Formatting passed" >> "${COMMENT_FILE}"
    fi
    echo "format check complete"
}

function check_build() {
    echo "checking build"
    cargo build
    if [ $? -eq 0 ]
      then
      echo ":white_check_mark: Build Compilation passed for ${flavor} feature" >> "${COMMENT_FILE}"
      else
      echo ":rotating_light: Build failed due to Compilation errors." >> "${COMMENT_FILE}"
      echo ":rotating_light: Try running **./cargo build** in the local dpab_appsanity folder to learn more." >> "${COMMENT_FILE}"
        exit 1
      fi
    
    # for ripple releases its mandatory for all tests across submodules pass
    if [[ "$1" == "-r" ]]; then
        git submodule foreach --recursive cargo  build --features "appsanity"
    fi
}

function run_unit_tests() {
    echo "running unit test"
    cargo test
    if [ $? -eq 0 ]
      then
        echo ":white_check_mark: Unit Tests passed" >> "${COMMENT_FILE}"
      else
      echo ":rotating_light: Build failed due to Unit test errors." >> "${COMMENT_FILE}"
      echo "Try running **cargo test** locally to learn more." >> "${COMMENT_FILE}"
        exit 1
      fi
    
    # for ripple releases its mandatory for all tests across submodules pass
    if [[ "$1" == "-r" ]]; then
        git submodule foreach --recursive cargo test
    fi
}

function get_current_unit_test_coverage() 
{
  echo "Checking current unit test coverage"
  grcov . -s src --binary-path ./target/debug/ -t lcov --branch --ignore-not-existing --llvm --ignore "*cargo*" --ignore "*dpab*" --ignore "*thunder_rs*" --ignore "*dpab*" --ignore "*target*" -o ./target/debug/lcov.info
  new_unit_test_coverage=$(genhtml -o report/ --show-details --highlight --ignore-errors source --legend ./target/debug/lcov.info | grep lines |  cut -d':' -f2 | cut -d% -f1 | xargs)
  echo "${new_unit_test_coverage}"
}

function check_unit_test_coverage_decreases() 
{
  echo "Checking if the unit test coverage has decreased"
  new_unit_test_coverage=$(get_current_unit_test_coverage)
  echo "The new test coverage: " $new_unit_test_coverage 
  #default to current for local cases, should only check against coverage branch in ci
  curr_unit_test_coverage=$new_unit_test_coverage
  # Read current coverage value from code-coverage branch
  cd ${COVERAGE}
  if [ ! -f "$UNIT_TEST_COVERAGE_FILENAME" ]; then
      echo "Error! The $UNIT_TEST_COVERAGE_FILENAME file does not exist."
      exit 1;
  fi
  curr_unit_test_coverage=$(cat "${UNIT_TEST_COVERAGE_FILENAME}")
  echo "The current test coverage: " $curr_unit_test_coverage
  # Go back to the dpab_core folder
  cd ${CODEBASE}

  # Compare the new coverage value with the current one
  if (( $(echo "$new_unit_test_coverage < $curr_unit_test_coverage" | bc -l) )); then
    echo "The unit test coverage has decreased from " $curr_unit_test_coverage "% to " $new_unit_test_coverage "%"
    echo ":rotating_light: The unit test coverage has decreased from " $curr_unit_test_coverage "% to " $new_unit_test_coverage "%">> "${COMMENT_FILE}"
    if [ "$FAIL_ON_DECREASED_UNIT_COVERAGE" == "true" ]; then
      echo ":rotating_light: FAIL_ON_DECREASED_UNIT_COVERAGE is true, failing due to unit coverage decreasing" >> "${COMMENT_FILE}"
      exit 1;
    else 
      echo ":white_check_mark: FAIL_ON_DECREASED_UNIT_COVERAGE is false, NOT failing due to unit coverage decreasing" >> "${COMMENT_FILE}"
    fi
  else
    echo ":white_check_mark: Test coverage level passed! Current coverage: ${curr_unit_test_coverage}%, new coverage: ${new_unit_test_coverage}%" >> "${COMMENT_FILE}"
  fi
}

function get_coverage_from_main() 
{
  echo "get_coverage_from_main() - Run test coverage on main branch and save the value into ${UNIT_TEST_COVERAGE_FILENAME}"
  # Run build and test
  cargo test
  # Run the grcov tool
  grcov . -s src --binary-path ./target/debug/ -t lcov --branch --ignore-not-existing --llvm --ignore "*cargo*" --ignore "*dpab*" --ignore "*thunder_rs*" --ignore "*dpab*" --ignore "*target*" -o ./target/debug/lcov.info
  # Run the genhtml tool
  new_unit_test_coverage=$(genhtml -o report/ --show-details --highlight --ignore-errors source --legend ./target/debug/lcov.info | grep lines |  cut -d':' -f2 | cut -d% -f1 | xargs)
  echo "The new test coverage: " $new_unit_test_coverage
  echo "ls -s"
  ls -s
  echo "ls .. -s"
  ls .. -s
  cd ../code-coverage
  git checkout code-coverage 
  git pull
  echo "ls -s"
  ls -s
  cat coverage.txt
  
  if [ ! -f "coverage.txt" ]; then
    echo "Error! The coverage.txt file does not exist."
    exit 1;
  fi
  
  # Read current coverage value from file
  curr_unit_test_coverage=$(cat "coverage.txt")
  echo "The current unit test coverage value: " $curr_unit_test_coverage "%"
  echo "New unit test coverage value: " $new_unit_test_coverage "%"

  # Compare the new coverage value with the current one
  # Check if we have improvements in test coverage and save the new value if so.
  if (($(echo "$new_unit_test_coverage == $curr_unit_test_coverage" | bc -l))); then
  # The value has not changed, do nothing
    echo "Note! The new unit test coverage (" $new_unit_test_coverage "% ) is equal to the current one (" $curr_unit_test_coverage "% )" 
    echo "The ${UNIT_TEST_COVERAGE_FILENAME} file won't be modified"
  else
  # Write new coverage value to a file
    echo $new_unit_test_coverage > "${UNIT_TEST_COVERAGE_FILENAME}"
    git add "${UNIT_TEST_COVERAGE_FILENAME}"
    git commit -m "The code coverage value is set to $new_unit_test_coverage%"
    ls -l
    git status
    echo "The new $new_unit_test_coverage value is saved into the ${UNIT_TEST_COVERAGE_FILENAME} file"
  fi
}

function get_current_branch () {
  cat .git/resource/head_name
}

function is_in_remote() {
    branch=${1}
    remote_branch_exists=$(git ls-remote --heads origin ${branch})

    if [[ -z ${remote_branch_exists} ]]; then
        echo 0
    else
        echo 1
    fi
}

function detect_change() {
    base_branch=$1
    current_branch=$(get_current_branch)

    echo "${base_branch} ${current_branch}"
    log_changes=$(git log $base_branch..$current_branch --oneline)

    if grep -E "feat|fix|perf|refactor|revert" <<< "$log_changes"; then
      true
    else 
      false
    fi
}

function get_deploy_path () {
  release_type=$1
  version=$2
  suffix=$3
  case ${release_type} in 
    "pr") 
      "dev/pr/${suffix}/"
	  ;;
    "staging") 
      "staging/${PACKAGE_VERSION_TARGET}/"
	  ;;
    *)
      "${PACKAGE_VERSION_TARGET}/"
    ;;

  esac
}

function get_target_name () {
  release_type=$1
  version=$2
  suffix=$3
  case ${release_type} in 
    "pr") 
      "${device-type}-${PACKAGE_VERSION_TARGET}-${suffix}.tgz"
	  ;;
    "staging") 
      "${device-type}-${PACKAGE_VERSION_TARGET}-${suffix}.tgz"
	  ;;
    *)
      "${device-type}-${PACKAGE_VERSION_TARGET}.tgz"
    ;;

  esac
}

function deploy_to_artifactory () {
  bundle_path=$1
  device_type=$2
  type=$3
  suffix=$4
  curl -u ${ARTIFACTORY_SERVICE_ACCOUNT_ID}:${ARTIFACTORY_SERVICE_ACCOUNT_SECRET} \
    -X PUT "https://partners.artifactory.comcast.com/artifactory/ottx_generic/releases/ripple/${PACKAGE_VERSION_TARGET}/ripple-${device_type}-${PACKAGE_VERSION_TARGET}.tgz" \
    -T bundle_path
}


function check_sub_modules () {
  base_branch=$1
  submodule_status=$(git submodule status --recursive)
  if grep -E "dpab\s\(heads\/${base_branch}" <<< "$submodule_status"; then
    echo "DPAB library is on ${1} branch"
    if grep -E "dpab_core\s\(heads\/${base_branch}" <<< "$submodule_status"; then
      echo "Dpab Core is on ${1} branch"
      if grep -E "dpab_appsanity\s\(heads\/${base_branch}" <<< "$submodule_status"; then
        echo "Dpab appsanity is on ${1} branch"
      else 
         echo "Dpab appsanity submodule is not on ${1} branch"
        exit 1
      fi
    else
      echo "Dpab Core submodule is not on ${1} branch"
      exit 1
    fi
  else
    echo "Dpab submodule is not on ${1} branch"
    exit 1
  fi


}

function git_config() {
  if [ -n "${GIT_USER_EMAIL}" ]; then
    git config --global user.email "${GIT_USER_EMAIL}"
  fi

  if [ -n "${GIT_USER_NAME}" ]; then
    git config --global user.name "${GIT_USER_NAME}"
  fi
}

function setup_ssh() {
  key_filename=$1
  private_key=$2
  public_key=$3

  if [[ -z ${private_key} ]]; then
    return
  fi

  mkdir -p "${HOME}/.ssh"

  echo "StrictHostKeyChecking no" > "${HOME}/.ssh/config"
  echo "ForwardAgent yes" >> "${HOME}/.ssh/config"
  chmod og-rw "${HOME}/.ssh/config"

  if [ -n "${public_key}" ]; then
      echo "${public_key}" > "${HOME}/.ssh/${key_filename}.pub"
      chmod og-rw "${HOME}/.ssh/${key_filename}".pub
  fi

  echo "${private_key}" > "${HOME}/.ssh/${key_filename}"
  chmod og-rw "${HOME}/.ssh/${key_filename}"

  # shellcheck disable=SC2046
  eval $(ssh-agent)
  ssh-add "${HOME}/.ssh/${key_filename}"
}
