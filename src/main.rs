use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{self, Command};

use comrak::{markdown_to_html, ComrakOptions, ComrakExtensionOptions};
use printpdf::*;
use std::io::BufWriter;

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

// Add a new function for pure Rust PDF export
fn convert_markdown_to_pdf(markdown: &str, pdf_path: &Path) -> Result<(), ConversionError> {
    use comrak::{parse_document, Arena, ComrakOptions, nodes::{AstNode, NodeValue}};
    use printpdf::{PdfDocument, Mm, Pt, PdfLayerReference};

    let arena = Arena::new();
    let options = create_comrak_options();
    let root = parse_document(&arena, markdown, &options);

    let (doc, page1, layer1) = PdfDocument::new("Markdown PDF", Mm(210.0), Mm(297.0), "Layer 1");
    let current_layer = doc.get_page(page1).get_layer(layer1);

    let font = doc.add_builtin_font(BuiltinFont::Helvetica).unwrap();
    let mut y = Mm(287.0); // Start near top of A4

    render_node_to_pdf(&root, &current_layer, &font, &mut y)?;

    let mut file = BufWriter::new(fs::File::create(pdf_path)?);
    doc.save(&mut file).map_err(|e| ConversionError::PdfConversionFailed(format!("PDF save error: {e}")))?;
    Ok(())
}

// Recursively render AST nodes to PDF
fn render_node_to_pdf<'a>(
    node: &'a AstNode<'a>,
    layer: &PdfLayerReference,
    font: &IndirectFontRef,
    y: &mut Mm,
) -> Result<(), ConversionError> {
    use comrak::nodes::NodeValue::*;
    for child in node.children() {
        match &child.data.borrow().value {
            Heading(h) => {
                let text = collect_text(child);
                let size = match h.level {
                    1 => 24.0,
                    2 => 20.0,
                    3 => 16.0,
                    _ => 14.0,
                };
                *y -= Mm(size * 0.7);
                layer.use_text(text, size, Mm(20.0), *y, font);
                *y -= Mm(4.0);
            }
            Paragraph => {
                let text = collect_text(child);
                *y -= Mm(10.0);
                layer.use_text(text, 12.0, Mm(20.0), *y, font);
                *y -= Mm(4.0);
            }
            List(_) => {
                for item in child.children() {
                    if let Item = item.data.borrow().value {
                        let text = collect_text(item);
                        *y -= Mm(8.0);
                        layer.use_text(format!("• {}", text), 12.0, Mm(25.0), *y, font);
                        *y -= Mm(2.0);
                    }
                }
            }
            Text(t) => {
                // handled in collect_text
            }
            Emph | Strong | Code | HtmlInline(_) | SoftBreak | LineBreak | CodeBlock(_) | ThematicBreak | BlockQuote | HtmlBlock(_) | FootnoteDefinition(_) | Table(_) | TableRow | TableCell | TaskItem { .. } | DescriptionList | DescriptionItem(_) | DescriptionTerm | DescriptionDetails => {
                // Not implemented for brevity
            }
            _ => {}
        }
        render_node_to_pdf(child, layer, font, y)?;
    }
    Ok(())
}

// Helper to collect text from a node
fn collect_text<'a>(node: &'a AstNode<'a>) -> String {
    use comrak::nodes::NodeValue::*;
    let mut text = String::new();
    for child in node.children() {
        match &child.data.borrow().value {
            Text(t) => text.push_str(&String::from_utf8_lossy(t)),
            Code(t) => text.push_str(&String::from_utf8_lossy(t)),
            Emph | Strong => text.push_str(&collect_text(child)),
            SoftBreak | LineBreak => text.push(' '),
            _ => text.push_str(&collect_text(child)),
        }
    }
    text
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
    println!();
    println!("PDF Requirements:");
    println!("  • wkhtmltopdf must be installed for PDF conversion");
    println!("  • Install: sudo apt-get install wkhtmltopdf (Ubuntu/Debian)");
    println!("  • Install: brew install wkhtmltopdf (macOS)");
    println!("  • Windows: Download from https://wkhtmltopdf.org/downloads.html");
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
