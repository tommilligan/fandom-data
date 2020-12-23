use anyhow::{anyhow, Context, Error, Result};
use chord::{Chord, Plot};
use elasticsearch::{http::transport::Transport, Elasticsearch, SearchParts};
use itertools::Itertools;
use serde_json::{json, Value};
use std::{collections::HashMap, str::FromStr};
use structopt::StructOpt;

const WORKS_INDEX: &str = "works";
const AGGREGATION_KEY: &str = "aggregation_key";
const FIELD_RELATIONSHIPS_KEYWORD: &str = "relationships.keyword";

#[derive(Debug, StructOpt)]
#[structopt(name = "fetch", about = "Fetch ao3 data")]
struct Opt {
    /// Endpoint of elasticsearch cluster
    #[structopt(long = "elasticsearch")]
    elasticsearch: String,

    /// Minimum number of works a tag must have to be displayed
    #[structopt(long = "min-works", default_value = "50")]
    min_works: usize,

    /// Maximum number of ships to display
    #[structopt(long = "limit", default_value = "1000")]
    limit: usize,

    /// Relationship type to display.
    #[structopt(long = "ship-type", default_value = "romantic")]
    ship_type: Ship,
}

async fn relationship_frequencies(
    client: &Elasticsearch,
    min_works: usize,
    limit: usize,
) -> Result<Vec<(String, u64)>> {
    let response = client
        .search(SearchParts::Index(&[WORKS_INDEX]))
        .body(json!({
          "aggs": {
              AGGREGATION_KEY: {
                "terms": {
                  "field": FIELD_RELATIONSHIPS_KEYWORD,
                  "min_doc_count": min_works,
                  "size": limit,
                  "order": {
                    "_count": "desc"
                  },
                }
              }
            },
          "size": 0,
          "query": {
              "match_all": {}
          }
        }))
        .allow_no_indices(true)
        .send()
        .await?;

    let response_body = response.json::<Value>().await?;
    let buckets = response_body
        .get("aggregations")
        .context("Response aggregations key")?
        .get(AGGREGATION_KEY)
        .context("Response aggregation key")?
        .get("buckets")
        .context("Response buckets key")?
        .as_array()
        .context("Response buckets array")?;
    Ok(buckets
        .into_iter()
        .map(|bucket| {
            Ok((
                bucket
                    .get("key")
                    .context("bucket key")?
                    .as_str()
                    .context("bucket key string")?
                    .to_owned(),
                bucket
                    .get("doc_count")
                    .context("bucket doc count")?
                    .as_u64()
                    .context("bucket doc count integer")?,
            ))
        })
        .collect::<Result<_>>()?)
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let opt = Opt::from_args();

    let transport = Transport::single_node(&opt.elasticsearch)?;
    let client = Elasticsearch::new(transport);

    let mut freqs: Vec<_> = relationship_frequencies(&client, opt.min_works, opt.limit)
        .await?
        .into_iter()
        .filter_map(|(ship, count)| ship_to_characters(&ship).map(|characters| (characters, count)))
        .filter(|(ship, _count)| ship.1 == opt.ship_type)
        .collect();
    let original_freq_length = freqs.len();
    freqs.sort_by_key(|(ship, count)| (ship.0.clone(), u64::MAX - count));
    freqs.dedup_by_key(|(ship, _count)| ship.0.clone());
    let dedup_freq_length = freqs.len();
    let removed_length = original_freq_length - dedup_freq_length;
    if removed_length > 0 {
        log::warn!("Removed {} duplicate ship tags", removed_length);
    }

    // Count up mentions of each character
    let mut characters: HashMap<&str, u64> = HashMap::default();
    for (ship, count) in freqs.iter() {
        *characters.entry(&ship.0 .0).or_default() += count;
        *characters.entry(&ship.0 .1).or_default() += count
    }

    let mut character_list = characters
        .keys()
        .map(|character| (*character).to_owned())
        .collect::<Vec<String>>();
    character_list.sort_unstable();
    let character_index: HashMap<&str, usize> = character_list
        .iter()
        .enumerate()
        .map(|(index, character)| (character.as_ref(), index))
        .collect();

    let mut matrix: Vec<Vec<f64>> = vec![vec![0.; character_list.len()]; character_list.len()];
    for (ship, count) in freqs.iter() {
        // let character_one_freq: f64 = *count as f64
        //     / *characters
        //         .get(&character_one.as_ref())
        //         .expect("character to have total frequency") as f64;
        // let character_two_freq: f64 = *count as f64
        //     / *characters
        //         .get(&character_two.as_ref())
        //         .expect("character to have total frequency") as f64;
        let character_one_index = *character_index
            .get(&ship.0 .0.as_ref())
            .expect("character to have index");
        let character_two_index = *character_index
            .get(&ship.0 .1.as_ref())
            .expect("character to have index");
        matrix[character_one_index][character_two_index] = *count as f64;
        matrix[character_two_index][character_one_index] = *count as f64;
    }

    Chord {
        matrix,
        names: character_list,
        wrap_labels: false,
        width: 1150.,
        margin: 75.,
        font_size_large: "14px".to_owned(),
        ..Chord::default()
    }
    .to_html();

    Ok(())
}

#[derive(Debug, PartialEq)]
enum Ship {
    Romantic,
    Platonic,
}

impl FromStr for Ship {
    type Err = Error;

    fn from_str(string: &str) -> Result<Self> {
        match string {
            "romantic" => Ok(Self::Romantic),
            "platonic" => Ok(Self::Platonic),
            _ => Err(anyhow!("Invalid ship type: '{}'", string)),
        }
    }
}

/// Given a ship tag, returns a list of characters in the ship.
///
/// A bit of data munging here to remove duplicates.
fn ship_to_characters(original_ship: &str) -> Option<((String, String), Ship)> {
    let mut ship = original_ship;
    // Trim off fandom if present
    if let Some(fandom_start) = ship.find('(') {
        ship = &ship[..fandom_start];
    }

    // Split on separators
    let mut characters: Vec<String> = ship
        .split(&['/', '&'][..])
        .map(|s| s.trim().to_owned())
        .collect();

    // Coocurrance matrix doesn't work for poly ships?
    if characters.len() != 2 {
        log::warn!("Invalid characters length: '{}'", original_ship);
        return None;
    }
    characters.sort_unstable();
    let characters: (String, String) = characters.into_iter().collect_tuple().unwrap();

    if ship.contains('/') {
        Some((characters, Ship::Romantic))
    } else if ship.contains('&') {
        Some((characters, Ship::Platonic))
    } else {
        log::warn!("Invalid ship name: '{}'", original_ship);
        None
    }
}
