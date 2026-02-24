fn find_child_by_name(widget: &impl IsA<gtk4::Widget>, name: &str) -> Option<gtk4::Widget> {
    let mut child = widget.first_child();
    while let Some(c) = child {
        if c.widget_name() == name {
            return Some(c);
        }
        if let Some(found) = find_child_by_name(&c, name) {
            return Some(found);
        }
        child = c.next_sibling();
    }
    None
}

use crate::bandcamp::{AlbumDetails, BandcampClient};
use crate::discover::{DiscoverMsg, DiscoverOutput, DiscoverPage};
use crate::library::{LibraryMsg, LibraryOutput, LibraryPage};
use crate::login::{LoginOutput, LoginPage};
use crate::player::{Player, PlayerMsg, PlayerOutput, Track};
use crate::search::{SearchMsg, SearchOutput, SearchPage};
use crate::storage::{self, UiState};
use gtk4::gdk;
use gtk4::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;
use relm4::prelude::*;

pub struct App {
    mode: AppMode,
    login: Controller<LoginPage>,
    discover: Option<Controller<DiscoverPage>>,
    search: Option<Controller<SearchPage>>,
    library: Option<Controller<LibraryPage>>,
    player: Option<Controller<Player>>,
    client: Option<BandcampClient>,
    current_album: Option<AlbumDetails>,
    toast_overlay: adw::ToastOverlay,
    toolbars: Option<Toolbars>,
    narrow_breakpoint: adw::Breakpoint,
    ui_state: UiState,
}

struct Toolbars {
    stack: gtk4::Stack,
}

#[derive(Debug, Default, PartialEq)]
enum AppMode {
    #[default]
    Login,
    Main,
}

#[derive(Debug)]
pub enum AppMsg {
    LoginSuccess(String),
    ClientReady(BandcampClient),
    ClientError(String),
    DiscoverAction(DiscoverOutput),
    SearchAction(SearchOutput),
    LibraryAction(LibraryOutput),
    PlayerAction(PlayerOutput),
    PlayAlbum(String),
    AlbumLoaded(Result<AlbumDetails, String>),
    AddToWishlist,
    TabChanged,
    SaveUiState,
    Logout,
    ShowToast(String),
    PlayerToggle,
    PlayerNext,
    PlayerPrev,
    PlayerVolumeUp,
    PlayerVolumeDown,
}

#[relm4::component(pub)]
impl Component for App {
    type Init = ();
    type Input = AppMsg;
    type Output = ();
    type CommandOutput = AppCmd;

    view! {
        adw::ApplicationWindow {
            set_title: Some("Camper"),
            set_default_width: 625,
            set_default_height: 625,
            set_size_request: (625, 400),

            #[local_ref]
            toast_overlay -> adw::ToastOverlay {
                #[name = "main_stack"]
                gtk4::Stack {
                    set_transition_type: gtk4::StackTransitionType::Crossfade,

                    add_named[Some("login")] = model.login.widget() {},

                    add_named[Some("main")] = &gtk4::Box {
                        set_orientation: gtk4::Orientation::Vertical,

                        #[name = "header_bar"]
                        adw::HeaderBar {
                            #[wrap(Some)]
                            #[name = "view_switcher"]
                            set_title_widget = &adw::ViewSwitcher {
                                set_policy: adw::ViewSwitcherPolicy::Wide,
                            },

                            #[name = "logout_button"]
                            pack_end = &gtk4::Button {
                                set_icon_name: "system-log-out-symbolic",
                                set_tooltip_text: Some("Logout"),
                                connect_clicked => AppMsg::Logout,
                            },
                        },

                        #[name = "content_stack"]
                        adw::ViewStack {
                            set_vexpand: true,
                        },

                        gtk4::Separator {},

                        #[name = "player_box"]
                        gtk4::Box {},
                    },
                },
            },
        }
    }

