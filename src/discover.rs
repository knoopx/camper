use crate::album_grid::{AlbumData, AlbumGrid, AlbumGridMsg, AlbumGridOutput};
use crate::bandcamp::{BandcampClient, DiscoverParams, GENRES, SORT_OPTIONS, FORMAT_OPTIONS, subgenres_for};
use gtk4::prelude::*;
use relm4::prelude::*;

pub struct DiscoverPage {
    client: Option<BandcampClient>,
    grid: Controller<AlbumGrid>,
    params: DiscoverParams,
    loading: bool,
}

#[derive(Debug)]
pub enum DiscoverMsg {
    SetClient(BandcampClient),
    Refresh,
    LoadMore,
    SetGenre(u32),
    SetSubgenre(u32),
    SetSort(u32),
    SetFormat(u32),
    Loaded(Result<Vec<AlbumData>, String>),
    GridAction(AlbumGridOutput),
}

#[derive(Debug)]
pub enum DiscoverOutput {
    Play(String),
    GenreChanged(u32),
    SubgenreChanged(u32),
    SortChanged(u32),
    FormatChanged(u32),
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
                self.grid.emit(AlbumGridMsg::Clear);
                self.fetch(sender.clone());
            }
            DiscoverMsg::LoadMore => {
                if !self.loading {
                    self.params.page += 1;
                    self.fetch(sender.clone());
                }
            }
            DiscoverMsg::SetGenre(i) => {
                if let Some((k, _)) = GENRES.get(i as usize) {
                    self.params.genre = k.to_string();
                    self.params.subgenre = 0;
                    sender.output(DiscoverOutput::GenreChanged(i)).ok();
                    sender.output(DiscoverOutput::SubgenreChanged(0)).ok();
                    sender.input(DiscoverMsg::Refresh);
                }
            }
            DiscoverMsg::SetSubgenre(i) => {
                let subs = subgenres_for(&self.params.genre);
                if i == 0 {
                    self.params.subgenre = 0;
                } else if let Some((id, _)) = subs.get((i - 1) as usize) {
                    self.params.subgenre = *id;
                }
                sender.output(DiscoverOutput::SubgenreChanged(i)).ok();
                sender.input(DiscoverMsg::Refresh);
            }
            DiscoverMsg::SetSort(i) => {
                if let Some((k, _)) = SORT_OPTIONS.get(i as usize) {
                    self.params.sort = k.to_string();
                    sender.output(DiscoverOutput::SortChanged(i)).ok();
                    sender.input(DiscoverMsg::Refresh);
                }
            }
            DiscoverMsg::SetFormat(i) => {
                if let Some((k, _)) = FORMAT_OPTIONS.get(i as usize) {
                    self.params.format = k.to_string();
                    sender.output(DiscoverOutput::FormatChanged(i)).ok();
                    sender.input(DiscoverMsg::Refresh);
                }
            }
            DiscoverMsg::Loaded(result) => {
                self.loading = false;
                if let Ok(albums) = result {
                    self.grid.emit(AlbumGridMsg::Append(albums));
                }
            }
            DiscoverMsg::GridAction(action) => match action {
                AlbumGridOutput::Clicked(data) => {
                    sender.output(DiscoverOutput::Play(data.url)).ok();
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
            client.discover(&params).await
                .map(|albums| albums.into_iter().map(|a| AlbumData {
                    title: a.title,
                    artist: a.artist,
                    genre: a.genre,
                    art_url: a.art_url,
                    url: a.url,
                }).collect())
                .map_err(|e| e.to_string())
        });
    }
}

pub fn build_toolbar(sender: &relm4::Sender<DiscoverMsg>, ui_state: &crate::storage::UiState) -> gtk4::Box {
    let toolbar = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);

    let genre_dd = gtk4::DropDown::new(
        Some(gtk4::StringList::new(&GENRES.iter().map(|(_, l)| *l).collect::<Vec<_>>())),
        None::<gtk4::Expression>,
    );
    if let Some(i) = ui_state.discover_genre {
        genre_dd.set_selected(i);
    }
    toolbar.append(&genre_dd);

    // Subgenre dropdown — populated based on selected genre
    let subgenre_dd = gtk4::DropDown::new(
        Some(gtk4::StringList::new(&["All"])),
        None::<gtk4::Expression>,
    );
    toolbar.append(&subgenre_dd);

    // Populate subgenre for initial genre
    let initial_genre_idx = ui_state.discover_genre.unwrap_or(0) as usize;
    if let Some((slug, _)) = GENRES.get(initial_genre_idx) {
        populate_subgenre_dropdown(&subgenre_dd, slug);
        if let Some(i) = ui_state.discover_subgenre {
            subgenre_dd.set_selected(i);
        }
    }

    // Genre change updates subgenre list
    let sub_dd = subgenre_dd.clone();
    let s = sender.clone();
    genre_dd.connect_selected_notify(move |dd| {
        let idx = dd.selected();
        if let Some((slug, _)) = GENRES.get(idx as usize) {
            populate_subgenre_dropdown(&sub_dd, slug);
        }
        s.emit(DiscoverMsg::SetGenre(idx));
    });

    let s = sender.clone();
    subgenre_dd.connect_selected_notify(move |dd| { s.emit(DiscoverMsg::SetSubgenre(dd.selected())); });

    let sort_dd = gtk4::DropDown::new(
        Some(gtk4::StringList::new(&SORT_OPTIONS.iter().map(|(_, l)| *l).collect::<Vec<_>>())),
        None::<gtk4::Expression>,
    );
    if let Some(i) = ui_state.discover_sort {
        sort_dd.set_selected(i);
    }
    let s = sender.clone();
    sort_dd.connect_selected_notify(move |dd| { s.emit(DiscoverMsg::SetSort(dd.selected())); });
    toolbar.append(&sort_dd);

    let format_dd = gtk4::DropDown::new(
        Some(gtk4::StringList::new(&FORMAT_OPTIONS.iter().map(|(_, l)| *l).collect::<Vec<_>>())),
        None::<gtk4::Expression>,
    );
    if let Some(i) = ui_state.discover_format {
        format_dd.set_selected(i);
    }
    let s = sender.clone();
    format_dd.connect_selected_notify(move |dd| { s.emit(DiscoverMsg::SetFormat(dd.selected())); });
    toolbar.append(&format_dd);

    toolbar
}

fn populate_subgenre_dropdown(dd: &gtk4::DropDown, genre_slug: &str) {
    let subs = subgenres_for(genre_slug);
    let mut labels: Vec<&str> = vec!["All"];
    labels.extend(subs.iter().map(|(_, l)| *l));
    dd.set_model(Some(&gtk4::StringList::new(&labels)));
    dd.set_selected(0);
}
