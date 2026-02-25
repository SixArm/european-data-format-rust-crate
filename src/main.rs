//! EDF/EDF+ command-line converter.
//!
//! Converts between EDF binary, JSON, and XML formats. The input and
//! output formats are detected from the file extension.
//!
//! # Usage
//!
//! ```text
//! edf --input recording.edf --output recording.json
//! edf --input recording.edf --output recording.xml
//! edf --input recording.json --output recording.edf
//! edf --input recording.xml --output recording.edf
//! ```

use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::PathBuf;

use clap::Parser;

use european_data_format::{io_edf, io_json, io_xml, EdfError, EdfFile};

/// European Data Format (EDF/EDF+) converter.
///
/// Reads an EDF, JSON, or XML file and writes it in another format.
/// The format is detected from the file extension (.edf, .json, .xml).
#[derive(Parser, Debug)]
#[command(
    name = "edf",
    version,
    about = "European Data Format (EDF/EDF+) converter",
    long_about = "Converts between EDF/EDF+ binary files and JSON/XML representations.\n\n\
                  Supported conversions:\n  \
                  .edf -> .json    Convert EDF binary to JSON\n  \
                  .edf -> .xml     Convert EDF binary to XML\n  \
                  .json -> .edf    Convert JSON to EDF binary\n  \
                  .xml -> .edf     Convert XML to EDF binary\n  \
                  .json -> .xml    Convert JSON to XML\n  \
                  .xml -> .json    Convert XML to JSON"
)]
struct Cli {
    /// Input file path (.edf, .xml, or .json).
    ///
    /// The format is detected from the file extension.
    #[arg(short, long)]
    input: PathBuf,

    /// Output file path (.edf, .xml, or .json).
    ///
    /// The format is detected from the file extension.
    #[arg(short, long)]
    output: PathBuf,
}

/// Detected file format based on extension.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Format {
    Edf,
    Json,
    Xml,
}

/// Detect the file format from a path's extension.
fn detect_format(path: &PathBuf) -> Result<Format, EdfError> {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .as_deref()
    {
        Some("edf") => Ok(Format::Edf),
        Some("json") => Ok(Format::Json),
        Some("xml") => Ok(Format::Xml),
        _ => Err(EdfError::Parse {
            message: format!(
                "unsupported file extension for '{}' (expected .edf, .json, or .xml)",
                path.display()
            ),
        }),
    }
}

/// Read an EdfFile from the given path, detecting format from extension.
fn read_input(path: &PathBuf) -> Result<EdfFile, EdfError> {
    let format = detect_format(path)?;
    match format {
        Format::Edf => {
            let file = File::open(path)?;
            let mut reader = BufReader::new(file);
            io_edf::read_edf(&mut reader)
        }
        Format::Json => {
            let mut contents = String::new();
            File::open(path)?.read_to_string(&mut contents)?;
            io_json::from_json(&contents)
        }
        Format::Xml => {
            let mut contents = String::new();
            File::open(path)?.read_to_string(&mut contents)?;
            io_xml::from_xml(&contents)
        }
    }
}

/// Write an EdfFile to the given path, detecting format from extension.
fn write_output(edf: &EdfFile, path: &PathBuf) -> Result<(), EdfError> {
    let format = detect_format(path)?;
    match format {
        Format::Edf => {
            let file = File::create(path)?;
            let mut writer = BufWriter::new(file);
            io_edf::write_edf(edf, &mut writer)?;
            writer.flush()?;
        }
        Format::Json => {
            let json = io_json::to_json(edf)?;
            let mut file = File::create(path)?;
            file.write_all(json.as_bytes())?;
            file.write_all(b"\n")?;
        }
        Format::Xml => {
            let xml = io_xml::to_xml(edf)?;
            let mut file = File::create(path)?;
            file.write_all(xml.as_bytes())?;
            file.write_all(b"\n")?;
        }
    }
    Ok(())
}

fn main() {
    let cli = Cli::parse();

    match run(&cli) {
        Ok(()) => {
            eprintln!(
                "Converted {} -> {}",
                cli.input.display(),
                cli.output.display()
            );
        }
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }
}

fn run(cli: &Cli) -> Result<(), EdfError> {
    let edf = read_input(&cli.input)?;
    write_output(&edf, &cli.output)?;
    Ok(())
}
