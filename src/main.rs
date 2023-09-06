use std::{collections::HashMap, fs};

use clap::Parser;
use cli::{CliArgs, Commands};
use directories::BaseDirs;
use error::WikiError;
use formats::plain_text::convert_page_to_plain_text;
use itertools::Itertools;
use url::Url;
use wiki_api::fetch_page_by_url;

use crate::{
    categories::{fetch_all_pages, list_pages},
    formats::{html::convert_page_to_html, markdown::convert_page_to_markdown, PageFormat},
    languages::{fetch_all_langs, format_lang_table},
    search::{format_open_search_table, format_text_search_table, open_search_to_page_url_tupel},
    utils::{create_cache_page_path, get_page_content, page_cache_exists},
    wiki_api::{fetch_open_search, fetch_page, fetch_text_search},
};

mod categories;
mod cli;
mod error;
mod formats;
mod languages;
mod search;
mod utils;
mod wiki_api;

const PAGE_FILE_NAME: &str = "pages.yml";

#[tokio::main]
#[termination::display]
async fn main() -> Result<(), WikiError> {
    human_panic::setup_panic!();

    let args = CliArgs::parse();
    let base_dir = match BaseDirs::new() {
        Some(base_dir) => base_dir,
        None => {
            return Err(WikiError::Path(
                "failed to get valid home directory".to_owned(),
            ))
        }
    };

    let cache_dir = base_dir.cache_dir().join("archwiki-rs");
    let data_dir = base_dir.data_local_dir().join("archwiki-rs");
    fs::create_dir_all(&cache_dir)?;
    fs::create_dir_all(&data_dir)?;

    let pages_path = data_dir.join(PAGE_FILE_NAME);
    let pages_map: HashMap<String, Vec<String>> = match fs::read_to_string(&pages_path) {
        Ok(file) => serde_yaml::from_str(&file)?,
        Err(_) => HashMap::default(),
    };

    match args.command {
        Commands::ReadPage {
            page,
            no_cache_write,
            ignore_cache,
            disable_cache_invalidation,
            show_urls,
            lang,
            format,
        } => {
            let page_cache_path = create_cache_page_path(&page, &format, &cache_dir);
            let use_cached_page = !ignore_cache
                && page_cache_exists(&page_cache_path, disable_cache_invalidation).unwrap_or(false);

            let out = if use_cached_page {
                fs::read_to_string(&page_cache_path)?
            } else {
                let document = match Url::parse(&page) {
                    Ok(url) => {
                        let document = fetch_page_by_url(url).await?;
                        if get_page_content(&document).is_none() {
                            return Err(WikiError::NoPageFound(
                                "page is not a valid ArchWiki page".to_owned(),
                            ));
                        }

                        document
                    }
                    Err(_) => fetch_page(&page, lang.as_ref().map(|x| x.as_str())).await?,
                };

                match format {
                    PageFormat::PlainText => convert_page_to_plain_text(&document, show_urls),
                    PageFormat::Markdown => convert_page_to_markdown(&document, &page),
                    PageFormat::Html => convert_page_to_html(&document, &page),
                }
            };

            if !no_cache_write {
                fs::write(&page_cache_path, out.as_bytes())?;
            }

            println!("{out}");
        }
        Commands::Search {
            search,
            limit,
            lang,
            text_search,
        } => {
            let out = if !text_search {
                let search_res = fetch_open_search(&search, &lang, limit).await?;
                let name_url_pairs = open_search_to_page_url_tupel(&search_res)?;
                format_open_search_table(&name_url_pairs)
            } else {
                let search_res = fetch_text_search(&search, &lang, limit).await?;
                format_text_search_table(&search_res)
            };

            println!("{out}");
        }
        Commands::ListPages { flatten } => {
            let out = list_pages(&pages_map, flatten);
            println!("{out}");
        }
        Commands::ListCategories => {
            let out = pages_map.keys().unique().sorted().join("\n");
            println!("{out}");
        }
        Commands::ListLanguages => {
            let langs = fetch_all_langs().await?;
            let out = format_lang_table(&langs);

            println!("{out}");
        }
        Commands::SyncWiki {
            hide_progress,
            thread_count,
        } => {
            let thread_count = thread_count.unwrap_or(num_cpus::get_physical());
            let out = fetch_all_pages(hide_progress, thread_count).await?;

            fs::write(&pages_path, serde_yaml::to_string(&out)?)?;

            if !hide_progress {
                println!("data saved to {}", pages_path.to_string_lossy());
            }
        }
        Commands::Info {
            show_cache_dir,
            show_data_dir,
            only_values,
        } => {
            let no_flags_provided = !show_data_dir && !show_cache_dir;
            let info = [
                (
                    !only_values,
                    "VALUE".into(),
                    "NAME",
                    "DESCRIPTION",
                ),
                (
                    show_cache_dir || no_flags_provided,
                    cache_dir,
                    "cache directory",
                    "stores caches of ArchWiki pages after download to speed up future requests",
                ),
                (
                    show_data_dir || no_flags_provided,
                    data_dir,
                    "data directory",  
                    "stores the 'pages.yml' file that is used for suggestions about what ArchWiki pages exist"
                ),
            ];

            let out = info
                .iter()
                .filter_map(|entry| {
                    entry.0.then_some(if only_values {
                        format!("{val}", val = entry.1.to_string_lossy())
                    } else {
                        format!(
                            "{name:20} | {desc:90} | {val}",
                            name = entry.2,
                            desc = entry.3,
                            val = entry.1.to_string_lossy()
                        )
                    })
                })
                .join("\n");

            println!("{out}");
        }
    }

    Ok(())
}
