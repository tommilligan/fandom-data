use anyhow::Result;
use ao3_fandom_vis::search::{significant_tags, TagKind};
use elasticsearch::{http::transport::Transport, Elasticsearch};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "fetch", about = "Fetch ao3 data")]
struct Opt {
    /// Endpoint of elasticsearch cluster
    #[structopt(long = "elasticsearch")]
    elasticsearch: String,

    /// Maximum number of ships to display
    #[structopt(long = "limit", default_value = "5")]
    limit: usize,

    /// Tag kind to show significant terms for.
    #[structopt(long = "tag-kind", default_value = "relationship")]
    tag_kind: TagKind,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let opt = Opt::from_args();

    let transport = Transport::single_node(&opt.elasticsearch)?;
    let client = Elasticsearch::new(transport);

    let significant_tags = significant_tags(&client, 50, opt.limit, opt.tag_kind).await?;

    println!("# Significant tags\n");
    for (ship, tags) in significant_tags.iter() {
        println!("## {}\n", ship);
        for tag in tags.iter() {
            println!("- {}", tag);
        }
        println!("");
    }
    Ok(())
}
