use anyhow::Result;
use argh::FromArgs;

mod converter;

/// Device Faker configuration tool
#[derive(FromArgs)]
struct Cli {
    #[argh(subcommand)]
    command: Command,
}

#[derive(FromArgs)]
#[argh(subcommand)]
enum Command {
    /// Convert Magisk module ZIP to TOML configuration
    Convert(ConvertArgs),
}

/// Convert Magisk module ZIP to TOML configuration
#[derive(FromArgs)]
#[argh(subcommand, name = "convert")]
struct ConvertArgs {
    /// input ZIP file path
    #[argh(option, short = 'i', long = "input")]
    input: String,

    /// output file path
    #[argh(option, short = 'o', long = "output")]
    output: String,
}

fn main() -> Result<()> {
    let cli: Cli = argh::from_env();

    match cli.command {
        Command::Convert(args) => {
            converter::convert_zip_config(&args.input, &args.output)?;
        }
    }

    Ok(())
}
