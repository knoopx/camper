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
    let app = RelmApp::new("net.knoopx.camper");
    gtk4::Window::set_default_icon_name("camper");
    app.run::<App>(());
}
