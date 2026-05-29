mod bydate;
mod utils;

use std::env;

use bydate::Bydate;

fn main() {
    Bydate::new().main(env::args());
}
