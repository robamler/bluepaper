use bluepaper_core::MarkdownToLatex;

use confy;
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use serde_json::json;
use structopt::StructOpt;

use std::io::prelude::*;
use std::io::{BufReader, BufWriter};

#[derive(Debug, StructOpt)]
#[structopt(name = "bluepaper", about = "Export Dropbox Paper documents to LaTeX.")]
struct Opt {
    // Turn off progress messages.
    #[structopt(short, long)]
    quiet: bool,

    /// Show warnings.
    /// Use twice ("-vv") to also turn on informational progress messages.
    #[structopt(short, long, parse(from_occurrences))]
    verbose: usize,

    /// Read Markdown from a local file instead of downloading a Dropbox Paper document.
    /// Set to "-" to read from STDIN.
    #[structopt(short, long)]
    input: Option<String>,

    /// Specify custom output path for main LaTeX file.
    /// If not specified, a save output file name will be chosen based on the document
    /// title in such a way that no existing files are overwritten. This option
    /// overwrites the automatically chosen file name, and any existing file at the
    /// specified path will be overwritten.
    /// Set to "-" to write to stdout.
    #[structopt(short, long)]
    output: Option<String>,

    /// Overwrite figures if files with the same names exist.
    /// The default is to choose a new file name for each downloaded figure.
    #[structopt(short = "f", long)]
    overwrite_figures: bool,
}

#[derive(Serialize, Deserialize)]
struct Config {
    version: u32,
    api_key: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: 0,
            api_key: None,
        }
    }
}

fn run_cli(opt: Opt) -> Result<(), String> {
    let (markdown, meta_data) = if let Some(path) = opt.input {
        if path == "-" {
            info!("Reading Markdown from STDIN ...");
            let mut input = BufReader::new(std::io::stdin());
            let mut input_string = String::new();
            input
                .read_to_string(&mut input_string)
                .map_err(|e| format!("Error reading from STDIN: {}", e))?;
            (input_string, None)
        } else {
            info!("Reading Markdown from file \"{}\" ...", &path);
            (
                std::fs::read_to_string(path)
                    .map_err(|e| format!("Error reading input file: {}", e))?,
                None,
            )
        }
    } else {
        let (markdown, meta_data) = navigate_to_dropbox_paper()?;
        info!("Downloaded Markdown for Document \"{}\".", meta_data.title);
        (markdown, Some(meta_data))
    };

    // The dynamic dispatch shouldn't really hurt much here because we wrap `output`
    // in a BufWriter below, so writes to the inner `output` will be infrequent.
    let (mut latex_output, latex_path): (Box<dyn Write>, _) = if let Some(path) = opt.output {
        if path == "-" {
            info!("Printing LaTeX to STDOUT ...");
            (Box::new(std::io::stdout()), None)
        } else {
            info!("Writing LaTeX to file \"{}\" ...", path); // TODO: always show if in interactive mode
            let latex_output = Box::new(
                std::fs::File::create(&path)
                    .map_err(|e| format!("Cannot open LaTeX output file: {}", e))?,
            );
            (latex_output, Some(path))
        }
    } else {
        // Todo if "-i" is provided, set output file name according to that
        let title = meta_data
            .as_ref()
            .map(|m| &m.title[..])
            .unwrap_or_else(|| "Bluepaper");

        let title_beginning = title
            .split(|c: char| c.is_whitespace() || c == '-' || c == '_')
            .filter(|s| !s.is_empty())
            .take(2)
            .flat_map(|s| {
                s.chars()
                    .filter(|&c| c.is_alphanumeric())
                    .flat_map(char::to_lowercase)
                    .chain("_".chars())
            })
            .collect::<String>();

        let base_name = if title_beginning.len() <= 4 {
            "bluepaper"
        } else {
            &title_beginning[0..title_beginning.len() - 1] // Discard trailing "_"
        };

        let (file, file_name) = open_unique_file(base_name, ".tex")
            .map_err(|e| format!("Could not open output file: {:?}", e))?;
        info!("Writing LaTeX to file \"{}\" ...", file_name); // TODO: always show if in interactive mode

        (Box::new(file), Some(file_name))
    };

    if let Some(latex_path) = latex_path {
        let latex = MarkdownToLatex::from_string(markdown).into_string();
        latex_output
            .write_all(latex.as_bytes())
            .map_err(|e| format!("IO error when writing LaTeX file: {}", e))?;

        let basename = if latex_path.ends_with(".tex") {
            &latex_path[0..latex_path.len() - 4]
        } else {
            &latex_path
        };
        let pdf_path = format!("{}.pdf", basename);

        info!(
            "Compiling LaTeX code and generating PDF file \"{}\" ...",
            &pdf_path
        );
        let mut pdf_output = std::fs::File::create(pdf_path)
            .map_err(|e| format!("Cannot open PDF output file: {}", e))?;
        let pdf_data = tectonic::latex_to_pdf(latex).map_err(|e| format!("LaTeX error: {}", e))?;
        pdf_output
            .write_all(&pdf_data)
            .map_err(|e| format!("IO error when writing PDF file: {}", e))?;
    } else {
        MarkdownToLatex::from_string(markdown)
            .write_to(BufWriter::new(latex_output))
            .map_err(|e| format!("IO Error on terminal output: {}", e))?;
    }

    Ok(())
}

