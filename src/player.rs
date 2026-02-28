use gstreamer as gst;
use gstreamer::prelude::*;
use gtk4::prelude::*;
use mpris_server::{Metadata, PlaybackStatus, Player as MprisPlayer, Time};
use relm4::prelude::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

const WAVEFORM_BARS: usize = 120;

#[derive(Debug, Clone)]
pub struct Track {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub art_url: Option<String>,
    pub stream_url: String,
    pub duration: Option<f64>,
}

impl From<crate::bandcamp::TrackInfo> for Track {
    fn from(t: crate::bandcamp::TrackInfo) -> Self {
        Self {
            title: t.title,
            artist: t.artist,
            album: t.album,
            art_url: t.art_url,
            stream_url: t.stream_url.unwrap_or_default(),
            duration: t.duration,
        }
    }
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
    tracklist_visible: bool,
    art_pixbuf: Option<gtk4::gdk_pixbuf::Pixbuf>,
    mpris: Rc<RefCell<Option<MprisPlayer>>>,
    waveform_bars: Rc<RefCell<Vec<f64>>>,
    waveform_progress: Rc<Cell<f64>>,
    waveform_dragging: Rc<Cell<bool>>,
    waveform_area: gtk4::DrawingArea,
    tracklist_box: gtk4::ListBox,
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
    ToggleTracklist,
    JumpToTrack(usize),
}

#[derive(Debug)]
pub enum PlayerOutput {
    NowPlaying,
    Wishlist,
    VolumeChanged(f64),
}

fn volume_icon(vol: f64) -> &'static str {
    if vol <= 0.0 {
        "audio-volume-muted-symbolic"
    } else if vol < 0.33 {
        "audio-volume-low-symbolic"
    } else if vol < 0.66 {
        "audio-volume-medium-symbolic"
    } else {
        "audio-volume-high-symbolic"
    }
}

fn generate_waveform(seed: &str) -> Vec<f64> {
    let mut h: u64 = 5381;
    for b in seed.bytes() {
        h = h.wrapping_mul(33).wrapping_add(b as u64);
    }
    (0..WAVEFORM_BARS)
        .map(|_| {
            h = h
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let v = ((h >> 33) as f64) / (u32::MAX as f64);
            0.2 + 0.8 * v
        })
        .collect()
}

#[relm4::component(pub)]
impl Component for Player {
    type Init = ();
    type Input = PlayerMsg;
    type Output = PlayerOutput;
    type CommandOutput = Vec<u8>;

