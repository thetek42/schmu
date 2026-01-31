# Schmu

A simple music player which allows listeners to suggest songs.

## Usage

When the Schmu client is started, it automatically connects to the Schmu server. By connecting, the
client receives a unique ID. In order to use a consistent or simply meaningful ID, one can request
an ID using the `--request-id` command line parameter. This ID is then used to display a QR code in
the graphical user interface, which directs users to the song submission page. There, they can
search for a song, which will then be added to the queue of the client. After the client finishes
downloading the song from YouTube Music, it will be played.

A fallback playlist that plays songs while there are no pending requests can be specified with the
`--fallback-playlist <PATH>` option. The path must point to a file that contains one YouTube video
ID per line.

## Client Controls

| Key    | Scope         | Description                      |
| ------ | ------------- | -------------------------------- |
| 1 .. 9 | Anywhere      | Enable edit mode for song 1 .. 9 |
| Escape | Edit mode     | Quit edit mode                   |
| D      | Edit mode     | Delete song                      |
| J      | Edit mode     | Move song down                   |
| K      | Edit mode     | Move song up                     |
| N      | Not edit mode | Next song                        |
| Space  | Not edit mode | Toggle pause                     |
| Q      | Not edit mode | Decrease QR contrast             |
| W      | Not edit mode | Increase QR contrast             |
| A      | Not edit mode | Decrease QR size                 |
| S      | Not edit mode | Increase QR size                 |

## Prerequisites

These applications must be installed on your system in addition to the Schmu client:
- MPV
- yt-dlp

## Compiling

Prerequisites:
- Rust

```
git clone https://git.tjdev.de/thetek/schmu.git
cd schmu/client
cargo build --release
```
