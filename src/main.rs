mod start_page;
mod state;
mod ui;

use serde::Deserialize;
use state::BrowserState;
use std::hash::BuildHasher;
use tao::{
    dpi::{LogicalPosition, LogicalSize, Position, Size},
    event::{Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy},
    window::{Icon, WindowBuilder},
};
use ui::{SIDEBAR_WIDTH_COLLAPSED, SIDEBAR_WIDTH_EXPANDED};
use wry::{PageLoadEvent, Rect, WebView, WebViewBuilder};

#[derive(Debug, Deserialize)]
#[serde(tag = "action")]
enum IpcMessage {
    #[serde(rename = "navigate")]
    Navigate { url: String },
    #[serde(rename = "back")]
    Back,
    #[serde(rename = "forward")]
    Forward,
    #[serde(rename = "reload")]
    Reload,
    #[serde(rename = "new_tab")]
    NewTab { url: Option<String> },
    #[serde(rename = "switch_tab")]
    SwitchTab { id: usize },
    #[serde(rename = "close_tab")]
    CloseTab { id: usize },
    #[serde(rename = "toggle_sleep")]
    ToggleSleep { id: usize },
    #[serde(rename = "sleep_inactive_tabs")]
    SleepInactiveTabs,
    #[serde(rename = "clear_unpinned_tabs")]
    ClearUnpinnedTabs,
    #[serde(rename = "toggle_sidebar")]
    ToggleSidebar,
    #[serde(rename = "switch_workspace")]
    SwitchWorkspace { id: usize },
    #[serde(rename = "restore_closed_tab")]
    RestoreClosedTab,
    #[serde(rename = "toggle_pin")]
    TogglePin { id: usize },
    #[serde(rename = "focus_address")]
    FocusAddress,
    #[serde(rename = "close_active_tab")]
    CloseActiveTab,
    #[serde(rename = "cycle_tab")]
    CycleTab { backwards: bool },
    #[serde(rename = "stop")]
    Stop,
}

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
enum NewTabMessage {
    SubmitNewTab { url: String },
    DismissNewTab,
    SwitchTab { id: usize },
}

#[derive(Deserialize)]
struct ContentMessage {
    #[serde(flatten)]
    message: IpcMessage,
    shortcut_token: Option<String>,
}

#[derive(Debug)]
enum UserEvent {
    Navigate(String),
    Back,
    Forward,
    Reload,
    NewTab(Option<String>),
    SwitchTab(usize),
    CloseTab(usize),
    ToggleSleep(usize),
    SleepInactiveTabs,
    ClearUnpinnedTabs,
    ToggleSidebar,
    SwitchWorkspace(usize),
    RestoreClosedTab,
    TogglePin(usize),
    FocusAddress,
    CloseActiveTab,
    CycleTab(bool),
    Stop,
    SidebarReady,
    NewTabReady,
    DismissNewTab,
    TitleChanged(String),
    PageLoad(bool, String),
}

fn load_content_url(view: &WebView, state: &BrowserState, url: &str) {
    if url == "felaris://newtab" {
        let _ = view.load_html(&start_page::get_start_page_html(&state.top_shortcuts()));
    } else {
        let _ = view.load_url(url);
    }
}

fn show_new_tab(view: &WebView, state: &BrowserState) {
    if let Ok(json) = serde_json::to_string(state) {
        let _ = view.set_visible(true);
        let _ = view.focus();
        let _ = view.evaluate_script(&format!("window.openNewTab?.({json})"));
    }
}

