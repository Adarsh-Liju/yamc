# YAMC - Yet Another Markdown Converter

A fast, feature-rich command-line tool for converting Markdown files to beautiful HTML with GitHub-style rendering.

## Features

- 🚀 **Fast conversion** using the high-performance Comrak markdown parser
- 🎨 **GitHub-style rendering** with responsive design
- 📱 **Mobile-friendly** output with responsive CSS
- 🔧 **Flexible output** - specify custom output files or use automatic naming
- ✅ **Rich markdown support** including:
  - Tables
  - Task lists
  - Strikethrough text
  - Footnotes
  - Description lists
  - Front matter
  - And much more!

## Installation

### From Source

```bash
git clone https://github.com/yourusername/yamc.git
cd yamc
cargo build --release
```

### Using Cargo

```bash
cargo install --git https://github.com/yourusername/yamc.git
```

## Usage

### Basic Usage

Convert a markdown file to HTML:

```bash
yamc convert README.md
```

This will create `README.html` in the same directory.

### Specify Output File

```bash
yamc convert README.md output.html
```

### Convert Files in Different Directories

```bash
yamc convert ./docs/manual.md ./public/index.html
```

### Get Help

```bash
yamc --help
# or
yamc -h
# or
yamc help
```

## Examples

### Input Markdown (`example.md`)

```markdown
# My Awesome Project

This is a **bold** and *italic* text with `code`.

## Features

- [x] Feature 1
- [ ] Feature 2
- [x] Feature 3

## Code Example

```rust
fn main() {
    println!("Hello, World!");
}
```

## Table

| Name | Age | City |
|------|-----|------|
| Alice | 25 | New York |
| Bob | 30 | London |

~~This text is strikethrough~~
```

### Output HTML

The tool generates a complete HTML document with:
- Proper HTML5 structure
- GitHub-style CSS
- Responsive design
- Meta tags for better SEO
- Mobile-friendly layout

## Error Handling

The tool provides clear error messages for common issues:

- File not found
- Invalid file paths
- Permission errors
- Conversion failures

## Development

### Building

```bash
cargo build
```

### Running Tests

```bash
cargo test
```

### Running in Development Mode

```bash
cargo run convert example.md
```

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or https://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or https://opensource.org/licenses/MIT)

at your option.

## Acknowledgments

- [Comrak](https://github.com/kivikakk/comrak) - The excellent markdown parser
- [GitHub Markdown CSS](https://github.com/sindresorhus/github-markdown-css) - The beautiful styling