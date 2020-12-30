# AO3 Fandom Visualization

## Downloading data for a fandom

Download the data to `jsonl` file (line delimited JSON objects), with the following command:

```bash
cargo run --bin fetch -- --count 2000 --interval 10 -n 1 > output.jsonl
```

Adding an interval between requests is recommended, to avoid hitting the Archive's rate limiting.

If the command fails or you need to resume from a later page, add `--start <page number>`

## Indexing raw data

> From this point on the guide uses [docker-compose](https://docs.docker.com/compose/), which you can install with `pip install docker-compose`

To shove our data into an elasticsearch cluster, first start the cluster with:

```bash
docker-compose up -d elasticsearch
```

You can tail the logs with `docker-compose logs -f` to find when the cluster is ready to go.

Then index the data with:

```bash
cargo run --bin index -- --elasticsearch http://172.17.0.1:9200 --input output_2020-12-05T21:59:00.jsonl
```

where `--input` is the path to the file you fetched earlier.

Works are stored with their Archive id, so it's fine to rerun this step multiple times. Old documents will be replaced.

## Inspecting the data

You can view the raw data using the Kibana toolset by running `docker-compose up -d kibana` and then going to `http://172.17.0.1`.
