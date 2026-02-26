use crate::album_grid::{AlbumData, AlbumGrid, AlbumGridMsg, AlbumGridOutput};
use crate::bandcamp::{BandcampClient, DiscoverParams, GENRES, SORT_OPTIONS};
use gtk4::prelude::*;
use relm4::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FetchMode {
    Fresh,
    LoadMore,
}

pub struct DiscoverPage {
    client: Option<BandcampClient>,
    grid: Controller<AlbumGrid>,
    params: DiscoverParams,
    loading: bool,
    fetch_mode: FetchMode,
}

#[derive(Debug)]
pub enum DiscoverMsg {
    SetClient(BandcampClient),
    Refresh,
    LoadMore,
    SetGenre(u32),
    SetTag(String),
    SetSort(u32),

    Loaded(Result<Vec<AlbumData>, String>),
    GridAction(AlbumGridOutput),
}

#[derive(Debug)]
pub enum DiscoverOutput {
    Play(AlbumData),
    GenreChanged(u32),
    TagChanged(String),
    SortChanged(u32),
}

#[relm4::component(pub)]
impl Component for DiscoverPage {
    type Init = ();
    type Input = DiscoverMsg;
    type Output = DiscoverOutput;
    type CommandOutput = Result<Vec<AlbumData>, String>;

    view! {
        gtk4::Box {
            set_orientation: gtk4::Orientation::Vertical,
            set_hexpand: true,
            set_vexpand: true,
        }
    }

    fn init(_: Self::Init, root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let grid = AlbumGrid::builder()
            .launch(())
            .forward(sender.input_sender(), DiscoverMsg::GridAction);

        let model = Self {
            client: None,
            grid,
            params: DiscoverParams::default(),
            loading: false,
            fetch_mode: FetchMode::Fresh,
        };

        let widgets = view_output!();
        root.append(model.grid.widget());
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            DiscoverMsg::SetClient(client) => {
                self.client = Some(client);
                sender.input(DiscoverMsg::Refresh);
            }
            DiscoverMsg::Refresh => {
                self.params.page = 0;
                self.fetch_mode = FetchMode::Fresh;
                self.fetch(sender.clone());
            }
            DiscoverMsg::LoadMore => {
                if !self.loading {
                    self.params.page += 1;
                    self.fetch_mode = FetchMode::LoadMore;
                    self.fetch(sender.clone());
                }
            }
            DiscoverMsg::SetGenre(i) => {
                if let Some((k, _)) = GENRES.get(i as usize) {
                    self.params.genre = k.to_string();
                    self.params.tag = String::new();
                    sender.output(DiscoverOutput::GenreChanged(i)).ok();
                    sender.output(DiscoverOutput::TagChanged(String::new())).ok();
                    sender.input(DiscoverMsg::Refresh);
                }
            }
            DiscoverMsg::SetTag(tag) => {
                self.params.tag = tag.clone();
                sender.output(DiscoverOutput::TagChanged(tag)).ok();
                sender.input(DiscoverMsg::Refresh);
            }
            DiscoverMsg::SetSort(i) => {
                if let Some((k, _)) = SORT_OPTIONS.get(i as usize) {
                    self.params.sort = k.to_string();
                    sender.output(DiscoverOutput::SortChanged(i)).ok();
                    sender.input(DiscoverMsg::Refresh);
                }
            }
            DiscoverMsg::Loaded(result) => {
                self.loading = false;
                match result {
                    Ok(albums) => match self.fetch_mode {
                        FetchMode::Fresh => self.grid.emit(AlbumGridMsg::Replace(albums)),
                        FetchMode::LoadMore => self.grid.emit(AlbumGridMsg::Append(albums)),
                    },
                    Err(e) => eprintln!("Discover fetch failed: {e}"),
                }
            }
            DiscoverMsg::GridAction(action) => match action {
                AlbumGridOutput::Clicked(data) => {
                    sender.output(DiscoverOutput::Play(data)).ok();
                }
                AlbumGridOutput::ScrolledToBottom => {
                    sender.input(DiscoverMsg::LoadMore);
                }
            },
        }
    }

    fn update_cmd(&mut self, msg: Self::CommandOutput, sender: ComponentSender<Self>, _root: &Self::Root) {
        sender.input(DiscoverMsg::Loaded(msg));
    }
}

impl DiscoverPage {
    fn fetch(&mut self, sender: ComponentSender<Self>) {
        let Some(client) = self.client.clone() else { return };
        self.loading = true;
        let params = self.params.clone();
        sender.oneshot_command(async move {
            client
                .discover(&params)
                .await
                .map(|albums| {
                    albums
                        .into_iter()
                        .map(|a| AlbumData {
                            title: a.title,
                            artist: a.artist,
                            genre: a.genre,
                            art_url: a.art_url,
                            url: a.url,
                            band_id: a.band_id,
                            item_id: a.item_id,
                            item_type: a.item_type,
                        })
                        .collect()
                })
                .map_err(|e| e.to_string())
        });
    }
}

pub fn build_toolbar(
    sender: &relm4::Sender<DiscoverMsg>,
    ui_state: &crate::storage::UiState,
) -> gtk4::Box {
    let toolbar = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    toolbar.add_css_class("compact-toolbar");

    let genre_dd = gtk4::DropDown::new(
        Some(gtk4::StringList::new(
            &GENRES.iter().map(|(_, l)| *l).collect::<Vec<_>>(),
        )),
        None::<gtk4::Expression>,
    );
    if let Some(i) = ui_state.discover_genre {
        genre_dd.set_selected(i);
    }
    toolbar.append(&genre_dd);

    let tag_entry = gtk4::SearchEntry::new();
    tag_entry.set_placeholder_text(Some("Tag filter..."));
    tag_entry.set_max_width_chars(16);
    if let Some(ref t) = ui_state.discover_tag {
        tag_entry.set_text(t);
    }
    toolbar.append(&tag_entry);

    let tag_entry_for_genre = tag_entry.clone();
    let s = sender.clone();
    genre_dd.connect_selected_notify(move |dd| {
        tag_entry_for_genre.set_text("");
        s.emit(DiscoverMsg::SetGenre(dd.selected()));
    });

    let s = sender.clone();
    tag_entry.connect_activate(move |entry| {
        let text = entry
            .text()
            .to_string()
            .trim()
            .to_lowercase()
            .replace(' ', "-");
        s.emit(DiscoverMsg::SetTag(text));
    });

    let sort_dd = gtk4::DropDown::new(
        Some(gtk4::StringList::new(
            &SORT_OPTIONS.iter().map(|(_, l)| *l).collect::<Vec<_>>(),
        )),
        None::<gtk4::Expression>,
    );
    if let Some(i) = ui_state.discover_sort {
        sort_dd.set_selected(i);
    }
    let s = sender.clone();
    sort_dd.connect_selected_notify(move |dd| {
        s.emit(DiscoverMsg::SetSort(dd.selected()));
    });
    toolbar.append(&sort_dd);

    toolbar
}
