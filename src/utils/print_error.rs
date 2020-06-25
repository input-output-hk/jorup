use std::error::Error;

pub fn print_error(error: impl Error) {
    eprintln!("{}", error);
    let mut source = error.source();
    while let Some(err) = source {
        eprintln!(" |-> {}", err);
        source = err.source();
    }
}