fn open_unique_file(
    base_name: &str,
    suffix: &str,
) -> Result<(std::fs::File, String), std::io::Error> {
    let mut open_options = std::fs::OpenOptions::new();
    open_options.write(true).create_new(true);

    let file_name = format!("{}.tex", base_name);
    match open_options.open(&file_name) {
        Ok(file) => Ok((file, file_name)),
        Err(ref e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            let mut i = 2;
            loop {
                let file_name = format!("{}{}{}", base_name, i, suffix);
                match open_options.open(&file_name) {
                    Ok(file) => return Ok((file, file_name)),
                    Err(ref e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                        i += 1;
                    }
                    Err(e) => return Err(e),
                };
            }
        }
        Err(e) => Err(e),
    }
}

fn navigate_to_dropbox_paper() -> Result<(String, PaperMetaData), String> {
    let mut cfg: Config = confy::load("bluepaper")
        .map_err(|e| format!("Unable to load configuration file: {}", e))?;

    if let Some(api_key) = cfg.api_key {
        select_dropbox_paper(&api_key)
    } else {
        println!("No Paper link provided and no Dropbox access token found.");
        println!("Please navigate to the following web site to authorize this app:");
        println!();
        println!(
            "https://www.dropbox.com/oauth2/authorize?client_id=djln19deuietsl5\
             &response_type=token&redirect_uri=http%3A%2F%2Flocalhost%3A8080%2Foauth2redirect"
        );
        println!();
        println!("Then, please paste the displayed access token below:");

        let mut buf = String::new();
        std::io::stdin()
            .read_line(&mut buf)
            .map_err(|e| format!("Could not interpret input: {}", e))?;
        let user_response = buf.trim().to_string();

        cfg.api_key = Some(user_response.to_string());
        confy::store("bluepaper", cfg)
            .map_err(|e| format!("Unable to write to configuration file: {}", e))?;

        select_dropbox_paper(&user_response)
    }
}

#[derive(Deserialize, Debug)]
struct PaperList {
    doc_ids: Vec<String>,
    // There are more fields, which we currently ignore:
    // - cursor: { value: String, expiration: String }
    // - has_more: bool
}

#[derive(Serialize)]
struct DownloadRequest<'a, 'b> {
    doc_id: &'a str,
    export_format: &'b str,
}

#[derive(Deserialize, Debug)]
struct PaperMetaData {
    owner: String,
    title: String,
    revision: usize,
    mime_type: String,
}

fn select_dropbox_paper(api_key: &str) -> Result<(String, PaperMetaData), String> {
    println!("Getting list of most recently accesed Paper documents ...");
    debug!("Access token: \"{}\"", api_key);

    let client = reqwest::blocking::Client::new();

    let paper_list: PaperList = client
        .post("https://api.dropboxapi.com/2/paper/docs/list")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .body("{\"filter_by\": \"docs_created\", \"sort_by\": \"accessed\", \"sort_order\": \"descending\", \"limit\": 5}")
        .send()
        .unwrap()
        .json()
        .unwrap();

    debug!("Paper list: {:?}", paper_list);

    let mut recent_papers =
        Vec::<Option<(String, PaperMetaData)>>::with_capacity(paper_list.doc_ids.len());

    for (i, doc_id) in paper_list.doc_ids.iter().enumerate() {
        let (markdown, meta_data) = download_dropbox_paper(&client, api_key, doc_id);
        println!("{:2}: {}", i + 1, meta_data.title);
        recent_papers.push(Some((markdown, meta_data)));
    }

    println!();

    if paper_list.doc_ids.is_empty() {
        println!(
            "Dropbox returned an empty list of documents created from your account. You can\n\
             export a Dropbox Paper document to LaTeX by pasting a link to the document below:"
        );
    } else {
        println!(
            "Please select the document (1-{}) that you would like to export to LaTeX, or\n\
             paste the link to a different Dropbox Paper document:",
            paper_list.doc_ids.len()
        );
    }

    let mut buf = String::new();
    std::io::stdin()
        .read_line(&mut buf)
        .map_err(|e| format!("Could not interpret input: {}", e))?;
    let user_response = buf.trim();

    let result = if let Ok(selection) = user_response.parse::<usize>() {
        if selection == 0 || selection > paper_list.doc_ids.len() {
            Err("Selection is out of range".to_string())?;
        }
        std::mem::replace(&mut recent_papers[selection - 1], None).unwrap()
    } else if let (Some(_), Some(pos)) = (user_response.find('/'), user_response.rfind('-')) {
        // User provided a link instead of a number.
        let doc_id = &user_response[(pos + 1)..];
        download_dropbox_paper(&client, api_key, doc_id)
    } else {
        return Err(format!(
            "Cannot interpret input. Must be either a number from 1 to {} or a link to a \
             Dropbox Paper document.",
            paper_list.doc_ids.len()
        ));
    };

    Ok(result)
}

fn download_dropbox_paper(
    client: &reqwest::blocking::Client,
    api_key: &str,
    doc_id: &str,
) -> (String, PaperMetaData) {
    let args = json!(DownloadRequest {
        doc_id,
        export_format: "markdown"
    })
    .to_string();

    let mut response = client
        .post("https://api.dropboxapi.com/2/paper/docs/download")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Dropbox-API-Arg", args)
        .send()
        .unwrap();

    let meta_data: PaperMetaData = serde_json::from_str(
        std::str::from_utf8(
            response
                .headers()
                .get("dropbox-api-result")
                .unwrap()
                .as_ref(),
        )
        .unwrap(),
    )
    .unwrap();

    (response.text().unwrap(), meta_data)
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
        println!("Done."); // TODO: turn into an `info!` in non-interactive mode.
    }
}
