use anyhow::Result;
use ao3_fandom_vis::scrape::{page_url, search_page_to_works, ENDPOINT_AO3};
use rayon::prelude::*;
use reqwest::{blocking::Client, Url};
use std::io::{self, Write};
use std::{thread::sleep, time::Duration};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "fetch", about = "Fetch ao3 data")]
struct Opt {
    /// Page to start fetching from
    #[structopt(long = "start", default_value = "1")]
    start: u32,

    /// Number of pages to fetch at most
    #[structopt(long = "count", default_value = "1")]
    count: u32,

    /// Interval between requests in seconds, to avoid rate limiting
    #[structopt(long = "interval")]
    interval: Option<u64>,

    /// Number of requests to process in parallel
    #[structopt(short = "n", long = "threads", default_value = "1")]
    threads: usize,
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let opt = Opt::from_args();
    rayon::ThreadPoolBuilder::new()
        .num_threads(opt.threads)
        .build_global()
        .unwrap();

    let interval = opt.interval.map(Duration::from_secs);
    let page_start = opt.start;
    let page_count = opt.count;
    let page_end = page_start + page_count;
    let client = Client::new();

    let stdout = io::stdout();

    (page_start..page_end)
        .into_par_iter()
        .map::<_, Result<(u32, Vec<_>)>>(|page_number| {
            log::info!("Processing page {}", page_number);
            let url = Url::parse(&page_url(ENDPOINT_AO3, page_number))?;
            let html = &client.get(url).send()?.text()?;
            let works = search_page_to_works(html)?;

            let mut handle = stdout.lock();
            for work in works.iter() {
                handle.write(&serde_json::to_string(work)?.as_bytes())?;
                handle.write("\n".as_bytes())?;
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
