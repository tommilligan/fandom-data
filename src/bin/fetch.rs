use anyhow::Result;
use clap::Parser;
use fandom_data::scrape::{page_url, search_page_to_works, ENDPOINT_AO3};
use rayon::prelude::*;
use reqwest::{blocking::Client, Url};
use std::io::{self, Write};
use std::{thread::sleep, time::Duration};

#[derive(Debug, Parser)]
#[clap(name = "fetch", about = "Fetch ao3 data")]
struct Opt {
    /// Page to start fetching from
    #[clap(long, default_value = "1")]
    start: u32,

    /// Number of pages to fetch at most
    #[clap(long, default_value = "1")]
    count: u32,

    /// Name of fandom
    #[clap(long, required_unless_present_any(&["author"]))]
    fandom: Option<String>,

    /// Name of author
    #[clap(long, required_unless_present_any(&["fandom"]))]
    author: Option<String>,

    /// Interval between requests in seconds, to avoid rate limiting
    #[clap(long)]
    interval: Option<u64>,

    /// Number of requests to process in parallel
    #[clap(short = 'n', long, default_value = "1")]
    threads: usize,
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let opt = Opt::parse();
    rayon::ThreadPoolBuilder::new()
        .num_threads(opt.threads)
        .build_global()
        .unwrap();

    let interval = opt.interval.map(Duration::from_secs);
    let page_start = opt.start;
    let page_count = opt.count;
    let page_end = page_start + page_count;
    let fandom = opt.fandom.unwrap_or_default();
    let author = opt.author.unwrap_or_default();
    let client = Client::new();

    let stdout = io::stdout();

    (page_start..page_end)
        .into_par_iter()
        .map::<_, Result<(u32, Vec<_>)>>(|page_number| {
            log::info!("Processing page {}", page_number);
            let url = Url::parse(&page_url(ENDPOINT_AO3, page_number, &fandom, &author))?;
            let html = &client.get(url).send()?.text()?;
            let works = search_page_to_works(html)?;

            let mut handle = stdout.lock();
            for work in works.iter() {
                handle.write_all(&serde_json::to_string(work)?.as_bytes())?;
                handle.write_all(b"\n")?;
            }

            if let Some(interval) = interval {
                sleep(interval);
            }

            Ok((page_number, works))
        })
        .find_first(|result| match result {
            Err(error) => {
                log::error!("Error: {}", error);
                true
            }
            Ok((page_number, works)) => {
                if works.is_empty() {
                    log::info!("Received no works on page {}, stopping", page_number);
                    true
                } else {
                    false
                }
            }
        });
    Ok(())
}
