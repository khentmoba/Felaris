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
use ui::UI_HEIGHT;
use wry::{Rect, WebViewBuilder};

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
}

fn main() -> wry::Result<()> {
    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy: EventLoopProxy<UserEvent> = event_loop.create_proxy();

    let window = WindowBuilder::new()
        .with_title("Felaris")
        .with_inner_size(LogicalSize::new(1280.0, 800.0))
        .with_min_inner_size(LogicalSize::new(480.0, 360.0))
        .build(&event_loop)
        .expect("Failed to create main window");

    let initial_size = window.inner_size().to_logical::<f64>(window.scale_factor());
    let mut browser_state = BrowserState::new();

    // 1. UI Webview (Tabs + Address Bar)
    let ui_bounds = Rect {
        position: Position::Logical(LogicalPosition::new(0.0, 0.0)),
        size: Size::Logical(LogicalSize::new(initial_size.width, UI_HEIGHT)),
    };

    let proxy_clone = proxy.clone();
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
                };
                let _ = proxy_clone.send_event(event);
            }
        })
        .build(&window)?;

    // 2. Content Webview (Web viewport)
    let content_height = (initial_size.height - UI_HEIGHT).max(10.0);
    let content_bounds = Rect {
        position: Position::Logical(LogicalPosition::new(0.0, UI_HEIGHT)),
        size: Size::Logical(LogicalSize::new(initial_size.width, content_height)),
    };

    let initial_url = browser_state.active_tab().unwrap().url.clone();
    let content_view = WebViewBuilder::new()
        .with_bounds(content_bounds)
        .with_url(&initial_url)
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
                event: WindowEvent::Resized(physical_size),
                ..
            } => {
                let logical = physical_size.to_logical::<f64>(window.scale_factor());
                let content_h = (logical.height - UI_HEIGHT).max(10.0);

                let _ = ui_view.set_bounds(Rect {
                    position: Position::Logical(LogicalPosition::new(0.0, 0.0)),
                    size: Size::Logical(LogicalSize::new(logical.width, UI_HEIGHT)),
                });

                let _ = content_view.set_bounds(Rect {
                    position: Position::Logical(LogicalPosition::new(0.0, UI_HEIGHT)),
                    size: Size::Logical(LogicalSize::new(logical.width, content_h)),
                });
            }

            Event::UserEvent(user_event) => match user_event {
                UserEvent::Navigate(url) => {
                    browser_state.update_active_tab(url.clone(), None);
                    let _ = content_view.load_url(&url);
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
                        let _ = content_view.load_url(&active.url);
                    }
                    sync_ui(&browser_state, &ui_view);
                }

                UserEvent::SwitchTab(id) => {
                    if let Some(tab) = browser_state.switch_tab(id) {
                        let target_url = tab.url.clone();
                        let _ = content_view.load_url(&target_url);
                    }
                    sync_ui(&browser_state, &ui_view);
                }

                UserEvent::CloseTab(id) => {
                    browser_state.close_tab(id);
                    if let Some(active) = browser_state.active_tab() {
                        let _ = content_view.load_url(&active.url);
                    }
                    sync_ui(&browser_state, &ui_view);
                }

                UserEvent::ToggleSleep(id) => {
                    browser_state.toggle_sleep(id);
                    sync_ui(&browser_state, &ui_view);
                }
            },

            _ => (),
        }
    });
}
