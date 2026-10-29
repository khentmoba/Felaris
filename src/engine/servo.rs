//! Experimental Servo engine (`--features servo`, launched with `--engine=servo`).
//!
//! All tabs share one child window over the content area and one ANGLE rendering context;
//! only the visible tab is painted. Sign-in popups get a second surface in the floating sign-in
//! window. Servo has no host IPC channel, so `window.ipc.postMessage` is shimmed onto a tagged
//! console message that the delegate turns back into an app event.

mod controls;
mod downloads;

use super::{PopupView, TabSpec, TabView, allow_navigation};
use crate::{
    app::{self, Command, PopupHost, UserEvent},
    recovery,
};
use servo::{
    AuthenticationRequest, CompositionEvent, CompositionState, ConsoleLogLevel, Cursor,
    DownloadHandle, DownloadRequest, EmbedderControl, EmbedderControlId, ImeEvent, InputEvent,
    JSValue, Key, KeyState, LoadStatus, Location, Modifiers, MouseButton, MouseButtonAction,
    MouseButtonEvent, MouseMoveEvent, NamedKey, NavigationRequest, PermissionRequest,
    RenderingContext, Servo, ServoBuilder, StorageType, UserContentManager, UserScript, WebView,
    WebViewBuilder, WebViewDelegate, WebViewPoint, WheelDelta, WheelEvent, WheelMode,
    WindowRenderingContext,
};
use std::{
    cell::{Cell, RefCell},
    path::{Path, PathBuf},
    rc::{Rc, Weak},
};
use tao::{
    dpi::PhysicalSize,
    event::{ElementState, MouseScrollDelta, WindowEvent},
    event_loop::{EventLoopProxy, EventLoopWindowTarget},
    keyboard::{KeyCode, ModifiersState},
    platform::windows::{WindowBuilderExtWindows, WindowExtWindows},
    rwh_06::{HasDisplayHandle, HasWindowHandle},
    window::{CursorIcon, Window, WindowBuilder, WindowId},
};
use wry::Rect;

const IPC_PREFIX: &str = "\u{1}felaris-ipc:";

/// A child window that Servo paints one webview into at a time.
struct Surface {
    window: Window,
    rendering: Rc<WindowRenderingContext>,
    /// The top-level window that owns this surface's dialogs.
    owner: isize,
    proxy: EventLoopProxy<UserEvent>,
    /// The webview currently painted into the surface.
    active: RefCell<Option<WebView>>,
    dirty: Cell<bool>,
    size: Cell<PhysicalSize<u32>>,
    pointer: Cell<servo::DevicePoint>,
}

impl Surface {
    fn new(
        target: &EventLoopWindowTarget<UserEvent>,
        owner: &Window,
        proxy: EventLoopProxy<UserEvent>,
    ) -> Result<Self, String> {
        let window = WindowBuilder::new()
            .with_parent_window(owner.hwnd() as _)
            .with_decorations(false)
            .with_resizable(false)
            .with_undecorated_shadow(false)
            .with_visible(false)
            .with_focused(false)
            .with_inner_size(PhysicalSize::new(100, 100))
            .build(target)
            .map_err(|error| error.to_string())?;
        let size = window.inner_size();
        let rendering = Rc::new(
            WindowRenderingContext::new(
                window.display_handle().map_err(|error| error.to_string())?,
                window.window_handle().map_err(|error| error.to_string())?,
                size,
            )
            .map_err(|error| format!("Servo could not start OpenGL: {error:?}"))?,
        );
        Ok(Self {
            window,
            rendering,
            owner: owner.hwnd(),
            proxy,
            active: RefCell::new(None),
            dirty: Cell::new(false),
            size: Cell::new(size),
            pointer: Cell::new(servo::DevicePoint::new(0., 0.)),
        })
    }

    fn is_active(&self, webview: &WebView) -> bool {
        self.active
            .borrow()
            .as_ref()
            .is_some_and(|active| active.id() == webview.id())
    }

    fn request_paint(&self) {
        self.dirty.set(true);
        let _ = self.proxy.send_event(UserEvent::EngineWake);
    }

    fn paint(&self) {
        if self.dirty.replace(false)
            && let Some(webview) = self.active.borrow().as_ref()
        {
            webview.paint();
            self.rendering.present();
        }
    }

    /// Moves the surface to `bounds` (in its owner's client area) and sizes the webview to it.
    fn place(&self, bounds: Rect, webview: &WebView) -> Result<(), String> {
        use windows::Win32::{
            Foundation::HWND,
            UI::WindowsAndMessaging::{SWP_NOACTIVATE, SWP_NOZORDER, SetWindowPos},
        };
        let scale = self.window.scale_factor();
        let position = bounds.position.to_physical::<i32>(scale);
        let size = bounds.size.to_physical::<u32>(scale);
        unsafe {
            SetWindowPos(
                HWND(self.window.hwnd() as _),
                None,
                position.x,
                position.y,
                size.width as i32,
                size.height as i32,
                SWP_NOZORDER | SWP_NOACTIVATE,
            )
            .map_err(|error| error.to_string())?;
        }
        // Resizes the shared rendering context and every viewport on it. Resizing the context
        // directly would make this a no-op and leave the viewports at their old size.
        if self.size.replace(size) != size {
            webview.resize(size);
        }
        self.request_paint();
        Ok(())
    }

    /// Stops painting `webview` here, hiding the surface if it was showing it.
    fn release(&self, webview: &WebView) {
        if self.is_active(webview) {
            *self.active.borrow_mut() = None;
            self.window.set_visible(false);
        }
    }
}

