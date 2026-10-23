use super::{TabSpec, TabView, allow_navigation};
use crate::{
    app::{self, Command, PopupHost, UserEvent},
    desktop, extensions, profile, recovery,
};
use std::path::Path;
use tao::{event_loop::EventLoopProxy, window::Window};
use wry::{NewWindowResponse, Rect, WebContext, WebView, WebViewBuilder};

pub struct WebView2 {
    context: WebContext,
    popup_host: PopupHost,
    proxy: EventLoopProxy<UserEvent>,
}

impl WebView2 {
    pub fn new(directory: &Path, popup_host: PopupHost, proxy: EventLoopProxy<UserEvent>) -> Self {
        Self {
            context: WebContext::new(Some(directory.join("WebView2"))),
            popup_host,
            proxy,
        }
    }
}

impl super::Engine for WebView2 {
    fn create_tab(&mut self, window: &Window, spec: TabSpec) -> Result<Box<dyn TabView>, String> {
        let TabSpec {
            id,
            generation,
            private,
            muted,
            bounds,
            script,
            token,
        } = spec;
        let ipc = self.proxy.clone();
        let title = self.proxy.clone();
        let popup = self.proxy.clone();
        let popup_host = self.popup_host.clone();
        let navigation = self.proxy.clone();
        let mut builder = WebViewBuilder::new_with_web_context(&mut self.context)
            .with_bounds(bounds)
            .with_visible(false)
            .with_focused(false)
            .with_incognito(private)
            .with_initialization_script(&script)
            .with_initialization_script(include_str!("../sleep.js"))
            .with_document_title_changed_handler(move |text| {
                let _ = title.send_event(UserEvent::Title(id, generation, text));
            })
            .with_navigation_handler(move |url| allow_navigation(&url, &navigation))
            .with_new_window_req_handler(move |url, features| {
                let is_popup = features.size.is_some()
                    || features.position.is_some()
                    || profile::is_auth_url(&url);
                if is_popup
                    && (url == "about:blank" || profile::allowed_url(&url))
                    && let Some(response) =
                        open_popup(&popup_host, &url, &features, private, &popup)
                {
                    return response;
                }
                if profile::allowed_url(&url) {
                    let _ =
                        popup.send_event(UserEvent::Command(Command::NewTab { url: Some(url) }));
                }
                NewWindowResponse::Deny
            })
            .with_ipc_handler(move |request| {
                let local = request
                    .uri()
                    .scheme_str()
                    .is_none_or(|scheme| scheme == "data")
                    || request.uri() == "about:blank";
                if let Some(event) =
                    app::content_message(id, generation, &token, request.body(), local)
                {
                    let _ = ipc.send_event(event);
                }
            });
        if std::env::var_os("FELARIS_SMOKE_TEST").is_some() {
            #[cfg(windows)]
            if let Ok(arguments) = std::env::var("FELARIS_SMOKE_CONTENT_BROWSER_ARGUMENTS") {
                use wry::WebViewBuilderExtWindows;
                builder = builder.with_additional_browser_args(arguments);
            }
            builder = builder.with_download_started_handler(|_, path| {
                *path = profile::directory().join("smoke-download.txt");
                true
            });
        }
        #[cfg(windows)]
        if extensions::enabled_at_startup() {
            use wry::WebViewBuilderExtWindows;
            let _ = std::fs::create_dir_all(extensions::directory());
            builder = builder
                .with_browser_extensions_enabled(true)
                .with_extensions_path(extensions::directory());
        }
        let view = builder
            .build_as_child(window)
            .map_err(|error| error.to_string())?;
        desktop::attach(&view, id, generation, self.proxy.clone())
            .map_err(|error| error.to_string())?;
        let _ = desktop::apply_privacy(&view, &crate::settings::get());
        let _ = desktop::mute(&view, muted);
        Ok(Box::new(view))
    }
}

/// Opens sign-in popups in the shared floating window so they keep `window.opener`.
#[cfg(windows)]
fn open_popup(
    host: &PopupHost,
    url: &str,
    features: &wry::NewWindowFeatures,
    private: bool,
    proxy: &EventLoopProxy<UserEvent>,
) -> Option<NewWindowResponse> {
    use wry::{WebViewBuilderExtWindows, WebViewExtWindows};
    host.present(url, features.size, features.position);
    let title_proxy = proxy.clone();
    let nav_proxy = proxy.clone();
    let tab_proxy = proxy.clone();
    let (_, content_rect) = app::popup_bounds(&host.window);
    let mut builder = WebViewBuilder::new()
        .with_environment(features.opener.environment.clone())
        .with_incognito(private)
        .with_bounds(content_rect)
        .with_focused(!host.headless)
        .with_clipboard(true)
        .with_document_title_changed_handler(move |text| {
            let _ = title_proxy.send_event(UserEvent::PopupTitle(text));
        })
        .with_navigation_handler(move |target| allow_navigation(&target, &nav_proxy))
        .with_new_window_req_handler(move |target, _| {
            if profile::allowed_url(&target) {
                let _ =
                    tab_proxy.send_event(UserEvent::Command(Command::NewTab { url: Some(target) }));
            }
            NewWindowResponse::Deny
        });
    if host.headless
        && let Ok(arguments) = std::env::var("FELARIS_SMOKE_CONTENT_BROWSER_ARGUMENTS")
    {
        builder = builder.with_additional_browser_args(arguments);
    }
    let view = builder.build_as_child(&*host.window).ok()?;
    let _ = desktop::attach_popup(&view, proxy.clone());
    let raw = view.webview();
    *host.view.borrow_mut() = Some(Box::new(view));
    let _ = proxy.send_event(UserEvent::PopupState(url.to_string(), true));
    Some(NewWindowResponse::Create { webview: raw })
}

