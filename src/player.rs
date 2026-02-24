use gstreamer as gst;
use gstreamer::prelude::*;
use gtk4::prelude::*;
use mpris_server::{Metadata, PlaybackStatus, Player as MprisPlayer, Time};
use relm4::prelude::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Track {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub art_url: Option<String>,
    pub stream_url: String,
    pub duration: Option<f64>,
}

pub struct Player {
    pipeline: gst::Element,
    current_track: Option<Track>,
    queue: Vec<Track>,
    queue_index: usize,
    playing: bool,
    position: f64,
    duration: f64,
    volume: f64,
    art_pixbuf: Option<gtk4::gdk_pixbuf::Pixbuf>,
    mpris: Rc<RefCell<Option<MprisPlayer>>>,
    updating_scale: Rc<Cell<bool>>,
    _bus_watch: Option<gst::bus::BusWatchGuard>,
}

#[derive(Debug)]
pub enum PlayerMsg {
    PlayQueue(Vec<Track>, usize),
    Toggle,
    Stop,
    Next,
    Prev,
    Seek(f64),
    SetVolume(f64),
    Tick,
    EOS,
    SetArt(Vec<u8>),
    Wishlist,
}

#[derive(Debug)]
pub enum PlayerOutput {
    NowPlaying,
    Wishlist,
}

#[relm4::component(pub)]
impl Component for Player {
    type Init = ();
    type Input = PlayerMsg;
    type Output = PlayerOutput;
    type CommandOutput = Vec<u8>;

    view! {
        gtk4::Box {
            set_orientation: gtk4::Orientation::Horizontal,
            set_spacing: 8,
            set_margin_start: 8,
            set_margin_end: 8,
            set_margin_top: 4,
            set_margin_bottom: 4,
            #[watch]
            set_visible: model.current_track.is_some(),

            // Album art
            gtk4::Frame {
                set_width_request: 48,
                set_height_request: 48,
                set_valign: gtk4::Align::Center,
                set_overflow: gtk4::Overflow::Hidden,

                #[name = "art_image"]
                gtk4::Image {
                    set_pixel_size: 48,
                },
            },

            // Track info
            gtk4::Box {
                set_orientation: gtk4::Orientation::Vertical,
                set_valign: gtk4::Align::Center,
                set_width_request: 200,

                gtk4::Label {
                    set_xalign: 0.0,
                    set_ellipsize: gtk4::pango::EllipsizeMode::End,
                    #[watch]
                    set_label: &model.current_track.as_ref().map(|t| t.title.as_str()).unwrap_or(""),
                },

                gtk4::Label {
                    set_xalign: 0.0,
                    set_ellipsize: gtk4::pango::EllipsizeMode::End,
                    add_css_class: "dim-label",
                    add_css_class: "caption",
                    #[watch]
                    set_label: &model.current_track.as_ref().map(|t| t.album.as_str()).unwrap_or(""),
                },

                gtk4::Label {
                    set_xalign: 0.0,
                    set_ellipsize: gtk4::pango::EllipsizeMode::End,
                    add_css_class: "dim-label",
                    add_css_class: "caption",
                    #[watch]
                    set_label: &model.current_track.as_ref().map(|t| t.artist.as_str()).unwrap_or(""),
                },
            },

            // Controls
            gtk4::Button {
                set_icon_name: "media-skip-backward-symbolic",
                add_css_class: "flat",
                set_valign: gtk4::Align::Center,
                connect_clicked => PlayerMsg::Prev,
            },

            gtk4::Button {
                #[watch]
                set_icon_name: if model.playing { "media-playback-pause-symbolic" } else { "media-playback-start-symbolic" },
                add_css_class: "circular",
                add_css_class: "suggested-action",
                set_valign: gtk4::Align::Center,
                connect_clicked => PlayerMsg::Toggle,
            },

            gtk4::Button {
                set_icon_name: "media-skip-forward-symbolic",
                add_css_class: "flat",
                set_valign: gtk4::Align::Center,
                connect_clicked => PlayerMsg::Next,
            },

            // Time + seek
            gtk4::Label {
                set_width_chars: 5,
                add_css_class: "caption",
                add_css_class: "numeric",
                set_valign: gtk4::Align::Center,
                #[watch]
                set_label: &format_time(model.position),
            },

            #[name = "seek_scale"]
            gtk4::Scale {
                set_hexpand: true,
                set_valign: gtk4::Align::Center,
                set_range: (0.0, 1.0),
                set_draw_value: false,
            },

            gtk4::Label {
                set_width_chars: 5,
                add_css_class: "caption",
                add_css_class: "numeric",
                set_valign: gtk4::Align::Center,
                #[watch]
                set_label: &format_time(model.duration),
            },

            // Volume + open in browser
            #[name = "extra_controls"]
            gtk4::Box {
                set_orientation: gtk4::Orientation::Horizontal,
                set_spacing: 8,
                set_valign: gtk4::Align::Center,
                set_widget_name: "player-extra-controls",

                gtk4::Image {
                    set_icon_name: Some("audio-volume-high-symbolic"),
                    set_valign: gtk4::Align::Center,
                },

                gtk4::Scale {
                    set_orientation: gtk4::Orientation::Horizontal,
                    set_valign: gtk4::Align::Center,
                    set_width_request: 80,
                    set_range: (0.0, 1.0),
                    set_value: 1.0,
                    set_draw_value: false,
                    connect_value_changed[sender] => move |scale| {
                        sender.input(PlayerMsg::SetVolume(scale.value()));
                    },
                },

                gtk4::Button {
                    set_icon_name: "web-browser-symbolic",
                    add_css_class: "flat",
                    set_valign: gtk4::Align::Center,
                    set_tooltip_text: Some("Open in Browser"),
                    connect_clicked => PlayerMsg::Wishlist,
                },
            },
        }
    }