struct Shared {
    /// Tab content, over the main window's content area.
    tabs: Surface,
    /// The page in the floating sign-in window.
    popup: Surface,
    popup_host: PopupHost,
    proxy: EventLoopProxy<UserEvent>,
    modifiers: Cell<ModifiersState>,
    /// `window.open` targets, kept until their first URL is known.
    openers: RefCell<Vec<(servo::WebViewId, WebView)>>,
    /// `window.open` targets to show in the sign-in window once Servo's event loop returns.
    popups: RefCell<Vec<(WebView, url::Url)>>,
    /// Form controls and dialogs to show once Servo's event loop returns.
    controls: RefCell<Vec<(WebView, EmbedderControl)>>,
    /// HTTP sign-in requests, shown at the same point as `controls`.
    logins: RefCell<Vec<(WebView, AuthenticationRequest)>>,
    /// The most recent finished download, which Open Downloads selects.
    last_download: RefCell<Option<PathBuf>>,
    /// Downloads in progress: owning tab id, file and a handle to cancel it.
    downloads: RefCell<Vec<(usize, PathBuf, DownloadHandle)>>,
}

impl Shared {
    /// Releases pending children recursively, including a pending view that is itself closing.
    /// Drop the removed views after releasing the RefCell borrow: closing them can call back.
    fn release_auxiliaries(&self, opener: servo::WebViewId) {
        let mut closing = vec![opener];
        while let Some(id) = closing.pop() {
            let pending = std::mem::take(&mut *self.openers.borrow_mut());
            let (removed, retained): (Vec<_>, Vec<_>) = pending
                .into_iter()
                .partition(|(parent, view)| *parent == id || view.id() == id);
            *self.openers.borrow_mut() = retained;
            closing.extend(
                removed
                    .iter()
                    .filter(|(_, view)| view.id() != id)
                    .map(|(_, view)| view.id()),
            );
            drop(removed);
        }
    }

    /// The surface that shows `webview` and owns its dialogs.
    fn surface_of(&self, webview: &WebView) -> &Surface {
        if self.popup.is_active(webview) {
            &self.popup
        } else {
            &self.tabs
        }
    }
}

pub struct ServoEngine {
    servo: Servo,
    shared: Rc<Shared>,
}

impl Drop for ServoEngine {
    fn drop(&mut self) {
        self.shared.popup_host.view.borrow_mut().take();
        self.shared.tabs.active.borrow_mut().take();
        self.shared.popup.active.borrow_mut().take();
        self.shared.openers.borrow_mut().clear();
        self.shared.popups.borrow_mut().clear();
        self.shared.controls.borrow_mut().clear();
        self.shared.logins.borrow_mut().clear();
    }
}

#[derive(Clone)]
struct Waker(EventLoopProxy<UserEvent>);

impl servo::EventLoopWaker for Waker {
    fn clone_box(&self) -> Box<dyn servo::EventLoopWaker> {
        Box::new(self.clone())
    }

    fn wake(&self) {
        let _ = self.0.send_event(UserEvent::EngineWake);
    }
}

impl ServoEngine {
    pub fn new(
        target: &EventLoopWindowTarget<UserEvent>,
        window: &Window,
        directory: &Path,
        popup_host: PopupHost,
        proxy: EventLoopProxy<UserEvent>,
    ) -> Result<Self, String> {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
        let tabs = Surface::new(target, window, proxy.clone())?;
        let popup = Surface::new(target, &popup_host.window, proxy.clone())?;
        tabs.rendering
            .make_current()
            .map_err(|error| format!("Servo could not start OpenGL: {error:?}"))?;
        let mut preferences = servo::Preferences::default();
        // Exercise permission decisions in isolated diagnostic runs without advertising a
        // notification backend to normal browsing sessions.
        if cfg!(debug_assertions)
            && std::env::var_os("FELARIS_SMOKE_TEST").is_some()
            && std::env::var_os("FELARIS_SERVO_TEST_PERMISSIONS").is_some()
        {
            preferences.dom_notification_enabled = true;
            preferences.dom_permissions_enabled = true;
        }
        let servo = ServoBuilder::default()
            .preferences(preferences)
            .opts(servo::Opts {
                config_dir: Some(directory.join("Servo")),
                hard_fail: false,
                // Windows provides process separation here, but no Servo sandbox.
                multiprocess: std::env::var_os("FELARIS_SERVO_SINGLE_PROCESS").is_none(),
                ..Default::default()
            })
            .event_loop_waker(Box::new(Waker(proxy.clone())))
            .build();
        Ok(Self {
            servo,
            shared: Rc::new(Shared {
                tabs,
                popup,
                popup_host,
                proxy,
                modifiers: Cell::new(ModifiersState::empty()),
                openers: RefCell::new(Vec::new()),
                popups: RefCell::new(Vec::new()),
                controls: RefCell::new(Vec::new()),
                logins: RefCell::new(Vec::new()),
                last_download: RefCell::new(None),
                downloads: RefCell::new(Vec::new()),
            }),
        })
    }
}

