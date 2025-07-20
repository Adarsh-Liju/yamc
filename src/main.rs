use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{self, Command};

use comrak::{markdown_to_html, ComrakOptions, ComrakExtensionOptions};
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tempfile::NamedTempFile;
use tokio;

#[derive(Debug)]
enum OutputFormat {
    Html,
    Pdf,
}

#[derive(Debug)]
struct Config {
    input_file: PathBuf,
    output_file: PathBuf,
    output_format: OutputFormat,
    css_url: String,
    css_class: String,
}

impl Config {
    fn new(input_file: &str, output_file: Option<&str>, format: OutputFormat) -> Result<Self, String> {
        let input_path = PathBuf::from(input_file);
        
        if !input_path.exists() {
            return Err(format!("Input file '{}' does not exist", input_file));
        }
        
        if !input_path.is_file() {
            return Err(format!("'{}' is not a file", input_file));
        }
        
        let output_path = if let Some(output) = output_file {
            PathBuf::from(output)
        } else {
            // Generate output filename based on input and format
            let mut output = input_path.clone();
            match format {
                OutputFormat::Html => output.set_extension("html"),
                OutputFormat::Pdf => output.set_extension("pdf"),
            };
            output
        };
        
        Ok(Config {
            input_file: input_path,
            output_file: output_path,
            output_format: format,
            css_url: "https://cdnjs.cloudflare.com/ajax/libs/github-markdown-css/4.0.0/github-markdown.min.css".to_string(),
            css_class: "markdown-body".to_string(),
        })
    }
}

#[derive(Debug)]
enum ConversionError {
    IoError(io::Error),
    PdfConversionFailed(String),
    ChromeError(String),
    NetworkError(String),
}

impl std::fmt::Display for ConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConversionError::IoError(e) => write!(f, "I/O error: {}", e),
            ConversionError::PdfConversionFailed(e) => write!(f, "PDF conversion failed: {}", e),
            ConversionError::ChromeError(e) => write!(f, "Chrome error: {}", e),
            ConversionError::NetworkError(e) => write!(f, "Network error: {}", e),
        }
    }
}

impl std::error::Error for ConversionError {}

impl From<io::Error> for ConversionError {
    fn from(err: io::Error) -> Self {
        ConversionError::IoError(err)
    }
}

impl From<reqwest::Error> for ConversionError {
    fn from(err: reqwest::Error) -> Self {
        ConversionError::NetworkError(err.to_string())
    }
}

fn create_comrak_options() -> ComrakOptions {
    ComrakOptions {
        extension: ComrakExtensionOptions {
            strikethrough: true,
            tagfilter: true,
            table: true,
            autolink: true,
            tasklist: true,
            superscript: true,
            header_ids: Some("".to_string()),
            footnotes: true,
            description_lists: true,
            front_matter_delimiter: Some("---".to_string()),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn read_markdown_file(path: &Path) -> Result<String, ConversionError> {
    fs::read_to_string(path)
        .map_err(|e| ConversionError::IoError(e))
        .map_err(|e| {
            eprintln!("Failed to read markdown file: {}", e);
            e
        })
}

fn convert_markdown_to_html(markdown: &str) -> Result<String, ConversionError> {
    let options = create_comrak_options();
    Ok(markdown_to_html(markdown, &options))
}

fn create_html_document(html_content: &str, css_url: &str, css_class: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Converted Markdown</title>
    <link rel="stylesheet" href="{}">
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Noto Sans', Helvetica, Arial, sans-serif;
            line-height: 1.6;
            max-width: 800px;
            margin: 0 auto;
            padding: 20px;
        }}
        .{} {{
            box-sizing: border-box;
            min-width: 200px;
            max-width: 980px;
            margin: 0 auto;
            padding: 45px;
        }}
        @media (max-width: 767px) {{
            .{} {{
                padding: 15px;
            }}
        }}
        @media print {{
            body {{
                max-width: none;
                margin: 0;
                padding: 0;
            }}
            .{} {{
                max-width: none;
                margin: 0;
                padding: 20px;
            }}
        }}
    </style>
</head>
<body class="{}">
{}
</body>
</html>"#,
        css_url, css_class, css_class, css_class, css_class, html_content
    )
}

fn write_html_file(path: &Path, content: &str) -> Result<(), ConversionError> {
    let mut file = fs::File::create(path)?;
    file.write_all(content.as_bytes())?;
    Ok(())
}