    fn init(_: Self::Init, root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        gst::init().expect("GStreamer init failed");

        let pipeline = gst::ElementFactory::make("playbin").build().unwrap();
        pipeline.set_property("buffer-duration", 5_000_000_000i64);

        let bus = pipeline.bus().unwrap();
        let s = sender.clone();
        let bus_watch = bus.add_watch_local(move |_, msg| {
            match msg.view() {
                gst::MessageView::Eos(_) => s.input(PlayerMsg::EOS),
                gst::MessageView::Error(err) => {
                    eprintln!("GStreamer error: {:?}", err.error());
                    s.input(PlayerMsg::EOS);
                }
                gst::MessageView::StateChanged(_) => {}
                gst::MessageView::StreamStart(_) => {}
                gst::MessageView::Buffering(_) => {}
                _ => {}
            }
            gst::glib::ControlFlow::Continue
        }).unwrap();

        let s = sender.clone();
        gtk4::glib::timeout_add_local(Duration::from_millis(500), move || {
            s.input(PlayerMsg::Tick);
            gtk4::glib::ControlFlow::Continue
        });

        let mpris: Rc<RefCell<Option<MprisPlayer>>> = Rc::new(RefCell::new(None));
        let mpris_clone = mpris.clone();
        let st = sender.clone();
        let sn = sender.clone();
        let sp = sender.clone();
        let ss = sender.clone();

        gtk4::glib::MainContext::default().spawn_local(async move {
            if let Ok(m) = MprisPlayer::builder("camper")
                .identity("Camper")
                .can_play(true).can_pause(true)
                .can_go_next(true).can_go_previous(true)
                .can_seek(true).can_control(true)
                .build().await
            {
                m.connect_play_pause(move |_| st.input(PlayerMsg::Toggle));
                m.connect_next(move |_| sn.input(PlayerMsg::Next));
                m.connect_previous(move |_| sp.input(PlayerMsg::Prev));
                m.connect_stop(move |_| ss.input(PlayerMsg::Stop));
                let run_task = m.run();
                *mpris_clone.borrow_mut() = Some(m);
                run_task.await;
            }
        });

        let updating_scale = Rc::new(Cell::new(false));

        let model = Self {
            pipeline,
            current_track: None,
            queue: Vec::new(),
            queue_index: 0,
            playing: false,
            position: 0.0,
            duration: 0.0,
            volume: 1.0,
            art_pixbuf: None,
            mpris,
            updating_scale: updating_scale.clone(),
            _bus_watch: Some(bus_watch),
        };

        let widgets = view_output!();

        let flag = updating_scale.clone();
        let s = sender.clone();
        widgets.seek_scale.connect_value_changed(move |scale| {
            if !flag.get() {
                s.input(PlayerMsg::Seek(scale.value()));
            }
        });

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
            PlayerMsg::PlayQueue(tracks, idx) => {
                self.queue = tracks;
                self.queue_index = idx;
                self.play_current(sender.clone());
            }
            PlayerMsg::Toggle => {
                if self.playing {
                    self.pipeline.set_state(gst::State::Paused).ok();
                    self.playing = false;
                } else if self.current_track.is_some() {
                    self.pipeline.set_state(gst::State::Playing).ok();
                    self.playing = true;
                }
                self.sync_mpris();
            }
            PlayerMsg::Stop => {
                self.pipeline.set_state(gst::State::Null).ok();
                self.playing = false;
                self.position = 0.0;
                self.sync_mpris();
            }
            PlayerMsg::Next => {
                if self.queue_index + 1 < self.queue.len() {
                    self.queue_index += 1;
                    self.play_current(sender.clone());
                }
            }
            PlayerMsg::Prev => {
                if self.queue_index > 0 {
                    self.queue_index -= 1;
                    self.play_current(sender.clone());
                }
            }
            PlayerMsg::Seek(frac) => {
                if self.duration > 0.0 {
                    let ns = (frac * self.duration * 1_000_000_000.0) as u64;
                    self.pipeline.seek_simple(gst::SeekFlags::FLUSH, gst::ClockTime::from_nseconds(ns)).ok();
                    self.position = frac * self.duration;
                }
            }
            PlayerMsg::SetVolume(v) => {
                self.volume = v;
                self.pipeline.set_property("volume", v);
            }
            PlayerMsg::Tick => {
                if self.playing {
                    if let Some(pos) = self.pipeline.query_position::<gst::ClockTime>() {
                        self.position = pos.seconds() as f64;
                    }
                    if let Some(dur) = self.pipeline.query_duration::<gst::ClockTime>() {
                        self.duration = dur.seconds() as f64;
                    }
                    if self.duration > 0.0 {
                        self.updating_scale.set(true);
                        widgets.seek_scale.set_value(self.position / self.duration);
                        self.updating_scale.set(false);

                        if self.position >= self.duration {
                            sender.input(PlayerMsg::EOS);
                        }
                    }
                    self.sync_mpris_position();
                }
            }
            PlayerMsg::EOS => {
                if self.queue_index + 1 < self.queue.len() {
                    self.queue_index += 1;
                    self.play_current(sender.clone());
                } else {
                    self.pipeline.set_state(gst::State::Null).ok();
                    self.playing = false;
                    self.position = 0.0;
                    self.sync_mpris();
                }
            }
            PlayerMsg::SetArt(bytes) => {
                if let Some(pb) = load_pixbuf(&bytes, 48) {
                    let texture = gtk4::gdk::Texture::for_pixbuf(&pb);
                    widgets.art_image.set_paintable(Some(&texture));
                    self.art_pixbuf = Some(pb);
                }
            }
            PlayerMsg::Wishlist => {
                if self.current_track.is_some() {
                    sender.output(PlayerOutput::Wishlist).ok();
                }
            }
        }