impl ServoEngine {
    fn spin(&mut self) {
        self.servo.spin_event_loop();
        // Resizing a webview borrows Servo's painter, so popups open outside the event loop.
        let popups = std::mem::take(&mut *self.shared.popups.borrow_mut());
        for (webview, url) in popups {
            open_popup(&self.shared, webview, &url);
        }
        let pending = std::mem::take(&mut *self.shared.controls.borrow_mut());
        let logins = std::mem::take(&mut *self.shared.logins.borrow_mut());
        if pending.is_empty() && logins.is_empty() {
            return;
        }
        for (webview, control) in pending {
            controls::show(
                &self.shared,
                self.shared.surface_of(&webview),
                &webview,
                control,
            );
        }
        for (webview, request) in logins {
            controls::sign_in(self.shared.surface_of(&webview), request);
        }
        self.shared.tabs.request_paint();
        self.shared.popup.request_paint();
    }
}

impl super::Engine for ServoEngine {
    fn create_tab(&mut self, _: &Window, spec: TabSpec) -> Result<Box<dyn TabView>, String> {
        let TabSpec {
            id,
            generation,
            private,
            bounds,
            script,
            token,
            ..
        } = spec;
        let startup = Rc::new(Startup::default());
        let content = Rc::new(UserContentManager::new(&self.servo));
        let shim = format!(
            "if(!window.ipc){{const log=console.log.bind(console);Object.defineProperty(window,'ipc',{{value:Object.freeze({{postMessage:m=>log({prefix}+m)}})}});}}",
            prefix = serde_json::to_string(IPC_PREFIX).unwrap_or_default()
        );
        for source in [shim, script, include_str!("../sleep.js").to_owned()] {
            content.add_script(Rc::new(UserScript::new(source, None)));
        }
        let webview = WebViewBuilder::new(&self.servo, self.shared.tabs.rendering.clone())
            .hidpi_scale_factor(euclid::Scale::new(
                self.shared.tabs.window.scale_factor() as f32
            ))
            .user_content_manager(content)
            .private(private)
            .delegate(Rc::new(TabDelegate {
                id,
                generation,
                token,
                shared: self.shared.clone(),
                startup: startup.clone(),
            }))
            .build();
        let tab = ServoTab {
            servo: self.servo.clone(),
            webview,
            shared: self.shared.clone(),
            startup,
            private,
        };
        let _ = tab.set_bounds(bounds);
        Ok(Box::new(tab))
    }

    fn window_event(&mut self, window_id: WindowId, event: &WindowEvent) -> bool {
        let shared = self.shared.clone();
        let is_tab = window_id == shared.tabs.window.id();
        let surface = if is_tab {
            &shared.tabs
        } else if window_id == shared.popup.window.id() {
            &shared.popup
        } else {
            // A re-activated window focuses itself; hand focus back to the page unless the
            // chrome (an address field, say) already took it.
            if matches!(event, WindowEvent::Focused(true)) {
                for surface in [&shared.tabs, &shared.popup] {
                    if surface.active.borrow().is_some() && focused_window() == Some(surface.owner)
                    {
                        focus_surface(&surface.window);
                    }
                }
            }
            return false;
        };
        let Some(webview) = surface.active.borrow().clone() else {
            return true;
        };
        let point = || WebViewPoint::Device(surface.pointer.get());
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                surface.pointer.set(servo::DevicePoint::new(
                    position.x as f32,
                    position.y as f32,
                ));
                webview.notify_input_event(InputEvent::MouseMove(MouseMoveEvent::new(point())));
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let action = match state {
                    ElementState::Pressed => {
                        focus_surface(&surface.window);
                        webview.set_focused(true);
                        MouseButtonAction::Down
                    }
                    _ => MouseButtonAction::Up,
                };
                let button = match button {
                    tao::event::MouseButton::Left => MouseButton::Primary,
                    tao::event::MouseButton::Right => MouseButton::Secondary,
                    tao::event::MouseButton::Middle => MouseButton::Auxiliary,
                    tao::event::MouseButton::Other(other) => MouseButton::Other(*other),
                    _ => MouseButton::None,
                };
                webview.notify_input_event(InputEvent::MouseButton(MouseButtonEvent::new(
                    action,
                    button,
                    point(),
                )));
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let (x, y, mode) = match delta {
                    MouseScrollDelta::LineDelta(x, y) => (
                        f64::from(*x) * 76.,
                        f64::from(*y) * 76.,
                        WheelMode::DeltaLine,
                    ),
                    MouseScrollDelta::PixelDelta(delta) => {
                        (delta.x, delta.y, WheelMode::DeltaPixel)
                    }
                    _ => return true,
                };
                webview.notify_input_event(InputEvent::Wheel(WheelEvent::new(
                    WheelDelta { x, y, z: 0., mode },
                    point(),
                )));
            }
            WindowEvent::ModifiersChanged(modifiers) => shared.modifiers.set(*modifiers),
            // tao reports IME text only once composition ends; the system IME window shows it
            // until then. Servo inserts the text on `compositionend`.
            WindowEvent::ReceivedImeText(text) => {
                for (state, data) in [
                    (CompositionState::Start, String::new()),
                    (CompositionState::End, text.clone()),
                ] {
                    webview.notify_input_event(InputEvent::Ime(ImeEvent::Composition(
                        CompositionEvent { state, data },
                    )));
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let modifiers = shared.modifiers.get();
                // Browser shortcuts win over the page, as they do in WebView2 tabs. The sign-in
                // window has none.
                if let Some(command) =
                    virtual_key(event.physical_key)
                        .filter(|_| is_tab)
                        .and_then(|key| {
                            app::shortcut(
                                key,
                                modifiers.control_key(),
                                modifiers.shift_key(),
                                modifiers.alt_key(),
                            )
                        })
                {
                    if event.state == ElementState::Pressed && !event.repeat {
                        let _ = shared.proxy.send_event(UserEvent::Command(command));
                    }
                    return true;
                }
                let state = match event.state {
                    ElementState::Pressed => KeyState::Down,
                    _ => KeyState::Up,
                };
                let mut keys = Modifiers::empty();
                keys.set(Modifiers::SHIFT, modifiers.shift_key());
                keys.set(Modifiers::CONTROL, modifiers.control_key());
                keys.set(Modifiers::ALT, modifiers.alt_key());
                keys.set(Modifiers::META, modifiers.super_key());
                webview.notify_input_event(InputEvent::Keyboard(servo::KeyboardEvent::new(
                    keyboard_types::KeyboardEvent {
                        state,
                        key: key(&event.logical_key),
                        code: code(event.physical_key),
                        location: match event.location {
                            tao::keyboard::KeyLocation::Left => Location::Left,
                            tao::keyboard::KeyLocation::Right => Location::Right,
                            tao::keyboard::KeyLocation::Numpad => Location::Numpad,
                            _ => Location::Standard,
                        },
                        modifiers: keys,
                        repeat: event.repeat,
                        is_composing: false,
                    },
                )));
            }
            _ => {}
        }
        self.spin();
        true
    }

    fn downloads(&self) -> std::collections::HashMap<usize, usize> {
        let mut counts = std::collections::HashMap::new();
        for (id, ..) in self.shared.downloads.borrow().iter() {
            *counts.entry(*id).or_default() += 1;
        }
        counts
    }

    fn end_private_session(&mut self) {
        self.servo.site_data_manager().clear_private_data();
    }

    fn wake(&mut self) {
        self.spin();
        self.shared.tabs.paint();
        self.shared.popup.paint();
    }
}

