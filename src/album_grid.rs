use gtk4::gdk_pixbuf::Pixbuf;
use gtk4::prelude::*;
use relm4::prelude::*;

#[derive(Debug, Clone)]
pub struct AlbumData {
    pub title: String,
    pub artist: String,
    pub genre: Option<String>,
    pub art_url: Option<String>,
    pub url: String,
}

pub struct AlbumGrid {
    wrap_box: libadwaita::WrapBox,
}

#[derive(Debug)]
pub enum AlbumGridMsg {
    Clear,
    Append(Vec<AlbumData>),
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
        gtk4::ScrolledWindow {
            set_hscrollbar_policy: gtk4::PolicyType::Never,
            set_vexpand: true,
            set_hexpand: true,
        }
    }

    fn init(_: Self::Init, root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let wrap_box = libadwaita::WrapBox::new();
        wrap_box.set_child_spacing(16);
        wrap_box.set_line_spacing(24);
        wrap_box.set_margin_start(16);
        wrap_box.set_margin_end(16);
        wrap_box.set_margin_top(16);
        wrap_box.set_margin_bottom(16);
        wrap_box.set_valign(gtk4::Align::Start);
        wrap_box.set_halign(gtk4::Align::Center);

        let model = Self { wrap_box };
        let widgets = view_output!();

        root.set_child(Some(&model.wrap_box));

        // Infinite scroll
        let adj = root.vadjustment();
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
            AlbumGridMsg::Clear => {
                let mut child = self.wrap_box.first_child();
                while let Some(widget) = child {
                    child = widget.next_sibling();
                    self.wrap_box.remove(&widget);
                }
            }
            AlbumGridMsg::Append(items) => {
                for data in items {
                    let card = build_card(&data, &sender);
                    self.wrap_box.append(&card);
                }
            }
        }
    }
}

fn build_card(data: &AlbumData, sender: &ComponentSender<AlbumGrid>) -> libadwaita::Clamp {
    let card = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    card.set_width_request(180);

    let image = gtk4::Image::new();
    image.set_pixel_size(180);

    let frame = gtk4::Frame::new(None);
    frame.set_overflow(gtk4::Overflow::Hidden);
    frame.set_child(Some(&image));
    card.append(&frame);

    let title = gtk4::Label::new(Some(&data.title));
    title.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    title.set_lines(1);
    title.set_halign(gtk4::Align::Start);
    title.set_margin_top(10);
    card.append(&title);

    let artist = gtk4::Label::new(Some(&data.artist));
    artist.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    artist.set_lines(1);
    artist.set_halign(gtk4::Align::Start);
    artist.add_css_class("dim-label");
    card.append(&artist);

    if let Some(genre) = &data.genre {
        let genre_label = gtk4::Label::new(Some(genre));
        genre_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        genre_label.set_lines(1);
        genre_label.set_halign(gtk4::Align::Start);
        genre_label.add_css_class("dim-label");
        genre_label.add_css_class("caption");
        card.append(&genre_label);
    }

    // Load image async
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

    let clamp = libadwaita::Clamp::new();
    clamp.set_maximum_size(180);
    clamp.set_child(Some(&card));

    // Click handler
    let click_data = data.clone();
    let click_sender = sender.clone();
    let gesture = gtk4::GestureClick::new();
    gesture.connect_released(move |_, _, _, _| {
        click_sender.output(AlbumGridOutput::Clicked(click_data.clone())).ok();
    });
    clamp.add_controller(gesture);
    clamp.set_cursor_from_name(Some("pointer"));

    clamp
}
