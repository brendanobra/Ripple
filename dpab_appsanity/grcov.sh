# Enable the required Rust flags to generate the coverage files
export CARGO_INCREMENTAL=0
export RUSTC_BOOTSTRAP=1
export RUSTFLAGS="-Zprofile -Ccodegen-units=1 -Copt-level=0 -Clink-dead-code -Coverflow-checks=off"
find target -name '*.gcda' -delete
cargo test

# generate the html report
grcov . -s src --binary-path ./target/debug/ -t lcov --branch --ignore-not-existing --llvm --ignore "*cargo*" --ignore "*target*" --ignore "*dpab_core*" -o ./target/debug/lcov.info
(cd src && genhtml -o ../target/coverageReport/ --show-details --highlight --ignore-errors source --legend ../target/debug/lcov.info | grep lines |  cut -d':' -f2 | cut -d% -f1 | xargs)
# open the report
open target/coverageReport/index.html