/// tao names keys after the W3C `KeyboardEvent.key` values except for `Super` (W3C `Meta`).
fn key(key: &tao::keyboard::Key<'static>) -> Key {
    use tao::keyboard::Key as Tao;
    match key {
        Tao::Character(text) => Key::Character((*text).to_owned()),
        Tao::Space => Key::Character(" ".into()),
        Tao::Super => Key::Named(NamedKey::Meta),
        other => format!("{other:?}")
            .parse()
            .map(Key::Named)
            .unwrap_or(Key::Named(NamedKey::Unidentified)),
    }
}

/// tao names physical keys after the W3C `KeyboardEvent.code` values except for the Meta keys.
fn code(code: KeyCode) -> servo::Code {
    match code {
        KeyCode::SuperLeft => servo::Code::MetaLeft,
        KeyCode::SuperRight => servo::Code::MetaRight,
        other => format!("{other:?}")
            .parse()
            .unwrap_or(servo::Code::Unidentified),
    }
}

/// The Windows virtual-key code that `app::shortcut` matches on.
fn virtual_key(code: KeyCode) -> Option<u32> {
    use KeyCode::*;
    let letters = [
        KeyA, KeyB, KeyC, KeyD, KeyE, KeyF, KeyG, KeyH, KeyI, KeyJ, KeyK, KeyL, KeyM, KeyN, KeyO,
        KeyP, KeyQ, KeyR, KeyS, KeyT, KeyU, KeyV, KeyW, KeyX, KeyY, KeyZ,
    ];
    let digits = [
        Digit0, Digit1, Digit2, Digit3, Digit4, Digit5, Digit6, Digit7, Digit8, Digit9,
    ];
    if let Some(index) = letters.iter().position(|key| *key == code) {
        return Some(u32::from(b'A') + index as u32);
    }
    if let Some(index) = digits.iter().position(|key| *key == code) {
        return Some(u32::from(b'0') + index as u32);
    }
    Some(match code {
        Tab => 0x09,
        ArrowLeft => 0x25,
        ArrowRight => 0x27,
        Numpad0 => 0x60,
        NumpadAdd => 0x6B,
        NumpadSubtract => 0x6D,
        F5 => 0x74,
        F11 => 0x7A,
        Equal => 0xBB,
        Minus => 0xBD,
        _ => return None,
    })
}

/// The surface was created before the chrome WebViews, so it starts beneath their (hidden or
/// transparent) windows; the app raises the sidebar and topbar back above it.
fn raise(surface: &Window) {
    use windows::Win32::{
        Foundation::HWND,
        UI::WindowsAndMessaging::{HWND_TOP, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetWindowPos},
    };
    unsafe {
        let _ = SetWindowPos(
            HWND(surface.hwnd() as _),
            Some(HWND_TOP),
            0,
            0,
            0,
            0,
            SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE,
        );
    }
}

fn focused_window() -> Option<isize> {
    use windows::Win32::UI::Input::KeyboardAndMouse::GetFocus;
    let focused = unsafe { GetFocus() };
    (!focused.is_invalid()).then_some(focused.0 as isize)
}

fn focus_surface(surface: &Window) {
    use windows::Win32::{Foundation::HWND, UI::Input::KeyboardAndMouse::SetFocus};
    unsafe {
        let _ = SetFocus(Some(HWND(surface.hwnd() as _)));
    }
}

/// A new WebView first loads `about:blank`, and a load issued before that finishes is lost.
/// Loads wait for it, and the blank page is kept out of the app's events and back history.
#[derive(Default)]
struct Startup {
    ready: Cell<bool>,
    pending: RefCell<Option<url::Url>>,
    history_cleared: Cell<bool>,
}

struct ServoTab {
    servo: Servo,
    webview: WebView,
    shared: Rc<Shared>,
    startup: Rc<Startup>,
    private: bool,
}

