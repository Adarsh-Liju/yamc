extern crate comrak;
use std::env;
use std::fs;
use std::io::Write;

fn markdown_convert(str: &String) {
    let path = str;
    let markdown = fs::read_to_string(path).expect("Should be able to read the file");
    let html = comrak::markdown_to_html(&markdown, &comrak::ComrakOptions::default());
    let mut file = fs::File::create("output.html").expect("Unable to create file");
    file.write_all(html.as_bytes()).expect("Unable to write data");
    add_css_to_html();
}

// need to refactor this very well , just now I am using it for adding stye to markdown

fn add_css_to_html() {
    // add css to html
    let html = fs::read_to_string("output.html").expect("Should be able to read the file");
    // add cdn of stylesheet
    let html = format!(
        "{}{}",
        "<link rel=\"stylesheet\" href=\"https://cdnjs.cloudflare.com/ajax/libs/github-markdown-css/4.0.0/github-markdown.min.css\">",
        html
    );
    // add class to body
    let html = format!(
        "{}{}",
        "<body class=\"markdown-body\">",
        html
    );
    let mut file = fs::File::create("output.html").expect("Unable to create file");
    file.write_all(html.as_bytes()).expect("Unable to write data")  

}
fn main() {
    let args: Vec<String> = env::args().collect();
    println!("Flag : {:?}", args[1]);
    if args[1] == "convert" {
        markdown_convert(&args[2]);
    } else {
        println!("The Flag is not correct")
    }
}
