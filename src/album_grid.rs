use gtk4::gdk_pixbuf::Pixbuf;
use gtk4::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;
use relm4::prelude::*;

#[derive(Debug, Clone)]
pub struct AlbumData {
    pub title: String,
    pub artist: String,
    pub genre: Option<String>,
    pub art_url: Option<String>,
    pub url: String,
    pub band_id: Option<u64>,
    pub item_id: Option<u64>,
    pub item_type: Option<String>,
}

impl From<crate::bandcamp::Album> for AlbumData {
    fn from(a: crate::bandcamp::Album) -> Self {
        Self {
            title: a.title,
            artist: a.artist,
            genre: a.genre,
            art_url: a.art_url,
            url: a.url,
            band_id: a.band_id,
            item_id: a.item_id,
            item_type: a.item_type,
        }
    }
}

impl From<crate::bandcamp::CollectionItem> for AlbumData {
    fn from(item: crate::bandcamp::CollectionItem) -> Self {
        Self {
            title: item.title,
            artist: item.artist,
            genre: None,
            art_url: item.art_url,
            url: item.url,
            band_id: None,
            item_id: None,
            item_type: None,
        }
    }
}

pub struct AlbumGrid {
    wrap_box: adw::WrapBox,
    stack: gtk4::Stack,
    current: Vec<AlbumData>,
}

#[derive(Debug)]
pub enum AlbumGridMsg {
    Append(Vec<AlbumData>),
    Replace(Vec<AlbumData>),
}

#[derive(Debug, Clone)]
pub enum AlbumGridOutput {
    Clicked(AlbumData),
    ScrolledToBottom,
}

#[relm4::component(pub)]
impl SimpleComponent for AlbumGrid {
    type Init = ();
    type Input = AlbumGridMsg;
    type Output = AlbumGridOutput;

    view! {
        gtk4::Box {
            set_orientation: gtk4::Orientation::Vertical,
            set_hexpand: true,
            set_vexpand: true,
        }
    }

    fn init(_: Self::Init, root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let wrap_box = adw::WrapBox::new();
        wrap_box.set_child_spacing(6);
        wrap_box.set_line_spacing(8);
        wrap_box.set_margin_start(8);
        wrap_box.set_margin_end(8);
        wrap_box.set_margin_top(8);
        wrap_box.set_margin_bottom(8);
        wrap_box.set_valign(gtk4::Align::Start);
        wrap_box.set_halign(gtk4::Align::Fill);
        wrap_box.set_justify(adw::JustifyMode::Fill);

        let scroll = gtk4::ScrolledWindow::new();
        scroll.set_hscrollbar_policy(gtk4::PolicyType::Never);
        scroll.set_vexpand(true);
        scroll.set_hexpand(true);
        scroll.set_child(Some(&wrap_box));

        let empty_page = adw::StatusPage::new();
        empty_page.set_icon_name(Some("folder-music-symbolic"));
        empty_page.set_title("No Albums");
        empty_page.set_vexpand(true);

        let stack = gtk4::Stack::new();
        stack.set_vexpand(true);
        stack.set_hexpand(true);
        stack.set_transition_type(gtk4::StackTransitionType::Crossfade);
        stack.set_transition_duration(150);
        stack.add_named(&empty_page, Some("empty"));
        stack.add_named(&scroll, Some("content"));
        stack.set_visible_child_name("empty");

        let model = Self {
            wrap_box,
            stack: stack.clone(),
            current: Vec::new(),
        };
        let widgets = view_output!();
        root.append(&stack);

        let adj = scroll.vadjustment();
        let s = sender.clone();
        adj.connect_value_changed(move |a| {
            if a.value() + a.page_size() >= a.upper() - 100.0 {
                s.output(AlbumGridOutput::ScrolledToBottom).ok();
            }
        });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            AlbumGridMsg::Append(items) => {
                if !items.is_empty() {
                    self.stack.set_visible_child_name("content");
                }
                self.append_cards(&items, &sender);
                self.current.extend(items);
            }
            AlbumGridMsg::Replace(items) => {
                if self.same_albums(&items) {
                    return;
                }
                while let Some(child) = self.wrap_box.first_child() {
                    self.wrap_box.remove(&child);
                }
                if items.is_empty() {
                    self.stack.set_visible_child_name("empty");
                } else {
                    self.stack.set_visible_child_name("content");
                    self.append_cards(&items, &sender);
                }
                self.current = items;
            }
        }
    }
}

impl AlbumGrid {
    fn append_cards(&self, items: &[AlbumData], sender: &ComponentSender<Self>) {
        for data in items {
            let card = build_card(data, sender);
            self.wrap_box.append(&card);
        }
    }

