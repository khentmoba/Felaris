use crate::{
    desktop, extensions, history, instance::Instance, profile, settings, start_page,
    state::BrowserState, ui, updater,
};
use serde::Deserialize;
use std::{collections::HashMap, hash::BuildHasher};
use tao::platform::windows::WindowBuilderExtWindows;
use tao::{
    dpi::{LogicalPosition, LogicalSize, Position, Size},
    event::{Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy},
    window::{Icon, Window, WindowBuilder},
};
use wry::{NewWindowResponse, PageLoadEvent, Rect, WebContext, WebView, WebViewBuilder};

#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Command {
    Navigate {
        url: String,
    },
    Back,
    Forward,
    Reload,
    Stop,
    NewTab {
        url: Option<String>,
    },
    SwitchTab {
        id: usize,
    },
    CloseTab {
        id: usize,
    },
    ToggleSleep {
        id: usize,
    },
    SleepInactiveTabs,
    ClearUnpinnedTabs,
    ToggleSidebar,
    Peek {
        open: bool,
    },
    TopbarPeek {
        open: bool,
    },
    WindowMinimize,
    WindowMaximize,
    WindowClose,
    DragWindow,
    SwitchWorkspace {
        id: usize,
    },
    RestoreClosedTab,
    TogglePin {
        id: usize,
    },
    FocusAddress,
    CloseActiveTab,
    CycleTab {
        backwards: bool,
    },
    CheckUpdates,
    ApplyUpdate,
    OpenDownloads,
    DefaultBrowser,
    OpenSettings,
    OpenHistory,
    DeleteHistory {
        url: String,
    },
    ClearHistory,
    SetSetting {
        key: String,
        value: serde_json::Value,
    },
    InstallAdBlocker,
    OpenExtensionsFolder,
    Zoom {
        delta: i32,
    },
    Print,
    Exit,
    DismissNewTab,
    SubmitNewTab {
        url: String,
    },
    Retry,
}

#[derive(Deserialize)]
struct ContentCommand {
    #[serde(flatten)]
    command: Command,
    shortcut_token: Option<String>,
}

#[derive(Debug)]
pub enum UserEvent {
    SmokeTick,
    SmokeResult(usize, bool),
    Command(Command),
    Ready,
    PaletteReady,
    TopbarReady,
    External(Vec<String>),
    Title(usize, u64, String),
    PageLoad(usize, u64, bool, String),
    History(usize, u64, bool, bool),
    Source(usize, u64, String),
    Failed(usize, u64, String, String),
    Crashed(usize, u64),
    Favicon(usize, u64, String),
    DownloadStarted(usize),
    DownloadFinished(usize, bool),
    UpdateChecked(Result<Option<updater::UpdateManifest>, String>),
    Status(String),
    UpdateFailed(String),
    AdBlockerInstalled(Result<(), String>),
}

fn bounds(window: &Window, state: &BrowserState, side: f64, top: f64) -> (Rect, Rect, Rect, Rect) {
    let size = window.inner_size().to_logical::<f64>(window.scale_factor());
    let full = ui::SIDEBAR_WIDTH_EXPANDED;
    let hot = ui::SIDEBAR_HOT_ZONE;
    let (side_x, side_width, top_left) = if !state.sidebar_collapsed {
        (0., full, full)
    } else if side <= 0. {
        (0., hot, 0.)
    } else {
        let right = (full * side).max(hot);
        (right - full, full, right)
    };
    let margin = if window.is_maximized() || window.fullscreen().is_some() {
        0.
    } else {
        ui::WINDOW_MARGIN
    };
    let content_left = if state.sidebar_collapsed {
        margin
    } else {
        full + margin
    };
    let top_height = ui::TOPBAR_HOT_ZONE + (ui::TOPBAR_EXPANDED - ui::TOPBAR_HOT_ZONE) * top;
    (
        Rect {
            position: Position::Logical(LogicalPosition::new(side_x, 0.)),
            size: Size::Logical(LogicalSize::new(side_width, size.height)),
        },
        Rect {
            position: Position::Logical(LogicalPosition::new(content_left, margin)),
            size: Size::Logical(LogicalSize::new(
                (size.width - content_left - margin).max(10.),
                (size.height - margin * 2.).max(10.),
            )),
        },
        Rect {
            position: Position::Logical(LogicalPosition::new(0., 0.)),
            size: Size::Logical(size),
        },
        Rect {
            position: Position::Logical(LogicalPosition::new(top_left, 0.)),
            size: Size::Logical(LogicalSize::new(
                (size.width - top_left).max(10.),
                top_height,
            )),
        },
    )
}