    view! {
        gtk4::Revealer {
            set_transition_type: gtk4::RevealerTransitionType::SlideUp,
            set_transition_duration: 200,
            #[watch]
            set_reveal_child: model.current_track.is_some(),

            gtk4::Box {
            set_orientation: gtk4::Orientation::Vertical,

            // Tracklist revealer
            #[name = "tracklist_revealer"]
            gtk4::Revealer {
                set_transition_type: gtk4::RevealerTransitionType::SlideDown,
                set_transition_duration: 150,
                #[watch]
                set_reveal_child: model.tracklist_visible && model.queue.len() > 1,

                gtk4::ScrolledWindow {
                    set_max_content_height: 200,
                    set_propagate_natural_height: true,
                    set_hscrollbar_policy: gtk4::PolicyType::Never,
                    add_css_class: "tracklist-scroll",

                    #[name = "tracklist_box_ref"]
                    gtk4::ListBox {
                        set_selection_mode: gtk4::SelectionMode::None,
                        add_css_class: "tracklist",
                    },
                },
            },

            gtk4::Separator {},

            // Row 1: Art, info, controls
            gtk4::Box {
                set_orientation: gtk4::Orientation::Horizontal,
                set_spacing: 8,
                set_margin_start: 8,
                set_margin_end: 8,
                set_margin_top: 8,

                #[name = "art_box"]
                gtk4::Box {
                    set_valign: gtk4::Align::Center,
                    set_cursor_from_name: Some("pointer"),
                    set_tooltip_text: Some("Open in Browser"),

                    gtk4::Frame {
                        add_css_class: "album-art",

                        #[name = "art_image"]
                        gtk4::Image {
                            set_pixel_size: 42,
                        },
                    },
                },

                gtk4::Box {
                    set_orientation: gtk4::Orientation::Vertical,
                    set_valign: gtk4::Align::Center,
                    set_hexpand: true,

                    gtk4::Label {
                        set_xalign: 0.0,
                        set_ellipsize: gtk4::pango::EllipsizeMode::End,
                        add_css_class: "album-title",
                        add_css_class: "caption",
                        #[watch]
                        set_label: &model.current_track.as_ref().map(|t| t.title.as_str()).unwrap_or(""),
                    },

                    gtk4::Label {
                        set_xalign: 0.0,
                        set_ellipsize: gtk4::pango::EllipsizeMode::End,
                        add_css_class: "dim-label",
                        add_css_class: "caption",
                        #[watch]
                        set_label: &{
                            let artist = model.current_track.as_ref().map(|t| t.artist.as_str()).unwrap_or("");
                            let album = model.current_track.as_ref().map(|t| t.album.as_str()).unwrap_or("");
                            if artist == album || album.is_empty() {
                                artist.to_string()
                            } else {
                                format!("{} — {}", artist, album)
                            }
                        },
                    },
                },

                // Tracklist toggle button
                gtk4::Button {
                    set_icon_name: "view-list-symbolic",
                    add_css_class: "flat",
                    set_valign: gtk4::Align::Center,
                    set_tooltip_text: Some("Track list"),
                    #[watch]
                    set_visible: model.queue.len() > 1,
                    connect_clicked => PlayerMsg::ToggleTracklist,
                },

                gtk4::Label {
                    add_css_class: "dim-label",
                    add_css_class: "caption",
                    add_css_class: "numeric",
                    set_valign: gtk4::Align::Center,
                    #[watch]
                    set_label: &if model.queue.len() > 1 {
                        format!("{}/{}", model.queue_index + 1, model.queue.len())
                    } else {
                        String::new()
                    },
                    #[watch]
                    set_visible: model.queue.len() > 1,
                },

                gtk4::Button {
                    set_icon_name: "media-skip-backward-symbolic",
                    add_css_class: "flat",
                    set_valign: gtk4::Align::Center,
                    #[watch]
                    set_sensitive: model.queue_index > 0,
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
                    #[watch]
                    set_sensitive: model.queue_index + 1 < model.queue.len(),
                    connect_clicked => PlayerMsg::Next,
                },

                #[name = "extra_controls"]
                gtk4::Box {
                    set_orientation: gtk4::Orientation::Horizontal,
                    set_spacing: 4,
                    set_valign: gtk4::Align::Center,
                    set_widget_name: "player-extra-controls",

                    gtk4::Image {
                        #[watch]
                        set_icon_name: Some(volume_icon(model.volume)),
                        set_valign: gtk4::Align::Center,
                    },

                    #[name = "volume_scale"]
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
                },
            },

            // Row 2: Waveform seek
            gtk4::Box {
                set_orientation: gtk4::Orientation::Horizontal,
                set_spacing: 8,
                set_margin_start: 8,
                set_margin_end: 8,
                set_margin_top: 4,
                set_margin_bottom: 8,

                gtk4::Label {
                    set_width_chars: 5,
                    add_css_class: "caption",
                    add_css_class: "numeric",
                    set_valign: gtk4::Align::Center,
                    #[watch]
                    set_label: &format_time(model.position),
                },

                #[name = "waveform_container"]
                gtk4::Box {
                    set_hexpand: true,
                    set_valign: gtk4::Align::Center,
                },

                gtk4::Label {
                    set_width_chars: 5,
                    add_css_class: "caption",
                    add_css_class: "numeric",
                    set_valign: gtk4::Align::Center,
                    #[watch]
                    set_label: &format_time(model.duration),
                },
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
        let bus_watch = bus
            .add_watch_local(move |_, msg| {
                match msg.view() {
                    gst::MessageView::Eos(_) => s.input(PlayerMsg::EOS),
                    gst::MessageView::Error(err) => {
                        eprintln!("GStreamer error: {:?}", err.error());
                        s.input(PlayerMsg::EOS);
                    }
                    _ => {}
                }
                gst::glib::ControlFlow::Continue
            })
            .unwrap();

        let s = sender.clone();
        gtk4::glib::timeout_add_local(Duration::from_millis(250), move || {
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
                .can_play(true)
                .can_pause(true)
                .can_go_next(true)
                .can_go_previous(true)
                .can_seek(true)
                .can_control(true)
                .build()
                .await
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

        let waveform_bars: Rc<RefCell<Vec<f64>>> = Rc::new(RefCell::new(Vec::new()));
        let waveform_progress: Rc<Cell<f64>> = Rc::new(Cell::new(0.0));
        let waveform_dragging: Rc<Cell<bool>> = Rc::new(Cell::new(false));

        let waveform_area = gtk4::DrawingArea::new();
        waveform_area.set_content_height(28);
        waveform_area.set_hexpand(true);
        waveform_area.set_cursor_from_name(Some("pointer"));

        let bars_draw = waveform_bars.clone();
        let progress_draw = waveform_progress.clone();
        waveform_area.set_draw_func(move |_, cr, w, h| {
            let bars = bars_draw.borrow();
            let n = bars.len();
            if n == 0 {
                return;
            }

            let w = w as f64;
            let h = h as f64;
            let progress = progress_draw.get();
            let bar_pitch = w / n as f64;
            let gap = 1.0_f64.min(bar_pitch * 0.25);
            let bar_w = (bar_pitch - gap).max(1.0);

            cr.set_source_rgba(0.85, 0.28, 0.28, 1.0);
            for (i, &val) in bars.iter().enumerate() {
                let x = i as f64 * bar_pitch;
                if (x + bar_w * 0.5) / w > progress {
                    continue;
                }
                let bar_h = val * h * 0.85;
                let y = (h - bar_h) / 2.0;
                cr.rectangle(x, y, bar_w, bar_h);
            }
            let _ = cr.fill();

            cr.set_source_rgba(1.0, 1.0, 1.0, 0.12);
            for (i, &val) in bars.iter().enumerate() {
                let x = i as f64 * bar_pitch;
                if (x + bar_w * 0.5) / w <= progress {
                    continue;
                }
                let bar_h = val * h * 0.85;
                let y = (h - bar_h) / 2.0;
                cr.rectangle(x, y, bar_w, bar_h);
            }
            let _ = cr.fill();
        });

        let drag = gtk4::GestureDrag::new();
        {
            let area = waveform_area.clone();
            let progress = waveform_progress.clone();
            let dragging = waveform_dragging.clone();
            drag.connect_drag_begin(move |_, start_x, _| {
                dragging.set(true);
                let w = area.width() as f64;
                if w > 0.0 {
                    progress.set((start_x / w).clamp(0.0, 1.0));
                    area.queue_draw();
                }
            });
        }
        {
            let area = waveform_area.clone();
            let progress = waveform_progress.clone();
            drag.connect_drag_update(move |gesture, offset_x, _| {
                if let Some((start_x, _)) = gesture.start_point() {
                    let w = area.width() as f64;
                    if w > 0.0 {
                        progress.set(((start_x + offset_x) / w).clamp(0.0, 1.0));
                        area.queue_draw();
                    }
                }
            });
        }
        {
            let area = waveform_area.clone();
            let dragging = waveform_dragging.clone();
            let s = sender.clone();
            drag.connect_drag_end(move |gesture, offset_x, _| {
                dragging.set(false);
                if let Some((start_x, _)) = gesture.start_point() {
                    let w = area.width() as f64;
                    if w > 0.0 {
                        let frac = ((start_x + offset_x) / w).clamp(0.0, 1.0);
                        s.input(PlayerMsg::Seek(frac));
                    }
                }
            });
        }
        waveform_area.add_controller(drag);

        // Placeholder — replaced after view_output!()
        let tracklist_box_placeholder = gtk4::ListBox::new();

        let mut model = Self {
            pipeline,
            current_track: None,
            queue: Vec::new(),
            queue_index: 0,
            playing: false,
            position: 0.0,
            duration: 0.0,
            volume: 1.0,
            tracklist_visible: false,
            art_pixbuf: None,
            mpris,
            waveform_bars,
            waveform_progress,
            waveform_dragging,
            waveform_area: waveform_area.clone(),
            tracklist_box: tracklist_box_placeholder,
            _bus_watch: Some(bus_watch),
        };

        let widgets = view_output!();
        model.tracklist_box = widgets.tracklist_box_ref.clone();
        widgets.waveform_container.append(&waveform_area);

        let s = sender.clone();
        let art_click = gtk4::GestureClick::new();
        art_click.connect_released(move |_, _, _, _| {
            s.input(PlayerMsg::Wishlist);
        });
        widgets.art_box.add_controller(art_click);

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
                self.rebuild_tracklist(&sender);
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
                    self.highlight_current_track();
                    self.play_current(sender.clone());
                }
            }
            PlayerMsg::Prev => {
                if self.queue_index > 0 {
                    self.queue_index -= 1;
                    self.highlight_current_track();
                    self.play_current(sender.clone());
                }
            }
            PlayerMsg::Seek(frac) => {
                if self.duration > 0.0 {
                    let ns = (frac * self.duration * 1_000_000_000.0) as u64;
                    self.pipeline
                        .seek_simple(gst::SeekFlags::FLUSH, gst::ClockTime::from_nseconds(ns))
                        .ok();
                    self.position = frac * self.duration;
                    self.waveform_progress.set(frac);
                    self.waveform_area.queue_draw();
                }
            }
            PlayerMsg::SetVolume(v) => {
                self.volume = v;
                self.pipeline.set_property("volume", v);
                if (widgets.volume_scale.value() - v).abs() > 0.001 {
                    widgets.volume_scale.set_value(v);
                }
                sender.output(PlayerOutput::VolumeChanged(v)).ok();
            }
            PlayerMsg::Tick => {
                if self.playing {
                    if let Some(pos) = self.pipeline.query_position::<gst::ClockTime>() {
                        self.position = pos.seconds() as f64;
                    }
                    if let Some(dur) = self.pipeline.query_duration::<gst::ClockTime>() {
                        self.duration = dur.seconds() as f64;
                    }
                    if self.duration > 0.0 && !self.waveform_dragging.get() {
                        self.waveform_progress.set(self.position / self.duration);
                        self.waveform_area.queue_draw();
                    }
                    self.sync_mpris_position();
                }
            }
            PlayerMsg::EOS => {
                if self.queue_index + 1 < self.queue.len() {
                    self.queue_index += 1;
                    self.highlight_current_track();
                    self.play_current(sender.clone());
                } else {
                    self.pipeline.set_state(gst::State::Null).ok();
                    self.playing = false;
                    self.position = 0.0;
                    self.sync_mpris();
                }
            }
            PlayerMsg::SetArt(bytes) => {
                if let Some(pb) = load_pixbuf(&bytes, 42) {
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
            PlayerMsg::ToggleTracklist => {
                self.tracklist_visible = !self.tracklist_visible;
            }
            PlayerMsg::JumpToTrack(idx) => {
                if idx < self.queue.len() {
                    self.queue_index = idx;
                    self.highlight_current_track();
                    self.play_current(sender.clone());
                }
            }
        }

        self.update_view(widgets, sender);
    }

    fn update_cmd(
        &mut self,
        bytes: Self::CommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        if !bytes.is_empty() {
            sender.input(PlayerMsg::SetArt(bytes));
        }
    }
}

impl Player {
    fn play_current(&mut self, sender: ComponentSender<Self>) {
        let Some(track) = self.queue.get(self.queue_index).cloned() else {
            return;
        };

        self.pipeline.set_state(gst::State::Null).ok();
        self.pipeline.set_property("uri", &track.stream_url);
        self.pipeline.set_property("volume", self.volume);
        self.pipeline.set_state(gst::State::Playing).ok();

        self.playing = true;
        self.position = 0.0;
        self.duration = track.duration.unwrap_or(0.0);
        self.art_pixbuf = None;
        self.current_track = Some(track.clone());

        let seed = format!("{}-{}", track.title, track.artist);
        *self.waveform_bars.borrow_mut() = generate_waveform(&seed);
        self.waveform_progress.set(0.0);
        self.waveform_area.queue_draw();

        if let Some(url) = &track.art_url {
            let url = url.clone();
            sender.oneshot_command(async move {
                match reqwest::get(&url).await {
                    Ok(r) => r.bytes().await.map(|b| b.to_vec()).unwrap_or_default(),
                    Err(_) => Vec::new(),
                }
            });
        }

        self.sync_mpris();
        sender.output(PlayerOutput::NowPlaying).ok();
    }

    fn rebuild_tracklist(&self, sender: &ComponentSender<Self>) {
        while let Some(child) = self.tracklist_box.first_child() {
            self.tracklist_box.remove(&child);
        }

        for (i, track) in self.queue.iter().enumerate() {
            let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
            row.set_margin_start(12);
            row.set_margin_end(12);
            row.set_margin_top(4);
            row.set_margin_bottom(4);

            let num_label = gtk4::Label::new(Some(&format!("{}", i + 1)));
            num_label.add_css_class("dim-label");
            num_label.add_css_class("caption");
            num_label.add_css_class("numeric");
            num_label.set_width_chars(3);
            num_label.set_xalign(1.0);
            row.append(&num_label);

            let title_label = gtk4::Label::new(Some(&track.title));
            title_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
            title_label.set_hexpand(true);
            title_label.set_xalign(0.0);
            title_label.add_css_class("caption");
            if i == self.queue_index {
                title_label.add_css_class("accent");
            }
            row.append(&title_label);

            if let Some(dur) = track.duration {
                let dur_label = gtk4::Label::new(Some(&format_time(dur)));
                dur_label.add_css_class("dim-label");
                dur_label.add_css_class("caption");
                dur_label.add_css_class("numeric");
                row.append(&dur_label);
            }

            let list_row = gtk4::ListBoxRow::new();
            list_row.set_child(Some(&row));
            list_row.set_cursor_from_name(Some("pointer"));

            let s = sender.clone();
            let click = gtk4::GestureClick::new();
            click.connect_released(move |_, _, _, _| {
                s.input(PlayerMsg::JumpToTrack(i));
            });
            list_row.add_controller(click);

            self.tracklist_box.append(&list_row);
        }
    }

    fn highlight_current_track(&self) {
        let mut idx = 0;
        let mut row = self.tracklist_box.first_child();
        while let Some(widget) = row {
            if let Some(list_row) = widget.downcast_ref::<gtk4::ListBoxRow>() {
                if let Some(hbox) = list_row.child() {
                    // The title label is the second child (after the number label)
                    let mut child = hbox.first_child();
                    let mut child_idx = 0;
                    while let Some(c) = child {
                        if child_idx == 1 {
                            if idx == self.queue_index {
                                c.add_css_class("accent");
                            } else {
                                c.remove_css_class("accent");
                            }
                        }
                        child = c.next_sibling();
                        child_idx += 1;
                    }
                }
            }
            row = widget.next_sibling();
            idx += 1;
        }
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
            if let Some(url) = &t.art_url {
                m.set_art_url(Some(url));
            }
            if let Some(d) = t.duration {
                m.set_length(Some(Time::from_micros((d * 1_000_000.0) as i64)));
            }
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

impl Drop for Player {
    fn drop(&mut self) {
        self.pipeline.set_state(gst::State::Null).ok();
    }
}

fn load_pixbuf(bytes: &[u8], size: i32) -> Option<gtk4::gdk_pixbuf::Pixbuf> {
    let stream = gtk4::gio::MemoryInputStream::from_bytes(&gtk4::glib::Bytes::from(bytes));
    gtk4::gdk_pixbuf::Pixbuf::from_stream_at_scale(
        &stream,
        size,
        size,
        true,
        None::<&gtk4::gio::Cancellable>,
    )
    .ok()
}

fn format_time(secs: f64) -> String {
    let t = secs as u64;
    format!("{}:{:02}", t / 60, t % 60)
}
