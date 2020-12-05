use anyhow::Result;
use ao3_fandom_vis::{request::page, scrape::search_page_to_works};
use reqwest::{blocking::Client, Url};
use std::{thread::sleep, time::Duration};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "fetch", about = "Fetch ao3 data")]
struct Opt {
    #[structopt(long = "start", default_value = "1")]
    start: u32,

    #[structopt(long = "count", default_value = "1")]
    count: u32,

    /// Interval between request, to avoid rate limiting.
    #[structopt(long = "interval", default_value = "10")]
    interval: u64,
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let opt = Opt::from_args();

    let interval = Duration::from_secs(opt.interval);
    let page_start = opt.start;
    let page_count = opt.count;
    let page_end = page_start + page_count;
    let client = Client::new();

    for page_number in page_start..page_end {
        log::info!("Processing page {}", page_number);

        let url = Url::parse(&page(page_number))?;
        let html = &client.get(url).send()?.text()?;
        let works = search_page_to_works(html)?;
        if works.is_empty() {
            log::info!("Received no works on page {}, breaking", page_number);
            break;
        }
        for work in works.iter() {
            println!("{}", serde_json::to_string(work)?);
        }

        sleep(interval)
    }
    Ok(())
}
