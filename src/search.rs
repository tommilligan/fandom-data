use anyhow::{Context, Result};
use elasticsearch::{Elasticsearch, SearchParts};
use serde_json::{json, Value};

const WORKS_INDEX: &str = "works";
const AGGREGATION_KEY: &str = "aggregation_key";
const FIELD_RELATIONSHIPS_KEYWORD: &str = "relationships.keyword";

/// Load the frequencies of ship tags from all works.
///
/// Returns a list of `(ship name, count)` pairs.
pub async fn ship_frequencies(
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