impl Drop for ServoTab {
    fn drop(&mut self) {
        self.shared.tabs.release(&self.webview);
        self.shared.release_auxiliaries(self.webview.id());
    }
}

const UNSUPPORTED: &str = "The Servo preview does not support this yet.";

impl ServoTab {
    /// Drops the HTTP cache entries for `url`'s host and its subdomains. The page's
    /// subresources from other hosts stay cached.
    fn clear_cache_for(&self, url: &str) {
        if let Some(host) = url::Url::parse(url)
            .ok()
            .and_then(|url| url.host_str().map(str::to_owned))
        {
            self.servo.network_manager().clear_cache_for_sites(&[&host]);
        }
    }
}

impl TabView for ServoTab {
    fn load_url(&self, url: &str, fresh: bool) -> Result<(), String> {
        let url = url::Url::parse(url).map_err(|error| error.to_string())?;
        // Servo cannot skip the cache for one load, only drop the site's entries.
        if fresh {
            self.clear_cache_for(url.as_str());
        }
        if self.startup.ready.get() {
            self.webview.load(url);
        } else {
            *self.startup.pending.borrow_mut() = Some(url);
        }
        Ok(())
    }

    fn load_html(&self, html: &str) -> Result<(), String> {
        let mut url = String::from("data:text/html;charset=utf-8,");
        for byte in html.bytes() {
            if byte.is_ascii_alphanumeric() || b"-_.~ !$'()*,;:@/?=&+".contains(&byte) {
                url.push(byte as char);
            } else {
                url.push_str(&format!("%{byte:02X}"));
            }
        }
        self.load_url(&url, false)
    }

    fn reload(&self) -> Result<(), String> {
        self.webview.reload();
        Ok(())
    }

    fn navigate(&self, backwards: bool) {
        if backwards {
            self.webview.go_back(1);
        } else {
            self.webview.go_forward(1);
        }
    }

    fn url(&self) -> Result<String, String> {
        self.webview
            .url()
            .map(String::from)
            .ok_or_else(|| "This tab has no address yet.".into())
    }

    fn evaluate_script(&self, script: &str) -> Result<(), String> {
        self.webview.evaluate_javascript(script, |_| {});
        Ok(())
    }

    fn evaluate_script_with_callback(
        &self,
        script: &str,
        callback: Box<dyn Fn(String) + Send + 'static>,
    ) -> Result<(), String> {
        self.webview.evaluate_javascript(script, move |result| {
            callback(match result {
                Ok(JSValue::Boolean(value)) => value.to_string(),
                Ok(JSValue::Number(value)) => value.to_string(),
                Ok(JSValue::String(value)) => serde_json::to_string(&value).unwrap_or_default(),
                _ => "null".into(),
            });
        });
        Ok(())
    }

    fn set_bounds(&self, bounds: Rect) -> Result<(), String> {
        self.shared.tabs.place(bounds, &self.webview)
    }

    fn show(&self, visible: bool) {
        let surface = &self.shared.tabs;
        if visible {
            self.webview.show();
            *surface.active.borrow_mut() = Some(self.webview.clone());
            surface.window.set_visible(true);
            raise(&surface.window);
            surface.request_paint();
        } else {
            self.webview.hide();
            surface.release(&self.webview);
        }
    }

    fn round(&self, width: i32, height: i32, radius: i32) {
        use windows::Win32::{
            Foundation::HWND,
            Graphics::Gdi::{CreateRoundRectRgn, SetWindowRgn},
        };
        unsafe {
            let region = (radius > 0)
                .then(|| CreateRoundRectRgn(0, 0, width + 1, height + 1, radius * 2, radius * 2));
            SetWindowRgn(HWND(self.shared.tabs.window.hwnd() as _), region, true);
        }
    }

    fn focus(&self) -> Result<(), String> {
        focus_surface(&self.shared.tabs.window);
        self.webview.set_focused(true);
        Ok(())
    }

    fn zoom(&self, scale: f64) -> Result<(), String> {
        self.webview.set_page_zoom(scale as f32);
        Ok(())
    }

    fn print(&self) -> Result<(), String> {
        Err(UNSUPPORTED.into())
    }

    fn is_quiet(&self) -> bool {
        true
    }

    fn set_muted(&self, _: bool) -> Result<(), String> {
        Err(UNSUPPORTED.into())
    }

    /// Servo has no download manager window, so running downloads can be cancelled here.
    fn open_downloads(&self) -> Result<(), String> {
        let running = self.shared.downloads.borrow().len();
        if running > 0 && downloads::confirm_cancel(self.shared.tabs.owner, running) {
            for (_, _, handle) in self.shared.downloads.borrow().iter() {
                handle.cancel();
            }
            return Ok(());
        }
        let latest = self
            .shared
            .downloads
            .borrow()
            .last()
            .map(|(_, path, _)| path.clone());
        downloads::reveal(
            latest
                .or_else(|| self.shared.last_download.borrow().clone())
                .as_deref(),
        )
    }

