use crate::app::UserEvent;
use tao::event_loop::EventLoopProxy;
use wry::WebView;

pub fn raise(view: &WebView) {
    #[cfg(windows)]
    {
        use windows::Win32::UI::WindowsAndMessaging::{
            HWND_TOP, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetWindowPos,
        };
        use wry::WebViewExtWindows;
        unsafe {
            let _ = SetWindowPos(
                view.hwnd(),
                Some(HWND_TOP),
                0,
                0,
                0,
                0,
                SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE,
            );
        }
    }
    #[cfg(not(windows))]
    {
        let _ = view;
    }
}

pub fn message(title: &str, text: &str, confirm: bool) -> bool {
    if std::env::var_os("FELARIS_SMOKE_TEST").is_some() {
        eprintln!("{title}: {text}");
        return false;
    }
    #[cfg(windows)]
    {
        use windows::{
            Win32::UI::WindowsAndMessaging::{
                IDYES, MB_DEFBUTTON2, MB_ICONINFORMATION, MB_ICONWARNING, MB_YESNO, MessageBoxW,
            },
            core::PCWSTR,
        };
        let text: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
        let title: Vec<u16> = title.encode_utf16().chain(Some(0)).collect();
        let flags = if confirm {
            MB_YESNO | MB_ICONWARNING | MB_DEFBUTTON2
        } else {
            MB_ICONINFORMATION
        };
        unsafe { MessageBoxW(None, PCWSTR(text.as_ptr()), PCWSTR(title.as_ptr()), flags) == IDYES }
    }
    #[cfg(not(windows))]
    {
        eprintln!("{title}: {text}");
        !confirm
    }
}

