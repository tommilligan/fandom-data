use anyhow::{anyhow, Context, Error, Result};
use chord::{Chord, Plot};
use elasticsearch::{http::transport::Transport, Elasticsearch, SearchParts};
use palette::{rgb::LinSrgb, Hsv, IntoColor};
use serde_json::{json, Value};
use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
};
use structopt::StructOpt;

const WORKS_INDEX: &str = "works";
const AGGREGATION_KEY: &str = "aggregation_key";
const FIELD_RELATIONSHIPS_KEYWORD: &str = "relationships.keyword";
const GOLDEN_RATIO: f32 = 1.618033;

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

    /// Relationship kidn to display.
    #[structopt(long = "ship-kind", default_value = "romantic")]
    ship_kind: ShipKind,
}

/// Load the frequencies of ship tags from all works.
///
/// Returns a list of `(ship name, count)` pairs.
async fn ship_frequencies(
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

    let results = ship_frequencies(&client, opt.min_works, opt.limit).await?;
    let freqs: Vec<_> = results
        .into_iter()
        .filter_map(|(ship, count)| {
            Ship::from_str(&ship)
                // A bit of munging - we can't handle tags where we don't have 2 characters
                .and_then(|ship| {
                    if ship.characters.len() == 2 {
                        Ok(ship)
                    } else {
                        Err(anyhow!(
                            "Ship must have exactly two characters: '{:?}'",
                            ship.characters
                        ))
                    }
                })
                .map_err(|error| {
                    log::warn!("Dropping ship: {}", error);
                    error
                })
                .ok()
                .map(|ship| (ship, count))
        })
        .filter(|(ship, _count)| ship.kind == opt.ship_kind)
        .collect();

    // Get unique, sorted list of all characters
    let mut characters: HashSet<&str> = HashSet::default();
    for (ship, _count) in freqs.iter() {
        for character in ship.characters.iter() {
            characters.insert(&character);
        }
    }
    let mut names: Vec<String> = characters.into_iter().map(ToOwned::to_owned).collect();
    names.sort_unstable();

    // Lookup from character name to index in the sorted list above
    // which will also be the index in the co-occurance matrix below
    let character_index: HashMap<&str, usize> = names
        .iter()
        .enumerate()
        .map(|(index, character)| (character.as_ref(), index))
        .collect();

    // Initialize the matrix with zeroes
    let mut matrix: Vec<Vec<f64>> = vec![vec![0.; names.len()]; names.len()];
    for (ship, count) in freqs.iter() {
        let character_one_index = *character_index
            .get(&ship.characters[0].as_ref())
            .expect("character to have index");
        let character_two_index = *character_index
            .get(&ship.characters[1].as_ref())
            .expect("character to have index");
        // Add rather than assigning here, to allow for duplicate ship tags
        matrix[character_one_index][character_two_index] += *count as f64;
        matrix[character_two_index][character_one_index] += *count as f64;
    }

    // Generate colors for each name
    let colors: Vec<String> = names
        .iter()
        .enumerate()
        .map(|(index, _name)| {
            let color = golden_color(index);
            color.as_hex()
        })
        .collect();

    Chord {
        matrix,
        names,
        wrap_labels: false,
        width: 1150.,
        margin: 75.,
        font_size_large: "14px".to_owned(),
        colors,
        ..Chord::default()
    }
    .to_html();

    Ok(())
}

/// Use the golden ratio to deal out differing colors for a large number of items.
///
/// Color hues remain evently distributed across both small and large sets.
fn golden_color(index: usize) -> LinSrgb<u8> {
    Hsv::new((index * 360) as f32 / GOLDEN_RATIO, 0.68, 0.69)
        .into_rgb()
        .into_format::<u8>()
}

trait DisplayHex {
    fn as_hex(&self) -> String;
}

impl DisplayHex for LinSrgb<u8> {
    fn as_hex(&self) -> String {
        format!("#{:X}{:X}{:X}", self.red, self.green, self.blue)
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
struct Ship {
    characters: Vec<String>,
    kind: ShipKind,
}

impl FromStr for Ship {
    type Err = Error;

    /// Given a ship tag, returns a pair of characters in the ship.
    ///
    /// The pair of characters will be sorted, to make tag deduplication easier.
    ///
    /// This function will return `None` if:
    ///
    /// - the ship kind could not be determined
    fn from_str(ship: &str) -> Result<Self> {
        let (delimiter, kind) = if ship.contains('/') {
            ('/', ShipKind::Romantic)
        } else if ship.contains('&') {
            ('&', ShipKind::Platonic)
        } else {
            return Err(anyhow!("Unknown ship kind in: '{}'", ship));
        };

        // Split on separators to get characters
        let mut characters: Vec<String> = ship
            .split(delimiter)
            .map(|mut name| {
                if let Some(fandom_start) = name.find('(') {
                    name = &name[..fandom_start];
                }
                name.trim().to_owned()
            })
            .collect();
        characters.sort_unstable();

        Ok(Self { characters, kind })
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
enum ShipKind {
    Romantic,
    Platonic,
}

impl FromStr for ShipKind {
    type Err = Error;

    fn from_str(string: &str) -> Result<Self> {
        match string {
            "romantic" => Ok(Self::Romantic),
            "platonic" => Ok(Self::Platonic),
            _ => Err(anyhow!("Invalid ship kind: '{}'", string)),
        }
    }
}