    fn clear_site_data(
        &self,
        id: usize,
        generation: u64,
        origin: &str,
        proxy: EventLoopProxy<UserEvent>,
    ) -> Result<(), String> {
        // Servo clears a site from public and private storage together.
        if self.private {
            return Err("Private tabs keep site data only until Felaris closes.".into());
        }
        let host = url::Url::parse(origin)
            .ok()
            .and_then(|url| url.host_str().map(str::to_owned))
            .ok_or("This page has no web origin.")?;
        // Servo keys site data by registrable domain, which covers the origin's host.
        let manager = self.servo.site_data_manager();
        let covering = manager
            .site_data(StorageType::all())
            .into_iter()
            .map(|site| site.name())
            .filter(|name| host == *name || host.ends_with(&format!(".{name}")))
            .collect::<Vec<_>>();
        let names = covering.iter().map(String::as_str).collect::<Vec<_>>();
        manager.clear_site_data(&names, StorageType::all());
        let _ = proxy.send_event(UserEvent::SiteDataCleared(
            id,
            generation,
            origin.to_owned(),
            Ok(()),
        ));
        Ok(())
    }

    /// Servo has no Cache Storage or service workers, so a site's files are its HTTP cache.
    fn clear_site_files(
        &self,
        page: recovery::Page,
        proxy: EventLoopProxy<UserEvent>,
    ) -> Result<(), String> {
        self.clear_cache_for(&page.url);
        let _ = proxy.send_event(UserEvent::SiteFilesCleared(page, Ok(())));
        Ok(())
    }

    fn reload_fresh(
        &self,
        page: recovery::Page,
        _: EventLoopProxy<UserEvent>,
    ) -> Result<(), String> {
        self.clear_cache_for(&page.url);
        self.webview.reload();
        Ok(())
    }
}

struct TabDelegate {
    id: usize,
    generation: u64,
    token: String,
    shared: Rc<Shared>,
    startup: Rc<Startup>,
}

impl TabDelegate {
    fn send(&self, event: UserEvent) {
        let _ = self.shared.proxy.send_event(event);
    }
}

impl WebViewDelegate for TabDelegate {
    fn notify_new_frame_ready(&self, webview: WebView) {
        if self.shared.tabs.is_active(&webview) {
            self.shared.tabs.dirty.set(true);
        }
    }

    fn notify_page_title_changed(&self, _: WebView, title: Option<String>) {
        if !self.startup.ready.get() {
            return;
        }
        self.send(UserEvent::Title(
            self.id,
            self.generation,
            title.unwrap_or_default(),
        ));
    }

    fn notify_url_changed(&self, webview: WebView, url: url::Url) {
        if !self.startup.ready.get() {
            return;
        }
        // A URL change after the load completed is a same-document navigation.
        self.send(if webview.load_status() == LoadStatus::Complete {
            UserEvent::Source(self.id, self.generation, url.into())
        } else {
            UserEvent::PageLoad(self.id, self.generation, true, url.into())
        });
    }

    fn notify_load_status_changed(&self, webview: WebView, status: LoadStatus) {
        if status != LoadStatus::Complete {
            return;
        }
        if !self.startup.ready.replace(true) {
            if let Some(url) = self.startup.pending.take() {
                webview.load(url);
            }
            return;
        }
        if !self.startup.history_cleared.replace(true) {
            webview.clear_session_history();
        }
        {
            let url = webview.url().map(String::from).unwrap_or_default();
            self.send(UserEvent::PageLoad(self.id, self.generation, false, url));
        }
    }

    fn notify_history_changed(&self, _: WebView, entries: Vec<url::Url>, current: usize) {
        self.send(UserEvent::History(
            self.id,
            self.generation,
            current > 0,
            current + 1 < entries.len(),
        ));
    }

    fn notify_crashed(&self, _: WebView, _: String, _: Option<String>) {
        self.send(UserEvent::Crashed(self.id, self.generation));
    }

    fn notify_cursor_changed(&self, webview: WebView, cursor: Cursor) {
        set_cursor(&self.shared.tabs, &webview, cursor);
    }

    /// Only page-initiated navigations reach here; Felaris's own loads go straight to the
    /// constellation. Internal pages are `data:` documents trusted with local IPC, so a page
    /// may not navigate its tab to one.
    fn request_navigation(&self, _: WebView, request: NavigationRequest) {
        if request.url.scheme() != "data"
            && allow_navigation(request.url.as_str(), &self.shared.proxy)
        {
            request.allow();
        } else {
            request.deny();
        }
    }

    fn request_create_new(&self, webview: WebView, request: servo::CreateNewWebViewRequest) {
        create_auxiliary(&self.shared, request, true, webview.id());
    }

    fn show_embedder_control(&self, webview: WebView, control: EmbedderControl) {
        self.shared.controls.borrow_mut().push((webview, control));
    }

    fn hide_embedder_control(&self, _: WebView, id: EmbedderControlId) {
        hide_control(&self.shared, id);
    }

    fn request_authentication(&self, webview: WebView, request: AuthenticationRequest) {
        self.shared.logins.borrow_mut().push((webview, request));
    }

    fn request_permission(&self, webview: WebView, request: PermissionRequest) {
        let kind = match request.feature() {
            servo::PermissionFeature::Notifications => "notifications",
            servo::PermissionFeature::Geolocation => "location",
            servo::PermissionFeature::Camera => "camera",
            servo::PermissionFeature::Microphone => "microphone",
            _ => return request.deny(),
        };
        if webview.load_status() != LoadStatus::Complete {
            return request.deny();
        }
        let Some(origin) = request.requesting_origin().map(str::to_owned) else {
            return request.deny();
        };
        if webview.url().is_none_or(|url| {
            !matches!(url.scheme(), "http" | "https")
                || url.origin().ascii_serialization() != origin
        }) {
            return request.deny();
        }
        crate::permission::servo_request(
            request,
            self.id,
            self.generation,
            origin,
            kind,
            self.shared.proxy.clone(),
        );
    }

