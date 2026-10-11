//! The plugin against replies recorded from PodcastIndex, through Chilpa's
//! proxy, on 2026-10-01.
//!
//! Nothing here goes out: the SDK's test host answers each address with the
//! file recorded for it, and one nobody recorded fails like no network.

use chilpa_pg_podcasts::{browse, resolve, search};
use chilpa_plugin_sdk::api::{Browse, EntryKind, Query, Resolve};
use chilpa_plugin_sdk::testing;
use std::collections::BTreeMap;

const PROXY: &str = "https://api.chilpa.app/podcastindex";

fn recorded(name: &str) -> String {
    std::fs::read_to_string(format!("{}/tests/recorded/{name}.json", env!("CARGO_MANIFEST_DIR")))
        .expect("the recorded reply is in the repository")
}

fn picks(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
}

/// A reply of `n` copies of the first record of a recorded one, each with
/// its own number: a page that came full.
fn full(name: &str, key: &str, n: usize) -> String {
    let mut reply: serde_json::Value = serde_json::from_str(&recorded(name)).unwrap();
    let first = reply[key][0].clone();
    reply[key] = (0..n)
        .map(|i| {
            let mut copy = first.clone();
            copy["id"] = (i as u64 + 1).into();
            if key == "items" {
                copy["enclosureUrl"] = format!("https://a.b/{i}.mp3").into();
            }
            copy
        })
        .collect();
    reply.to_string()
}

#[test]
fn the_root_is_the_trending_shows_of_the_window_s_language_with_two_columns() {
    testing::reset();
    testing::answer(&format!("{PROXY}/podcasts/trending?max=20&lang=es"), 200, &recorded("trending"));
    let root = browse(Browse { language: "es".into(), ..Browse::root() }).unwrap();
    assert_eq!(root.entries[0].kind, EntryKind::Search);
    assert_eq!(root.entries.len(), 4, "the place to type and three shows");
    let show = &root.entries[1];
    assert_eq!((show.id.as_str(), show.title.as_str()), ("feed:220107", "YslaMac"));
    assert_eq!(show.kind, EntryKind::Folder);
    assert!(show.image_url.as_deref().unwrap().starts_with("https://"));
    assert_eq!(show.facts[0].value, "Español");
    assert!(show.facts[1].value.contains("Technology"));

    let titles: Vec<&str> = root.columns.iter().map(|c| c.title.as_str()).collect();
    assert_eq!(titles, ["Idioma", "Categoría"]);
    assert_eq!(root.columns[0].preset.as_deref(), Some("es"), "it starts on the window's language");
    assert!(root.columns[1].entries.iter().any(|c| c.id == "True Crime" && c.title == "True crime"));
    assert_eq!(root.next, None, "three is not a full page");
}

#[test]
fn picks_narrow_by_language_and_category_and_all_languages_is_a_pick_too() {
    testing::reset();
    testing::answer(&format!("{PROXY}/podcasts/trending?max=20&lang=pt&cat=News"), 200, &recorded("trending"));
    let narrowed = browse(Browse { picks: picks(&[("language", "pt"), ("category", "News")]), ..Browse::root() }).unwrap();
    assert_eq!(narrowed.entries.len(), 4);
    assert!(narrowed.columns.is_empty(), "the columns came with the first page");

    // "All" in the language column comes as an empty pick: no language.
    testing::answer(&format!("{PROXY}/podcasts/trending?max=20"), 200, &recorded("trending"));
    let every = browse(Browse { picks: picks(&[("language", "")]), ..Browse::root() }).unwrap();
    assert_eq!(every.entries.len(), 4);

    // What is not offered is not sent.
    testing::answer(&format!("{PROXY}/podcasts/trending?max=20&lang=en"), 200, &recorded("trending"));
    let odd = browse(Browse { picks: picks(&[("category", "x&lang=zz")]), ..Browse::root() }).unwrap();
    assert_eq!(odd.entries.len(), 4);
    assert_eq!(testing::asked().last().unwrap(), &format!("{PROXY}/podcasts/trending?max=20&lang=en"));
}

#[test]
fn the_next_page_asks_for_what_reaches_it_and_skips_what_was_shown() {
    testing::reset();
    testing::answer(&format!("{PROXY}/podcasts/trending?max=20&lang=en"), 200, &full("trending", "feeds", 20));
    let first = browse(Browse::root()).unwrap();
    assert_eq!(first.next.as_deref(), Some("20"));
    testing::answer(&format!("{PROXY}/podcasts/trending?max=40&lang=en"), 200, &full("trending", "feeds", 35));
    let second = browse(Browse { cursor: first.next, ..Browse::root() }).unwrap();
    assert_eq!(second.entries.len(), 15, "the twenty already shown are skipped, and no place to type again");
    assert_eq!(second.entries[0].id, "feed:21");
    assert_eq!(second.next, None, "fifteen is not a full page");
}

