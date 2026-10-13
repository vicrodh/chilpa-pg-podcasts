//! Podcasts for Chilpa.
//!
//! It reads PodcastIndex, an open index of podcasts, through Chilpa's proxy,
//! which signs each request. The plugin only lists and points: Chilpa
//! fetches the episode, plays it, and remembers where the listener was.
//!
//! The root is the shows most listened to now, narrowed by two columns the
//! listener picks in: language and category. The index filters by both at
//! once. A show is a folder whose entries are its episodes.
//!
//! ```text
//! (root)                shows, narrowed by the picks
//!   search              shows by name, author or owner
//! feed:<number>/        a show, whose entries are its episodes
//! show:<number>/        the same, known by its iTunes number (saved earlier)
//! ep:<address>          an episode, which is what plays
//! ```
//!
//! An episode's id is the address its publisher gave for it. Resolving it
//! asks nobody: the id says where the file is.

mod directory;
mod words;

use chilpa_plugin_sdk::api::{
    Browse, Column, Entry, EntryKind, Fact, Listing, Query, RemoteAlbum, Resolve, Results, Stream,
};
use chilpa_plugin_sdk::plugin;
use directory::{Feed, Item, PAGE};
use std::collections::BTreeMap;
use words::Words;

/// How much of a show's or an episode's notes travels with a listing.
const NOTES: usize = 32_768;

const FEED: &str = "feed:";
const ITUNES: &str = "show:";
const EPISODE: &str = "ep:";

fn folder(id: &str, title: &str) -> Entry {
    Entry { id: id.to_string(), title: title.to_string(), playable: true, ..Entry::default() }
}

/// Cuts a text at a character, not in the middle of one.
fn cut(text: &str, most: usize) -> String {
    let text = text.trim();
    if text.chars().count() <= most {
        return text.to_string();
    }
    let kept: String = text.chars().take(most).collect();
    format!("{}…", kept.trim_end())
}

fn show_entry(feed: &Feed, words: &Words) -> Option<Entry> {
    if feed.id == 0 || feed.title.trim().is_empty() || feed.dead != 0 {
        return None;
    }
    let categories = feed.category_names().join(", ");
    let mut facts = vec![
        Fact::new(words.language, &words.language_name(feed.language.trim())),
        Fact::new(words.categories, &categories),
    ];
    if feed.episode_count > 0 {
        facts.push(Fact::new(words.episodes, &feed.episode_count.to_string()));
    }
    let site = feed.link.trim();
    if site.starts_with("https://") || site.starts_with("http://") {
        facts.push(Fact { label: words.website.into(), value: site.into(), url: Some(site.into()) });
    }
    facts.push(Fact::new(words.directory, "PodcastIndex"));
    facts.retain(|fact| !fact.value.trim().is_empty());
    Some(Entry {
        subtitle: feed.author.trim().to_string(),
        image_url: feed.picture(),
        description: cut(&directory::plain(&feed.description), NOTES),
        facts,
        ..folder(&format!("{FEED}{}", feed.id), feed.title.trim())
    })
}