    /// Saves to the Downloads folder without asking, as WebView2 tabs do.
    fn request_download(&self, webview: WebView, request: DownloadRequest) {
        let Some((path, file)) = downloads::folder()
            .and_then(|folder| downloads::reserve(&folder, request.suggested_filename()))
        else {
            self.send(UserEvent::Status(
                "Felaris could not create a file in your Downloads folder.".into(),
            ));
            return;
        };
        let handle = match request.save_to_file(path.clone(), file) {
            Ok(handle) => handle,
            Err(error) => {
                self.send(UserEvent::Status(error));
                return;
            }
        };
        self.shared
            .downloads
            .borrow_mut()
            .push((self.id, path, handle));
        let url = webview.url();
        let blank = url.as_ref().is_none_or(|url| url.as_str() == "about:blank");
        let document = url
            .filter(|url| matches!(url.scheme(), "http" | "https"))
            .map(String::from);
        self.send(UserEvent::DownloadStarted(
            self.id,
            self.generation,
            blank,
            document,
        ));
    }

    fn notify_download_finished(&self, _: WebView, path: PathBuf, result: Result<(), String>) {
        self.shared
            .downloads
            .borrow_mut()
            .retain(|(_, running, _)| *running != path);
        if result.is_ok() {
            *self.shared.last_download.borrow_mut() = Some(path);
        }
        self.send(UserEvent::DownloadFinished(
            self.id,
            self.generation,
            result.is_ok(),
        ));
    }

    fn show_console_message(&self, webview: WebView, _: ConsoleLogLevel, message: String) {
        let Some(body) = message.strip_prefix(IPC_PREFIX) else {
            return;
        };
        let local = webview
            .url()
            .is_none_or(|url| url.scheme() == "data" || url.as_str() == "about:blank");
        if let Some(event) =
            app::content_message(self.id, self.generation, &self.token, body, local)
        {
            self.send(event);
        }
    }
}

fn set_cursor(surface: &Surface, webview: &WebView, cursor: Cursor) {
    if !surface.is_active(webview) {
        return;
    }
    surface.window.set_cursor_icon(match cursor {
        Cursor::Pointer => CursorIcon::Hand,
        Cursor::Text | Cursor::VerticalText => CursorIcon::Text,
        Cursor::Wait => CursorIcon::Wait,
        Cursor::Progress => CursorIcon::Progress,
        Cursor::Move => CursorIcon::Move,
        Cursor::Grab => CursorIcon::Grab,
        Cursor::Grabbing => CursorIcon::Grabbing,
        Cursor::EwResize | Cursor::EResize | Cursor::WResize => CursorIcon::EwResize,
        Cursor::NsResize | Cursor::NResize | Cursor::SResize => CursorIcon::NsResize,
        _ => CursorIcon::Default,
    });
}

fn hide_control(shared: &Shared, id: EmbedderControlId) {
    shared
        .controls
        .borrow_mut()
        .retain(|(_, control)| control.id() != id);
}

/// Creates an auxiliary webview on the popup rendering context. Servo fixes that context
/// before reporting the first URL, so eligible pages stay here to preserve their opener.
fn create_auxiliary(
    shared: &Rc<Shared>,
    request: servo::CreateNewWebViewRequest,
    popup: bool,
    opener: servo::WebViewId,
) {
    let webview = request
        .builder(shared.popup.rendering.clone())
        .hidpi_scale_factor(euclid::Scale::new(shared.popup.window.scale_factor() as f32))
        .delegate(Rc::new(AuxiliaryDelegate {
            shared: Rc::downgrade(shared),
            may_popup: popup,
            state: Cell::new(Auxiliary::Pending),
        }))
        .build();
    shared.openers.borrow_mut().push((opener, webview));
}

#[derive(Clone, Copy, PartialEq)]
enum Auxiliary {
    /// Still on its initial `about:blank`.
    Pending,
    /// Showing in the sign-in window.
    Popup,
    /// Reopened as a tab; this webview is closing.
    Moved,
}

/// Pages opened with `window.open` preserve their opener in the floating popup window.
/// A nested popup currently reopens as a normal tab because the embedder has one popup surface.
struct AuxiliaryDelegate {
    shared: Weak<Shared>,
    /// False for pages opened from the sign-in window, which always become tabs.
    may_popup: bool,
    state: Cell<Auxiliary>,
}

impl AuxiliaryDelegate {
    /// Decides where the page goes once its first real URL is known.
    fn settle(&self, webview: &WebView, url: &url::Url) {
        let Some(shared) = self.shared.upgrade() else {
            return;
        };
        if url.as_str() == "about:blank" || self.state.get() != Auxiliary::Pending {
            return;
        }
        shared
            .openers
            .borrow_mut()
            .retain(|(_, opener)| opener.id() != webview.id());
        if self.may_popup && crate::profile::allowed_url(url.as_str()) {
            self.state.set(Auxiliary::Popup);
            shared
                .popups
                .borrow_mut()
                .push((webview.clone(), url.clone()));
            return;
        }
        self.state.set(Auxiliary::Moved);
        if crate::profile::allowed_url(url.as_str()) {
            self.send(UserEvent::Command(Command::NewTab {
                url: Some(url.to_string()),
            }));
        }
    }

    fn send(&self, event: UserEvent) {
        if let Some(shared) = self.shared.upgrade() {
            let _ = shared.proxy.send_event(event);
        }
    }

    fn is_popup(&self) -> bool {
        self.state.get() == Auxiliary::Popup
    }
}

