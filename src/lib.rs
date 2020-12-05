use chrono::NaiveDate;
use once_cell::sync::Lazy;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Work {
    id: String,
    title: String,
    #[serde(default)]
    author: String,
    relationships: Vec<String>,
    characters: Vec<String>,
    freeforms: Vec<String>,
    #[serde(default = "default_naivedate")]
    date: NaiveDate,
    #[serde(default)]
    language: String,
    #[serde(default)]
    words: u32,
    #[serde(default)]
    kudos: u32,
    #[serde(default)]
    hits: u32,
}

fn default_naivedate() -> NaiveDate {
    NaiveDate::parse_from_str("05 Dec 2020", "%d %b %Y").unwrap()
}

static SELECTOR_WORK: Lazy<Selector> = Lazy::new(|| Selector::parse("li.work").unwrap());
static SELECTOR_TITLE_AUTHOR: Lazy<Selector> =
    Lazy::new(|| Selector::parse("h4.heading > a").unwrap());
static SELECTOR_RELATIONSHIP: Lazy<Selector> =
    Lazy::new(|| Selector::parse("li.relationships > a.tag").unwrap());
static SELECTOR_CHARACTER: Lazy<Selector> =
    Lazy::new(|| Selector::parse("li.characters > a.tag").unwrap());
static SELECTOR_FREEFORM: Lazy<Selector> =
    Lazy::new(|| Selector::parse("li.freeforms > a.tag").unwrap());
static SELECTOR_DATE: Lazy<Selector> = Lazy::new(|| Selector::parse("p.datetime").unwrap());
static SELECTOR_LANGUAGE: Lazy<Selector> =
    Lazy::new(|| Selector::parse("dl.stats > dd.language").unwrap());
static SELECTOR_WORDS: Lazy<Selector> =
    Lazy::new(|| Selector::parse("dl.stats > dd.words").unwrap());
static SELECTOR_KUDOS: Lazy<Selector> =
    Lazy::new(|| Selector::parse("dl.stats > dd.kudos").unwrap());
static SELECTOR_HITS: Lazy<Selector> = Lazy::new(|| Selector::parse("dl.stats > dd.hits").unwrap());

pub fn search_page_to_works(body: &str) -> Vec<Work> {
    let fragment = Html::parse_document(&body);
    fragment
        .select(&*SELECTOR_WORK)
        .map(|work_element| {
            let id = work_element
                .value()
                .attr("id")
                .expect("work to have id")
                .strip_prefix("work_")
                .expect("work id to have prefix")
                .to_owned();
            let mut title_author = work_element.select(&*SELECTOR_TITLE_AUTHOR);
            let title = title_author
                .next()
                .expect("work to have title")
                .text()
                .next()
                .expect("title to have text")
                .to_owned();
            let author = title_author
                .next()
                .expect("work to have author")
                .text()
                .next()
                .expect("title to have text")
                .to_owned();
            let relationships = work_element
                .select(&*SELECTOR_RELATIONSHIP)
                .map(|tag_element| {
                    tag_element
                        .text()
                        .next()
                        .expect("relationship tag to contain text")
                        .to_owned()
                })
                .collect();
            let characters = work_element
                .select(&*SELECTOR_CHARACTER)
                .map(|tag_element| {
                    tag_element
                        .text()
                        .next()
                        .expect("character tag to contain text")
                        .to_owned()
                })
                .collect();
            let freeforms = work_element
                .select(&*SELECTOR_FREEFORM)
                .map(|tag_element| {
                    tag_element
                        .text()
                        .next()
                        .expect("freeform tag to contain text")
                        .to_owned()
                })
                .collect();
            let date = NaiveDate::parse_from_str(
                work_element
                    .select(&*SELECTOR_DATE)
                    .next()
                    .expect("work to have date")
                    .text()
                    .next()
                    .expect("date to have text"),
                "%d %b %Y",
            )
            .expect("unexpected date format");
            let language = work_element
                .select(&*SELECTOR_LANGUAGE)
                .next()
                .expect("work to have language")
                .text()
                .next()
                .expect("language to have text")
                .to_owned();
            let words = work_element
                .select(&*SELECTOR_WORDS)
                .next()
                .expect("work to have words")
                .text()
                .next()
                .expect("words to have text")
                .replace(",", "")
                .parse()
                .expect("invalid word count");
            let kudos = work_element
                .select(&*SELECTOR_KUDOS)
                .next()
                .expect("work to have kudos")
                .text()
                .next()
                .expect("kudos to have text")
                .replace(",", "")
                .parse()
                .expect("invalid kudo count");
            let hits = work_element
                .select(&*SELECTOR_HITS)
                .next()
                .expect("work to have hits")
                .text()
                .next()
                .expect("hits to have text")
                .replace(",", "")
                .parse()
                .expect("invalid hit count");

            Work {
                id,
                title,
                author,
                relationships,
                characters,
                freeforms,
                date,
                language,
                words,
                kudos,
                hits,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    const SEARCH_HTML: &str = include_str!("search.html");
    const SEARCH_WORKS: &str = include_str!("search.json");

    #[test]
    fn test_search_page_to_works() {
        assert_eq!(
            search_page_to_works(SEARCH_HTML),
            serde_json::from_str::<Vec<_>>(SEARCH_WORKS).expect("invalid test data")
        );
    }
}
