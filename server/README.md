# Hosting the Server

## Prerequisites

- Rust

## Setup

1. `git clone https://git.tjdev.de/thetek/schmu.git`
2. `cd schmu`
3. Modify the server address in `shared/src/consts.rs`
4. Obtain a YouTube Music cookie file:
   - Open your browser dev tools, go to network tab
   - Go to `music.youtube.com`
   - Find a post request to `music.youtube.com`
   - Copy the contents of the `Cookie` header in the request
   - Put them into the file `server/cookie.txt`
5. `cd server`
6. `cargo build --release`
7. `target/release/schmu-server`
