use gtk4::prelude::*;
use libadwaita as adw;
use relm4::prelude::*;
use webkit6::prelude::*;
use webkit6::WebView;

const LOGIN_URL: &str = "https://bandcamp.com/login";
const BASE_URL: &str = "https://bandcamp.com";

pub struct LoginPage {
    webview: WebView,
}

#[derive(Debug)]
pub enum LoginMsg {
    UrlChanged,
    PageLoaded,
}

#[derive(Debug, Clone)]
pub enum LoginOutput {
    Success(String),
}

#[relm4::component(pub)]
impl SimpleComponent for LoginPage {
    type Init = ();
    type Input = LoginMsg;
    type Output = LoginOutput;

    view! {
        adw::ToolbarView {
            add_top_bar = &adw::HeaderBar {
                #[wrap(Some)]
                set_title_widget = &adw::WindowTitle {
                    set_title: "Login to Bandcamp",
                    set_subtitle: "Sign in to access your collection",
                },
                
                pack_start = &gtk4::Button {
                    set_icon_name: "go-home-symbolic",
                    set_tooltip_text: Some("Back to login"),
                    connect_clicked[webview] => move |_| {
                        webview.load_uri(LOGIN_URL);
                    },
                },
            },

            #[wrap(Some)]
            #[local_ref]
            set_content = webview_ref -> WebView {
                set_vexpand: true,
                set_hexpand: true,
            },
        }
    }

    fn init(
        _: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let webview = WebView::new();
        
        if let Some(settings) = webkit6::prelude::WebViewExt::settings(&webview) {
            settings.set_javascript_can_access_clipboard(true);
        }

        webview.load_uri(LOGIN_URL);

        let s = sender.clone();
        webview.connect_uri_notify(move |_| {
            s.input(LoginMsg::UrlChanged);
        });

        let s = sender.clone();
        webview.connect_load_changed(move |_, event| {
            if event == webkit6::LoadEvent::Finished {
                s.input(LoginMsg::PageLoaded);
            }
        });

        let model = LoginPage { webview: webview.clone() };
        let webview_ref = &model.webview;
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            LoginMsg::UrlChanged | LoginMsg::PageLoaded => {
                let uri = self.webview.uri().map(|u| u.to_string()).unwrap_or_default();
                
                if uri.starts_with(BASE_URL) && !uri.contains("/login") && !uri.contains("/signup") {
                    self.try_extract_cookies(sender);
                }
            }
        }
    }
}

impl LoginPage {
    fn try_extract_cookies(&self, sender: ComponentSender<Self>) {
        
        let Some(session) = self.webview.network_session() else {
            
            return;
        };
        let Some(manager) = session.cookie_manager() else {
            
            return;
        };

        manager.cookies(BASE_URL, None::<&gtk4::gio::Cancellable>, move |result| {
            
            if let Ok(mut cookies) = result {
                let mut parts = Vec::new();
                let mut has_identity = false;

                for cookie in &mut cookies {
                    if let (Some(name), Some(value)) = (cookie.name(), cookie.value()) {
                        
                        if name == "identity" {
                            has_identity = true;
                        }
                        parts.push(format!("{}={}", name, value));
                    }
                }

                
                if has_identity {
                    sender.output(LoginOutput::Success(parts.join("; "))).ok();
                }
            }
        });
    }
}
