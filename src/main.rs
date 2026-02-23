#![allow(unused_assignments)]

mod album_grid;
mod app;
mod bandcamp;
mod discover;
mod library;
mod login;
mod player;
mod search;
mod storage;

use app::App;
use relm4::prelude::*;

fn main() {
    relm4::RELM_THREADS.set(4).ok();
    let app = RelmApp::new("io.github.camper");
    app.run::<App>(());
}