        self.update_view(widgets, sender);
    }

    fn update_cmd(&mut self, bytes: Self::CommandOutput, sender: ComponentSender<Self>, _root: &Self::Root) {
        if !bytes.is_empty() {
            sender.input(PlayerMsg::SetArt(bytes));
        }
    }
}

impl Player {
    fn play_current(&mut self, sender: ComponentSender<Self>) {
        let Some(track) = self.queue.get(self.queue_index).cloned() else { return };

        self.pipeline.set_state(gst::State::Null).ok();
        self.pipeline.set_property("uri", &track.stream_url);
        self.pipeline.set_property("volume", self.volume);
        self.pipeline.set_state(gst::State::Playing).ok();

        self.playing = true;
        self.position = 0.0;
        self.duration = track.duration.unwrap_or(0.0);
        self.art_pixbuf = None;
        self.current_track = Some(track.clone());

        if let Some(url) = &track.art_url {
            let url = url.clone();
            sender.oneshot_command(async move {
                reqwest::get(&url).await.ok()
                    .and_then(|r| futures::executor::block_on(r.bytes()).ok())
                    .map(|b| b.to_vec())
                    .unwrap_or_default()
            });
        }

        self.sync_mpris();
        sender.output(PlayerOutput::NowPlaying).ok();
    }

    fn sync_mpris(&self) {
        let mpris = self.mpris.clone();

        let status = if self.playing {
            PlaybackStatus::Playing
        } else if self.current_track.is_some() {
            PlaybackStatus::Paused
        } else {
            PlaybackStatus::Stopped
        };

        let meta = self.current_track.as_ref().map(|t| {
            let mut m = Metadata::new();
            m.set_title(Some(&t.title));
            m.set_artist(Some([&t.artist]));
            m.set_album(Some(&t.album));
            if let Some(url) = &t.art_url { m.set_art_url(Some(url)); }
            if let Some(d) = t.duration { m.set_length(Some(Time::from_micros((d * 1_000_000.0) as i64))); }
            m
        });

        gtk4::glib::spawn_future_local(async move {
            let binding = mpris.borrow();
            let Some(m) = binding.as_ref() else { return };
            m.set_playback_status(status).await.ok();
            if let Some(meta) = meta {
                m.set_metadata(meta).await.ok();
            }
        });
    }

    fn sync_mpris_position(&self) {
        let mpris = self.mpris.clone();
        let pos_micros = (self.position * 1_000_000.0) as i64;
        gtk4::glib::spawn_future_local(async move {
            let binding = mpris.borrow();
            let Some(m) = binding.as_ref() else { return };
            m.set_position(Time::from_micros(pos_micros));
        });
    }
}

fn load_pixbuf(bytes: &[u8], size: i32) -> Option<gtk4::gdk_pixbuf::Pixbuf> {
    let stream = gtk4::gio::MemoryInputStream::from_bytes(&gtk4::glib::Bytes::from(bytes));
    gtk4::gdk_pixbuf::Pixbuf::from_stream_at_scale(&stream, size, size, true, None::<&gtk4::gio::Cancellable>).ok()
}

fn format_time(secs: f64) -> String {
    let t = secs as u64;
    format!("{}:{:02}", t / 60, t % 60)
}
