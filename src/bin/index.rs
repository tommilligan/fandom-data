use anyhow::{Context, Result};
use elasticsearch::{
    http::transport::Transport,
    indices::{Indices, IndicesPutMappingParts},
    BulkOperation, BulkOperations, BulkParts, Elasticsearch,
};
use fandom_data::{scrape::Work, search::TagKind};
use itertools::Itertools;
use once_cell::sync::Lazy;
use serde_json::{json, Value};
use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::PathBuf,
};
use structopt::StructOpt;

const WORKS_INDEX: &str = "works";

static MAPPING_WORKS: Lazy<Value> = Lazy::new(|| {
    json!({
      "properties": {
        "id": {
          "type": "keyword"
        },
        "title": {
          "type": "text"
        },
        "author": {
          "type": "keyword"
        },
        TagKind::Relationship.to_field(): {
          "type": "keyword"
        },
        TagKind::Character.to_field(): {
          "type": "keyword"
        },
        TagKind::Freeform.to_field(): {
          "type": "keyword"
        },
        "date": {
          "type": "date"
        },
        "language": {
          "type": "keyword"
        },
        "words": {
          "type": "long"
        },
        "kudos": {
          "type": "long"
        },
        "hits": {
          "type": "long"
        },
      }
    })
});

#[derive(Debug, StructOpt)]
#[structopt(name = "fetch", about = "Fetch ao3 data")]
struct Opt {
    /// Works data to index
    #[structopt(long = "input")]
    input: PathBuf,

    /// Endpoint of elasticsearch cluster
    #[structopt(long = "elasticsearch")]
    elasticsearch: String,

    /// Document chunk size to upload in one request
    #[structopt(long = "chunk-size", default_value = "1024")]
    chunk_size: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let opt = Opt::from_args();

    let transport = Transport::single_node(&opt.elasticsearch)?;
    let client = Elasticsearch::new(transport);
    let indices = Indices::new(client.transport());

    indices
        .put_mapping(IndicesPutMappingParts::Index(&[WORKS_INDEX]))
        .body(&*MAPPING_WORKS);

    let file = BufReader::new(File::open(opt.input).context("input file")?);
    for (chunk_index, lines) in file.lines().chunks(opt.chunk_size).into_iter().enumerate() {
        log::info!(
            "Processing chunk {} ({} documents)",
            chunk_index,
            (chunk_index + 1) * opt.chunk_size
        );
        let mut ops = BulkOperations::new();
        for line in lines.into_iter() {
            let work: Work =
                serde_json::from_str(&line.context("input line")?).context("line json")?;
            let id = work.id.clone();
            ops.push(BulkOperation::index(work).id(id))?;
        }

        client
            .bulk(BulkParts::Index("works"))
            .body(vec![ops])
            .send()
            .await?;
    }

    Ok(())
}
