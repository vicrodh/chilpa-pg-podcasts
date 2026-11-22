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
cargo fetch --locked
tools/build-plugin . <out-dir>
tools/install-dev <out-dir>/app.chilpa.podcasts <test-XDG_DATA_HOME>
cargo test --locked --offline
```

The SDK comes from [chilpa-plugins-sdk](https://github.com/vicrodh/chilpa-plugins-sdk),
pinned to revision `dc88a629ab646877a7328ee38ea4fbfe7669b362` in `Cargo.toml`. Recorded service replies
exercise the directory paths without network access.

The build helper uses Python 3 and packages the portable WASM module with its manifest.

## Release packages

The Package checks workflow runs the tests and builds one portable WASM ZIP
for native and Flatpak hosts. Its artifact includes the ZIP, SHA-256 and an
archive validation report. Releases are published after exact-package host
checks; a successful CI build does not publish a release automatically.
