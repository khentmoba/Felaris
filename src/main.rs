mod start_page;
mod state;
mod ui;

use serde::Deserialize;
use state::BrowserState;
use tao::{
    dpi::{LogicalPosition, LogicalSize, Position, Size},
    event::{Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy},
    window::WindowBuilder,
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
    TitleChanged(String),
    PageLoad(bool, String),
}

fn load_content_url(view: &WebView, url: &str) {
    if url == "felaris://newtab" {
        let _ = view.load_html(start_page::get_start_page_html());
    } else {
        let _ = view.load_url(url);
    }
}

fn resize_viewports(
    window: &tao::window::Window,
    sidebar_view: &WebView,
    content_view: &WebView,
    collapsed: bool,
) {
    let physical = window.inner_size();
    let logical = physical.to_logical::<f64>(window.scale_factor());
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

fn main() -> wry::Result<()> {
    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy: EventLoopProxy<UserEvent> = event_loop.create_proxy();

    let window = WindowBuilder::new()
        .with_title("Felaris")
        .with_inner_size(LogicalSize::new(1380.0, 860.0))
        .with_min_inner_size(LogicalSize::new(640.0, 480.0))
        .build(&event_loop)
        .expect("Failed to create main window");

    let initial_size = window.inner_size().to_logical::<f64>(window.scale_factor());
    let mut browser_state = BrowserState::new();

    // 1. Sidebar Webview (Arc/Zen-inspired Left Rail)
    let sidebar_bounds = Rect {
        position: Position::Logical(LogicalPosition::new(0.0, 0.0)),
        size: Size::Logical(LogicalSize::new(SIDEBAR_WIDTH_EXPANDED, initial_size.height)),
    };

    let proxy_for_ui = proxy.clone();
    let sidebar_view = WebViewBuilder::new()
        .with_bounds(sidebar_bounds)
        .with_html(ui::get_ui_html())
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
                };
                let _ = proxy_for_ui.send_event(event);
            }
        })
        .build(&window)?;

    // 2. Framed Content Webview (Main Viewport)
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

    let content_view = WebViewBuilder::new()
        .with_bounds(content_bounds)
        .with_html(start_page::get_start_page_html())
        .with_document_title_changed_handler(move |title| {
            let _ = proxy_title.send_event(UserEvent::TitleChanged(title));
        })
        .with_on_page_load_handler(move |event, url| {
            let loading = matches!(event, PageLoadEvent::Started);
            let _ = proxy_for_content.send_event(UserEvent::PageLoad(loading, url));
        })
        .with_ipc_handler(move |req| {
            let body = req.body();
            if let Ok(msg) = serde_json::from_str::<IpcMessage>(body) {
                if let IpcMessage::Navigate { url } = msg {
                    let _ = proxy_ipc.send_event(UserEvent::Navigate(url));
                }
            }
        })
        .build(&window)?;

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
                    browser_state.sidebar_collapsed,
                );
            }

            Event::UserEvent(user_event) => match user_event {
                UserEvent::Navigate(url) => {
                    browser_state.update_active_tab(url.clone(), None);
                    load_content_url(&content_view, &url);
                    sync_ui(&browser_state, &sidebar_view);
                }

                UserEvent::Back => {
                    let _ = content_view.evaluate_script("history.back()");
                }

                UserEvent::Forward => {
                    let _ = content_view.evaluate_script("history.forward()");
                }

                UserEvent::Reload => {
                    let _ = content_view.evaluate_script("location.reload()");
                }

                UserEvent::NewTab(url) => {
                    let _id = browser_state.add_tab(url, false);
                    if let Some(active) = browser_state.active_tab() {
                        load_content_url(&content_view, &active.url);
                    }
                    sync_ui(&browser_state, &sidebar_view);
                }

                UserEvent::SwitchTab(id) => {
                    if let Some(tab) = browser_state.switch_tab(id) {
                        let target_url = tab.url.clone();
                        load_content_url(&content_view, &target_url);
                    }
                    sync_ui(&browser_state, &sidebar_view);
                }

                UserEvent::CloseTab(id) => {
                    browser_state.close_tab(id);
                    if let Some(active) = browser_state.active_tab() {
                        load_content_url(&content_view, &active.url);
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
                    resize_viewports(&window, &sidebar_view, &content_view, collapsed);
                    sync_ui(&browser_state, &sidebar_view);
                }

                UserEvent::SwitchWorkspace(id) => {
                    if let Some(active) = browser_state.switch_workspace(id) {
                        let target_url = active.url.clone();
                        load_content_url(&content_view, &target_url);
                    }
                    sync_ui(&browser_state, &sidebar_view);
                }

                UserEvent::RestoreClosedTab => {
                    if let Some(_id) = browser_state.restore_closed_tab() {
                        if let Some(active) = browser_state.active_tab() {
                            load_content_url(&content_view, &active.url);
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
