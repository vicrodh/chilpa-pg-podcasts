# chilpa-pg-podcasts

Podcasts for Chilpa: search shows, browse trending shows by language and
category, and play audio episodes. The plugin reads PodcastIndex through
Chilpa's proxy, which signs directory requests. Listeners need no account.
Chilpa fetches and plays each publisher's episode URL and keeps playback
progress.

## What it does

- The root shows trending podcasts in Chilpa's language, a search entry,
  and language and category filters.
- A show's folder lists episodes newest first, with duration, date, and notes.
  Results paginate up to the index limit of 200 episodes.
- Episode identity stays tied to the publisher's enclosure URL. Resolving
  an audio episode makes no additional directory request.
- Video episodes are listed as unavailable for audio playback.

## Building

```sh
rustup target add wasm32-unknown-unknown
tools/build-plugin . <out-dir>
tools/install-dev <out-dir>/app.chilpa.podcasts <test-XDG_DATA_HOME>
cargo test
```

The SDK comes from [chilpa-plugins](https://github.com/vicrodh/chilpa-plugins),
pinned to the `protocol-4` tag in `Cargo.toml`. Recorded service replies
exercise the directory paths without network access.
