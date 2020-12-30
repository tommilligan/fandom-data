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

## Fair Use

I believe this codebase and derived tooling is in line with [AO3's Terms of Service](https://archiveofourown.org/tos) as of 2020-12-30.

Some portions of this code act as a scraper by making multiple requests to the Archive. Please respect AO3's rate limiting if you hit it, and do not try to circumvent it.

While using this repo, please be aware of the following sections of the ToS:

> #### I.D.7
>
> You agree not to use the Service (as well as the e-mail addresses and URLs of OTW sites): to interfere with or disrupt the Service, any OTW-hosted Content or sites, servers, Services or networks connected to OTW sites;
>
> #### IV.C
>
> Conduct that threatens the technical integrity of the Archive [...] will result in an immediate account suspension [...]
>
> Users may be permanently suspended for threatening the technical integrity of the Archive the first time they do so. Such suspensions may be appealed using the ordinary appeal process.

Thank you to everyone who donates their time to support the Archive! Learn more about [contributing here](https://github.com/otwcode/otwarchive#how-to-contribute)