async fn convert_html_to_pdf_with_chrome(html_file: &Path, pdf_file: &Path) -> Result<(), ConversionError> {
    // Start headless Chrome
    let mut chrome_process = Command::new("chrome")
        .args(&[
            "--headless",
            "--disable-gpu",
            "--no-sandbox",
            "--disable-dev-shm-usage",
            "--remote-debugging-port=9222",
            "--disable-web-security",
            "--allow-running-insecure-content"
        ])
        .spawn()
        .or_else(|_| Command::new("chromium")
            .args(&[
                "--headless",
                "--disable-gpu",
                "--no-sandbox",
                "--disable-dev-shm-usage",
                "--remote-debugging-port=9222",
                "--disable-web-security",
                "--allow-running-insecure-content"
            ])
            .spawn())
        .or_else(|_| Command::new("google-chrome")
            .args(&[
                "--headless",
                "--disable-gpu",
                "--no-sandbox",
                "--disable-dev-shm-usage",
                "--remote-debugging-port=9222",
                "--disable-web-security",
                "--allow-running-insecure-content"
            ])
            .spawn())
        .map_err(|_| ConversionError::ChromeError(
            "Could not start Chrome/Chromium. Please ensure Chrome or Chromium is installed.".to_string()
        ))?;

    // Wait a moment for Chrome to start
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

    // Get the list of available targets
    let client = reqwest::Client::new();
    let targets_response = client.get("http://localhost:9222/json")
        .send()
        .await?;
    
    let targets: Vec<Value> = targets_response.json().await?;
    let target = targets.into_iter()
        .find(|t| t["type"] == "page")
        .ok_or_else(|| ConversionError::ChromeError("No page target found".to_string()))?;
    
    let ws_url = target["webSocketDebuggerUrl"].as_str()
        .ok_or_else(|| ConversionError::ChromeError("No WebSocket URL found".to_string()))?;

    // Connect to the page and navigate to our HTML file
    let file_url = format!("file://{}", html_file.to_string_lossy());
    
    // Use the CDP (Chrome DevTools Protocol) to navigate and print
    let cdp_client = reqwest::Client::new();
    
    // Create a new tab
    let create_tab_response = cdp_client.post("http://localhost:9222/json/new")
        .send()
        .await?;
    
    let tab_info: Value = create_tab_response.json().await?;
    let tab_id = tab_info["id"].as_str()
        .ok_or_else(|| ConversionError::ChromeError("Failed to get tab ID".to_string()))?;
    
    // Navigate to the HTML file
    let navigate_response = cdp_client.post(&format!("http://localhost:9222/json/navigate/{}", tab_id))
        .json(&json!({
            "url": file_url
        }))
        .send()
        .await?;
    
    if !navigate_response.status().is_success() {
        return Err(ConversionError::ChromeError("Failed to navigate to HTML file".to_string()));
    }
    
    // Wait for the page to load
    tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
    
    // Print to PDF
    let print_response = cdp_client.post(&format!("http://localhost:9222/json/print/{}", tab_id))
        .json(&json!({
            "landscape": false,
            "displayHeaderFooter": false,
            "printBackground": true,
            "preferCSSPageSize": true,
            "paperWidth": 8.27,  // A4 width in inches
            "paperHeight": 11.69, // A4 height in inches
            "marginTop": 0.4,
            "marginBottom": 0.4,
            "marginLeft": 0.4,
            "marginRight": 0.4
        }))
        .send()
        .await?;
    
    if !print_response.status().is_success() {
        return Err(ConversionError::ChromeError("Failed to generate PDF".to_string()));
    }
    
    let print_result: Value = print_response.json().await?;
    let pdf_data = print_result["data"].as_str()
        .ok_or_else(|| ConversionError::ChromeError("No PDF data received".to_string()))?;
    
    // Decode base64 PDF data and write to file
    let pdf_bytes = base64::decode(pdf_data)
        .map_err(|e| ConversionError::PdfConversionFailed(format!("Failed to decode PDF data: {}", e)))?;
    
    fs::write(pdf_file, pdf_bytes)?;
    
    // Close the tab
    let _ = cdp_client.post(&format!("http://localhost:9222/json/close/{}", tab_id))
        .send()
        .await;
    
    // Terminate Chrome
    let _ = chrome_process.kill();
    
    Ok(())
}

