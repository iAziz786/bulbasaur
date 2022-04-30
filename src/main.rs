use std::env;
use std::process;

mod app;
mod cli_config;
use cli_config::CliConfig;

fn main() {
    let args: Vec<String> = env::args().collect();

    let config = CliConfig::new(&args).unwrap_or_else(|err| {
        println!("{}", err);
        process::exit(1)
    });

    if let Err(err) = app::run(config) {
        println!("error while running the application: {}", err);
        process::exit(1)
    }
}