    fn init(_: Self::Init, root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        // Load custom CSS
        let css = gtk4::CssProvider::new();
        css.load_from_string(include_str!("style.css"));
        gtk4::style_context_add_provider_for_display(
            &gtk4::prelude::WidgetExt::display(&root),
            &css,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        let login = LoginPage::builder()
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                LoginOutput::Success(cookies) => AppMsg::LoginSuccess(cookies),
            });

        let toast_overlay = adw::ToastOverlay::new();

        let narrow_breakpoint = adw::Breakpoint::new(adw::BreakpointCondition::new_length(
            adw::BreakpointConditionLengthType::MaxWidth,
            900.0,
            adw::LengthUnit::Px,
        ));

        let model = Self {
            mode: AppMode::Login,
            login,
            discover: None,
            search: None,
            library: None,
            player: None,
            client: None,
            current_album: None,
            toast_overlay: toast_overlay.clone(),
            toolbars: None,
            narrow_breakpoint: narrow_breakpoint.clone(),
            ui_state: storage::load_ui_state(),
        };

        let toast_overlay = &model.toast_overlay;
        let widgets = view_output!();

        narrow_breakpoint.add_setter(
            &widgets.view_switcher,
            "policy",
            Some(&adw::ViewSwitcherPolicy::Narrow.to_value()),
        );
        narrow_breakpoint.add_setter(
            &widgets.logout_button,
            "visible",
            Some(&false.to_value()),
        );
        root.add_breakpoint(narrow_breakpoint);

        // Global keyboard shortcuts
        let s = sender.clone();
        let content_stack = widgets.content_stack.clone();
        let key_ctrl = gtk4::EventControllerKey::new();
        key_ctrl.set_propagation_phase(gtk4::PropagationPhase::Capture);
        key_ctrl.connect_key_pressed(move |_, key, _, modifiers| {
            let ctrl = modifiers.contains(gdk::ModifierType::CONTROL_MASK);

            // Ctrl+1/2/3 — switch tabs
            if ctrl {
                let tab = match key {
                    gdk::Key::_1 => Some("search"),
                    gdk::Key::_2 => Some("discover"),
                    gdk::Key::_3 => Some("library"),
                    _ => None,
                };
                if let Some(name) = tab {
                    content_stack.set_visible_child_name(name);
                    return gtk4::glib::Propagation::Stop;
                }
            }

            // Skip media shortcuts when focus is on a text input widget
            let root_widget = content_stack.root();
            let focused_on_text = root_widget
                .as_ref()
                .and_then(|r| r.focus())
                .map(|w| w.is::<gtk4::SearchEntry>() || w.is::<gtk4::Entry>() || w.is::<gtk4::Text>())
                .unwrap_or(false);

            if !focused_on_text {
                match key {
                    // Space — play/pause
                    gdk::Key::space => {
                        s.input(AppMsg::PlayerToggle);
                        return gtk4::glib::Propagation::Stop;
                    }
                    // Ctrl+Right / Ctrl+Left — next/prev track
                    gdk::Key::Right if ctrl => {
                        s.input(AppMsg::PlayerNext);
                        return gtk4::glib::Propagation::Stop;
                    }
                    gdk::Key::Left if ctrl => {
                        s.input(AppMsg::PlayerPrev);
                        return gtk4::glib::Propagation::Stop;
                    }
                    // Ctrl+Up / Ctrl+Down — volume
                    gdk::Key::Up if ctrl => {
                        s.input(AppMsg::PlayerVolumeUp);
                        return gtk4::glib::Propagation::Stop;
                    }
                    gdk::Key::Down if ctrl => {
                        s.input(AppMsg::PlayerVolumeDown);
                        return gtk4::glib::Propagation::Stop;
                    }
                    _ => {}
                }
            }

            gtk4::glib::Propagation::Proceed
        });
        root.add_controller(key_ctrl);

        if let Some(cookies) = storage::load_cookies() {
            sender.input(AppMsg::LoginSuccess(cookies));
        }

