//! The requests to PodcastIndex, and what comes back.
//!
//! PodcastIndex signs every call with an application secret, so the plugin
//! asks Chilpa's proxy, which signs it and hands the answer back as it came.
//! Its terms ask clients not to walk the index through the API, so nothing
//! here asks for more than the listener opened: one request per listing,
//! and the next page only when the listener reaches the end of this one.

use serde::Deserialize;
use std::collections::BTreeMap;

pub use chilpa_plugin_sdk::host::encode;

const PROXY: &str = "https://api.chilpa.app/podcastindex";

/// How much one listing asks for, and the most a show or a list gives.
pub const PAGE: usize = 20;
pub const MOST: usize = 1_000;

/// A show, as the index writes it.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Feed {
    pub id: u64,
    pub title: String,
    pub author: String,
    pub description: String,
    pub image: String,
    pub artwork: String,
    pub language: String,
    pub link: String,
    pub episode_count: u64,
    /// By the index's own number.
    pub categories: Option<BTreeMap<String, String>>,
    /// Not zero when the feed stopped answering for good.
    pub dead: u64,
    pub itunes_id: Option<u64>,
}

impl Feed {
    pub fn picture(&self) -> Option<String> {
        [&self.artwork, &self.image].into_iter().find(|u| u.starts_with("https://")).cloned()
    }

    pub fn category_names(&self) -> Vec<String> {
        self.categories.as_ref().map(|c| c.values().cloned().collect()).unwrap_or_default()
    }
}

/// An episode, as the index writes it.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Item {
    pub id: u64,
    pub title: String,
    pub description: String,
    pub date_published: i64,
    /// In seconds; zero when the feed does not say.
    pub duration: u64,
    pub enclosure_url: String,
    pub enclosure_type: String,
    pub image: String,
    pub feed_image: String,
    pub season: Option<u64>,
    pub episode: Option<u64>,
    pub episode_type: Option<String>,
    pub enclosure_length: Option<u64>,
    pub chapters_url: Option<String>,
    pub transcript_url: Option<String>,
    pub transcripts: Option<Vec<Transcript>>,
    pub persons: Option<Vec<chilpa_plugin_sdk::serde_json::Value>>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Transcript {
    pub url: String,
    #[serde(rename = "type")]
    pub mime: String,
}

impl Item {
    pub fn transcript(&self) -> &str {
        self.transcripts.as_deref().unwrap_or_default().iter()
            .find(|item| ["text/vtt", "application/x-subrip", "text/srt", "text/plain", "application/json"].contains(&item.mime.as_str()))
            .map(|item| item.url.as_str()).or(self.transcript_url.as_deref()).unwrap_or_default()
    }

    pub fn picture(&self) -> Option<String> {
        [&self.image, &self.feed_image].into_iter().find(|u| u.starts_with("https://")).cloned()
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct Feeds {
    feeds: Vec<Feed>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct Items {
    items: Vec<Item>,
}

fn ask<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, String> {
    chilpa_plugin_sdk::cache::get(&format!("{PROXY}/{path}"), 300)?.json()
}

/// How many to ask for to reach the page after `offset`, which the index
/// gives by count and not by place: it is asked for that many and the ones
/// already shown are skipped.
pub fn reach(offset: usize) -> usize {
    (offset + PAGE).min(MOST)
}

/// The shows most listened to now, in a language and a category when given.
pub fn trending(language: Option<&str>, category: Option<&str>, offset: usize) -> Result<Vec<Feed>, String> {
    let mut path = format!("podcasts/trending?max={}", reach(offset));
    if let Some(language) = language {
        path.push_str(&format!("&lang={}", encode(language)));
    }
    if let Some(category) = category {
        path.push_str(&format!("&cat={}", encode(category)));
    }
    let found: Feeds = ask(&path)?;
    Ok(found.feeds.into_iter().skip(offset).collect())
}

/// Shows whose name, author or owner carries the words.
pub fn search(text: &str) -> Result<Vec<Feed>, String> {
    let found: Feeds = ask(&format!("search/byterm?q={}&max={PAGE}", encode(text)))?;
    Ok(found.feeds.into_iter().filter(|f| f.dead == 0).collect())
}

/// A show's episodes, the newest first, past the first `offset`.
pub fn episodes(feed: &str, offset: usize) -> Result<Vec<Item>, String> {
    let found: Items = ask(&format!("episodes/byfeedid?id={}&max={}", encode(feed), reach(offset)))?;
    Ok(found.items.into_iter().skip(offset).collect())
}

/// The same, for a show known by its number in the iTunes directory: a
/// show saved before this plugin read PodcastIndex.
pub fn episodes_of_itunes(id: &str, offset: usize) -> Result<Vec<Item>, String> {
    let found: Items = ask(&format!("episodes/byitunesid?id={}&max={}", encode(id), reach(offset)))?;
    Ok(found.items.into_iter().skip(offset).collect())
}

/// Plain text from the HTML a feed writes its notes in: no tags, the usual
/// entities, and one space where there were several.
pub fn plain(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    let mut chars = html.chars().peekable();
    while let Some(c) = chars.next() {
        // A "<" opens a tag only when a tag could follow it: "a < b" is text.
        let opens = c == '<' && chars.peek().is_some_and(|n| n.is_ascii_alphabetic() || *n == '/' || *n == '!');
        match c {
            '<' if opens => {
                in_tag = true;
                out.push(' ');
            }
            '>' if in_tag => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    let decoded = out
        .replace("&nbsp;", " ")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&");
    decoded.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_page_asks_for_what_reaches_it_and_never_past_the_most() {
        assert_eq!(reach(0), 20);
        assert_eq!(reach(20), 40);
        assert_eq!(reach(990), MOST);
    }

    #[test]
    fn notes_lose_their_tags_and_keep_their_words() {
        assert_eq!(plain("<p>One &amp; two</p><br/>three&nbsp; four"), "One & two three four");
        assert_eq!(plain("Plain already"), "Plain already");
        assert_eq!(plain("a < b, so"), "a < b, so", "a bare < is text");
    }
}
