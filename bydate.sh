# Source this file

function td() {
    cd "$(target/release/bydate today)"
}

function yd() {
    cd "$(target/release/bydate yesterday)"
}
