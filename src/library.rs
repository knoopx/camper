use crate::album_grid::{AlbumData, AlbumGrid, AlbumGridMsg, AlbumGridOutput};
use crate::bandcamp::{BandcampClient, CollectionItem};
use gtk4::prelude::*;
use relm4::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Filter {
    #[default]
    All,
    Collection,
    Wishlist,
}

pub struct LibraryPage {
    client: Option<BandcampClient>,
    grid: Controller<AlbumGrid>,
    all_items: Vec<CollectionItem>,
    filter: Filter,
    loading: bool,
}

#[derive(Debug)]
pub enum LibraryMsg {
    SetClient(BandcampClient),
    Refresh,
    SetFilter(Filter),
    Loaded(Result<(Vec<CollectionItem>, Vec<CollectionItem>), String>),
    GridAction(AlbumGridOutput),
}

#[derive(Debug)]
pub enum LibraryOutput {
    Play(String),
    FilterChanged(Filter),
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
            filter: Filter::All,
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
            LibraryMsg::SetFilter(filter) => {
                self.filter = filter;
                self.apply_filter();
                sender.output(LibraryOutput::FilterChanged(filter)).ok();
            }
            LibraryMsg::Loaded(result) => {
                self.loading = false;
                if let Ok((collection, wishlist)) = result {
                    self.all_items.clear();
                    self.all_items.extend(collection);
                    self.all_items.extend(wishlist);
                    self.apply_filter();
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

    fn apply_filter(&mut self) {
        self.grid.emit(AlbumGridMsg::Clear);

        let items: Vec<AlbumData> = self.all_items.iter()
            .filter(|item| match self.filter {
                Filter::All => true,
                Filter::Collection => !item.is_wishlist,
                Filter::Wishlist => item.is_wishlist,
            })
            .map(|item| AlbumData {
                title: item.title.clone(),
                artist: item.artist.clone(),
                genre: None,
                art_url: item.art_url.clone(),
                url: item.url.clone(),
            })
            .collect();

        self.grid.emit(AlbumGridMsg::Append(items));
    }
}

pub fn build_toolbar(sender: &relm4::Sender<LibraryMsg>, ui_state: &crate::storage::UiState) -> gtk4::Box {
    let toolbar = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);

    let saved = ui_state.library_filter.as_deref().unwrap_or("all");

    let all_btn = gtk4::ToggleButton::with_label("All");
    all_btn.set_active(saved == "all");
    let s = sender.clone();
    all_btn.connect_clicked(move |_| { s.emit(LibraryMsg::SetFilter(Filter::All)); });
    toolbar.append(&all_btn);

    let col_btn = gtk4::ToggleButton::with_label("Collection");
    col_btn.set_group(Some(&all_btn));
    col_btn.set_active(saved == "collection");
    let s = sender.clone();
    col_btn.connect_clicked(move |_| { s.emit(LibraryMsg::SetFilter(Filter::Collection)); });
    toolbar.append(&col_btn);

    let wish_btn = gtk4::ToggleButton::with_label("Wishlist");
    wish_btn.set_group(Some(&all_btn));
    wish_btn.set_active(saved == "wishlist");
    let s = sender.clone();
    wish_btn.connect_clicked(move |_| { s.emit(LibraryMsg::SetFilter(Filter::Wishlist)); });
    toolbar.append(&wish_btn);

    toolbar
}
