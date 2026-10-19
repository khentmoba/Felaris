use crate::{desktop, instance::Instance, profile, start_page, state::BrowserState, ui, updater};
use serde::Deserialize;
use std::{collections::HashMap, hash::BuildHasher};
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
    Navigate { url: String },
    Back,
    Forward,
    Reload,
    Stop,
    NewTab { url: Option<String> },
    SwitchTab { id: usize },
    CloseTab { id: usize },
    ToggleSleep { id: usize },
    SleepInactiveTabs,
    ClearUnpinnedTabs,
    ToggleSidebar,
    SwitchWorkspace { id: usize },
    RestoreClosedTab,
    TogglePin { id: usize },
    FocusAddress,
    CloseActiveTab,
    CycleTab { backwards: bool },
    CheckUpdates,
    ApplyUpdate,
    InstallApp,
    OpenDownloads,
    DefaultBrowser,
    DismissNewTab,
    SubmitNewTab { url: String },
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
    External(Vec<String>),
    Title(usize, u64, String),
    PageLoad(usize, u64, bool, String),
    History(usize, u64, bool, bool),
    Source(usize, u64, String),
    Failed(usize, u64, String, String),
    Crashed(usize, u64),
    DownloadStarted(usize),
    DownloadFinished(usize, bool),
    UpdateChecked(Result<Option<updater::UpdateManifest>, String>),
    Status(String),
    UpdateFailed(String),
}

fn bounds(window: &Window, collapsed: bool) -> (Rect, Rect, Rect) {
    let size = window.inner_size().to_logical::<f64>(window.scale_factor());
    let width = if collapsed {
        ui::SIDEBAR_WIDTH_COLLAPSED
    } else {
        ui::SIDEBAR_WIDTH_EXPANDED
    };
    (
        Rect {
            position: Position::Logical(LogicalPosition::new(0., 0.)),
            size: Size::Logical(LogicalSize::new(width, size.height)),
        },
        Rect {
            position: Position::Logical(LogicalPosition::new(width + 8., 8.)),
            size: Size::Logical(LogicalSize::new(
                (size.width - width - 16.).max(10.),
                (size.height - 16.).max(10.),
            )),
        },
        Rect {
            position: Position::Logical(LogicalPosition::new(0., 0.)),
            size: Size::Logical(size),
        },
    )
}