#[cfg(not(windows))]
fn open_popup(
    _: &PopupHost,
    _: &str,
    _: &wry::NewWindowFeatures,
    _: bool,
    _: &EventLoopProxy<UserEvent>,
) -> Option<NewWindowResponse> {
    None
}

impl TabView for WebView {
    fn load_url(&self, url: &str, fresh: bool) -> Result<(), String> {
        if fresh {
            let mut headers = wry::http::HeaderMap::new();
            headers.insert(
                wry::http::header::CACHE_CONTROL,
                wry::http::HeaderValue::from_static("no-cache"),
            );
            WebView::load_url_with_headers(self, url, headers)
        } else {
            WebView::load_url(self, url)
        }
        .map_err(|error| error.to_string())
    }

    fn load_html(&self, html: &str) -> Result<(), String> {
        WebView::load_html(self, html).map_err(|error| error.to_string())
    }

    fn reload(&self) -> Result<(), String> {
        WebView::reload(self).map_err(|error| error.to_string())
    }

    fn navigate(&self, backwards: bool) {
        desktop::navigate(self, backwards);
    }

    fn url(&self) -> Result<String, String> {
        WebView::url(self).map_err(|error| error.to_string())
    }

    fn evaluate_script(&self, script: &str) -> Result<(), String> {
        WebView::evaluate_script(self, script).map_err(|error| error.to_string())
    }

    fn evaluate_script_with_callback(
        &self,
        script: &str,
        callback: Box<dyn Fn(String) + Send + 'static>,
    ) -> Result<(), String> {
        WebView::evaluate_script_with_callback(self, script, callback)
            .map_err(|error| error.to_string())
    }

    fn set_bounds(&self, bounds: Rect) -> Result<(), String> {
        WebView::set_bounds(self, bounds).map_err(|error| error.to_string())
    }

    fn show(&self, visible: bool) {
        desktop::show(self, visible);
    }

    fn round(&self, width: i32, height: i32, radius: i32) {
        desktop::round(self, width, height, radius);
    }

    fn focus(&self) -> Result<(), String> {
        WebView::focus(self).map_err(|error| error.to_string())
    }

    fn zoom(&self, scale: f64) -> Result<(), String> {
        WebView::zoom(self, scale).map_err(|error| error.to_string())
    }

    fn print(&self) -> Result<(), String> {
        WebView::print(self).map_err(|error| error.to_string())
    }

    fn is_quiet(&self) -> bool {
        desktop::quiet(self)
    }

    fn set_muted(&self, muted: bool) -> Result<(), String> {
        desktop::mute(self, muted)
    }

    fn apply_privacy(&self, settings: &crate::settings::Settings) -> Result<(), String> {
        desktop::apply_privacy(self, settings)
    }

    fn clear_browsing_data(
        &self,
        kinds: desktop::DataKinds,
        range_ms: Option<u64>,
        completed: Box<dyn FnOnce(Result<(), String>)>,
    ) -> Result<(), String> {
        desktop::clear_browsing_data(self, kinds, range_ms, completed)
    }

    fn screenshot(
        &self,
        title: &str,
        completed: Box<dyn FnOnce(Result<std::path::PathBuf, String>)>,
    ) -> Result<(), String> {
        desktop::screenshot(self, title, completed)
    }

    fn open_downloads(&self) -> Result<(), String> {
        desktop::downloads(self)
    }

    fn clear_site_data(
        &self,
        id: usize,
        generation: u64,
        origin: &str,
        proxy: EventLoopProxy<UserEvent>,
    ) -> Result<(), String> {
        desktop::clear_site_data(self, id, generation, origin, proxy)
    }

    fn clear_site_files(
        &self,
        page: recovery::Page,
        proxy: EventLoopProxy<UserEvent>,
    ) -> Result<(), String> {
        desktop::clear_site_files(self, page, proxy)
    }

    fn reload_fresh(
        &self,
        page: recovery::Page,
        proxy: EventLoopProxy<UserEvent>,
    ) -> Result<(), String> {
        desktop::reload_fresh(self, page, proxy)
    }
}