        ComponentParts { model, widgets }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        msg: Self::Input,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            AppMsg::LoginSuccess(cookies) => {
                if self.client.is_some() || self.mode == AppMode::Main {
                    return;
                }
                let cookies_clone = cookies.clone();
                sender.oneshot_command(async move {
                    match BandcampClient::new(cookies).await {
                        Ok(client) => {
                            let _ = storage::save_cookies(&cookies_clone);
                            AppCmd::ClientReady(client)
                        }
                        Err(e) => {
                            storage::clear_cookies();
                            AppCmd::ClientError(e.to_string())
                        }
                    }
                });
            }
            AppMsg::ClientReady(client) => {
                let username = client.fan().username.clone();
                sender.input(AppMsg::ShowToast(format!("Welcome, {}!", username)));

                let discover = DiscoverPage::builder()
                    .launch(())
                    .forward(sender.input_sender(), AppMsg::DiscoverAction);
                discover.emit(DiscoverMsg::SetClient(client.clone()));

                let search = SearchPage::builder()
                    .launch(())
                    .forward(sender.input_sender(), AppMsg::SearchAction);
                search.emit(SearchMsg::SetClient(client.clone()));

                let library = LibraryPage::builder()
                    .launch(())
                    .forward(sender.input_sender(), AppMsg::LibraryAction);
                library.emit(LibraryMsg::SetClient(client.clone()));

                let player = Player::builder()
                    .launch(())
                    .forward(sender.input_sender(), AppMsg::PlayerAction);

                // Restore saved volume
                if let Some(vol) = self.ui_state.volume {
                    player.emit(PlayerMsg::SetVolume(vol));
                }

                // Restore saved search query
                if let Some(ref q) = self.ui_state.search_query {
                    if !q.is_empty() {
                        search.emit(SearchMsg::QueryChanged(q.clone()));
                    }
                }

                // Restore saved discover filters
                if let Some(genre) = self.ui_state.discover_genre {
                    discover.emit(DiscoverMsg::SetGenre(genre));
                }
                if let Some(subgenre) = self.ui_state.discover_subgenre {
                    discover.emit(DiscoverMsg::SetSubgenre(subgenre));
                }
                if let Some(sort) = self.ui_state.discover_sort {
                    discover.emit(DiscoverMsg::SetSort(sort));
                }


                // Restore saved library sort/query
                if let Some(ref s) = self.ui_state.library_sort {
                    let sort = match s.as_str() {
                        "name" => crate::library::Sort::Name,
                        _ => crate::library::Sort::Date,
                    };
                    library.emit(LibraryMsg::SetSort(sort));
                }
                if let Some(ref q) = self.ui_state.library_query {
                    if !q.is_empty() {
                        library.emit(LibraryMsg::SetQuery(q.clone()));
                    }
                }

                // Build toolbars and pack into header bar
                let search_toolbar = crate::search::build_toolbar(search.sender(), &self.ui_state);
                let discover_toolbar = crate::discover::build_toolbar(discover.sender(), &self.ui_state);
                let library_toolbar = crate::library::build_toolbar(library.sender(), &self.ui_state);

                let toolbar_stack = gtk4::Stack::new();
                toolbar_stack.set_hhomogeneous(true);
                toolbar_stack.add_named(&search_toolbar, Some("search"));
                toolbar_stack.add_named(&discover_toolbar, Some("discover"));
                toolbar_stack.add_named(&library_toolbar, Some("library"));
                widgets.header_bar.pack_start(&toolbar_stack);

                self.toolbars = Some(Toolbars {
                    stack: toolbar_stack,
                });

                widgets.content_stack.add_titled_with_icon(
                    search.widget(), Some("search"), "Search", "system-search-symbolic",
                );
                widgets.content_stack.add_titled_with_icon(
                    discover.widget(), Some("discover"), "Discover", "web-browser-symbolic",
                );
                widgets.content_stack.add_titled_with_icon(
                    library.widget(), Some("library"), "Library", "folder-music-symbolic",
                );

                widgets.player_box.append(player.widget());

                // Hide player extra controls (volume, open in browser) on narrow layout
                if let Some(extra) = find_child_by_name(player.widget(), "player-extra-controls") {
                    self.narrow_breakpoint.add_setter(&extra, "visible", Some(&false.to_value()));
                }

                widgets.view_switcher.set_stack(Some(&widgets.content_stack));

                // Listen for tab changes
                let s = sender.clone();
                widgets.content_stack.connect_visible_child_name_notify(move |_| {
                    s.input(AppMsg::TabChanged);
                });

                self.discover = Some(discover);
                self.search = Some(search);
                self.library = Some(library);
                self.player = Some(player);
                self.client = Some(client);
                self.mode = AppMode::Main;

                // Restore saved tab or default to library
                let tab = self.ui_state.active_tab.as_deref().unwrap_or("library");
                widgets.content_stack.set_visible_child_name(tab);
                sender.input(AppMsg::TabChanged);
            }
            AppMsg::TabChanged => {
                if let Some(toolbars) = &self.toolbars {
                    let active = widgets.content_stack.visible_child_name();
                    let name = active.as_ref().map(|s| s.as_str()).unwrap_or("");
                    toolbars.stack.set_visible_child_name(name);

                    if name == "library" {
                        if let Some(library) = &self.library {
                            library.emit(LibraryMsg::Refresh);
                        }
                    }

                    self.ui_state.active_tab = Some(name.to_string());
                    sender.input(AppMsg::SaveUiState);
                }
            }
            AppMsg::SaveUiState => {
                let _ = storage::save_ui_state(&self.ui_state);
            }
            AppMsg::ClientError(e) => {
                sender.input(AppMsg::ShowToast(format!("Login failed: {}", e)));
            }
            AppMsg::DiscoverAction(action) => match action {
                DiscoverOutput::Play(url) => sender.input(AppMsg::PlayAlbum(url)),
                DiscoverOutput::GenreChanged(i) => {
                    self.ui_state.discover_genre = Some(i);
                    self.ui_state.discover_subgenre = Some(0);
                    sender.input(AppMsg::SaveUiState);
                }
                DiscoverOutput::SubgenreChanged(i) => {
                    self.ui_state.discover_subgenre = Some(i);
                    sender.input(AppMsg::SaveUiState);
                }
                DiscoverOutput::SortChanged(i) => {
                    self.ui_state.discover_sort = Some(i);
                    sender.input(AppMsg::SaveUiState);
                }

            },
            AppMsg::SearchAction(action) => match action {
                SearchOutput::Play(url) => sender.input(AppMsg::PlayAlbum(url)),
                SearchOutput::QueryChanged(q) => {
                    self.ui_state.search_query = Some(q);
                    sender.input(AppMsg::SaveUiState);
                }
            },
            AppMsg::LibraryAction(action) => match action {
                LibraryOutput::Play(url) => sender.input(AppMsg::PlayAlbum(url)),
                LibraryOutput::SortChanged(s) => {
                    self.ui_state.library_sort = Some(match s {
                        crate::library::Sort::Date => "date",
                        crate::library::Sort::Name => "name",
                    }.to_string());
                    sender.input(AppMsg::SaveUiState);
                }
                LibraryOutput::QueryChanged(q) => {
                    self.ui_state.library_query = Some(q);
                    sender.input(AppMsg::SaveUiState);
                }
            },
            AppMsg::PlayerAction(output) => match output {
                PlayerOutput::NowPlaying => {}
                PlayerOutput::Wishlist => {
                    sender.input(AppMsg::AddToWishlist);
                }
                PlayerOutput::VolumeChanged(v) => {
                    self.ui_state.volume = Some(v);
                    sender.input(AppMsg::SaveUiState);
                }
            },
            AppMsg::PlayAlbum(url) => {
                if url.is_empty() {
                    sender.input(AppMsg::ShowToast("No album URL".to_string()));
                    return;
                }
                if let Some(client) = self.client.clone() {
                    sender.oneshot_command(async move {
                        match client.get_album_details(&url).await {
                            Ok(details) => AppCmd::AlbumLoaded(Ok(details)),
                            Err(e) => AppCmd::AlbumLoaded(Err(e.to_string())),
                        }
                    });
                }
            }
            AppMsg::AlbumLoaded(result) => {
                match result {
                    Ok(details) => {
                        let tracks: Vec<Track> = details.tracks.iter()
                            .filter_map(|t| Some(Track {
                                title: t.title.clone(),
                                artist: t.artist.clone(),
                                album: t.album.clone(),
                                art_url: t.art_url.clone(),
                                stream_url: t.stream_url.clone()?,
                                duration: t.duration,
                            }))
                            .collect();

                        if tracks.is_empty() {
                            sender.input(AppMsg::ShowToast("No playable tracks".to_string()));
                        } else {
                            self.current_album = Some(details);
                            if let Some(player) = &self.player {
                                player.emit(PlayerMsg::PlayQueue(tracks, 0));
                            }
                        }
                    }
                    Err(e) => sender.input(AppMsg::ShowToast(format!("Failed: {}", e))),
                }
            }
            AppMsg::AddToWishlist => {
                if let Some(album) = self.current_album.as_ref() {
                    if let Err(e) = open::that(&album.url) {
                        sender.input(AppMsg::ShowToast(format!("Failed to open browser: {}", e)));
                    }
                }
            }
            AppMsg::Logout => {
                storage::clear_cookies();
                self.mode = AppMode::Login;
                self.client = None;

                if let Some(d) = self.discover.take() { widgets.content_stack.remove(d.widget()); }
                if let Some(s) = self.search.take() { widgets.content_stack.remove(s.widget()); }
                if let Some(l) = self.library.take() { widgets.content_stack.remove(l.widget()); }
                if let Some(p) = self.player.take() { widgets.player_box.remove(p.widget()); }

                if let Some(toolbars) = self.toolbars.take() {
                    widgets.header_bar.remove(&toolbars.stack);
                }
            }
            AppMsg::PlayerToggle => {
                if let Some(player) = &self.player {
                    player.emit(PlayerMsg::Toggle);
                }
            }
            AppMsg::PlayerNext => {
                if let Some(player) = &self.player {
                    player.emit(PlayerMsg::Next);
                }
            }
            AppMsg::PlayerPrev => {
                if let Some(player) = &self.player {
                    player.emit(PlayerMsg::Prev);
                }
            }
            AppMsg::PlayerVolumeUp => {
                if let Some(player) = &self.player {
                    let vol = (self.ui_state.volume.unwrap_or(1.0) + 0.05).min(1.0);
                    player.emit(PlayerMsg::SetVolume(vol));
                }
            }
            AppMsg::PlayerVolumeDown => {
                if let Some(player) = &self.player {
                    let vol = (self.ui_state.volume.unwrap_or(1.0) - 0.05).max(0.0);
                    player.emit(PlayerMsg::SetVolume(vol));
                }
            }
            AppMsg::ShowToast(msg) => {
                self.toast_overlay.add_toast(adw::Toast::new(&msg));
            }
        }

        widgets.main_stack.set_visible_child_name(match self.mode {
            AppMode::Login => "login",
            AppMode::Main => "main",
        });

        self.update_view(widgets, sender);
    }

    fn update_cmd(&mut self, msg: Self::CommandOutput, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            AppCmd::ClientReady(client) => sender.input(AppMsg::ClientReady(client)),
            AppCmd::ClientError(e) => sender.input(AppMsg::ClientError(e)),
            AppCmd::AlbumLoaded(r) => sender.input(AppMsg::AlbumLoaded(r)),
        }
    }
}

#[derive(Debug)]
pub enum AppCmd {
    ClientReady(BandcampClient),
    ClientError(String),
    AlbumLoaded(Result<AlbumDetails, String>),
}