fn resize_viewports(
    window: &tao::window::Window,
    sidebar_view: &WebView,
    content_view: &WebView,
    new_tab_view: &WebView,
    collapsed: bool,
) {
    let physical = window.inner_size();
    let logical = physical.to_logical::<f64>(window.scale_factor());
    let _ = new_tab_view.set_bounds(Rect {
        position: Position::Logical(LogicalPosition::new(0.0, 0.0)),
        size: Size::Logical(logical),
    });
    let sidebar_width = if collapsed {
        SIDEBAR_WIDTH_COLLAPSED
    } else {
        SIDEBAR_WIDTH_EXPANDED
    };

    let content_x = sidebar_width + 8.0;
    let content_y = 8.0;
    let content_w = (logical.width - content_x - 8.0).max(10.0);
    let content_h = (logical.height - 16.0).max(10.0);

    let _ = sidebar_view.set_bounds(Rect {
        position: Position::Logical(LogicalPosition::new(0.0, 0.0)),
        size: Size::Logical(LogicalSize::new(sidebar_width, logical.height)),
    });

    let _ = content_view.set_bounds(Rect {
        position: Position::Logical(LogicalPosition::new(content_x, content_y)),
        size: Size::Logical(LogicalSize::new(content_w, content_h)),
    });
}

fn load_app_icon() -> Option<Icon> {
    const ICON_RGBA: &[u8] = include_bytes!("../assets/icon_64.rgba");
    Icon::from_rgba(ICON_RGBA.to_vec(), 64, 64).ok()
}

