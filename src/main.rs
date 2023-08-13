use clap::{Parser, error::ErrorKind, CommandFactory};
use url::Url;
use reqwest;
use scraper::{Html, Selector};
use std::fs::{File, create_dir};
use std::os::unix::fs::symlink;
use std::path::Path;
use std::io::Write;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(value_name = "URL")]
    // clap can parse any type that implements the FromStr trait
    link: String,
    #[arg(value_name = "Filename")]
    name: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();
    let url_err = Cli::command().error(
        ErrorKind::ValueValidation,
        r#"<URL> must have the form: "https://eprint.iacr.org/<year>/<number>""#
    );

    let Ok(mut url) = Url::parse(args.link.as_str()) else { url_err.exit() };
    if !(url.host_str() == Some("eprint.iacr.org")) { url_err.exit(); }

    let client = reqwest::Client::builder().build()?;
    
    let temp_url = url.clone();
    let mut path_segments = temp_url.path_segments().unwrap();
    let Some(year) = path_segments.next() else { url_err.exit(); };
    let Some(mut number) = path_segments.next() else { url_err.exit(); };
    number = number.trim_end_matches(".pdf");
    url.set_path(format!("{year}/{number}").as_str());

    let filename = match args.name {
        Some(filename) => format!("{}.pdf", filename.trim_end_matches(".pdf")),
        None => {
            let text = client.get(url.as_str()).send().await?.text().await?;
            let doc = Html::parse_document(&text);
            let selector = Selector::parse(r#"pre[id="bibtex"]"#).unwrap();
            let text = doc.select(&selector).next().unwrap().inner_html();
            let names = text.lines().nth(1).unwrap().trim()
                .trim_start_matches("author = {").trim_end_matches("},").split(" and ");
            let len = names.clone().count();

            let mut res = names.map(|n| {
                n.rsplit(" ").next().unwrap()
                    .chars().filter(|c| c.is_ascii_alphabetic()).collect::<String>()
            }).map(|mut s| { match len {
                // according to cryptobib format
                1 => {s},
                2..=3 => {s.truncate(3); s},
                _ => {s.truncate(1); s},
            }}).collect::<String>();
            res.push_str(&year[2..]); res.push_str(".pdf");
            println!("Using the filename: {res}");
            String::from(res)
        },
    };
    
    let home_dir = dirs::home_dir().ok_or("Unable to find home directory")?;
    let path = home_dir.join(Path::new(".local/share/papers"));
    if !path.is_dir() { create_dir(&path).unwrap(); }

    let path = path.join(Path::new(filename.as_str()));

    if !path.is_file() {
        let mut dest = File::create(&path).unwrap();
        url.set_path(format!("{year}/{number}.pdf").as_str());
        let resp = client.get(url.as_str()).send().await?;
        dest.write_all(&resp.bytes().await?)?;
        println!("Linking to downloaded file at {}", path.to_str().unwrap());
    } else {
        println!("Linking to existing file at {}", path.to_str().unwrap());
    }
    let local_path = Path::new(filename.as_str());
    if !local_path.is_file() {
        symlink(path, Path::new(filename.as_str())).unwrap();
    }

    Ok(())
}
