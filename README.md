# Schmu

A simple music player which allows listeners to suggest songs.

## Usage

When the Schmu client is started, it automatically connects to the Schmu server. By connecting, the
client receives a unique ID. In order to use a consistent or simply meaningful ID, one can request
an ID using the `--request-id` command line parameter. This ID is then used to display a QR code in
the graphical user interface, which directs users to the song submission page. There, they can
search for a song, which will then be added to the queue of the client. After the client finishes
downloading the song from YouTube Music, it will be played.

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
