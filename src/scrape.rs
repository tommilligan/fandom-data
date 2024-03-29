use anyhow::{Context, Result};
use chrono::NaiveDate;
use once_cell::sync::Lazy;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Work {
    pub id: String,
    pub title: String,
    pub author: Option<String>,
    pub relationships: Vec<String>,
    pub characters: Vec<String>,
    pub freeforms: Vec<String>,
    pub date: NaiveDate,
    pub language: String,
    pub words: u32,
    pub kudos: u32,
    pub hits: u32,
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

trait SelectExt {
    fn next_text(&mut self) -> Result<&str>;

    fn next_number(&mut self) -> Result<u32>;

    fn collect_texts(&mut self) -> Result<Vec<String>>;
}

impl<'a, 'b> SelectExt for scraper::element_ref::Select<'a, 'b> {
    fn next_text(&mut self) -> Result<&str> {
        self.next()
            .context("selector to find element")?
            .text()
            .next()
            .context("element to have text")
    }

    fn next_number(&mut self) -> Result<u32> {
        self.next_text()?
            .replace(",", "")
            .parse()
            .context("failed to parse number")
    }

    fn collect_texts(&mut self) -> Result<Vec<String>> {
        self.map(|element| {
            element
                .text()
                .next()
                .context("element to have text")
                .map(ToOwned::to_owned)
        })
        .collect()
    }
}

pub fn search_page_to_works(body: &str) -> Result<Vec<Work>> {
    let fragment = Html::parse_document(&body);
    Ok(fragment
        .select(&*SELECTOR_WORK)
        .map(|work_element| {
            let id = work_element
                .value()
                .attr("id")
                .context("work to have id")?
                .strip_prefix("work_")
                .context("work id to have prefix")?
                .to_owned();

            let mut title_author = work_element.select(&*SELECTOR_TITLE_AUTHOR);
            let title = title_author.next_text().context("title")?.to_owned();
            let author = title_author.next_text().ok().map(ToOwned::to_owned);

            let relationships = work_element
                .select(&*SELECTOR_RELATIONSHIP)
                .collect_texts()
                .context("relationships")?;
            let characters = work_element
                .select(&*SELECTOR_CHARACTER)
                .collect_texts()
                .context("characters")?;
            let freeforms = work_element
                .select(&*SELECTOR_FREEFORM)
                .collect_texts()
                .context("freeforms")?;
            let date = NaiveDate::parse_from_str(
                work_element
                    .select(&*SELECTOR_DATE)
                    .next_text()
                    .context("date")?,
                "%d %b %Y",
            )
            .expect("unexpected date format");
            let language = work_element
                .select(&*SELECTOR_LANGUAGE)
                .next_text()
                .unwrap_or("")
                .to_owned();
            let words = work_element
                .select(&*SELECTOR_WORDS)
                .next_number()
                .unwrap_or(0);
            let kudos = work_element
                .select(&*SELECTOR_KUDOS)
                .next_number()
                .unwrap_or(0);
            let hits = work_element
                .select(&*SELECTOR_HITS)
                .next_number()
                .unwrap_or(0);

            Ok(Work {
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
            })
        })
        .collect::<Result<_>>()?)
}

pub const ENDPOINT_AO3: &str = "https://archiveofourown.org";

/// Get pages from the beginning of time onwards.
pub fn page_url(endpoint: &str, number: u32, fandom: &str, creators: &str) -> String {
    format!("{}/works/search?commit=Search&page={}&utf8=✓&work_search[bookmarks_count]=&work_search[character_names]=&work_search[comments_count]=&work_search[complete]=&work_search[creators]={creators}&work_search[crossover]=&work_search[fandom_names]={fandom}&work_search[freeform_names]=&work_search[hits]=&work_search[kudos_count]=&work_search[language_id]=&work_search[query]=&work_search[rating_ids]=&work_search[relationship_names]=&work_search[revised_at]=&work_search[single_chapter]=0&work_search[sort_column]=created_at&work_search[sort_direction]=asc&work_search[title]=&work_search[word_count]", endpoint, number)
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
            search_page_to_works(SEARCH_HTML).unwrap(),
            serde_json::from_str::<Vec<_>>(SEARCH_WORKS).expect("invalid test data")
        );
    }
}