impl WebViewDelegate for AuxiliaryDelegate {
    fn request_navigation(&self, webview: WebView, request: NavigationRequest) {
        let Some(shared) = self.shared.upgrade() else {
            return request.deny();
        };
        self.settle(&webview, &request.url);
        if self.state.get() != Auxiliary::Moved
            && allow_navigation(request.url.as_str(), &shared.proxy)
        {
            request.allow();
        } else {
            request.deny();
        }
    }

    fn notify_url_changed(&self, webview: WebView, url: url::Url) {
        self.settle(&webview, &url);
        if self.is_popup() {
            let loading = webview.load_status() != LoadStatus::Complete;
            self.send(UserEvent::PopupState(url.into(), loading));
        }
    }

    fn notify_load_status_changed(&self, webview: WebView, status: LoadStatus) {
        if !self.is_popup() || status == LoadStatus::HeadParsed {
            return;
        }
        let url = webview.url().map(String::from).unwrap_or_default();
        self.send(UserEvent::PopupState(url, status != LoadStatus::Complete));
    }

    fn notify_page_title_changed(&self, _: WebView, title: Option<String>) {
        if self.is_popup() {
            self.send(UserEvent::PopupTitle(title.unwrap_or_default()));
        }
    }

    fn notify_new_frame_ready(&self, webview: WebView) {
        let Some(shared) = self.shared.upgrade() else {
            return;
        };
        if shared.popup.is_active(&webview) {
            shared.popup.dirty.set(true);
        }
    }

    fn notify_cursor_changed(&self, webview: WebView, cursor: Cursor) {
        let Some(shared) = self.shared.upgrade() else {
            return;
        };
        set_cursor(&shared.popup, &webview, cursor);
    }

    /// `window.close()`, typically when sign-in finishes.
    fn notify_closed(&self, webview: WebView) {
        let Some(shared) = self.shared.upgrade() else {
            return;
        };
        if self.is_popup() && shared.popup.is_active(&webview) {
            self.send(UserEvent::PopupClosed);
        }
        shared.release_auxiliaries(webview.id());
    }

    fn request_create_new(&self, webview: WebView, request: servo::CreateNewWebViewRequest) {
        let Some(shared) = self.shared.upgrade() else {
            return;
        };
        create_auxiliary(&shared, request, false, webview.id());
    }

    fn show_embedder_control(&self, webview: WebView, control: EmbedderControl) {
        let Some(shared) = self.shared.upgrade() else {
            return;
        };
        shared.controls.borrow_mut().push((webview, control));
    }

    fn hide_embedder_control(&self, _: WebView, id: EmbedderControlId) {
        let Some(shared) = self.shared.upgrade() else {
            return;
        };
        hide_control(&shared, id);
    }

    fn request_authentication(&self, webview: WebView, request: AuthenticationRequest) {
        let Some(shared) = self.shared.upgrade() else {
            return;
        };
        shared.logins.borrow_mut().push((webview, request));
    }

    fn request_permission(&self, _: WebView, request: PermissionRequest) {
        request.deny();
    }
}

/// Shows a `window.open` sign-in page in the floating sign-in window. Replacing the window's
/// previous view closes that page.
fn open_popup(shared: &Rc<Shared>, webview: WebView, url: &url::Url) {
    let host = &shared.popup_host;
    host.present(url.as_str(), None, None);
    *host.view.borrow_mut() = Some(Box::new(ServoPopup {
        webview: webview.clone(),
        shared: Rc::downgrade(shared),
    }));
    let surface = &shared.popup;
    *surface.active.borrow_mut() = Some(webview.clone());
    let (_, content) = app::popup_bounds(&host.window);
    let _ = surface.place(content, &webview);
    webview.show();
    surface.window.set_visible(true);
    if !host.headless {
        focus_surface(&surface.window);
        webview.set_focused(true);
    }
    let _ = shared
        .proxy
        .send_event(UserEvent::PopupState(url.to_string(), true));
}

/// The sign-in window's Servo page. Dropping it closes the page.
struct ServoPopup {
    webview: WebView,
    shared: Weak<Shared>,
}

impl PopupView for ServoPopup {
    fn set_bounds(&self, bounds: Rect) -> Result<(), String> {
        self.shared
            .upgrade()
            .ok_or("Servo is shutting down.")?
            .popup
            .place(bounds, &self.webview)
    }
}

impl Drop for ServoPopup {
    fn drop(&mut self) {
        if let Some(shared) = self.shared.upgrade() {
            shared.popup.release(&self.webview);
            shared.release_auxiliaries(self.webview.id());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tao::keyboard::Key as Tao;

    #[test]
    fn keys_map_to_w3c_names() {
        assert_eq!(key(&Tao::Character("a")), Key::Character("a".into()));
        assert_eq!(key(&Tao::Space), Key::Character(" ".into()));
        assert_eq!(key(&Tao::F2), Key::Named(NamedKey::F2));
        assert_eq!(key(&Tao::ArrowUp), Key::Named(NamedKey::ArrowUp));
        assert_eq!(key(&Tao::Super), Key::Named(NamedKey::Meta));
        assert_eq!(code(KeyCode::KeyO), servo::Code::KeyO);
        assert_eq!(code(KeyCode::F2), servo::Code::F2);
        assert_eq!(code(KeyCode::SuperLeft), servo::Code::MetaLeft);
        assert_eq!(virtual_key(KeyCode::KeyT), Some(u32::from(b'T')));
        assert_eq!(virtual_key(KeyCode::Tab), Some(0x09));
    }
}
