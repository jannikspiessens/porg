use clap::{Parser, error::ErrorKind, CommandFactory};
use url::Url;
use reqwest::Client;
use scraper::{Html, Selector};
use std::fs::{File, create_dir};
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::io::Write;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(value_name = "URL")]
    // clap can parse any type that implements the FromStr trait
    link: String,
    #[arg(value_name = "Filename")]
    name: Option<String>,
}

const SCHEME: &str = "https://";
const HOST: &str = "eprint.iacr.org";


async fn get_info(args: Args, path_papers: PathBuf) 
    -> Result<(Client, String, String), Box<dyn std::error::Error>> {
    let url_err = Args::command().error(
        ErrorKind::ValueValidation,
        format!(r#"<URL> must have the form: "{}{}/<year>/<number>""#, SCHEME, HOST)
    );
    let Ok(mut url) = Url::parse(args.link.as_str()) else { url_err.exit() };
    if !(url.host_str() == Some(HOST)) { url_err.exit(); }

    let temp_url = url.clone();
    let mut path_segments = temp_url.path_segments().unwrap();
    let Some(year) = path_segments.next() else { url_err.exit() };
    let Some(mut number) = path_segments.next() else { url_err.exit() };
    number = number.trim_end_matches(".pdf");
    let eprint_id = format!("{year}/{number}");
    url.set_path(eprint_id.as_str());
    
    let client = Client::builder().build()?;
    let text = client.get(url.as_str()).send().await?.text().await?;
    let doc = Html::parse_document(&text);

    let selector = Selector::parse(r#"pre[id="bibtex"]"#).unwrap();
    let bibtex = doc.select(&selector).next().unwrap().inner_html();
    let mut bibtex_lines = bibtex.lines();
    let names = bibtex_lines.nth(1).unwrap().trim()
        .trim_start_matches("author = {").trim_end_matches("},").split(" and ");
    let len = names.clone().count();
    let mut cryptobib = names.map(|n| {
        n.rsplit(" ").next().unwrap()
            .chars().filter(|c| c.is_ascii_alphabetic()).collect::<String>()
    }).map(|mut s| { match len {
        // according to cryptobib format
        1 => {s},
        2..=3 => {s.truncate(3); s},
        _ => {s.truncate(1); s},
    }}).collect::<String>();
    cryptobib.push_str(&year[2..]);
    cryptobib.push_str(".pdf");

    println!("Using the filename: {cryptobib}");
    let filename = match args.name {
        Some(filename) => format!("{}.pdf", filename.trim_end_matches(".pdf")),
        None => cryptobib,
    };

    let title = bibtex_lines.next().unwrap().trim()
        .trim_start_matches("title = {").trim_end_matches("},");

    let mut abstr = "".to_string();
    let selector = Selector::parse(r#"p[style="white-space: pre-wrap;"]"#).unwrap();
    if let Some(abstr_tag) = doc.select(&selector).next() {
        abstr = abstr_tag.inner_html();
    }

    let data_path = path_papers.join(Path::new("data"));
    if !data_path.is_dir() { create_dir(&data_path).unwrap(); }
    let data_path = data_path.join(Path::new(filename.trim_end_matches(".pdf")));
    let mut dest = File::create(&data_path).unwrap();
    dest.write_all(title.as_bytes())?;
    dest.write_all("\n\n".as_bytes())?;
    dest.write_all(abstr.as_bytes())?;

    Ok((client, eprint_id, filename))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    
    let home_dir = dirs::home_dir().ok_or("Unable to find home directory")?;
    let path_papers = home_dir.join(Path::new(".local/share/papers"));
    if !path_papers.is_dir() { create_dir(&path_papers).unwrap(); }

    let (client, eprint_id, filename) = get_info(Args::parse(), path_papers.clone()).await?;

    let path = path_papers.join(Path::new(filename.as_str()));
    if !path.is_file() {
        let mut dest = File::create(&path).unwrap();
        let url = format!("{}{}/{}.pdf", SCHEME, HOST, eprint_id);
        let resp = client.get(url).send().await?;
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