    fn same_albums(&self, items: &[AlbumData]) -> bool {
        self.current.len() == items.len()
            && self.current.iter().zip(items).all(|(a, b)| a.url == b.url)
    }
}

fn build_card(data: &AlbumData, sender: &ComponentSender<AlbumGrid>) -> adw::Clamp {
    let card = gtk4::Box::new(gtk4::Orientation::Vertical, 0);

    let image = gtk4::Image::new();
    image.set_pixel_size(180);

    let art_frame = gtk4::Frame::new(None);
    art_frame.add_css_class("album-art");
    art_frame.set_child(Some(&image));

    let overlay = gtk4::Overlay::new();

    let play_icon = gtk4::Image::from_icon_name("media-playback-start-symbolic");
    play_icon.set_pixel_size(24);
    play_icon.add_css_class("play-overlay-icon");
    play_icon.set_valign(gtk4::Align::Center);
    play_icon.set_vexpand(true);

    let play_circle = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    play_circle.set_halign(gtk4::Align::Center);
    play_circle.set_valign(gtk4::Align::Center);
    play_circle.add_css_class("play-overlay");
    play_circle.set_width_request(48);
    play_circle.set_height_request(48);
    play_circle.append(&play_icon);
    play_circle.set_opacity(0.0);

    overlay.set_child(Some(&art_frame));
    overlay.add_overlay(&play_circle);
    card.append(&overlay);

    let title = gtk4::Label::new(Some(&data.title));
    title.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    title.set_lines(1);
    title.set_halign(gtk4::Align::Start);
    title.set_margin_top(4);
    title.add_css_class("album-title");
    card.append(&title);

    let artist = gtk4::Label::new(Some(&data.artist));
    artist.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    artist.set_lines(1);
    artist.set_halign(gtk4::Align::Start);
    artist.add_css_class("dim-label");
    artist.add_css_class("caption");
    card.append(&artist);

    if let Some(genre) = &data.genre {
        let genre_label = gtk4::Label::new(Some(genre));
        genre_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        genre_label.set_lines(1);
        genre_label.set_halign(gtk4::Align::Start);
        genre_label.add_css_class("dim-label");
        genre_label.add_css_class("caption");
        genre_label.add_css_class("album-genre");
        card.append(&genre_label);
    }

    if let Some(url) = data.art_url.clone() {
        gtk4::glib::spawn_future_local(async move {
            if let Ok(resp) = reqwest::get(&url).await {
                if let Ok(bytes) = resp.bytes().await {
                    let stream = gtk4::gio::MemoryInputStream::from_bytes(&gtk4::glib::Bytes::from(&bytes));
                    if let Ok(pb) = Pixbuf::from_stream(&stream, None::<&gtk4::gio::Cancellable>) {
                        let texture = gtk4::gdk::Texture::for_pixbuf(&pb);
                        image.set_paintable(Some(&texture));
                    }
                }
            }
        });
    }

    let clamp = adw::Clamp::new();
    clamp.set_maximum_size(180);
    clamp.set_child(Some(&card));
    clamp.set_focusable(true);
    clamp.set_cursor_from_name(Some("pointer"));

    let enter_circle = play_circle.clone();
    let leave_circle = play_circle.clone();
    let motion = gtk4::EventControllerMotion::new();
    motion.connect_enter(move |_, _, _| {
        let target = adw::PropertyAnimationTarget::new(&enter_circle, "opacity");
        let anim = adw::TimedAnimation::new(&enter_circle, enter_circle.opacity(), 1.0, 150, target);
        anim.play();
    });
    motion.connect_leave(move |_| {
        let target = adw::PropertyAnimationTarget::new(&leave_circle, "opacity");
        let anim = adw::TimedAnimation::new(&leave_circle, leave_circle.opacity(), 0.0, 150, target);
        anim.play();
    });
    clamp.add_controller(motion);

    let click_data = data.clone();
    let click_sender = sender.clone();
    let gesture = gtk4::GestureClick::new();
    gesture.connect_released(move |_, _, _, _| {
        click_sender.output(AlbumGridOutput::Clicked(click_data.clone())).ok();
    });
    clamp.add_controller(gesture);

    let key_data = data.clone();
    let key_sender = sender.clone();
    let key_ctrl = gtk4::EventControllerKey::new();
    key_ctrl.connect_key_pressed(move |_, key, _, _| {
        if key == gtk4::gdk::Key::Return || key == gtk4::gdk::Key::KP_Enter || key == gtk4::gdk::Key::space {
            key_sender.output(AlbumGridOutput::Clicked(key_data.clone())).ok();
            gtk4::glib::Propagation::Stop
        } else {
            gtk4::glib::Propagation::Proceed
        }
    });
    clamp.add_controller(key_ctrl);

    clamp
}
