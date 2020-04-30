mod commands;
mod common;
mod config;
mod utils;

use structopt::StructOpt;

fn main() {
    use std::error::Error;

    let app = commands::RootCmd::from_args();

    if let Err(error) = app.run() {
        eprintln!("{}", error);
        let mut source = error.source();
        while let Some(err) = source {
            eprintln!(" |-> {}", err);
            source = err.source();
        }

        // TODO: https://github.com/rust-lang/rust/issues/43301
        //
        // as soon as #43301 is stabilized it would be nice to no use
        // `exit` but the more appropriate:
        // https://doc.rust-lang.org/stable/std/process/trait.Termination.html
        std::process::exit(1);
    }
}
