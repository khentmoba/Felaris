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
use ui::{UI_HEIGHT_NORMAL, UI_HEIGHT_WITH_BOOKMARKS};
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
    #[serde(rename = "toggle_bookmarks_shelf")]
    ToggleBookmarksShelf,
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
    ToggleBookmarksShelf,
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
    ui_view: &WebView,
    content_view: &WebView,
    show_bookmarks: bool,
) {
    let physical = window.inner_size();
    let logical = physical.to_logical::<f64>(window.scale_factor());
    let ui_height = if show_bookmarks {
        UI_HEIGHT_WITH_BOOKMARKS
    } else {
        UI_HEIGHT_NORMAL
    };
    let content_height = (logical.height - ui_height).max(10.0);

    let _ = ui_view.set_bounds(Rect {
        position: Position::Logical(LogicalPosition::new(0.0, 0.0)),
        size: Size::Logical(LogicalSize::new(logical.width, ui_height)),
    });

    let _ = content_view.set_bounds(Rect {
        position: Position::Logical(LogicalPosition::new(0.0, ui_height)),
        size: Size::Logical(LogicalSize::new(logical.width, content_height)),
    });
}

fn main() -> wry::Result<()> {
    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy: EventLoopProxy<UserEvent> = event_loop.create_proxy();

    let window = WindowBuilder::new()
        .with_title("Felaris")
        .with_inner_size(LogicalSize::new(1280.0, 800.0))
        .with_min_inner_size(LogicalSize::new(520.0, 380.0))
        .build(&event_loop)
        .expect("Failed to create main window");

    let initial_size = window.inner_size().to_logical::<f64>(window.scale_factor());
    let mut browser_state = BrowserState::new();

    // 1. UI Webview (Chrome: Tabs + Nav + Omnibox + Bookmarks)
    let ui_bounds = Rect {
        position: Position::Logical(LogicalPosition::new(0.0, 0.0)),
        size: Size::Logical(LogicalSize::new(initial_size.width, UI_HEIGHT_NORMAL)),
    };

    let proxy_for_ui = proxy.clone();
    let ui_view = WebViewBuilder::new()
        .with_bounds(ui_bounds)
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
                    IpcMessage::ToggleBookmarksShelf => UserEvent::ToggleBookmarksShelf,
                    IpcMessage::RestoreClosedTab => UserEvent::RestoreClosedTab,
                };
                let _ = proxy_for_ui.send_event(event);
            }
        })
        .build(&window)?;

    // 2. Content Webview (Main Web Viewport)
    let content_height = (initial_size.height - UI_HEIGHT_NORMAL).max(10.0);
    let content_bounds = Rect {
        position: Position::Logical(LogicalPosition::new(0.0, UI_HEIGHT_NORMAL)),
        size: Size::Logical(LogicalSize::new(initial_size.width, content_height)),
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

    // Synchronize UI state helper
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
                sync_ui(&browser_state, &ui_view);
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
                    &ui_view,
                    &content_view,
                    browser_state.show_bookmarks,
                );
            }

            Event::UserEvent(user_event) => match user_event {
                UserEvent::Navigate(url) => {
                    browser_state.update_active_tab(url.clone(), None);
                    load_content_url(&content_view, &url);
                    sync_ui(&browser_state, &ui_view);
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
                    let _id = browser_state.add_tab(url);
                    if let Some(active) = browser_state.active_tab() {
                        load_content_url(&content_view, &active.url);
                    }
                    sync_ui(&browser_state, &ui_view);
                }

                UserEvent::SwitchTab(id) => {
                    if let Some(tab) = browser_state.switch_tab(id) {
                        let target_url = tab.url.clone();
                        load_content_url(&content_view, &target_url);
                    }
                    sync_ui(&browser_state, &ui_view);
                }

                UserEvent::CloseTab(id) => {
                    browser_state.close_tab(id);
                    if let Some(active) = browser_state.active_tab() {
                        load_content_url(&content_view, &active.url);
                    }
                    sync_ui(&browser_state, &ui_view);
                }

                UserEvent::ToggleSleep(id) => {
                    browser_state.toggle_sleep(id);
                    sync_ui(&browser_state, &ui_view);
                }

                UserEvent::SleepInactiveTabs => {
                    browser_state.sleep_inactive_tabs();
                    sync_ui(&browser_state, &ui_view);
                }

                UserEvent::ToggleBookmarksShelf => {
                    let show = browser_state.toggle_bookmarks_shelf();
                    resize_viewports(&window, &ui_view, &content_view, show);
                    sync_ui(&browser_state, &ui_view);
                }

                UserEvent::RestoreClosedTab => {
                    if let Some(_id) = browser_state.restore_closed_tab() {
                        if let Some(active) = browser_state.active_tab() {
                            load_content_url(&content_view, &active.url);
                        }
                        sync_ui(&browser_state, &ui_view);
                    }
                }

                UserEvent::TitleChanged(title) => {
                    if !title.is_empty() {
                        if let Some(active) = browser_state.active_tab() {
                            let url = active.url.clone();
                            browser_state.update_active_tab(url, Some(title));
                            sync_ui(&browser_state, &ui_view);
                        }
                    }
                }

                UserEvent::PageLoad(loading, url) => {
                    browser_state.set_loading(loading);
                    if !url.is_empty() && url != "about:blank" && !url.starts_with("data:") {
                        browser_state.update_active_tab(url, None);
                    }
                    sync_ui(&browser_state, &ui_view);
                }
            },

            _ => (),
        }
    });
}
