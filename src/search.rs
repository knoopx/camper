use crate::album_grid::{AlbumData, AlbumGrid, AlbumGridMsg, AlbumGridOutput};
use crate::bandcamp::BandcampClient;
use gtk4::prelude::*;
use relm4::prelude::*;

pub struct SearchPage {
    client: Option<BandcampClient>,
    grid: Controller<AlbumGrid>,
    query: String,
    loading: bool,
}

#[derive(Debug)]
pub enum SearchMsg {
    SetClient(BandcampClient),
    Submit,
    QueryChanged(String),
    Loaded(Result<Vec<AlbumData>, String>),
    GridAction(AlbumGridOutput),
}

#[derive(Debug)]
pub enum SearchOutput {
    Play(String),
    QueryChanged(String),
}

#[relm4::component(pub)]
impl Component for SearchPage {
    type Init = ();
    type Input = SearchMsg;
    type Output = SearchOutput;
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
            .forward(sender.input_sender(), SearchMsg::GridAction);

        let model = Self {
            client: None,
            grid,
            query: String::new(),
            loading: false,
        };

        let widgets = view_output!();
        root.append(model.grid.widget());
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            SearchMsg::SetClient(client) => {
                self.client = Some(client);
            }
            SearchMsg::QueryChanged(q) => {
                self.query = q.clone();
                sender.output(SearchOutput::QueryChanged(q)).ok();
            }
            SearchMsg::Submit => {
                if self.query.trim().is_empty() || self.loading {
                    return;
                }
                self.grid.emit(AlbumGridMsg::Clear);
                self.fetch(sender.clone());
            }
            SearchMsg::Loaded(result) => {
                self.loading = false;
                if let Ok(albums) = result {
                    self.grid.emit(AlbumGridMsg::Append(albums));
                }
            }
            SearchMsg::GridAction(action) => match action {
                AlbumGridOutput::Clicked(data) => {
                    sender.output(SearchOutput::Play(data.url)).ok();
                }
                AlbumGridOutput::ScrolledToBottom => {}
            },
        }
    }

    fn update_cmd(&mut self, msg: Self::CommandOutput, sender: ComponentSender<Self>, _root: &Self::Root) {
        sender.input(SearchMsg::Loaded(msg));
    }
}

impl SearchPage {
    fn fetch(&mut self, sender: ComponentSender<Self>) {
        let Some(client) = self.client.clone() else { return };
        self.loading = true;
        let query = self.query.clone();
        sender.oneshot_command(async move {
            client
                .search(&query)
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
                        })
                        .collect()
                })
                .map_err(|e| e.to_string())
        });
    }
}

pub fn build_toolbar(sender: &relm4::Sender<SearchMsg>, ui_state: &crate::storage::UiState) -> gtk4::Box {
    let toolbar = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);

    let entry = gtk4::SearchEntry::new();
    entry.set_placeholder_text(Some("Search albums..."));
    entry.set_width_request(300);
    if let Some(ref q) = ui_state.search_query {
        entry.set_text(q);
    }
    let s = sender.clone();
    entry.connect_search_changed(move |e| {
        s.emit(SearchMsg::QueryChanged(e.text().to_string()));
    });
    let s = sender.clone();
    entry.connect_activate(move |_| {
        s.emit(SearchMsg::Submit);
    });
    toolbar.append(&entry);

    toolbar
}