fn main() -> wry::Result<()> {
    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy: EventLoopProxy<UserEvent> = event_loop.create_proxy();

    let mut window_builder = WindowBuilder::new()
        .with_title("Felaris")
        .with_theme(Some(tao::window::Theme::Dark))
        .with_background_color((16, 16, 17, 255))
        .with_inner_size(LogicalSize::new(1380.0, 860.0))
        .with_min_inner_size(LogicalSize::new(640.0, 480.0));

    if let Some(icon) = load_app_icon() {
        window_builder = window_builder.with_window_icon(Some(icon));
    }

    let window = window_builder
        .build(&event_loop)
        .expect("Failed to create main window");

    let initial_size = window.inner_size().to_logical::<f64>(window.scale_factor());
    let mut browser_state = BrowserState::new();

    let sidebar_bounds = Rect {
        position: Position::Logical(LogicalPosition::new(0.0, 0.0)),
        size: Size::Logical(LogicalSize::new(
            SIDEBAR_WIDTH_EXPANDED,
            initial_size.height,
        )),
    };

    let proxy_for_ui = proxy.clone();
    let proxy_sidebar_ready = proxy.clone();
    let sidebar_view = WebViewBuilder::new()
        .with_bounds(sidebar_bounds)
        .with_html(ui::get_ui_html())
        .with_on_page_load_handler(move |event, _| {
            if matches!(event, PageLoadEvent::Finished) {
                let _ = proxy_sidebar_ready.send_event(UserEvent::SidebarReady);
            }
        })
        .with_ipc_handler(move |req| {
            let body = req.body();
            if let Ok(msg) = serde_json::from_str::<IpcMessage>(body) {
                let event = match msg {
                    IpcMessage::Navigate { url } => UserEvent::Navigate(url),
                    IpcMessage::Back => UserEvent::Back,
                    IpcMessage::Forward => UserEvent::Forward,
                    IpcMessage::Reload => UserEvent::Reload,
                    IpcMessage::NewTab { url } => UserEvent::NewTab(url),
                    IpcMessage::SwitchTab { id } => UserEvent::SwitchTab(id),
                    IpcMessage::CloseTab { id } => UserEvent::CloseTab(id),
                    IpcMessage::ToggleSleep { id } => UserEvent::ToggleSleep(id),
                    IpcMessage::SleepInactiveTabs => UserEvent::SleepInactiveTabs,
                    IpcMessage::ClearUnpinnedTabs => UserEvent::ClearUnpinnedTabs,
                    IpcMessage::ToggleSidebar => UserEvent::ToggleSidebar,
                    IpcMessage::SwitchWorkspace { id } => UserEvent::SwitchWorkspace(id),
                    IpcMessage::RestoreClosedTab => UserEvent::RestoreClosedTab,
                    IpcMessage::TogglePin { id } => UserEvent::TogglePin(id),
                    IpcMessage::FocusAddress => UserEvent::FocusAddress,
                    IpcMessage::CloseActiveTab => UserEvent::CloseActiveTab,
                    IpcMessage::CycleTab { backwards } => UserEvent::CycleTab(backwards),
                    IpcMessage::Stop => UserEvent::Stop,
                };
                let _ = proxy_for_ui.send_event(event);
            }
        })
        .build(&window)?;

    let content_x = SIDEBAR_WIDTH_EXPANDED + 8.0;
    let content_w = (initial_size.width - content_x - 8.0).max(10.0);
    let content_h = (initial_size.height - 16.0).max(10.0);

    let content_bounds = Rect {
        position: Position::Logical(LogicalPosition::new(content_x, 8.0)),
        size: Size::Logical(LogicalSize::new(content_w, content_h)),
    };

    let proxy_for_content = proxy.clone();
    let proxy_title = proxy.clone();
    let proxy_ipc = proxy.clone();

    let shortcut_token = std::collections::hash_map::RandomState::new()
        .hash_one("felaris-shortcuts")
        .to_string();
    let shortcut_script = include_str!("browser.js").replace("__SHORTCUT_TOKEN__", &shortcut_token);
    let content_view = WebViewBuilder::new()
        .with_initialization_script(&shortcut_script)
        .with_bounds(content_bounds)
        .with_html(start_page::get_start_page_html(
            &browser_state.top_shortcuts(),
        ))
        .with_document_title_changed_handler(move |title| {
            let _ = proxy_title.send_event(UserEvent::TitleChanged(title));
        })
        .with_on_page_load_handler(move |event, url| {
            let loading = matches!(event, PageLoadEvent::Started);
            let _ = proxy_for_content.send_event(UserEvent::PageLoad(loading, url));
        })
        .with_ipc_handler(move |req| {
            let body = req.body();
            if let Ok(msg) = serde_json::from_str::<ContentMessage>(body) {
                // Only the injected trusted-key handler can issue browser-management commands.
                if !matches!(&msg.message, IpcMessage::Navigate { .. })
                    && msg.shortcut_token.as_deref() != Some(shortcut_token.as_str())
                {
                    return;
                }
                let event = match msg.message {
                    IpcMessage::Navigate { url } => UserEvent::Navigate(url),
                    IpcMessage::NewTab { url } => UserEvent::NewTab(url),
                    IpcMessage::CloseActiveTab => UserEvent::CloseActiveTab,
                    IpcMessage::FocusAddress => UserEvent::FocusAddress,
                    IpcMessage::ToggleSidebar => UserEvent::ToggleSidebar,
                    IpcMessage::RestoreClosedTab => UserEvent::RestoreClosedTab,
                    IpcMessage::SleepInactiveTabs => UserEvent::SleepInactiveTabs,
                    IpcMessage::CycleTab { backwards } => UserEvent::CycleTab(backwards),
                    IpcMessage::SwitchWorkspace { id: id @ 1..=3 } => {
                        UserEvent::SwitchWorkspace(id)
                    }
                    IpcMessage::Back => UserEvent::Back,
                    IpcMessage::Forward => UserEvent::Forward,
                    IpcMessage::Reload => UserEvent::Reload,
                    _ => return,
                };
                let _ = proxy_ipc.send_event(event);
            }
        })
        .build(&window)?;

    let proxy_new_tab = proxy.clone();
    let proxy_new_tab_ready = proxy.clone();
    let new_tab_view = WebViewBuilder::new()
        .with_bounds(Rect {
            position: Position::Logical(LogicalPosition::new(0.0, 0.0)),
            size: Size::Logical(initial_size),
        })
        .with_visible(false)
        .with_focused(false)
        .with_transparent(true)
        .with_background_color((0, 0, 0, 0))
        .with_html(ui::get_new_tab_html())
        .with_navigation_handler(|url| {
            url == "about:blank" || url.starts_with("data:text/html;charset=utf-8;base64,")
        })
        .with_on_page_load_handler(move |event, _| {
            if matches!(event, PageLoadEvent::Finished) {
                let _ = proxy_new_tab_ready.send_event(UserEvent::NewTabReady);
            }
        })
        .with_ipc_handler(move |request| {
            if let Ok(message) = serde_json::from_str::<NewTabMessage>(request.body()) {
                let event = match message {
                    NewTabMessage::SubmitNewTab { url } => UserEvent::NewTab(Some(url)),
                    NewTabMessage::DismissNewTab => UserEvent::DismissNewTab,
                    NewTabMessage::SwitchTab { id } => UserEvent::SwitchTab(id),
                };
                let _ = proxy_new_tab.send_event(event);
            }
        })
        .build(&window)?;
    let mut new_tab_open = false;

    let sync_ui = |state: &BrowserState, view: &wry::WebView| {
        if let Ok(json) = serde_json::to_string(state) {
            let script = format!("window.updateState({});", json);
            let _ = view.evaluate_script(&script);
        }
    };

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::NewEvents(StartCause::Init) => {
                sync_ui(&browser_state, &sidebar_view);
            }

            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }

            Event::WindowEvent {
                event: WindowEvent::Resized(_),
                ..
            } => {
                resize_viewports(
                    &window,
                    &sidebar_view,
                    &content_view,
                    &new_tab_view,
                    browser_state.sidebar_collapsed,
                );
            }

            Event::UserEvent(user_event) => match user_event {
                UserEvent::SidebarReady => sync_ui(&browser_state, &sidebar_view),

                UserEvent::NewTabReady => {
                    if new_tab_open {
                        show_new_tab(&new_tab_view, &browser_state);
                    }
                }

                UserEvent::DismissNewTab => {
                    new_tab_open = false;
                    let _ = new_tab_view.set_visible(false);
                    let _ = content_view.focus();
                }

                UserEvent::Navigate(url) => {
                    browser_state.update_active_tab(url.clone(), None);
                    load_content_url(&content_view, &browser_state, &url);
                    sync_ui(&browser_state, &sidebar_view);
                }

                UserEvent::Back => {
                    let _ = content_view.evaluate_script("history.back()");
                }

                UserEvent::Forward => {
                    let _ = content_view.evaluate_script("history.forward()");
                }

                UserEvent::Reload => {
                    if let Some(active) = browser_state.active_tab() {
                        if active.url == "felaris://newtab" {
                            let url = active.url.clone();
                            load_content_url(&content_view, &browser_state, &url);
                        } else {
                            let _ = content_view.evaluate_script("location.reload()");
                        }
                    }
                }

                UserEvent::Stop => {
                    let _ = content_view.evaluate_script("window.stop()");
                    browser_state.set_loading(false);
                    sync_ui(&browser_state, &sidebar_view);
                }

                UserEvent::FocusAddress => {
                    if browser_state.sidebar_collapsed {
                        browser_state.toggle_sidebar();
                        resize_viewports(
                            &window,
                            &sidebar_view,
                            &content_view,
                            &new_tab_view,
                            false,
                        );
                    }
                    sync_ui(&browser_state, &sidebar_view);
                    let _ = sidebar_view.focus();
                    let _ = sidebar_view.evaluate_script("window.focusAddress()");
                }

                UserEvent::TogglePin(id) => {
                    browser_state.toggle_pin(id);
                    sync_ui(&browser_state, &sidebar_view);
                }

                UserEvent::CycleTab(backwards) => {
                    if let Some(url) = browser_state
                        .cycle_tab(backwards)
                        .map(|tab| tab.url.clone())
                    {
                        load_content_url(&content_view, &browser_state, &url);
                    }
                    sync_ui(&browser_state, &sidebar_view);
                }

                UserEvent::CloseActiveTab => {
                    let _ = proxy.send_event(UserEvent::CloseTab(browser_state.active_tab_id));
                }

                UserEvent::NewTab(None) => {
                    new_tab_open = true;
                    show_new_tab(&new_tab_view, &browser_state);
                }

                UserEvent::NewTab(Some(url)) => {
                    if new_tab_open {
                        new_tab_open = false;
                        let _ = new_tab_view.set_visible(false);
                    }
                    browser_state.add_tab(Some(url), false);
                    if let Some(url) = browser_state.active_tab().map(|active| active.url.clone()) {
                        load_content_url(&content_view, &browser_state, &url);
                    }
                    let _ = content_view.focus();
                    sync_ui(&browser_state, &sidebar_view);
                }

                UserEvent::SwitchTab(id) => {
                    if new_tab_open {
                        new_tab_open = false;
                        let _ = new_tab_view.set_visible(false);
                        let _ = content_view.focus();
                    }
                    if let Some(target_url) =
                        browser_state.switch_tab(id).map(|tab| tab.url.clone())
                    {
                        load_content_url(&content_view, &browser_state, &target_url);
                    }
                    sync_ui(&browser_state, &sidebar_view);
                }

                UserEvent::CloseTab(id) => {
                    let was_active = id == browser_state.active_tab_id;
                    browser_state.close_tab(id);
                    if was_active {
                        if let Some(url) =
                            browser_state.active_tab().map(|active| active.url.clone())
                        {
                            load_content_url(&content_view, &browser_state, &url);
                        }
                    }
                    sync_ui(&browser_state, &sidebar_view);
                }

                UserEvent::ToggleSleep(id) => {
                    browser_state.toggle_sleep(id);
                    sync_ui(&browser_state, &sidebar_view);
                }

                UserEvent::SleepInactiveTabs => {
                    browser_state.sleep_inactive_tabs();
                    sync_ui(&browser_state, &sidebar_view);
                }

                UserEvent::ClearUnpinnedTabs => {
                    browser_state.clear_unpinned_tabs();
                    sync_ui(&browser_state, &sidebar_view);
                }

                UserEvent::ToggleSidebar => {
                    let collapsed = browser_state.toggle_sidebar();
                    resize_viewports(
                        &window,
                        &sidebar_view,
                        &content_view,
                        &new_tab_view,
                        collapsed,
                    );
                    sync_ui(&browser_state, &sidebar_view);
                }

                UserEvent::SwitchWorkspace(id) => {
                    if let Some(target_url) = browser_state
                        .switch_workspace(id)
                        .map(|active| active.url.clone())
                    {
                        load_content_url(&content_view, &browser_state, &target_url);
                    }
                    sync_ui(&browser_state, &sidebar_view);
                }

                UserEvent::RestoreClosedTab => {
                    if let Some(_id) = browser_state.restore_closed_tab() {
                        if let Some(url) =
                            browser_state.active_tab().map(|active| active.url.clone())
                        {
                            load_content_url(&content_view, &browser_state, &url);
                        }
                        sync_ui(&browser_state, &sidebar_view);
                    }
                }

                UserEvent::TitleChanged(title) => {
                    if !title.is_empty() {
                        if let Some(active) = browser_state.active_tab() {
                            let url = active.url.clone();
                            browser_state.update_active_tab(url, Some(title));
                            sync_ui(&browser_state, &sidebar_view);
                        }
                    }
                }

                UserEvent::PageLoad(loading, url) => {
                    browser_state.set_loading(loading);
                    if !url.is_empty() && url != "about:blank" && !url.starts_with("data:") {
                        browser_state.update_active_tab(url, None);
                    }
                    sync_ui(&browser_state, &sidebar_view);
                }
            },

            _ => (),
        }
    });
}
