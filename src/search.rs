use anyhow::{anyhow, Context, Error, Result};
use elasticsearch::{Elasticsearch, SearchParts};
use serde::Serialize;
use serde_json::{json, Value};
use std::str::FromStr;

const WORKS_INDEX: &str = "works";
const AGGREGATION_KEY: &str = "aggregation_key";

/// Load the frequencies of ship tags from all works.
///
/// Returns a list of `(ship name, count)` pairs.
pub async fn ship_frequencies(
    client: &Elasticsearch,
    min_works: usize,
    limit: usize,
    field: TagKind,
    filter: Option<Value>,
) -> Result<Vec<(String, u64)>> {
    let query = filter.unwrap_or(json!({
      "match_all": {}
    }));

    let response = client
        .search(SearchParts::Index(&[WORKS_INDEX]))
        .body(json!({
          "aggs": {
              AGGREGATION_KEY: {
                "terms": {
                  "field": field.to_keyword_field(),
                  "min_doc_count": min_works,
                  "size": limit,
                  "order": {
                    "_count": "desc"
                  },
                }
              }
            },
          "size": 0,
          "query": query
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

/// Load the frequencies of ship tags from all works.
///
/// Returns a list of `(ship name, count)` pairs.
pub async fn significant_tags(
    client: &Elasticsearch,
    min_works: usize,
    limit: usize,
    field: TagKind,
) -> Result<Vec<(String, Vec<String>)>> {
    let response = client
        .search(SearchParts::Index(&[WORKS_INDEX]))
        .body(json!({
          "aggs": {
              AGGREGATION_KEY: {
                "terms": {
                  "field": TagKind::Relationship.to_keyword_field(),
                  "min_doc_count": min_works,
                  "size": limit,
                  "order": {
                    "_count": "desc"
                  },
                },
                "aggs": {
                  AGGREGATION_KEY: {
                    "significant_terms": {
                      "field": field.to_keyword_field()
                    }
                  }
                },
              }
            },
          "size": 0,
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
                    .get(AGGREGATION_KEY)
                    .context("bucket sub agg")?
                    .get("buckets")
                    .context("sub agg buckets key")?
                    .as_array()
                    .context("sub agg buckets array")?
                    .into_iter()
                    .map(|bucket| {
                        Ok(bucket
                            .get("key")
                            .context("significant term key")?
                            .as_str()
                            .context("bucket key string")?
                            .to_owned())
                    })
                    .collect::<Result<_>>()?,
            ))
        })
        .collect::<Result<_>>()?)
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ShipKind {
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

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum TagKind {
    Relationship,
    Character,
    Freeform,
}

impl FromStr for TagKind {
    type Err = Error;

    fn from_str(string: &str) -> Result<Self> {
        match string {
            "relationship" => Ok(Self::Relationship),
            "character" => Ok(Self::Character),
            "freeform" => Ok(Self::Freeform),
            _ => Err(anyhow!("Invalid tag kind: '{}'", string)),
        }
    }
}

impl TagKind {
    pub fn to_field(&self) -> &'static str {
        match self {
            Self::Relationship => "relationships",
            Self::Character => "characters",
            Self::Freeform => "freeforms",
        }
    }

    pub fn to_keyword_field(&self) -> String {
        format!("{}.keyword", self.to_field())
    }
}