fn episode_entry(item: &Item, words: &Words) -> Option<Entry> {
    let address = item.enclosure_url.trim();
    if item.title.trim().is_empty() || !(address.starts_with("https://") || address.starts_with("http://")) {
        return None;
    }
    // Video is listed by some shows; Chilpa plays sound.
    let video = item.enclosure_type.starts_with("video/");
    let mut facts = Vec::new();
    if let (Some(season), true) = (item.season, item.season.unwrap_or(0) > 0) {
        facts.push(Fact::new(words.season, &season.to_string()));
    }
    let extras = chilpa_plugin_sdk::serde_json::json!({
        "season":item.season.map(|value| value.to_string()).unwrap_or_default(),
        "episode_type":item.episode_type.as_deref().unwrap_or("full"), "file_size":item.enclosure_length.unwrap_or(0).to_string(),
        "chapters_url":item.chapters_url.as_deref().unwrap_or_default(), "transcript_url":item.transcript(),
        "people":chilpa_plugin_sdk::serde_json::to_string(&item.persons.as_deref().unwrap_or_default()).unwrap_or_default()
    });
    if let Ok(bytes) = chilpa_plugin_sdk::serde_json::to_vec(&extras) {
        chilpa_plugin_sdk::host::store_set(&format!("episode-extra-{}", item.id), &bytes);
    }
    facts.push(Fact::new("Episode reference", &item.id.to_string()));
    Some(Entry {
        id: format!("{EPISODE}{address}"),
        title: item.title.trim().to_string(),
        image_url: item.picture(),
        kind: EntryKind::Item,
        live: false,
        duration_ms: item.duration * 1000,
        published_at: (item.date_published > 0).then_some(item.date_published),
        playable: !video,
        why_not: if video { "Video episodes are not playable.".into() } else { String::new() },
        description: cut(&directory::plain(&item.description), NOTES),
        facts,
        ..Entry::default()
    })
}

/// A page of a list given by count, with the cursor to the next one when
/// this one came full and the most has not been reached.
fn page(entries: Vec<Entry>, offset: usize) -> Option<String> {
    let reached = offset + PAGE;
    (entries.len() >= PAGE && reached < directory::MOST).then(|| reached.to_string())
}

/// What was picked, kept to what is offered: a pick goes into an address.
fn picked<'a>(picks: &'a BTreeMap<String, String>, words: &Words) -> (Option<&'a str>, Option<&'a str>) {
    let language = picks.get("language").map(String::as_str).filter(|l| words.offers_language(l));
    let category = picks.get("category").map(String::as_str).filter(|c| words.offers_topic(c));
    (language, category)
}

pub fn browse(ask: Browse) -> Result<Listing, String> {
    let words = Words::of(&ask.language);
    let offset: usize = ask.cursor.as_deref().and_then(|c| c.parse().ok()).unwrap_or(0);
    let at = ask.at.as_str();

    if at.is_empty() {
        let first = ask.cursor.is_none() && ask.picks.is_empty();
        let (language, category) = picked(&ask.picks, words);
        // With nothing picked yet the root starts in the window's language.
        let language = if ask.picks.contains_key("language") { language } else { Some(words.own) };
        let shows: Vec<Entry> = directory::trending(language, category, offset)?
            .iter()
            .filter_map(|feed| show_entry(feed, words))
            .collect();
        let next = page(shows.clone(), offset);
        let mut entries = shows;
        if ask.cursor.is_none() {
            entries.insert(0, Entry { kind: EntryKind::Search, ..folder("search", words.search) });
        }
        let column = |id: &str, title: &str, items: &[(&str, &str)], preset: Option<&str>| Column {
            id: id.into(),
            title: title.into(),
            entries: items.iter().map(|(key, name)| folder(key, name)).collect(),
            preset: preset.map(str::to_string),
        };
        let columns = if first {
            vec![
                column("language", words.language, words.languages, Some(words.own)),
                column("category", words.category, words.topics, None),
            ]
        } else {
            Vec::new()
        };
        return Ok(Listing { title: "Podcasts".into(), entries, next, columns });
    }
    if at == "search" {
        let text = ask.text.unwrap_or_default();
        let text = text.trim();
        if text.is_empty() {
            return Ok(Listing { title: words.search.into(), ..Listing::default() });
        }
        // A search stays inside the language picked, which the index does
        // not filter by: the shows it found are.
        let (language, _) = picked(&ask.picks, words);
        let entries = directory::search(text)?
            .iter()
            .filter(|feed| language.is_none_or(|l| feed.language.to_ascii_lowercase().starts_with(l)))
            .filter_map(|feed| show_entry(feed, words))
            .collect();
        return Ok(Listing { title: text.to_string(), entries, ..Listing::default() });
    }
    let items = if let Some(id) = at.strip_prefix(FEED) {
        directory::episodes(id, offset)?
    } else if let Some(id) = at.strip_prefix(ITUNES) {
        directory::episodes_of_itunes(id, offset)?
    } else {
        return Err(format!("there is no \"{at}\" here"));
    };
    let entries: Vec<Entry> = items.iter().filter_map(|item| episode_entry(item, words)).collect();
    // The index answers by count, so a full page is what says more may follow.
    let next = (items.len() >= PAGE && offset + PAGE < directory::MOST).then(|| (offset + PAGE).to_string());
    Ok(Listing { entries, next, ..Listing::default() })
}