fn load(view: &WebView, state: &BrowserState, url: &str) -> wry::Result<()> {
    if url == "felaris://newtab" {
        view.load_html(&start_page::get_start_page_html(&state.top_shortcuts()))
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
    let script = include_str!("browser.js").replace("__SHORTCUT_TOKEN__", &token);
    let ipc = proxy.clone();
    let title = proxy.clone();
    let popup = proxy.clone();
    let navigation = proxy.clone();
    let mut builder = WebViewBuilder::new_with_web_context(context)
        .with_bounds(bounds(window, state.sidebar_collapsed).1)
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
                if (local && matches!(message.command, Command::Navigate { .. } | Command::Retry))
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
                        ))
                {
                    let _ = ipc.send_event(UserEvent::Command(message.command));
                }
            }
        });
    if std::env::var_os("FELARIS_SMOKE_TEST").is_some() {
        builder = builder.with_download_started_handler(|_, path| {
            *path = profile::directory().join("smoke-download.txt");
            true
        });
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
    for url in urls {
        state.add_tab(Some(url), false);
    }
    let mut context = WebContext::new(Some(directory.join("WebView2")));
    let mut chrome_context = WebContext::new(Some(directory.join("Chrome")));
    let ui_proxy = proxy.clone();
    let ready = proxy.clone();
    let mut sidebar_builder = WebViewBuilder::new_with_web_context(&mut chrome_context)
        .with_bounds(bounds(&window, state.sidebar_collapsed).0)
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
        .with_bounds(bounds(&window, state.sidebar_collapsed).2)
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
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(3));
            let _ = background.send_event(UserEvent::UpdateChecked(updater::check_for_updates()));
        });
    }

    event_loop.run(move |event, _, flow| {
        let _keep_instance = &instance;
        let _keep_chrome_context = &chrome_context;
        *flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                if state.active_downloads > 0 && !desktop::message("Downloads in progress", "Closing Felaris will cancel active downloads. Close anyway?", true) { return; }
                if let Err(error) = state.save_session()
                    && !desktop::message("Session could not be saved", &format!("{error}\n\nClose without saving this session?"), true) { return; }
                *flow = ControlFlow::Exit;
            }
            Event::NewEvents(StartCause::Init) => {},
            Event::WindowEvent { event: WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. }, .. } => {},
            Event::UserEvent(event) => {
                match event {
                    UserEvent::SmokeTick => {
                        if let Some(smoke) = &mut smoke && smoke.tick(&state, &views, &sidebar, &palette, &proxy) { *flow = ControlFlow::Exit; }
                    }
                    UserEvent::SmokeResult(phase, success) => {
                        if let Some(smoke) = &mut smoke && smoke.result(phase, success, &proxy) { *flow = ControlFlow::Exit; }
                    }
                    UserEvent::Ready => {},
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
                                if let Some(view) = views.get(&id) { let _ = if url == "felaris://newtab" { load(view, &state, &url) } else { view.reload() }; }
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
                            Command::SwitchWorkspace { id } => { state.switch_workspace(id); },
                            Command::RestoreClosedTab => { state.restore_closed_tab(); },
                            Command::TogglePin { id } => state.toggle_pin(id),
                            Command::CycleTab { backwards } => { state.cycle_tab(backwards); },
                            Command::FocusAddress => { state.sidebar_collapsed = false; focus_address = true; },
                            Command::OpenDownloads => {
                                let id = downloads.iter().find(|(_, count)| **count > 0).map(|(id,_)| *id).unwrap_or(state.active_tab_id);
                                if let Some(view) = views.get(&id) && let Err(error) = desktop::downloads(view) { state.update_status = Some(error); }
                            },
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
                            Command::InstallApp => { let proxy = proxy.clone(); std::thread::spawn(move || { let result = updater::install_to_system().unwrap_or_else(|error| format!("Install failed: {error}")); let _ = proxy.send_event(UserEvent::Status(result)); }); },
                        }
                    }
                    UserEvent::Title(id, generation, title) => if let Some(tab) = state.tabs.iter_mut().find(|tab| tab.id == id && tab.generation == generation) && tab.error.is_none() && !title.is_empty() { tab.title = title; },
                    UserEvent::PageLoad(id, generation, loading, url) => if let Some(tab) = state.tabs.iter_mut().find(|tab| tab.id == id && tab.generation == generation) {
                        if loading {
                            if tab.pending_url.as_ref().is_some_and(|expected| expected != &url && !(expected == "felaris://newtab" && url == "about:blank")) { return; }
                            tab.pending_url = None;
                            if let Some(url) = profile::normalize_url(&url) { tab.url = url; tab.error = None; }
                            else if tab.error.is_none() && (url == "about:blank" || url.starts_with("data:text/html;")) { tab.url = "felaris://newtab".into(); }
                            tab.is_loading = true;
                        } else if tab.url == url || url == "about:blank" || url.starts_with("data:text/html;") { tab.is_loading = false; if profile::allowed_url(&url) { tab.error = None; } }
                    },
                    UserEvent::Source(id, generation, url) => if let Some(tab) = state.tabs.iter_mut().find(|tab| tab.id == id && tab.generation == generation && tab.pending_url.is_none())
                        && let Some(url) = profile::normalize_url(&url) { tab.url = url; },
                    UserEvent::History(id, generation, back, forward) => if let Some(tab) = state.tabs.iter_mut().find(|tab| tab.id == id && tab.generation == generation) { tab.can_go_back = back; tab.can_go_forward = forward; },
                    UserEvent::Failed(id, generation, url, message) => if let Some(tab) = state.tabs.iter_mut().find(|tab| tab.id == id && tab.generation == generation && tab.pending_url.as_ref().is_none_or(|pending| pending == &url) && tab.url == url) {
                        tab.error = Some(message); tab.pending_url = None; tab.is_loading = false;
                    },
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
                    UserEvent::UpdateFailed(status) => { state.update_busy = false; state.update_status = Some(status); },
                }
                if let Err(error) = state.save_session() { state.update_status = Some(format!("Could not save your session: {error}")); }
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
        let (sidebar_bounds, content_bounds, overlay_bounds) = bounds(&window, state.sidebar_collapsed);
        let _ = sidebar.set_bounds(sidebar_bounds);
        for (id,view) in &views { let _ = view.set_bounds(content_bounds); let _ = view.set_visible(*id == active_id); }
        let _ = palette.set_bounds(overlay_bounds);
        let _ = palette.set_visible(palette_open);
        if visible_id != Some(active_id) && !palette_open {
            if !headless && let Some(view) = views.get(&active_id) { let _ = view.focus(); }
            visible_id = Some(active_id);
        }
        state.is_loading = state.active_tab().is_some_and(|tab| tab.is_loading);
        if let Some(tab) = state.active_tab() { window.set_title(&format!("{} — Felaris", tab.title)); }
        if let Ok(json) = serde_json::to_string(&state) {
            let _ = sidebar.evaluate_script(&format!("window.updateState?.({json})"));
            if focus_address { if !headless { let _ = sidebar.focus(); } let _ = sidebar.evaluate_script("window.focusAddress()"); focus_address = false; }
            if palette_open && palette_needs_open { desktop::raise(&palette); let _ = palette.evaluate_script(&format!("window.openNewTab?.({json})")); if !headless { let _ = palette.focus(); } palette_needs_open = false; }
        }
    });
}
