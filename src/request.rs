const ENDPOINT: &str = "https://archiveofourown.org";

/// Get pages from the beginning of time onwards.
pub fn page(number: u32) -> String {
    format!("{}/works/search?commit=Search&page={}&utf8=✓&work_search[bookmarks_count]=&work_search[character_names]=&work_search[comments_count]=&work_search[complete]=&work_search[creators]=&work_search[crossover]=&work_search[fandom_names]=Avatar: The Last Airbender&work_search[freeform_names]=&work_search[hits]=&work_search[kudos_count]=&work_search[language_id]=&work_search[query]=&work_search[rating_ids]=&work_search[relationship_names]=&work_search[revised_at]=&work_search[single_chapter]=0&work_search[sort_column]=created_at&work_search[sort_direction]=asc&work_search[title]=&work_search[word_count]", ENDPOINT, number)
}
