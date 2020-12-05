use anyhow::{Context, Result};
use ao3_fandom_vis::{request::page, scrape::search_page_to_works};
use reqwest::{blocking::Client, Url};

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let page_start = 1;
    let page_count = 2;
    let page_end = page_start + page_count;
    let client = Client::new();

    for page_number in page_start..page_end {
        log::info!("Processing page {}", page_number);
        let works =
            search_page_to_works(&client.get(Url::parse(&page(page_number))?).send()?.text()?)?;
        if works.is_empty() {
            log::info!("Received no works on page {}, breaking", page_number);
            break;
        }
        for work in works.iter() {
            println!("{}", serde_json::to_string(work)?);
        }
    }
    Ok(())
}
