use std::{collections::HashMap, fs, io, process::exit};

use clap::{Parser, Subcommand};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use scraper::{ElementRef, Html, Node, Selector};
use thiserror::Error;

#[derive(Subcommand)]
enum Commands {
    ReadPage { page: String },
    UpdateCategory { category: String },
    UpdateAll,
}

#[derive(Parser)]
struct CliArgs {
    // The title of the page to retrieve from the Archwiki
    #[command(subcommand)]
    command: Commands,
}

#[derive(Error, Debug)]
enum WikiError {
    #[error("A network error occurred")]
    NetworkError(#[from] reqwest::Error),
    #[error("A yaml parsing error occurred")]
    YamlParsingError(#[from] serde_yaml::Error),
    #[error("An IO error occurred")]
    IOError(#[from] io::Error),
    #[error("A HTML parsing error occurred")]
    HtmlError(String),
}

#[tokio::main]
async fn main() -> Result<(), WikiError> {
    let args = CliArgs::parse();
    let page_map: HashMap<String, Vec<String>> =
        serde_yaml::from_str(&fs::read_to_string("pages.yml")?)?;

    match args.command {
        Commands::ReadPage { page } => {
            read_page(
                &page,
                &page_map
                    .iter()
                    .map(|(_k, v)| v.iter().map(|e| e.as_str()).collect())
                    .reduce(|acc: Vec<&str>, e| acc.into_iter().chain(e).collect())
                    .unwrap_or(Vec::new()),
            )
            .await?;
        }
        Commands::UpdateCategory { category } => {
            match fetch_page_names_from_categoriy(&category).await {
                Some(pages) => {
                    let mut content = page_map.clone();
                    content.insert(category, pages);
                    let yaml = serde_yaml::to_string(&content)?;
                    fs::write("pages.yml", yaml)?;
                }
                None => println!("Found no pages for category {category}"),
            }
        }
        Commands::UpdateAll => {
            let pages = fetch_all_page_names().await?;
            let yaml = serde_yaml::to_string(&pages)?;
            fs::write("pages.yml", yaml)?;
        }
    }

    Ok(())
}

async fn read_page(page: &str, pages: &[&str]) -> Result<(), WikiError> {
    let page = if !pages.contains(&page) {
        let recommendations = get_top_pages(page, 5, pages);
        eprintln!("{}", recommendations.join("\n"));
        exit(2);
    } else {
        page
    };

    let document = fetch_page(page).await?;
    let content = match get_page_content(&document) {
        Some(content) => content,
        None => {
            return Err(WikiError::HtmlError(
                "Failed to find page content".to_owned(),
            ))
        }
    };

    let res = content
        .descendants()
        .map(|node| match node.value() {
            Node::Text(text) => text.to_string(),
            _ => "".to_owned(),
        })
        .collect::<Vec<String>>()
        .join("");

    println!("{res}");
    Ok(())
}

fn get_top_pages<'a>(search: &str, amount: usize, pages: &[&'a str]) -> Vec<&'a str> {
    let matcher = SkimMatcherV2::default();
    let mut ranked_pages = pages
        .iter()
        .map(|page| (matcher.fuzzy_match(*page, search).unwrap_or(0), *page))
        .collect::<Vec<(i64, &str)>>();

    ranked_pages.sort_by(|a, b| a.0.cmp(&b.0));
    ranked_pages
        .into_iter()
        .rev()
        .take(amount)
        .map(|e| e.1)
        .collect()
}

// TODO: fix duplicate pages being found
async fn fetch_all_page_names() -> Result<HashMap<String, Vec<String>>, WikiError> {
    let document = fetch_page("Table_of_contents").await?;
    let selector = Selector::parse(".mw-parser-output").unwrap();

    let cat_hrefs = document
        .select(&selector)
        .next()
        .unwrap()
        .descendants()
        .filter_map(|node| extract_a_tag_attr(node.value(), "href"))
        .skip(1)
        .collect::<Vec<String>>();

    let mut pages = HashMap::new();
    for cat in cat_hrefs {
        let cat_name = cat.split(":").last().unwrap_or("");
        let res = fetch_page_names_from_categoriy(cat_name).await;
        pages.insert(cat_name.to_owned(), res.unwrap_or(Vec::new()));
    }

    Ok(pages)
}

async fn fetch_page_names_from_categoriy(category: &str) -> Option<Vec<String>> {
    let selector = Selector::parse("#mw-pages").unwrap();
    let document = fetch_html(&format!(
        "https://wiki.archlinux.org/title/Category:{category}"
    ))
    .await
    .unwrap();

    Some(
        document
            .select(&selector)
            .next()?
            .descendants()
            .filter_map(|node| extract_a_tag_attr(node.value(), "title"))
            .collect::<Vec<String>>(),
    )
}

fn extract_a_tag_attr(node: &Node, attr: &str) -> Option<String> {
    if let Node::Element(e) = node {
        if e.name() == "a" {
            if let Some(attr) = e.attr(attr) {
                Some(attr.to_owned())
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    }
}

async fn fetch_html(url: &str) -> Result<Html, reqwest::Error> {
    let body = reqwest::get(url).await?.text().await?;
    Ok(Html::parse_document(&body))
}

async fn fetch_page(page: &str) -> Result<Html, reqwest::Error> {
    fetch_html(&format!(
        "https://wiki.archlinux.org/title/{title}",
        title = page
    ))
    .await
}

fn get_page_content(document: &Html) -> Option<ElementRef<'_>> {
    let selector =
        Selector::parse(".mw-parser-output").expect(".mw-parser-output should be valid selector");
    document.select(&selector).next()
}
