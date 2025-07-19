use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{self, Command};

use comrak::{markdown_to_html, ComrakOptions, ComrakExtensionOptions};

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
    InvalidInput(String),
    ConversionFailed(String),
    PdfConversionFailed(String),
}

impl std::fmt::Display for ConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConversionError::IoError(e) => write!(f, "I/O error: {}", e),
            ConversionError::InvalidInput(e) => write!(f, "Invalid input: {}", e),
            ConversionError::ConversionFailed(e) => write!(f, "Conversion failed: {}", e),
            ConversionError::PdfConversionFailed(e) => write!(f, "PDF conversion failed: {}", e),
        }
    }
}

impl std::error::Error for ConversionError {}

impl From<io::Error> for ConversionError {
    fn from(err: io::Error) -> Self {
        ConversionError::IoError(err)
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

fn check_wkhtmltopdf_installed() -> bool {
    Command::new("wkhtmltopdf")
        .arg("--version")
        .output()
        .is_ok()
}

fn convert_html_to_pdf(html_file: &Path, pdf_file: &Path) -> Result<(), ConversionError> {
    if !check_wkhtmltopdf_installed() {
        return Err(ConversionError::PdfConversionFailed(
            "wkhtmltopdf is not installed. Please install it first:\n\
             Ubuntu/Debian: sudo apt-get install wkhtmltopdf\n\
             macOS: brew install wkhtmltopdf\n\
             Windows: Download from https://wkhtmltopdf.org/downloads.html\n\
             \n\
             Alternatively, you can use the 'convert' command to generate HTML first.".to_string()
        ));
    }

    let output = Command::new("wkhtmltopdf")
        .arg("--enable-local-file-access")
        .arg("--print-media-type")
        .arg("--margin-top")
        .arg("20mm")
        .arg("--margin-bottom")
        .arg("20mm")
        .arg("--margin-left")
        .arg("20mm")
        .arg("--margin-right")
        .arg("20mm")
        .arg("--page-size")
        .arg("A4")
        .arg("--encoding")
        .arg("UTF-8")
        .arg(html_file.to_str().unwrap())
        .arg(pdf_file.to_str().unwrap())
        .output()
        .map_err(|e| ConversionError::PdfConversionFailed(format!("Failed to run wkhtmltopdf: {}", e)))?;

    if !output.status.success() {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        return Err(ConversionError::PdfConversionFailed(
            format!("wkhtmltopdf failed: {}", error_msg)
        ));
    }

    Ok(())
}

fn convert_markdown_file(config: &Config) -> Result<(), ConversionError> {
    let format_str = match config.output_format {
        OutputFormat::Html => "HTML",
        OutputFormat::Pdf => "PDF",
    };
    
    println!("Converting '{}' to {}...", 
             config.input_file.display(), 
             format_str);
    
    // Read markdown content
    let markdown_content = read_markdown_file(&config.input_file)?;
    
    // Convert to HTML
    let html_content = convert_markdown_to_html(&markdown_content)?;
    
    match config.output_format {
        OutputFormat::Html => {
            // Create complete HTML document
            let full_html = create_html_document(&html_content, &config.css_url, &config.css_class);
            write_html_file(&config.output_file, &full_html)?;
        }
        OutputFormat::Pdf => {
            // Create temporary HTML file
            let temp_html = config.output_file.with_extension("temp.html");
            let full_html = create_html_document(&html_content, &config.css_url, &config.css_class);
            write_html_file(&temp_html, &full_html)?;
            
            // Convert HTML to PDF
            convert_html_to_pdf(&temp_html, &config.output_file)?;
            
            // Clean up temporary file
            let _ = fs::remove_file(temp_html);
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
    println!();
    println!("PDF Requirements:");
    println!("  • wkhtmltopdf must be installed for PDF conversion");
    println!("  • Install: sudo apt-get install wkhtmltopdf (Ubuntu/Debian)");
    println!("  • Install: brew install wkhtmltopdf (macOS)");
    println!("  • Windows: Download from https://wkhtmltopdf.org/downloads.html");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let program_name = &args[0];
    
    // Handle help flag
    if args.len() == 2 && (args[1] == "-h" || args[1] == "--help" || args[1] == "help") {
        print_usage(program_name);
        return;
    }
    
    // Check if we have enough arguments
    if args.len() < 3 {
        eprintln!("❌ Error: Insufficient arguments");
        eprintln!();
        print_usage(program_name);
        process::exit(1);
    }
    
    // Parse command
    match args[1].as_str() {
        "convert" => {
            let input_file = &args[2];
            let output_file = args.get(3).map(|s| s.as_str());
            
            // Create configuration
            let config = match Config::new(input_file, output_file, OutputFormat::Html) {
                Ok(config) => config,
                Err(e) => {
                    eprintln!("❌ Configuration error: {}", e);
                    process::exit(1);
                }
            };
            
            // Convert the file
            if let Err(e) = convert_markdown_file(&config) {
                eprintln!("❌ Conversion failed: {}", e);
                process::exit(1);
            }
        }
        "pdf" => {
            let input_file = &args[2];
            let output_file = args.get(3).map(|s| s.as_str());
            
            // Create configuration
            let config = match Config::new(input_file, output_file, OutputFormat::Pdf) {
                Ok(config) => config,
                Err(e) => {
                    eprintln!("❌ Configuration error: {}", e);
                    process::exit(1);
                }
            };
            
            // Convert the file
            if let Err(e) = convert_markdown_file(&config) {
                eprintln!("❌ Conversion failed: {}", e);
                process::exit(1);
            }
        }
        _ => {
            eprintln!("❌ Unknown command: '{}'", args[1]);
            eprintln!("Available commands: convert, pdf");
            eprintln!();
            print_usage(program_name);
            process::exit(1);
        }
    }
}