#[cfg(windows)]
pub fn attach(
    view: &WebView,
    id: usize,
    generation: u64,
    proxy: EventLoopProxy<UserEvent>,
) -> wry::Result<()> {
    use webview2_com::{Microsoft::Web::WebView2::Win32::*, *};
    use windows::core::{BOOL, Interface, PWSTR};
    use wry::WebViewExtWindows;
    let webview = view.webview();
    let mut token = 0;
    let history = proxy.clone();
    unsafe {
        webview.add_HistoryChanged(
            &HistoryChangedEventHandler::create(Box::new(move |view, _| {
                if let Some(view) = view {
                    let (mut back, mut forward) = (BOOL::default(), BOOL::default());
                    view.CanGoBack(&mut back)?;
                    view.CanGoForward(&mut forward)?;
                    let _ = history.send_event(UserEvent::History(
                        id,
                        generation,
                        back.as_bool(),
                        forward.as_bool(),
                    ));
                }
                Ok(())
            })),
            &mut token,
        )?;
        let source = proxy.clone();
        webview.add_SourceChanged(
            &SourceChangedEventHandler::create(Box::new(move |view, args| {
                if let (Some(view), Some(args)) = (view, args) {
                    let mut new_document = BOOL::default();
                    args.IsNewDocument(&mut new_document)?;
                    if !new_document.as_bool() {
                        let mut uri = PWSTR::null();
                        view.Source(&mut uri)?;
                        let _ =
                            source.send_event(UserEvent::Source(id, generation, take_pwstr(uri)));
                    }
                }
                Ok(())
            })),
            &mut token,
        )?;
        let navigations =
            std::rc::Rc::new(std::cell::RefCell::new(std::collections::HashMap::new()));
        let starting = navigations.clone();
        let started = proxy.clone();
        webview.add_NavigationStarting(
            &NavigationStartingEventHandler::create(Box::new(move |_, args| {
                if let Some(args) = args {
                    let mut navigation_id = 0;
                    let mut uri = PWSTR::null();
                    args.NavigationId(&mut navigation_id)?;
                    args.Uri(&mut uri)?;
                    let uri = take_pwstr(uri);
                    starting.borrow_mut().insert(navigation_id, uri.clone());
                    let _ = started.send_event(UserEvent::PageLoad(id, generation, true, uri));
                }
                Ok(())
            })),
            &mut token,
        )?;
        let failed = proxy.clone();
        webview.add_NavigationCompleted(&NavigationCompletedEventHandler::create(Box::new(move |_, args| {
            if let Some(args) = args {
                let mut navigation_id = 0;
                args.NavigationId(&mut navigation_id)?;
                let uri = navigations.borrow_mut().remove(&navigation_id).unwrap_or_default();
                let mut success = BOOL::default();
                args.IsSuccess(&mut success)?;
                if success.as_bool() {
                    let _ = failed.send_event(UserEvent::PageLoad(id, generation, false, uri));
                } else {
                    let mut status = COREWEBVIEW2_WEB_ERROR_STATUS::default();
                    args.WebErrorStatus(&mut status)?;
                    if status != COREWEBVIEW2_WEB_ERROR_STATUS_OPERATION_CANCELED {
                        let _ = failed.send_event(UserEvent::Failed(id, generation, uri, "This page could not be reached. Check your connection or the address, then try again.".into()));
                    }
                }
            }
            Ok(())
        })), &mut token)?;
        let crashed = proxy.clone();
        webview.add_ProcessFailed(
            &ProcessFailedEventHandler::create(Box::new(move |_, _| {
                let _ = crashed.send_event(UserEvent::Crashed(id, generation));
                Ok(())
            })),
            &mut token,
        )?;
        webview.add_PermissionRequested(
            &PermissionRequestedEventHandler::create(Box::new(move |_, args| {
                if let Some(args) = args {
                    let mut uri = PWSTR::null();
                    args.Uri(&mut uri)?;
                    let uri = take_pwstr(uri);
                    let origin = url::Url::parse(&uri)
                        .map(|url| url.origin().ascii_serialization())
                        .unwrap_or_else(|_| "This page".into());
                    let mut kind = COREWEBVIEW2_PERMISSION_KIND::default();
                    args.PermissionKind(&mut kind)?;
                    let permission = match kind {
                        COREWEBVIEW2_PERMISSION_KIND_CAMERA => "use your camera",
                        COREWEBVIEW2_PERMISSION_KIND_MICROPHONE => "use your microphone",
                        COREWEBVIEW2_PERMISSION_KIND_GEOLOCATION => "access your location",
                        COREWEBVIEW2_PERMISSION_KIND_NOTIFICATIONS => "show notifications",
                        COREWEBVIEW2_PERMISSION_KIND_CLIPBOARD_READ => "read your clipboard",
                        _ => {
                            args.SetState(COREWEBVIEW2_PERMISSION_STATE_DENY)?;
                            return Ok(());
                        }
                    };
                    let allow = message(
                        "Site permission",
                        &format!("{origin} wants to {permission}.\n\nAllow this request?"),
                        true,
                    );
                    args.SetState(if allow {
                        COREWEBVIEW2_PERMISSION_STATE_ALLOW
                    } else {
                        COREWEBVIEW2_PERMISSION_STATE_DENY
                    })?;
                    if let Ok(args) = args.cast::<ICoreWebView2PermissionRequestedEventArgs3>() {
                        args.SetSavesInProfile(false)?;
                    }
                }
                Ok(())
            })),
            &mut token,
        )?;
        let download_proxy = proxy.clone();
        webview.cast::<ICoreWebView2_4>()?.add_DownloadStarting(
            &DownloadStartingEventHandler::create(Box::new(move |_, args| {
                if let Some(args) = args {
                    let operation = args.DownloadOperation()?;
                    let _ = download_proxy.send_event(UserEvent::DownloadStarted(id));
                    let complete = download_proxy.clone();
                    let done = std::rc::Rc::new(std::cell::Cell::new(false));
                    operation.add_StateChanged(
                        &StateChangedEventHandler::create(Box::new(move |operation, _| {
                            if let Some(operation) = operation {
                                let mut state = COREWEBVIEW2_DOWNLOAD_STATE::default();
                                operation.State(&mut state)?;
                                if state != COREWEBVIEW2_DOWNLOAD_STATE_IN_PROGRESS
                                    && !done.replace(true)
                                {
                                    let _ = complete.send_event(UserEvent::DownloadFinished(
                                        id,
                                        state == COREWEBVIEW2_DOWNLOAD_STATE_COMPLETED,
                                    ));
                                }
                            }
                            Ok(())
                        })),
                        &mut 0,
                    )?;
                }
                Ok(())
            })),
            &mut token,
        )?;
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn attach(_: &WebView, _: usize, _: u64, _: EventLoopProxy<UserEvent>) -> wry::Result<()> {
    Ok(())
}

pub fn downloads(view: &WebView) -> Result<(), String> {
    #[cfg(windows)]
    {
        use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2_9;
        use windows::core::Interface;
        use wry::WebViewExtWindows;
        unsafe {
            view.webview()
                .cast::<ICoreWebView2_9>()
                .and_then(|view| view.OpenDefaultDownloadDialog())
                .map_err(|error| error.to_string())
        }
    }
    #[cfg(not(windows))]
    {
        let _ = view;
        Err("Downloads are available on Windows.".into())
    }
}

pub fn navigate(view: &WebView, backwards: bool) {
    #[cfg(windows)]
    {
        use wry::WebViewExtWindows;
        unsafe {
            let _ = if backwards {
                view.webview().GoBack()
            } else {
                view.webview().GoForward()
            };
        }
    }
    #[cfg(not(windows))]
    {
        let _ = view.evaluate_script(if backwards {
            "history.back()"
        } else {
            "history.forward()"
        });
    }
}
