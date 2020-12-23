use anyhow::{anyhow, Context, Error, Result};
use chord::{Chord, Plot};
use elasticsearch::{http::transport::Transport, Elasticsearch, SearchParts};
use palette::{rgb::LinSrgb, Hsv, IntoColor};
use serde_json::{json, Value};
use std::{collections::HashMap, str::FromStr};
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

    let freqs: Vec<_> = relationship_frequencies(&client, opt.min_works, opt.limit)
        .await?
        .into_iter()
        .filter_map(|(ship, count)| {
            Ship::from_str(&ship)
                .map(|ship| (ship, count))
                .map_err(|error| {
                    log::warn!("Dropping ship: {}", error);
                    error
                })
                .ok()
        })
        .filter(|(ship, _count)| ship.kind == opt.ship_kind)
        .collect();

    // Count up mentions of each character
    let mut characters: HashMap<&str, u64> = HashMap::default();
    for (ship, count) in freqs.iter() {
        // We add counts here, as it's possible we have duplicate ship tags
        // due to inconsistent tagging
        *characters.entry(&ship.characters[0]).or_default() += count;
        *characters.entry(&ship.characters[1]).or_default() += count;
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
        let character_one_index = *character_index
            .get(&ship.characters[0].as_ref())
            .expect("character to have index");
        let character_two_index = *character_index
            .get(&ship.characters[1].as_ref())
            .expect("character to have index");
        matrix[character_one_index][character_two_index] += *count as f64;
        matrix[character_two_index][character_one_index] += *count as f64;
    }

    let colors: Vec<String> = character_list
        .iter()
        .enumerate()
        .map(|(index, _name)| {
            let color: LinSrgb<u8> = Hsv::new((index * 360) as f32 / GOLDEN_RATIO, 0.68, 0.69)
                .into_rgb()
                .into_format();
            format!("#{:X}{:X}{:X}", color.red, color.green, color.blue)
        })
        .collect();

    Chord {
        matrix,
        names: character_list,
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

#[derive(Debug, PartialEq)]
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
    /// - the ship does not contain exactly two characters (sorry, poly ships =( )
    ///
    /// A bit of data munging here to remove duplicates.
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

        if characters.len() != 2 {
            return Err(anyhow!("Ship must have exactly two characters: '{}'", ship));
        }
        characters.sort_unstable();

        Ok(Self { characters, kind })
    }
}

#[derive(Debug, PartialEq)]
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
