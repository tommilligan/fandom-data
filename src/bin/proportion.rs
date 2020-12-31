use anyhow::{Context, Result};
use chrono::{Date, NaiveDateTime, TimeZone, Utc};
use elasticsearch::{http::transport::Transport, Elasticsearch, SearchParts};
use fandom_data::search::TagKind;
use plotters::prelude::*;
use serde_json::{json, Value};
use structopt::StructOpt;

const WORKS_INDEX: &str = "works";
const AGGREGATION_KEY: &str = "aggregation_key";

#[derive(Debug, StructOpt)]
#[structopt(name = "fetch", about = "Fetch ao3 data")]
struct Opt {
    /// Endpoint of elasticsearch cluster
    #[structopt(long = "elasticsearch")]
    elasticsearch: String,

    /// Maximum number of ships to display
    #[structopt(long = "limit", default_value = "5")]
    limit: usize,
}

/// Load timeseries points of counts of works over time.
///
/// Returns:
///
/// - a list of `(ship tag, Vec<(date, count)>)`
async fn ship_histogram(
    client: &Elasticsearch,
    limit: usize,
) -> Result<Vec<(String, Vec<(Date<Utc>, u64)>)>> {
    let response = client
        .search(SearchParts::Index(&[WORKS_INDEX]))
        .body(json!({
          "aggs": {
            AGGREGATION_KEY: {
              "terms": {
                "field": TagKind::Relationship.to_keyword_field(),
                "order": {
                  "_count": "desc"
                },
                "size": limit,
              },
              "aggs": {
                AGGREGATION_KEY: {
                  "date_histogram": {
                    "field": "date",
                    "calendar_interval": "1M",
                    "min_doc_count": 0
                  }
                }
              }
            }
          },
          "size": 0,
          "docvalue_fields": [
            {
              "field": "date",
              "format": "date_time"
            }
          ],
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
        .iter()
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
                    .context("bucket sub aggregation")?
                    .get("buckets")
                    .context("sub agg buckets key")?
                    .as_array()
                    .context("sub agg buckets array")?
                    .iter()
                    .map(|bucket| {
                        Ok((
                            Date::from_utc(
                                NaiveDateTime::from_timestamp(
                                    (bucket
                                        .get("key")
                                        .context("sub key")?
                                        .as_u64()
                                        .context("sub key as int")?
                                        / 1000) as i64,
                                    0,
                                )
                                .date(),
                                Utc,
                            ),
                            bucket
                                .get("doc_count")
                                .context("bucket doc count")?
                                .as_u64()
                                .context("bucket doc count integer")?,
                        ))
                    })
                    .collect::<Result<_>>()?,
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

    let results = ship_histogram(&client, opt.limit).await?;

    log::info!("Plotting chart");
    let root = BitMapBackend::new("proportion.png", (1024, 768)).into_drawing_area();

    root.fill(&WHITE)?;

    let mut chart = ChartBuilder::on(&root)
        .margin(10)
        .caption("Monthly Count of Ship Works", ("sans-serif", 40))
        .set_label_area_size(LabelAreaPosition::Left, 60)
        .set_label_area_size(LabelAreaPosition::Right, 60)
        .set_label_area_size(LabelAreaPosition::Bottom, 40)
        .build_cartesian_2d(
            (Utc.ymd(2008, 1, 1)..Utc.ymd(2020, 12, 1)).yearly(),
            0u64..600u64,
        )?;

    chart
        .configure_mesh()
        .disable_x_mesh()
        .disable_y_mesh()
        .x_labels(30)
        .y_desc("Work Count")
        .draw()?;

    for (index, (ship_name, data)) in results.into_iter().enumerate() {
        let color = Palette99::pick(index);
        chart
            .draw_series(LineSeries::new(data.into_iter(), &color))?
            .label(&ship_name)
            .legend(move |(x, y)| Rectangle::new([(x, y - 5), (x + 10, y + 5)], color.filled()));
    }

    chart
        .configure_series_labels()
        .position(SeriesLabelPosition::MiddleLeft)
        .border_style(&BLACK)
        .draw()?;

    Ok(())
}