#[test]
fn a_show_lists_its_episodes_with_a_length_a_day_and_its_notes() {
    testing::reset();
    testing::answer(&format!("{PROXY}/episodes/byfeedid?id=634274&max=20"), 200, &recorded("episodes"));
    let show = browse(Browse::at("feed:634274")).unwrap();
    assert_eq!(show.entries.len(), 3);
    let first = &show.entries[0];
    assert!(first.title.starts_with("Desde Central"));
    assert_eq!(first.kind, EntryKind::Item);
    assert!(!first.live && first.playable);
    assert_eq!(first.duration_ms, 2_194_000);
    assert_eq!(first.published_at, Some(1_789_013_700));
    assert!(first.id.starts_with("ep:https://"));
    assert!(!first.description.contains('<'), "the notes come as text: {}", first.description);
    assert_eq!(show.entries[2].facts[0].value, "15", "its season");

    testing::set_clock(301);
    testing::answer(&format!("{PROXY}/episodes/byfeedid?id=634274&max=20"), 200, &full("episodes", "items", 20));
    assert_eq!(browse(Browse::at("feed:634274")).unwrap().next.as_deref(), Some("20"));
}

#[test]
fn a_show_saved_by_its_itunes_number_still_opens() {
    testing::reset();
    testing::answer(&format!("{PROXY}/episodes/byitunesid?id=152249110&max=20"), 200, &recorded("episodes"));
    assert_eq!(browse(Browse::at("show:152249110")).unwrap().entries.len(), 3);
}

#[test]
fn searching_finds_shows_and_stays_in_the_language_picked() {
    testing::reset();
    testing::answer(&format!("{PROXY}/search/byterm?q=radio%20ambulante&max=20"), 200, &recorded("search"));
    let found = browse(Browse { text: Some("radio ambulante".into()), ..Browse::at("search") }).unwrap();
    let show = &found.entries[0];
    assert_eq!((show.id.as_str(), show.title.as_str()), ("feed:634274", "Radio Ambulante"));
    assert!(show.facts.iter().any(|f| f.value == "369"), "how many episodes it has");
    assert!(show.facts.iter().any(|f| f.url.is_some()), "and its website");
    let in_english = browse(Browse {
        text: Some("radio ambulante".into()),
        picks: picks(&[("language", "en")]),
        ..Browse::at("search")
    })
    .unwrap();
    assert!(in_english.entries.iter().all(|e| e.id != "feed:634274"), "a show in Spanish is not one in English");

    let records = search(Query::new("radio ambulante", 10)).unwrap();
    assert_eq!(records.albums[0].title, "Radio Ambulante");
}

#[test]
fn when_the_proxy_does_not_answer_the_reason_is_said() {
    testing::reset();
    assert!(browse(Browse::root()).is_err());
    testing::answer(&format!("{PROXY}/podcasts/trending?max=20&lang=en"), 429, "slow down");
    assert!(browse(Browse::root()).unwrap_err().contains("429"));
    assert!(browse(Browse::at("nowhere")).is_err());
}

#[test]
fn an_episode_resolves_to_its_own_address_without_asking() {
    testing::reset();
    let address = "https://a.b/c/episode.mp3?aid=rss_feed&x=1";
    let stream = resolve(Resolve::new(&format!("ep:{address}"))).unwrap();
    assert_eq!(stream.url, address);
    assert_eq!(stream.codec.as_deref(), Some("mp3"));
    assert!(stream.usable() && stream.delivery.playable());
    assert!(testing::asked().is_empty());
    assert!(resolve(Resolve::new("ep:file:///etc/passwd")).is_err());
    assert!(resolve(Resolve::new("feed:1")).is_err());
}

#[test]
fn medium_document_carries_language_independent_seasons() {
    testing::reset();
    testing::answer(&format!("{PROXY}/episodes/byfeedid?id=634274&max=20"), 200, &recorded("episodes"));
    let document = chilpa_pg_podcasts::browse_document(Browse { language: "es".into(), ..Browse::at("feed:634274") }).unwrap();
    assert_eq!(document["medium"], "podcasts");
    let id = document["entries"][2]["id"].as_str().unwrap();
    assert_eq!(document["presentation"][id]["metadata"]["season"], "15");
}

#[test]
fn returning_to_a_show_or_repeating_search_does_not_hit_the_directory() {
    testing::reset();
    testing::answer(&format!("{PROXY}/episodes/byfeedid?id=634274&max=20"), 200, &recorded("episodes"));
    browse(Browse::at("feed:634274")).unwrap();
    browse(Browse::at("feed:634274")).unwrap();
    assert_eq!(testing::asked().len(), 1);
    testing::answer(&format!("{PROXY}/search/byterm?q=radio&max=20"), 200, &recorded("search"));
    search(Query::new("radio", 10)).unwrap();
    search(Query::new("radio", 10)).unwrap();
    assert_eq!(testing::asked().len(), 2);
}
