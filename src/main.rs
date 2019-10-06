use std::io::prelude::*;
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;

use log::{error, info};
use structopt::StructOpt;

pub mod conversion;

#[derive(Debug, StructOpt)]
#[structopt(name = "bluepaper", about = "Export Dropbox Paper documents to LaTeX.")]
struct Opt {
    // Turn off error messages.
    #[structopt(short, long)]
    quiet: bool,

    /// Show warnings.
    /// Use twice ("-vv") to also turn on informational progress messages.
    #[structopt(short, long, parse(from_occurrences))]
    verbose: usize,

    /// Input Markdown file.
    /// Leave out to read from STDIN.
    #[structopt(short, long)]
    input: Option<PathBuf>,

    /// Main output file for generated LaTeX code.
    /// If the file exists it will be overwritten. If the Markdown contains images
    /// then bluepages will place them in a directory "figures" next to the output
    /// file. Leave out to write to STDOUT.
    output: Option<PathBuf>,

    /// Overwrite figures if files with the same names exist.
    /// The default is to choose a new file name for each downloaded figure.
    #[structopt(short = "f", long)]
    overwrite_figures: bool,
}

fn run_cli(opt: Opt) -> Result<(), String> {
    let input = if let Some(path) = opt.input {
        info!("Reading Markdown from input file ...");
        std::fs::read_to_string(path).map_err(|e| format!("Error reading input file: {}", e))?
    } else {
        info!("Reading Markdown from STDIN ...");
        let mut input = BufReader::new(std::io::stdin());
        let mut input_string = String::new();
        input
            .read_to_string(&mut input_string)
            .map_err(|e| format!("Error reading from STDIN: {}", e))?;
        input_string
    };

    info!("Converting to LaTeX and printing to stdout ...");
    let output = std::io::stdout();
    let mut output = BufWriter::new(output);

    conversion::markdown_to_latex(&input, &mut output)
        .map_err(|e| format!("IO Error on terminal output: {}", e))
}

fn main() {
    let opt = Opt::from_args();

    stderrlog::new()
        .quiet(opt.quiet)
        .verbosity(opt.verbose)
        .timestamp(stderrlog::Timestamp::Off)
        .init()
        .unwrap();

    if let Err(message) = run_cli(opt) {
        error!("{}", message);
        std::process::exit(1);
    } else {
        info!("Done.");
    }
}