fn convert_markdown_to_pdf(markdown: &str, pdf_path: &Path) -> Result<(), ConversionError> {
    // Convert markdown to HTML first
    let html_content = convert_markdown_to_html(markdown)?;
    let full_html = create_html_document(&html_content, 
        "https://cdnjs.cloudflare.com/ajax/libs/github-markdown-css/4.0.0/github-markdown.min.css", 
        "markdown-body");
    
    // Create a temporary HTML file
    let temp_html = NamedTempFile::new()
        .map_err(|e| ConversionError::IoError(e))?
        .into_temp_path();
    write_html_file(&temp_html, &full_html)?;
    
    // Convert HTML to PDF using headless Chrome
    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| ConversionError::ChromeError(format!("Failed to create async runtime: {}", e)))?;
    
    let result = runtime.block_on(convert_html_to_pdf_with_chrome(&temp_html, pdf_path));
    
    // Clean up temporary file
    let _ = fs::remove_file(&temp_html);
    
    result
}

// Update convert_markdown_file to use the new PDF function
fn convert_markdown_file(config: &Config) -> Result<(), ConversionError> {
    let format_str = match config.output_format {
        OutputFormat::Html => "HTML",
        OutputFormat::Pdf => "PDF",
    };

    println!("Converting '{}' to {}...", config.input_file.display(), format_str);

    let markdown_content = read_markdown_file(&config.input_file)?;

    match config.output_format {
        OutputFormat::Html => {
            let html_content = convert_markdown_to_html(&markdown_content)?;
            let full_html = create_html_document(&html_content, &config.css_url, &config.css_class);
            write_html_file(&config.output_file, &full_html)?;
        }
        OutputFormat::Pdf => {
            convert_markdown_to_pdf(&markdown_content, &config.output_file)?;
        }
    }

    println!("✅ Successfully converted markdown to {}!", format_str);
    println!("📁 Output file: {}", config.output_file.display());

    Ok(())
}

fn print_usage(program_name: &str) {
    println!("YAMC - Yet Another Markdown Converter");
    println!();
    println!("Usage: {} <command> <input_file> [output_file]", program_name);
    println!();
    println!("Commands:");
    println!("  convert     Convert markdown to HTML");
    println!("  pdf         Convert markdown to PDF");
    println!();
    println!("Arguments:");
    println!("  input_file   Path to the markdown file to convert");
    println!("  output_file  Optional output file path (defaults to input_file.html/pdf)");
    println!();
    println!("Examples:");
    println!("  {} convert README.md", program_name);
    println!("  {} convert README.md output.html", program_name);
    println!("  {} pdf README.md", program_name);
    println!("  {} pdf README.md output.pdf", program_name);
    println!("  {} pdf ./docs/manual.md ./public/manual.pdf", program_name);
    println!();
    println!("Features:");
    println!("  • GitHub-style markdown rendering");
    println!("  • Tables, task lists, strikethrough, and more");
    println!("  • Responsive design");
    println!("  • HTML and PDF output formats");
    println!("  • Automatic file extension handling");
    println!("  • Pure Rust implementation with headless Chrome");
    println!();
    println!("PDF Requirements:");
    println!("  • Chrome or Chromium must be installed for PDF conversion");
    println!("  • The tool will automatically detect and use Chrome/Chromium");
    println!("  • Uses headless mode for PDF generation");
    println!("  • No external dependencies like wkhtmltopdf required");
}

fn handle_command(
    command: &str,
    input_file: &str,
    output_file: Option<&str>,
    program_name: &str,
) -> Result<(), ConversionError> {
    let format = match command {
        "convert" => OutputFormat::Html,
        "pdf" => OutputFormat::Pdf,
        _ => {
            eprintln!("❌ Unknown command: '{}'", command);
            print_usage(program_name);
            process::exit(1);
        }
    };

    let config = Config::new(input_file, output_file, format)
        .map_err(|e| {
            eprintln!("❌ Configuration error: {}", e);
            process::exit(1);
        })
        .unwrap();

    convert_markdown_file(&config)
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let program_name = &args[0];

    if args.len() == 2 && ["-h", "--help", "help"].contains(&args[1].as_str()) {
        print_usage(program_name);
        return;
    }

    if args.len() < 3 {
        eprintln!("❌ Error: Insufficient arguments\n");
        print_usage(program_name);
        process::exit(1);
    }

    let command = &args[1];
    let input_file = &args[2];
    let output_file = args.get(3).map(|s| s.as_str());

    if let Err(e) = handle_command(command, input_file, output_file, program_name) {
        eprintln!("❌ Conversion failed: {}", e);
        process::exit(1);
    }
}
