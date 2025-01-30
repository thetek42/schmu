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

Instead of modifying the source code and providing a cookie file in steps 3 and 4, you can also set
the `SCHMU_SERVER_WEBSOCKET_PORT`, `SCHMU_SERVER_WEBSERVER_PORT` and `SCHMU_SERVER_YTAPI_COOKIE`
environment variables.
