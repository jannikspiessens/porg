use clap::Parser;
use url::Url;
use reqwest;
use scraper::{Html, Selector};
use std::fs::{File, create_dir};
use std::os::unix::fs::symlink;
use std::path::Path;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(value_name = "URL")]
    // clap can parse any type that implements the FromStr trait
    link: Url,
    #[arg(value_name = "Filename")]
    name: Option<String>,
}

fn main() -> Result<()> {
    let args = Cli::parse();
    let mut url = args.link;
    assert_eq!(url.host_str(), Some("eprint.iacr.org"));
    
    let temp_url = url.clone();
    let mut path_segments = temp_url.path_segments().ok_or("Invalid URL")?;
    let year = path_segments.next().ok_or("Invalid URL")?;
    let number = path_segments.next().ok_or("Invalid URL")?.trim_end_matches(".pdf");
    url.set_path(format!("{year}/{number}").as_str());

    let filename = match args.name {
        Some(filename) => format!("{}.pdf", filename),
        None => {
            let doc = Html::parse_document(&reqwest::blocking::get(url.as_str())?.text()?);
            let selector = Selector::parse(r#"span[class="authorName"]"#).unwrap();
            let len = doc.select(&selector).count();
            
            let select_iter = doc.select(&selector);
            let mut res = select_iter.map(|s| {
                String::from(s.inner_html().split(" ").nth(1).unwrap())
            }).map(|mut s| { match len {
                // according to cryptobib format
                1 => {s},
                2..=3 => {s.truncate(3); s},
                _ => {s.truncate(1); s},
            }}).collect::<String>();
            res.push_str(&year[2..]); res.push_str(".pdf");
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
        let mut resp = reqwest::blocking::get(url.as_str())?;
        resp.copy_to(&mut dest)?;
    }
    symlink(path, Path::new(filename.as_str()))?;
    
    Ok(())
}
