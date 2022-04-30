pub struct CliConfig {
    pub filename: String,
}

#[cfg(not(target_os = "windows"))]
const USAGE: &str = "
Usage:
    bulbasaur csv_filename.csv
";

#[cfg(target_os = "windows")]
const USAGE: &str = "
Usage:
    bulbasaur.exe csv_filename.csv
";

impl CliConfig {
    pub fn new(args: &[String]) -> Result<Self, &'static str> {
        if args.len() < 2 {
            return Err(USAGE);
        }

        let filename = args[1].clone();

        Ok(CliConfig { filename })
    }
}
