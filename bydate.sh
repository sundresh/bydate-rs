# Source this file
rust_bydate_dir=$(dirname "$(realpath "$0")")

function td() {
    cd "$(${rust_bydate_dir}/target/release/bydate today)"
}

function yd() {
    cd "$(${rust_bydate_dir}/target/release/bydate yesterday)"
}
