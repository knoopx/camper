use crate::album_grid::{AlbumData, AlbumGrid, AlbumGridMsg, AlbumGridOutput};
use crate::bandcamp::{BandcampClient, CollectionItem};
use gtk4::prelude::*;
use relm4::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Sort {
    #[default]
    Date,
    Name,
}

pub struct LibraryPage {
    client: Option<BandcampClient>,
    grid: Controller<AlbumGrid>,
    all_items: Vec<CollectionItem>,
    sort: Sort,
    query: String,
    loading: bool,
}

#[derive(Debug)]
pub enum LibraryMsg {
    SetClient(BandcampClient),
    Refresh,
    SetSort(Sort),
    SetQuery(String),
    Loaded(Result<(Vec<CollectionItem>, Vec<CollectionItem>), String>),
    GridAction(AlbumGridOutput),
}

#[derive(Debug)]
pub enum LibraryOutput {
    Play(String),
    SortChanged(Sort),
    QueryChanged(String),
}

#[relm4::component(pub)]
impl Component for LibraryPage {
    type Init = ();
    type Input = LibraryMsg;
    type Output = LibraryOutput;
    type CommandOutput = Result<(Vec<CollectionItem>, Vec<CollectionItem>), String>;

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
            .forward(sender.input_sender(), LibraryMsg::GridAction);

        let model = Self {
            client: None,
            grid,
            all_items: Vec::new(),
            sort: Sort::Date,
            query: String::new(),
            loading: false,
        };

        let widgets = view_output!();
        root.append(model.grid.widget());
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            LibraryMsg::SetClient(client) => {
                self.client = Some(client);
                sender.input(LibraryMsg::Refresh);
            }
            LibraryMsg::Refresh => {
                self.fetch(sender.clone());
            }
            LibraryMsg::SetSort(sort) => {
                self.sort = sort;
                self.apply_sort();
                sender.output(LibraryOutput::SortChanged(sort)).ok();
            }
            LibraryMsg::SetQuery(q) => {
                self.query = q.clone();
                self.apply_sort();
                sender.output(LibraryOutput::QueryChanged(q)).ok();
            }
            LibraryMsg::Loaded(result) => {
                self.loading = false;
                match result {
                    Ok((collection, wishlist)) => {
                        self.all_items.clear();
                        self.all_items.extend(collection);
                        self.all_items.extend(wishlist);
                        self.apply_sort();
                    }
                    Err(e) => eprintln!("Library fetch failed: {e}"),
                }
            }
            LibraryMsg::GridAction(action) => match action {
                AlbumGridOutput::Clicked(data) => {
                    sender.output(LibraryOutput::Play(data.url)).ok();
                }
                AlbumGridOutput::ScrolledToBottom => {}
            },
        }
    }

    fn update_cmd(&mut self, msg: Self::CommandOutput, sender: ComponentSender<Self>, _root: &Self::Root) {
        sender.input(LibraryMsg::Loaded(msg));
    }
}

impl LibraryPage {
    fn fetch(&mut self, sender: ComponentSender<Self>) {
        let Some(client) = self.client.clone() else { return };
        self.loading = true;

        sender.oneshot_command(async move {
            let collection = client.get_collection().await.map_err(|e| e.to_string())?;
            let wishlist = client.get_wishlist().await.map_err(|e| e.to_string())?;
            Ok((collection, wishlist))
        });
    }

    fn apply_sort(&mut self) {
        let q = self.query.to_lowercase();
        let mut items: Vec<&CollectionItem> = self.all_items.iter()
            .filter(|item| {
                q.is_empty()
                    || item.title.to_lowercase().contains(&q)
                    || item.artist.to_lowercase().contains(&q)
            })
            .collect();
        match self.sort {
            Sort::Date => {} // already in date order from API
            Sort::Name => items.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase())),
        }

        let albums: Vec<AlbumData> = items.iter()
            .map(|item| AlbumData {
                title: item.title.clone(),
                artist: item.artist.clone(),
                genre: None,
                art_url: item.art_url.clone(),
                url: item.url.clone(),
            })
            .collect();

        self.grid.emit(AlbumGridMsg::Replace(albums));
    }
}

pub fn build_toolbar(sender: &relm4::Sender<LibraryMsg>, ui_state: &crate::storage::UiState) -> gtk4::Box {
    let toolbar = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    toolbar.add_css_class("compact-toolbar");

    let entry = gtk4::SearchEntry::new();
    entry.set_placeholder_text(Some("Filter library..."));
    entry.set_hexpand(true);
    if let Some(ref q) = ui_state.library_query {
        entry.set_text(q);
    }
    let s = sender.clone();
    entry.connect_search_changed(move |e| {
        s.emit(LibraryMsg::SetQuery(e.text().to_string()));
    });
    toolbar.append(&entry);

    let sort_group = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    sort_group.add_css_class("linked");

    let saved_sort = ui_state.library_sort.as_deref().unwrap_or("date");

    let date_btn = gtk4::ToggleButton::new();
    date_btn.set_icon_name("document-open-recent-symbolic");
    date_btn.set_tooltip_text(Some("Sort by date"));
    date_btn.set_active(saved_sort != "name");
    let s = sender.clone();
    date_btn.connect_clicked(move |_| { s.emit(LibraryMsg::SetSort(Sort::Date)); });
    sort_group.append(&date_btn);

    let name_btn = gtk4::ToggleButton::new();
    name_btn.set_icon_name("view-sort-ascending-rtl-symbolic");
    name_btn.set_tooltip_text(Some("Sort by name"));
    name_btn.set_group(Some(&date_btn));
    name_btn.set_active(saved_sort == "name");
    let s = sender.clone();
    name_btn.connect_clicked(move |_| { s.emit(LibraryMsg::SetSort(Sort::Name)); });
    sort_group.append(&name_btn);

    toolbar.append(&sort_group);

    toolbar
}
