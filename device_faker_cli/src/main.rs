use anyhow::Result;
use argh::FromArgs;

mod converter;
mod template;

/// Device Faker configuration tool
#[derive(FromArgs)]
struct Cli {
    #[argh(subcommand)]
    command: Command,
}

#[derive(FromArgs)]
#[argh(subcommand)]
enum Command {
    /// Convert configuration formats
    Convert(ConvertArgs),
    /// Convert configuration from a ZIP archive containing system.prop
    ConvertZip(ConvertZipArgs),
    /// Import a template from a source
    Import(ImportArgs),
}

/// Convert configuration formats
#[derive(FromArgs)]
#[argh(subcommand, name = "convert")]
struct ConvertArgs {
    /// input file path
    #[argh(option, short = 'i', long = "input")]
    input: String,

    /// output file path
    #[argh(option, short = 'o', long = "output")]
    output: String,
}

/// Import a template from a source
#[derive(FromArgs)]
#[argh(subcommand, name = "import")]
struct ImportArgs {
    /// source of the template (e.g., URL or local path)
    #[argh(option, short = 's', long = "source")]
    source: String,

    /// output file path for the imported template
    #[argh(option, short = 'o', long = "output")]
    output: String,
}

/// Convert configuration from a ZIP archive that contains system.prop
#[derive(FromArgs)]
#[argh(subcommand, name = "convert-zip")]
struct ConvertZipArgs {
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
            converter::convert_config(&args.input, &args.output)?;
        }
        Command::ConvertZip(args) => {
            converter::convert_zip_config(&args.input, &args.output)?;
        }
        Command::Import(args) => {
            template::import_template(&args.source, &args.output)?;
        }
    }

    Ok(())
}