/// Shows by name, for Chilpa's own search: a show is the nearest thing to
/// a record it has.
pub fn search(query: Query) -> Result<Results, String> {
    let text = query.text.trim();
    if text.is_empty() {
        return Ok(Results::default());
    }
    let albums = directory::search(text)?
        .into_iter()
        .filter(|feed| feed.id != 0)
        .take(query.limit)
        .map(|feed| RemoteAlbum {
            external_id: format!("{FEED}{}", feed.id),
            title: feed.title.trim().to_string(),
            artist: feed.author.trim().to_string(),
            track_count: feed.episode_count as usize,
            artwork_url: feed.picture(),
            ..RemoteAlbum::default()
        })
        .collect();
    Ok(Results { albums, ..Results::default() })
}

pub fn resolve(ask: Resolve) -> Result<Stream, String> {
    let address = ask
        .external_id
        .strip_prefix(EPISODE)
        .filter(|a| a.starts_with("https://") || a.starts_with("http://"))
        .ok_or_else(|| format!("\"{}\" is not an episode", ask.external_id))?;
    // What the file is, from its name: the address often ends in tracking
    // parameters, so the name is what comes before them.
    let name = address.split(['?', '#']).next().unwrap_or(address).to_ascii_lowercase();
    let codec = ["mp3", "m4a", "aac", "ogg", "opus", "flac", "wav"]
        .into_iter()
        .find(|extension| name.ends_with(&format!(".{extension}")))
        .map(str::to_string);
    Ok(Stream { url: address.to_string(), codec, ..Stream::default() })
}

plugin! {
    api_version: 1,
    chilpa_browse => browse_document,
    chilpa_search => search,
    chilpa_resolver => resolve,
}

/// Names the medium so the host never has to recognize the provider.
pub fn browse_document(ask: Browse) -> Result<chilpa_plugin_sdk::serde_json::Value, String> {
    let season_label = Words::of(&ask.language).season;
    let mut listing = browse(ask)?;
    let mut presentation = chilpa_plugin_sdk::serde_json::Map::new();
    for entry in &listing.entries {
        let season = entry.facts.iter().find(|fact| fact.label == season_label).map(|fact| fact.value.as_str()).unwrap_or("");
        let metadata = entry.facts.iter().find(|fact| fact.label == "Episode reference")
            .and_then(|fact| chilpa_plugin_sdk::host::store_get(&format!("episode-extra-{}", fact.value)))
            .and_then(|bytes| chilpa_plugin_sdk::serde_json::from_slice::<chilpa_plugin_sdk::serde_json::Value>(&bytes).ok())
            .unwrap_or_else(|| chilpa_plugin_sdk::serde_json::json!({"season":season}));
        presentation.insert(entry.id.clone(), chilpa_plugin_sdk::serde_json::json!({"metadata":metadata}));
    }
    for entry in &mut listing.entries { entry.facts.retain(|fact| fact.label != "Episode reference"); }
    let mut value = chilpa_plugin_sdk::serde_json::to_value(listing).map_err(|error| error.to_string())?;
    value["presentation"] = presentation.into();
    value["medium"] = "podcasts".into();
    value["attribution"] = "Podcast data from PodcastIndex".into();
    Ok(value)
}
