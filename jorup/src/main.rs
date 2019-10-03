#[macro_use(error_chain, quick_main)]
extern crate error_chain;
#[macro_use(crate_name, crate_version, crate_authors, crate_description, value_t)]
extern crate clap;
#[macro_use(lazy_static)]
extern crate lazy_static;

mod common;

use clap::App;

quick_main!(run_main);

error_chain! {
    links {
        Common(common::Error, common::ErrorKind);
    }
}

fn run_main() -> Result<()> {
    let mut app = App::new(crate_name!())
        .version(crate_version!())
        .about(crate_description!())
        .author(crate_authors!("\n"))
        .arg(common::arg::jorup_home()?)
        .arg(common::arg::generate_autocompletion());

    let matches = app.clone().get_matches();

    if let Some(shell) = matches.value_of(common::arg::name::GENERATE_AUTOCOMPLETION) {
        // safe to unwrap as possible values have been validated first
        let shell = shell.parse().unwrap();

        app.gen_completions_to(crate_name!(), shell, &mut std::io::stdout());
        return Ok(());
    }

    let _cfg = common::JorupConfig::new(&matches)?;

    Ok(())
}