fn step(progress: f64, target: f64, dt: f64, snap: bool) -> f64 {
    if snap {
        return target;
    }
    let amount = dt / if target > progress { 0.30 } else { 0.22 };
    if target > progress {
        (progress + amount).min(target)
    } else {
        (progress - amount).max(target)
    }
}

fn ease(progress: f64) -> f64 {
    1. - (1. - progress).powi(3)
}

fn load(view: &WebView, state: &BrowserState, url: &str) -> wry::Result<()> {
    if url == "felaris://newtab" {
        view.load_html(&start_page::get_start_page_html(&state.top_shortcuts()))
    } else if url == "felaris://settings" {
        view.load_html(&ui::get_settings_html())
    } else if url == "felaris://history" {
        view.load_html(&ui::get_history_html())
    } else {
        view.load_url(url)
    }
}

fn content(
    window: &Window,
    context: &mut WebContext,
    state: &mut BrowserState,
    id: usize,
    proxy: &EventLoopProxy<UserEvent>,
) -> wry::Result<WebView> {
    let generation = std::collections::hash_map::RandomState::new().hash_one(id);
    state
        .tabs
        .iter_mut()
        .find(|tab| tab.id == id)
        .unwrap()
        .generation = generation;
    let token = generation.to_string();
    let script = ui::script().replace("__SHORTCUT_TOKEN__", &token);
    let ipc = proxy.clone();
    let title = proxy.clone();
    let popup = proxy.clone();
    let navigation = proxy.clone();
    let mut builder = WebViewBuilder::new_with_web_context(context)
        .with_bounds(bounds(window, state, 0., 0.).1)
        .with_visible(false)
        .with_focused(false)
        .with_initialization_script(&script)
        .with_document_title_changed_handler(move |text| {
            let _ = title.send_event(UserEvent::Title(id, generation, text));
        })
        .with_navigation_handler(move |url| {
            let allowed = profile::allowed_url(&url)
                || url == "about:blank"
                || url.starts_with("data:text/html;");
            if !allowed {
                let _ = navigation.send_event(UserEvent::Status(
                    "This link uses an unsupported application protocol.".into(),
                ));
            }
            allowed
        })
        .with_new_window_req_handler(move |url, _| {
            if profile::allowed_url(&url) {
                let _ = popup.send_event(UserEvent::Command(Command::NewTab { url: Some(url) }));
            }
            NewWindowResponse::Deny
        })
        .with_ipc_handler(move |request| {
            if let Ok(message) = serde_json::from_str::<ContentCommand>(request.body()) {
                let local = request
                    .uri()
                    .scheme_str()
                    .is_none_or(|scheme| scheme == "data")
                    || request.uri() == "about:blank";
                let trusted = message.shortcut_token.as_deref() == Some(&token);
                if (local
                    && matches!(
                        message.command,
                        Command::Navigate { .. }
                            | Command::Retry
                            | Command::SetSetting { .. }
                            | Command::DeleteHistory { .. }
                            | Command::ClearHistory
                            | Command::InstallAdBlocker
                            | Command::OpenExtensionsFolder
                            | Command::DefaultBrowser
                            | Command::CheckUpdates
                    ))
                    || (trusted
                        && matches!(
                            message.command,
                            Command::Back
                                | Command::Forward
                                | Command::Reload
                                | Command::Stop
                                | Command::NewTab { .. }
                                | Command::CloseActiveTab
                                | Command::FocusAddress
                                | Command::ToggleSidebar
                                | Command::RestoreClosedTab
                                | Command::SleepInactiveTabs
                                | Command::CycleTab { .. }
                                | Command::SwitchWorkspace { .. }
                                | Command::OpenDownloads
                                | Command::OpenHistory
                        ))
                {
                    let _ = ipc.send_event(UserEvent::Command(message.command));
                }
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
    let view = builder.build(window)?;
    desktop::attach(&view, id, generation, proxy.clone())?;
    let url = state
        .tabs
        .iter()
        .find(|tab| tab.id == id)
        .unwrap()
        .url
        .clone();
    if state
        .tabs
        .iter()
        .find(|tab| tab.id == id)
        .is_some_and(|tab| tab.error.is_some())
    {
        view.load_html(include_str!("error.html"))?;
    } else {
        load(&view, state, &url)?;
    }
    Ok(view)
}

fn may_exit(state: &BrowserState) -> bool {
    if state.active_downloads > 0
        && !desktop::message(
            "Downloads in progress",
            "Closing Felaris will cancel active downloads. Close anyway?",
            true,
        )
    {
        return false;
    }
    if let Err(error) = state.save_session()
        && !desktop::message(
            "Session could not be saved",
            &format!("{error}\n\nClose without saving this session?"),
            true,
        )
    {
        return false;
    }
    true
}

fn start_ad_blocker_install(proxy: &EventLoopProxy<UserEvent>) {
    let proxy = proxy.clone();
    extensions::install_in_background(move |result| {
        let _ = proxy.send_event(UserEvent::AdBlockerInstalled(result));
    });
}

pub fn run(args: &[String]) -> wry::Result<()> {
    let headless = std::env::var_os("FELARIS_SMOKE_TEST").is_some();
    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();
    let urls: Vec<_> = args
        .iter()
        .skip(1)
        .filter_map(|arg| profile::startup_url(arg))
        .collect();
    let directory = profile::directory();
    let instance =
        match Instance::acquire(&directory, &urls, args.iter().any(|arg| arg == "--updated")) {
            Ok(Some(instance)) => instance,
            Ok(None) => return Ok(()),
            Err(error) => {
                desktop::message("Felaris", &error, false);
                return Ok(());
            }
        };
    let external = proxy.clone();
    if let Err(error) = instance.listen(move |urls| {
        let _ = external.send_event(UserEvent::External(urls));
    }) {
        desktop::message("Felaris", &error, false);
        return Ok(());
    }
    updater::cleanup_old_version();
    let icon = Icon::from_rgba(include_bytes!("../assets/icon_64.rgba").to_vec(), 64, 64).ok();
    let window = WindowBuilder::new()
        .with_title("Felaris")
        .with_visible(!headless)
        .with_focused(!headless)
        .with_theme(Some(tao::window::Theme::Dark))
        .with_decorations(false)
        .with_undecorated_shadow(true)
        .with_background_color((16, 16, 17, 255))
        .with_window_icon(icon)
        .with_inner_size(if std::env::var_os("FELARIS_SMOKE_TEST").is_some() {
            LogicalSize::new(640., 480.)
        } else {
            LogicalSize::new(1380., 860.)
        })
        .with_min_inner_size(LogicalSize::new(640., 480.))
        .build(&event_loop)
        .expect("Cannot create window");
    let mut state = BrowserState::restore();
    state.sidebar_collapsed = !headless;
    state.sidebar_peek = false;
    for url in urls {
        state.add_tab(Some(url), false);
    }
    let mut context = WebContext::new(Some(directory.join("WebView2")));
    let mut chrome_context = WebContext::new(Some(directory.join("Chrome")));
    let ui_proxy = proxy.clone();
    let ready = proxy.clone();
    let mut sidebar_builder = WebViewBuilder::new_with_web_context(&mut chrome_context)
        .with_bounds(bounds(&window, &state, 0., 0.).0)
        .with_html(ui::get_ui_html())
        .with_focused(false)
        .with_clipboard(true)
        .with_navigation_handler(|url| url == "about:blank" || url.starts_with("data:text/html;"))
        .with_on_page_load_handler(move |event, _| {
            if matches!(event, PageLoadEvent::Finished) {
                let _ = ready.send_event(UserEvent::Ready);
            }
        })
        .with_ipc_handler(move |request| {
            if let Ok(command) = serde_json::from_str::<Command>(request.body()) {
                let _ = ui_proxy.send_event(UserEvent::Command(command));
            }
        });
    #[cfg(windows)]
    if headless && let Ok(arguments) = std::env::var("FELARIS_SMOKE_BROWSER_ARGUMENTS") {
        use wry::WebViewBuilderExtWindows;
        sidebar_builder = sidebar_builder.with_additional_browser_args(arguments);
    }
    let sidebar = sidebar_builder.build(&window)?;
    let palette_proxy = proxy.clone();
    let palette_ready = proxy.clone();
    let mut palette_builder = WebViewBuilder::new_with_web_context(&mut chrome_context)
        .with_bounds(bounds(&window, &state, 0., 0.).2)
        .with_visible(false)
        .with_focused(false)
        .with_transparent(true)
        .with_background_color((0, 0, 0, 0))
        .with_html(ui::get_new_tab_html())
        .with_navigation_handler(|url| url == "about:blank" || url.starts_with("data:text/html;"))
        .with_on_page_load_handler(move |event, _| {
            if matches!(event, PageLoadEvent::Finished) {
                let _ = palette_ready.send_event(UserEvent::PaletteReady);
            }
        })
        .with_ipc_handler(move |request| {
            if let Ok(
                command @ (Command::SubmitNewTab { .. }
                | Command::DismissNewTab
                | Command::SwitchTab { .. }),
            ) = serde_json::from_str::<Command>(request.body())
            {
                let _ = palette_proxy.send_event(UserEvent::Command(command));
            }
        });
    #[cfg(windows)]
    if headless && let Ok(arguments) = std::env::var("FELARIS_SMOKE_BROWSER_ARGUMENTS") {
        use wry::WebViewBuilderExtWindows;
        palette_builder = palette_builder.with_additional_browser_args(arguments);
    }
    let palette = palette_builder.build(&window)?;
    let topbar_ready = proxy.clone();
    let topbar_proxy = proxy.clone();
    let mut topbar_builder = WebViewBuilder::new_with_web_context(&mut chrome_context)
        .with_bounds(bounds(&window, &state, 0., 0.).3)
        .with_html(ui::get_topbar_html())
        .with_focused(false)
        .with_navigation_handler(|url| url == "about:blank" || url.starts_with("data:text/html;"))
        .with_on_page_load_handler(move |event, _| {
            if matches!(event, PageLoadEvent::Finished) {
                let _ = topbar_ready.send_event(UserEvent::TopbarReady);
            }
        })
        .with_ipc_handler(move |request| {
            if let Ok(
                command @ (Command::TopbarPeek { .. }
                | Command::WindowMinimize
                | Command::WindowMaximize
                | Command::WindowClose
                | Command::DragWindow),
            ) = serde_json::from_str::<Command>(request.body())
            {
                let _ = topbar_proxy.send_event(UserEvent::Command(command));
            }
        });
    #[cfg(windows)]
    if headless && let Ok(arguments) = std::env::var("FELARIS_SMOKE_BROWSER_ARGUMENTS") {
        use wry::WebViewBuilderExtWindows;
        topbar_builder = topbar_builder.with_additional_browser_args(arguments);
    }
    let topbar = topbar_builder.build(&window)?;
    let mut last_loading = false;
    let mut side_progress = if state.sidebar_collapsed { 0. } else { 1. };
    let mut top_progress = 0.;
    let mut animating = false;
    let mut last_frame = std::time::Instant::now();
    let mut rounded: HashMap<usize, (u32, u32, i32)> = HashMap::new();
    let mut views = HashMap::new();
    let initial_id = state.active_tab_id;
    views.insert(
        initial_id,
        content(&window, &mut context, &mut state, initial_id, &proxy)?,
    );
    let mut palette_open = false;
    let mut palette_needs_open = false;
    let mut visible_id = None;
    let mut focus_address = false;
    let mut downloads: HashMap<usize, usize> = HashMap::new();
    let mut check_busy = false;
    let mut zooms: HashMap<usize, u32> = HashMap::new();
    let mut smoke = crate::smoke::Smoke::new();
    if smoke.is_some() {
        let proxy = proxy.clone();
        std::thread::spawn(move || {
            for _ in 0..300 {
                std::thread::sleep(std::time::Duration::from_millis(100));
                if proxy.send_event(UserEvent::SmokeTick).is_err() {
                    break;
                }
            }
        });
    }
    let background = proxy.clone();
    if !headless {
        if extensions::enabled_at_startup() {
            start_ad_blocker_install(&proxy);
        }
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(3));
            if settings::get().auto_update {
                let _ =
                    background.send_event(UserEvent::UpdateChecked(updater::check_for_updates()));
            }
            loop {
                std::thread::sleep(std::time::Duration::from_secs(6 * 60 * 60));
                if settings::get().auto_update
                    && let Ok(Some(update)) = updater::check_for_updates()
                    && background
                        .send_event(UserEvent::UpdateChecked(Ok(Some(update))))
                        .is_err()
                {
                    break;
                }
            }
        });
    }

    event_loop.run(move |event, _, flow| {
        let _keep_instance = &instance;
        let _keep_chrome_context = &chrome_context;
        *flow = if animating { ControlFlow::WaitUntil(last_frame + std::time::Duration::from_millis(16)) } else { ControlFlow::Wait };
        let tick = matches!(event, Event::NewEvents(StartCause::ResumeTimeReached { .. }));
        match event {
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } | Event::UserEvent(UserEvent::Command(Command::WindowClose)) => {
                if may_exit(&state) { *flow = ControlFlow::Exit; }
            }
            Event::NewEvents(StartCause::Init | StartCause::ResumeTimeReached { .. }) => {},
            Event::WindowEvent { event: WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. }, .. } => {},
            Event::UserEvent(event) => {
                let transient = matches!(&event, UserEvent::TopbarReady | UserEvent::Command(Command::Peek { .. } | Command::TopbarPeek { .. } | Command::DragWindow));
                match event {
                    UserEvent::SmokeTick => {
                        if let Some(smoke) = &mut smoke && smoke.tick(&state, &views, &sidebar, &palette, &proxy) { *flow = ControlFlow::Exit; }
                    }
                    UserEvent::SmokeResult(phase, success) => {
                        if let Some(smoke) = &mut smoke && smoke.result(phase, success, &proxy) { *flow = ControlFlow::Exit; }
                    }
                    UserEvent::Ready => {},
                    UserEvent::TopbarReady => { last_loading = false; },
                    UserEvent::PaletteReady => { palette_needs_open = palette_open; },
                    UserEvent::External(urls) => {
                        if !headless { window.set_minimized(false); window.set_focus(); }
                        for url in urls { if profile::allowed_url(&url) { state.add_tab(Some(url), false); } }
                        palette_open = false;
                    }
                    UserEvent::Command(command) => {
                        match command {
                            Command::Navigate { url } => if let Some(url) = profile::normalize_url(&url) {
                                state.update_active_tab(url.clone(), None);
                                if let Some(tab) = state.tabs.iter_mut().find(|tab| tab.id == state.active_tab_id) { tab.error = None; tab.pending_url = Some(url.clone()); tab.is_loading = true; }
                                if let Some(view) = views.get(&state.active_tab_id) { let _ = load(view, &state, &url); if !headless { let _ = view.focus(); } }
                            },
                            Command::Back | Command::Forward => if let Some(view) = views.get(&state.active_tab_id) { desktop::navigate(view, matches!(command, Command::Back)); },
                            Command::Reload | Command::Retry => if let Some(tab) = state.active_tab() {
                                let url = tab.url.clone(); let id = tab.id;
                                let crashed = tab.error.is_some() && views.get(&id).and_then(|view| view.url().ok()).is_some_and(|url| url == "about:blank" || url.starts_with("data:"));
                                if crashed { views.remove(&id); }
                                if let Some(tab) = state.tabs.iter_mut().find(|tab| tab.id == id) { tab.error = None; tab.pending_url = Some(url.clone()); tab.is_loading = true; }
                                if let Some(view) = views.get(&id) { let _ = if profile::is_internal(&url) { load(view, &state, &url) } else { view.reload() }; }
                            },
                            Command::Stop => { if let Some(view) = views.get(&state.active_tab_id) { let _ = view.evaluate_script("window.stop()"); }
                                if let Some(tab) = state.tabs.iter_mut().find(|tab| tab.id == state.active_tab_id) { tab.is_loading = false; } },
                            Command::NewTab { url: None } => { palette_open = true; palette_needs_open = true; },
                            Command::NewTab { url: Some(url) } | Command::SubmitNewTab { url } => if profile::allowed_url(&url) { state.add_tab(Some(url), false); palette_open = false; },
                            Command::DismissNewTab => { palette_open = false; if !headless && let Some(view) = views.get(&state.active_tab_id) { let _ = view.focus(); } },
                            Command::SwitchTab { id } => { state.switch_tab(id); palette_open = false; },
                            Command::CloseTab { id } => { if downloads.get(&id).copied().unwrap_or(0) == 0 { state.close_tab(id); } else { state.update_status = Some("This tab has an active download. Finish or cancel it in Downloads before closing.".into()); } },
                            Command::CloseActiveTab => { let _ = proxy.send_event(UserEvent::Command(Command::CloseTab { id: state.active_tab_id })); },
                            Command::ToggleSleep { id } => { if downloads.get(&id).copied().unwrap_or(0) == 0 { state.toggle_sleep(id); } },
                            Command::SleepInactiveTabs => { state.sleep_inactive_tabs(); for tab in &mut state.tabs { if downloads.get(&tab.id).copied().unwrap_or(0) > 0 { tab.is_sleeping = false; } } },
                            Command::ClearUnpinnedTabs => {
                                let active = state.active_tab_id;
                                let ids: Vec<_> = state.tabs.iter().filter(|tab| tab.id != active && tab.workspace_id == state.active_workspace_id && !tab.is_pinned && downloads.get(&tab.id).copied().unwrap_or(0) == 0).map(|tab| tab.id).collect();
                                for id in ids { state.close_tab(id); }
                            },
                            Command::ToggleSidebar => { state.toggle_sidebar(); },
                            Command::Peek { open } => { state.sidebar_peek = open && state.sidebar_collapsed; },
                            Command::TopbarPeek { open } => { state.topbar_peek = open; },
                            Command::WindowMinimize => { window.set_minimized(true); },
                            Command::WindowMaximize => { window.set_maximized(!window.is_maximized()); },
                            Command::WindowClose => {},
                            Command::DragWindow => { let _ = window.drag_window(); },
                            Command::SwitchWorkspace { id } => { state.switch_workspace(id); },
                            Command::RestoreClosedTab => { state.restore_closed_tab(); },
                            Command::TogglePin { id } => state.toggle_pin(id),
                            Command::CycleTab { backwards } => { state.cycle_tab(backwards); },
                            Command::FocusAddress => { state.sidebar_collapsed = false; focus_address = true; },
                            Command::OpenDownloads => {
                                let id = downloads.iter().find(|(_, count)| **count > 0).map(|(id,_)| *id).unwrap_or(state.active_tab_id);
                                if let Some(view) = views.get(&id) && let Err(error) = desktop::downloads(view) { state.update_status = Some(error); }
                            },
                            Command::OpenSettings => { state.open_internal("felaris://settings"); palette_open = false; },
                            Command::OpenHistory => { state.open_internal("felaris://history"); palette_open = false; },
                            Command::DeleteHistory { url } => history::remove(&url),
                            Command::ClearHistory => history::clear(),
                            Command::SetSetting { key, value } => match settings::set(&key, &value) {
                                Ok(current) => {
                                    if key == "search_engine" {
                                        let script = ui::search_script();
                                        let _ = sidebar.evaluate_script(&script);
                                        let _ = palette.evaluate_script(&script);
                                        for view in views.values() { let _ = view.evaluate_script(&script); }
                                    } else if key == "ad_blocker" {
                                        if current.ad_blocker { start_ad_blocker_install(&proxy); }
                                        state.update_status = Some("Ad blocker changes apply after you restart Felaris.".into());
                                    }
                                },
                                Err(error) => { state.update_status = Some(error); },
                            },
                            Command::InstallAdBlocker => { start_ad_blocker_install(&proxy); },
                            Command::OpenExtensionsFolder => {
                                let _ = std::fs::create_dir_all(extensions::directory());
                                if let Err(error) = std::process::Command::new("explorer.exe").arg(extensions::directory()).spawn() { state.update_status = Some(format!("Could not open the extensions folder: {error}")); }
                            },
                            Command::Zoom { delta } => if let Some(view) = views.get(&state.active_tab_id) {
                                let current = zooms.get(&state.active_tab_id).copied().unwrap_or(100) as i32;
                                let next = if delta == 0 { 100 } else { (current + delta).clamp(30, 300) } as u32;
                                if view.zoom(f64::from(next) / 100.).is_ok() { zooms.insert(state.active_tab_id, next); }
                            },
                            Command::Print => if let Some(view) = views.get(&state.active_tab_id) { let _ = view.print(); },
                            Command::Exit => if may_exit(&state) { *flow = ControlFlow::Exit; },
                            Command::DefaultBrowser => { if let Err(error) = std::process::Command::new("explorer.exe").arg("ms-settings:defaultapps").spawn() { state.update_status = Some(format!("Could not open Windows default apps: {error}")); } },
                            Command::CheckUpdates => if !check_busy {
                                check_busy = true; state.update_status = Some("Checking for updates…".into());
                                let proxy = proxy.clone(); std::thread::spawn(move || { let _ = proxy.send_event(UserEvent::UpdateChecked(updater::check_for_updates())); });
                            },
                            Command::ApplyUpdate => if !state.update_busy && state.active_downloads == 0 && let Some(update) = &state.update_available {
                                let url = update.download_url.clone().unwrap_or_else(|| format!("https://github.com/{}/releases/download/v{}/felaris.exe", updater::GITHUB_REPO, update.version));
                                if state.save_session().is_ok() {
                                    state.update_busy = true; state.update_status = Some("Downloading update. Felaris will restart when it is ready.".into());
                                    let proxy = proxy.clone(); std::thread::spawn(move || { if let Err(error) = updater::download_and_apply_update(&url) { let _ = proxy.send_event(UserEvent::UpdateFailed(format!("Update failed: {error}. You can try again."))); } });
                                } else { state.update_status = Some("Save your session successfully before updating. Check available disk space.".into()); }
                            },
                        }
                    }
                    UserEvent::Title(id, generation, title) => if let Some(tab) = state.tabs.iter_mut().find(|tab| tab.id == id && tab.generation == generation) && tab.error.is_none() && !title.is_empty() { history::retitle(&tab.url, &title); tab.title = title; },
                    UserEvent::PageLoad(id, generation, loading, url) => if let Some(tab) = state.tabs.iter_mut().find(|tab| tab.id == id && tab.generation == generation) {
                        if loading {
                            if tab.pending_url.as_ref().is_some_and(|expected| expected != &url && !(profile::is_internal(expected) && url == "about:blank")) { return; }
                            tab.pending_url = None;
                            if let Some(url) = profile::normalize_url(&url) { tab.url = url; tab.error = None; }
                            else if tab.error.is_none() && (url == "about:blank" || url.starts_with("data:text/html;")) && !profile::is_internal(&tab.url) { tab.url = "felaris://newtab".into(); }
                            tab.is_loading = true;
                        } else {
                            let failed = tab.error.is_some();
                            tab.is_loading = false;
                            if tab.url == url && profile::allowed_url(&url) { tab.error = None; }
                            if tab.url == url && !failed { history::visit(&tab.url, &tab.title); }
                        }
                    },
                    UserEvent::Source(id, generation, url) => if let Some(tab) = state.tabs.iter_mut().find(|tab| tab.id == id && tab.generation == generation && tab.pending_url.is_none())
                        && let Some(url) = profile::normalize_url(&url) { tab.url = url; },
                    UserEvent::History(id, generation, back, forward) => if let Some(tab) = state.tabs.iter_mut().find(|tab| tab.id == id && tab.generation == generation) { tab.can_go_back = back; tab.can_go_forward = forward; },
                    UserEvent::Failed(id, generation, url, message) => if let Some(tab) = state.tabs.iter_mut().find(|tab| tab.id == id && tab.generation == generation && tab.pending_url.as_ref().is_none_or(|pending| pending == &url) && tab.url == url) {
                        tab.error = Some(message); tab.pending_url = None; tab.is_loading = false;
                    },
                    UserEvent::Favicon(id, generation, uri) => if state.tabs.iter().any(|tab| tab.id == id && tab.generation == generation) { state.set_favicon(id, (uri.starts_with("https://") || uri.starts_with("http://")).then_some(uri)); },
                    UserEvent::Crashed(id, generation) => {
                        if state.tabs.iter().any(|tab| tab.id == id && tab.generation == generation) { views.remove(&id); }
                        if let Some(tab) = state.tabs.iter_mut().find(|tab| tab.id == id && tab.generation == generation) { tab.error = Some("This tab stopped responding. Reload it to continue.".into()); tab.is_loading = false; }
                    },
                    UserEvent::DownloadStarted(id) => { *downloads.entry(id).or_default() += 1; },
                    UserEvent::DownloadFinished(id, success) => { let count = downloads.entry(id).or_default(); *count = count.saturating_sub(1); state.update_status = Some(if success { "Download complete. Open Downloads to view the file." } else { "Download interrupted. Open Downloads to retry or choose another location." }.into()); },
                    UserEvent::UpdateChecked(result) => { check_busy = false; match result {
                        Ok(Some(update)) => { state.update_available = Some(update); state.update_status = None; },
                        Ok(None) => { state.update_status = Some(format!("Felaris is up to date (v{}).", updater::CURRENT_VERSION)); },
                        Err(error) => { state.update_status = Some(error); }
                    } },
                    UserEvent::Status(status) => { state.update_status = Some(status); },
                    UserEvent::AdBlockerInstalled(result) => match result {
                        Ok(()) if extensions::enabled_at_startup() && state.active_downloads == 0 => {
                            // Rebuilding the pages makes WebView2 load the new extension.
                            views.clear();
                            state.update_status = Some("uBlock Origin Lite is installed and blocking ads.".into());
                        },
                        Ok(()) => { state.update_status = Some("uBlock Origin Lite is installed. Restart Felaris to start blocking ads.".into()); },
                        Err(error) => { state.update_status = Some(format!("Ad blocker install failed: {error} Try again from Settings.")); },
                    },
                    UserEvent::UpdateFailed(status) => { state.update_busy = false; state.update_status = Some(status); },
                }
                if !transient && let Err(error) = state.save_session() { state.update_status = Some(format!("Could not save your session: {error}")); }
            }
            _ => return,
        }
        state.active_downloads = downloads.values().sum();
        views.retain(|id,_| state.tabs.iter().any(|tab| tab.id == *id && !tab.is_sleeping));
        let active_id = state.active_tab_id;
        if let std::collections::hash_map::Entry::Vacant(entry) = views.entry(active_id) {
            match content(&window, &mut context, &mut state, active_id, &proxy) {
                Ok(view) => {
                    if state.active_tab().is_some_and(|tab| tab.error.is_some()) { let _ = view.load_html(include_str!("error.html")); }
                    entry.insert(view);

                }
                Err(error) => { state.update_status = Some(format!("Could not open this tab: {error}")); }
            }
        }
        zooms.retain(|id, _| views.contains_key(id));
        state.zoom = zooms.get(&active_id).copied().unwrap_or(100);
        let now = std::time::Instant::now();
        let dt = if animating { now.duration_since(last_frame).as_secs_f64().min(0.05) } else { 0. };
        last_frame = now;
        let side_target = if !state.sidebar_collapsed || state.sidebar_peek { 1. } else { 0. };
        let top_target = if state.topbar_peek { 1. } else { 0. };
        side_progress = step(side_progress, side_target, dt, !state.sidebar_collapsed);
        top_progress = step(top_progress, top_target, dt, false);
        animating = side_progress != side_target || top_progress != top_target;
        if animating { *flow = ControlFlow::WaitUntil(now + std::time::Duration::from_millis(16)); }
        let (sidebar_bounds, content_bounds, overlay_bounds, topbar_bounds) = bounds(&window, &state, ease(side_progress), ease(top_progress));
        let _ = sidebar.set_bounds(sidebar_bounds);
        for (id,view) in &views { let _ = view.set_bounds(content_bounds); let _ = view.set_visible(*id == active_id); }
        let physical = content_bounds.size.to_physical::<u32>(window.scale_factor());
        let radius = if window.is_maximized() || window.fullscreen().is_some() { 0 } else { (ui::CONTENT_RADIUS * window.scale_factor()) as i32 };
        let shape = (physical.width, physical.height, radius);
        rounded.retain(|id, _| views.contains_key(id));
        for (id, view) in &views { if rounded.get(id) != Some(&shape) { desktop::round(view, shape.0 as i32, shape.1 as i32, radius); rounded.insert(*id, shape); } }
        let _ = topbar.set_bounds(topbar_bounds);
        if !palette_open {
            if state.sidebar_collapsed && side_progress > 0. { desktop::raise(&sidebar); }
            desktop::raise(&topbar);
        }
        let _ = palette.set_bounds(overlay_bounds);
        let _ = palette.set_visible(palette_open);
        if visible_id != Some(active_id) && !palette_open {
            if !headless && let Some(view) = views.get(&active_id) { let _ = view.focus(); }
            visible_id = Some(active_id);
        }
        if tick { return; }
        state.is_loading = state.active_tab().is_some_and(|tab| tab.is_loading);
        if state.is_loading != last_loading { last_loading = state.is_loading; let _ = topbar.evaluate_script(&format!("window.setLoading?.({last_loading})")); }
        if let Some(tab) = state.active_tab() { window.set_title(&format!("{} — Felaris", tab.title)); }
        if let Ok(json) = serde_json::to_string(&state) {
            let _ = sidebar.evaluate_script(&format!("window.updateState?.({json})"));
            if focus_address { if !headless { let _ = sidebar.focus(); } let _ = sidebar.evaluate_script("window.focusAddress()"); focus_address = false; }
            if palette_open && palette_needs_open { desktop::raise(&palette); let _ = palette.evaluate_script(&format!("window.openNewTab?.({json})")); if !headless { let _ = palette.focus(); } palette_needs_open = false; }
        }
    });
}
