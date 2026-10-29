//! Native Windows UI for the form controls and dialogs Servo asks the embedder to show.
//! These run modal loops, so they are shown after `Servo::spin_event_loop` returns.

use super::{Shared, Surface};
use crate::app::{Command, UserEvent};
use servo::{
    AuthenticationRequest, ContextMenu, ContextMenuAction, ContextMenuItem, EmbedderControl,
    FilePicker, RgbColor, SelectElement, SelectElementOptionOrOptgroup, SimpleDialog, WebView,
};
use std::path::PathBuf;
use tao::dpi::PhysicalPosition;
use tao::platform::windows::WindowExtWindows;
use windows::{
    Win32::{
        Foundation::{COLORREF, HWND, LPARAM, POINT, WPARAM},
        Graphics::Gdi::ClientToScreen,
        System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance, CoTaskMemFree},
        UI::{
            Controls::Dialogs::{CC_FULLOPEN, CC_RGBINIT, CHOOSECOLORW, ChooseColorW},
            Shell::{
                Common::COMDLG_FILTERSPEC, FOS_ALLOWMULTISELECT, FOS_FILEMUSTEXIST, FileOpenDialog,
                IFileOpenDialog, SIGDN_FILESYSPATH,
            },
            WindowsAndMessaging::{
                AppendMenuW, BS_DEFPUSHBUTTON, BS_PUSHBUTTON, CreatePopupMenu, DLGTEMPLATE,
                DS_CENTER, DS_MODALFRAME, DS_SETFONT, DestroyMenu, DialogBoxIndirectParamW,
                ES_AUTOHSCROLL, ES_PASSWORD, EndDialog, GWLP_USERDATA, GetCursorPos, GetDlgItem,
                GetDlgItemTextW, GetWindowLongPtrW, GetWindowTextLengthW, HMENU, IDCANCEL, IDOK,
                MB_ICONINFORMATION, MB_ICONQUESTION, MB_OK, MB_OKCANCEL, MENU_ITEM_FLAGS,
                MF_CHECKED, MF_GRAYED, MF_SEPARATOR, MF_STRING, MessageBoxW, SetDlgItemTextW,
                SetWindowLongPtrW, TPM_LEFTALIGN, TPM_NONOTIFY, TPM_RETURNCMD, TPM_TOPALIGN,
                TrackPopupMenuEx, WM_COMMAND, WM_INITDIALOG, WS_BORDER, WS_CAPTION, WS_CHILD,
                WS_POPUP, WS_SYSMENU, WS_TABSTOP, WS_VISIBLE,
            },
        },
    },
    core::{HSTRING, PCWSTR},
};

pub(super) fn show(
    shared: &Shared,
    surface: &Surface,
    webview: &WebView,
    control: EmbedderControl,
) {
    match control {
        EmbedderControl::SelectElement(select) => select_option(surface, select),
        EmbedderControl::ContextMenu(menu) => context_menu(shared, surface, menu),
        EmbedderControl::SimpleDialog(dialog) => simple_dialog(surface, webview, dialog),
        EmbedderControl::FilePicker(picker) => pick_files(surface, picker),
        EmbedderControl::ColorPicker(mut picker) => {
            if let Some(color) = choose_color(surface, picker.current_color()) {
                picker.select(Some(color));
            }
            picker.submit();
        }
        // Keep the IME candidate window under the focused text field.
        EmbedderControl::InputMethod(input) => {
            let field = input.position();
            surface
                .window
                .set_ime_position(PhysicalPosition::new(field.min.x, field.max.y));
        }
    }
}

fn hwnd(surface: &Surface) -> HWND {
    HWND(surface.window.hwnd() as _)
}

fn owner(surface: &Surface) -> Option<HWND> {
    Some(HWND(surface.owner as _))
}

/// A popup menu that returns the chosen 1-based item index.
struct Menu(HMENU);

impl Menu {
    fn new() -> Option<Self> {
        unsafe { CreatePopupMenu() }.ok().map(Self)
    }

    fn add(&self, flags: MENU_ITEM_FLAGS, id: usize, label: &str) {
        let label = HSTRING::from(label);
        let text = if flags.contains(MF_SEPARATOR) {
            PCWSTR::null()
        } else {
            PCWSTR(label.as_ptr())
        };
        let _ = unsafe { AppendMenuW(self.0, flags, id, text) };
    }

    fn track(&self, surface: &Surface, at: POINT) -> Option<usize> {
        let flags = TPM_RETURNCMD | TPM_NONOTIFY | TPM_LEFTALIGN | TPM_TOPALIGN;
        let chosen = unsafe { TrackPopupMenuEx(self.0, flags.0, at.x, at.y, hwnd(surface), None) };
        usize::try_from(chosen.0).ok().filter(|id| *id > 0)
    }
}

impl Drop for Menu {
    fn drop(&mut self) {
        let _ = unsafe { DestroyMenu(self.0) };
    }
}

fn select_option(surface: &Surface, mut select: SelectElement) {
    let Some(menu) = Menu::new() else { return };
    let mut selected = select.selected_options();
    // Menu ids are option ids + 1; group labels are disabled entries.
    let add_option = |option: &servo::SelectElementOption, indent: &str| {
        let mut flags = MF_STRING;
        if option.is_disabled {
            flags |= MF_GRAYED;
        }
        if selected.contains(&option.id) {
            flags |= MF_CHECKED;
        }
        menu.add(flags, option.id + 1, &format!("{indent}{}", option.label));
    };
    for entry in select.options() {
        match entry {
            SelectElementOptionOrOptgroup::Option(option) => add_option(option, ""),
            SelectElementOptionOrOptgroup::Optgroup { label, options } => {
                menu.add(MF_STRING | MF_GRAYED, 0, label);
                for option in options {
                    add_option(option, "    ");
                }
            }
        }
    }
    let rect = select.position();
    let mut below = POINT {
        x: rect.min.x,
        y: rect.max.y,
    };
    let _ = unsafe { ClientToScreen(hwnd(surface), &mut below) };
    if let Some(id) = menu.track(surface, below).map(|id| id - 1) {
        if !select.allow_select_multiple() {
            selected.clear();
        }
        if let Some(index) = selected.iter().position(|chosen| *chosen == id) {
            selected.remove(index);
        } else {
            selected.push(id);
        }
        select.select(selected);
    }
    select.submit();
}

fn context_menu(shared: &Shared, surface: &Surface, menu: ContextMenu) {
    let Some(popup) = Menu::new() else { return };
    let mut actions = Vec::new();
    for item in menu.items() {
        match item {
            ContextMenuItem::Separator => popup.add(MF_SEPARATOR, 0, ""),
            ContextMenuItem::Item {
                label,
                action,
                enabled,
            } => {
                actions.push(*action);
                let flags = if *enabled {
                    MF_STRING
                } else {
                    MF_STRING | MF_GRAYED
                };
                let label = match action {
                    ContextMenuAction::OpenLinkInNewWebView => "Open Link in New Tab",
                    ContextMenuAction::OpenImageInNewView => "Open Image in New Tab",
                    _ => label,
                };
                popup.add(flags, actions.len(), label);
            }
        }
    }
    let mut at = POINT::default();
    let _ = unsafe { GetCursorPos(&mut at) };
    let Some(action) = popup.track(surface, at).map(|id| actions[id - 1]) else {
        return menu.dismiss();
    };
    // New views open as Felaris tabs rather than through Servo's popup path.
    let info = menu.element_info();
    let target = match action {
        ContextMenuAction::OpenLinkInNewWebView => info.link_url.clone(),
        ContextMenuAction::OpenImageInNewView => info.image_url.clone(),
        _ => return menu.select(action),
    };
    if let Some(url) = target.filter(|url| crate::profile::allowed_url(url.as_str())) {
        let _ = shared.proxy.send_event(UserEvent::Command(Command::NewTab {
            url: Some(url.into()),
        }));
    }
    menu.dismiss();
}

fn simple_dialog(surface: &Surface, webview: &WebView, dialog: SimpleDialog) {
    // Page text must not pass for browser UI, so the title names the page's host.
    let host = webview
        .url()
        .and_then(|url| url.host_str().map(str::to_owned))
        .unwrap_or_else(|| "This page".into());
    let title = format!("{host} says");
    let caption = HSTRING::from(&title);
    let message = |text: &str, style| unsafe {
        MessageBoxW(owner(surface), &HSTRING::from(text), &caption, style)
    };
    match dialog {
        SimpleDialog::Alert(alert) => {
            message(alert.message(), MB_OK | MB_ICONINFORMATION);
            alert.confirm();
        }
        SimpleDialog::Confirm(confirm) => {
            if message(confirm.message(), MB_OKCANCEL | MB_ICONQUESTION) == IDOK {
                confirm.confirm();
            } else {
                confirm.dismiss();
            }
        }
        SimpleDialog::Prompt(mut prompt) => {
            let mut field = [Field {
                label: "",
                value: prompt.current_value().to_owned(),
                secret: false,
            }];
            if form(surface, &title, prompt.message(), &mut field) {
                let [field] = field;
                prompt.set_current_value(&field.value);
                prompt.confirm();
            } else {
                prompt.dismiss();
            }
        }
    }
}

/// A text box in a [`form`] dialog.
struct Field<'a> {
    label: &'a str,
    value: String,
    secret: bool,
}

const FIRST_FIELD: i32 = 100;

/// Shows a modal dialog with a message, text boxes and OK/Cancel, built from an in-memory
/// template. Returns whether OK was chosen; the fields then hold what was typed.
fn form(surface: &Surface, title: &str, message: &str, fields: &mut [Field]) -> bool {
    let template = form_template(title, message, fields);
    let mut values = fields
        .iter()
        .map(|field| field.value.clone())
        .collect::<Vec<_>>();
    let result = unsafe {
        DialogBoxIndirectParamW(
            None,
            template.as_ptr().cast::<DLGTEMPLATE>(),
            owner(surface),
            Some(form_proc),
            LPARAM(&raw mut values as isize),
        )
    };
    if result != IDOK.0 as isize {
        return false;
    }
    for (field, value) in fields.iter_mut().zip(values) {
        field.value = value;
    }
    true
}

unsafe extern "system" fn form_proc(
    dialog: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> isize {
    unsafe {
        match message {
            WM_INITDIALOG => {
                SetWindowLongPtrW(dialog, GWLP_USERDATA, lparam.0);
                let values = &*(lparam.0 as *const Vec<String>);
                for (id, value) in (FIRST_FIELD..).zip(values) {
                    let _ = SetDlgItemTextW(dialog, id, &HSTRING::from(value.as_str()));
                }
                // The dialog manager focuses the first text box and selects its text.
                1
            }
            WM_COMMAND => {
                let id = (wparam.0 & 0xFFFF) as i32;
                if id == IDOK.0 {
                    let values =
                        &mut *(GetWindowLongPtrW(dialog, GWLP_USERDATA) as *mut Vec<String>);
                    for (id, value) in (FIRST_FIELD..).zip(values.iter_mut()) {
                        let Ok(edit) = GetDlgItem(Some(dialog), id) else {
                            continue;
                        };
                        let mut text = vec![0; GetWindowTextLengthW(edit) as usize + 1];
                        let copied = GetDlgItemTextW(dialog, id, &mut text) as usize;
                        *value = String::from_utf16_lossy(&text[..copied]);
                        text.fill(0);
                    }
                }
                if id == IDOK.0 || id == IDCANCEL.0 {
                    let _ = EndDialog(dialog, id as isize);
                    return 1;
                }
                0
            }
            _ => 0,
        }
    }
}

/// A `DLGTEMPLATE` for [`form`], laid out in dialog units.
fn form_template(title: &str, message: &str, fields: &[Field]) -> Vec<u32> {
    fn text(words: &mut Vec<u16>, value: &str) {
        words.extend(value.encode_utf16());
        words.push(0);
    }
    fn long(words: &mut Vec<u16>, value: u32) {
        words.extend([value as u16, (value >> 16) as u16]);
    }
    fn item(words: &mut Vec<u16>, style: u32, rect: [i16; 4], id: i32, class: u16, title: &str) {
        // Items start on a DWORD boundary.
        if words.len() % 2 == 1 {
            words.push(0);
        }
        long(words, (WS_CHILD | WS_VISIBLE).0 | style);
        long(words, 0);
        words.extend(rect.map(|unit| unit as u16));
        words.extend([id as u16, 0xFFFF, class]);
        text(words, title);
        words.push(0);
    }
    const BUTTON: u16 = 0x80;
    const EDIT: u16 = 0x81;
    const STATIC: u16 = 0x82;
    // SS_NOPREFIX: show `&` literally.
    const NO_PREFIX: u32 = 0x80;
    let mut items = Vec::new();
    // Static text wraps at about 55 characters.
    let lines = message
        .lines()
        .map(|line| line.chars().count() / 55 + 1)
        .sum::<usize>()
        .clamp(2, 8) as i16;
    item(
        &mut items,
        NO_PREFIX,
        [8, 8, 244, lines * 10],
        0,
        STATIC,
        message,
    );
    let mut y = 14 + lines * 10;
    for (id, field) in (FIRST_FIELD..).zip(fields) {
        if !field.label.is_empty() {
            item(
                &mut items,
                NO_PREFIX,
                [8, y, 244, 10],
                -1,
                STATIC,
                field.label,
            );
            y += 11;
        }
        let mut style = (WS_BORDER | WS_TABSTOP).0 | ES_AUTOHSCROLL as u32;
        if field.secret {
            style |= ES_PASSWORD as u32;
        }
        item(&mut items, style, [8, y, 244, 14], id, EDIT, "");
        y += 20;
    }
    y += 2;
    let ok = WS_TABSTOP.0 | BS_DEFPUSHBUTTON as u32;
    item(&mut items, ok, [148, y, 50, 14], IDOK.0, BUTTON, "OK");
    let cancel = WS_TABSTOP.0 | BS_PUSHBUTTON as u32;
    item(
        &mut items,
        cancel,
        [202, y, 50, 14],
        IDCANCEL.0,
        BUTTON,
        "Cancel",
    );
    let count = 3
        + fields.len()
        + fields
            .iter()
            .filter(|field| !field.label.is_empty())
            .count();
    let style =
        (WS_POPUP | WS_CAPTION | WS_SYSMENU).0 | (DS_MODALFRAME | DS_SETFONT | DS_CENTER) as u32;
    let mut words = Vec::new();
    long(&mut words, style);
    long(&mut words, 0);
    words.extend([count as i16, 0, 0, 260, y + 22].map(|value| value as u16));
    words.extend([0, 0]);
    text(&mut words, title);
    words.push(9);
    text(&mut words, "Segoe UI");
    if words.len() % 2 == 1 {
        words.push(0);
    }
    words.extend(items);
    if words.len() % 2 == 1 {
        words.push(0);
    }
    words
        .chunks(2)
        .map(|pair| u32::from(pair[0]) | u32::from(pair[1]) << 16)
        .collect()
}

/// Asks for HTTP credentials. Cancelling drops the request, which fails the sign-in.
pub(super) fn sign_in(surface: &Surface, request: AuthenticationRequest) {
    let url = request.url();
    let host = url.host_str().unwrap_or("This site");
    let message = if request.for_proxy() {
        format!("The proxy server {host} needs a username and password.")
    } else if url.scheme() == "https" {
        format!("{host} needs a username and password.")
    } else {
        format!(
            "{host} needs a username and password.\nThis connection is not secure, so others \
             may see what you send."
        )
    };
    let mut fields = [
        Field {
            label: "Username",
            value: String::new(),
            secret: false,
        },
        Field {
            label: "Password",
            value: String::new(),
            secret: true,
        },
    ];
    if form(
        surface,
        &format!("Sign in to {host}"),
        &message,
        &mut fields,
    ) {
        let [username, password] = fields.map(|field| field.value);
        request.authenticate(username, password);
    }
}

fn pick_files(surface: &Surface, mut picker: FilePicker) {
    let Some(paths) = open_dialog(surface, &picker) else {
        return picker.dismiss();
    };
    picker.select(&paths);
    picker.submit();
}

fn open_dialog(surface: &Surface, picker: &FilePicker) -> Option<Vec<PathBuf>> {
    unsafe {
        let dialog: IFileOpenDialog =
            CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER).ok()?;
        let mut options = dialog.GetOptions().ok()? | FOS_FILEMUSTEXIST;
        if picker.allow_select_multiple() {
            options |= FOS_ALLOWMULTISELECT;
        }
        dialog.SetOptions(options).ok()?;
        let spec = picker
            .filter_patterns()
            .iter()
            .map(|pattern| format!("*.{}", pattern.0.trim_start_matches(['*', '.'])))
            .collect::<Vec<_>>()
            .join(";");
        let (name, spec) = (HSTRING::from("Accepted files"), HSTRING::from(spec));
        if !spec.is_empty() {
            let filter = COMDLG_FILTERSPEC {
                pszName: PCWSTR(name.as_ptr()),
                pszSpec: PCWSTR(spec.as_ptr()),
            };
            dialog.SetFileTypes(&[filter]).ok()?;
        }
        dialog.Show(owner(surface)).ok()?;
        let results = dialog.GetResults().ok()?;
        let mut paths = Vec::new();
        for index in 0..results.GetCount().ok()? {
            let path = results
                .GetItemAt(index)
                .ok()?
                .GetDisplayName(SIGDN_FILESYSPATH)
                .ok()?;
            paths.push(PathBuf::from(path.to_string().unwrap_or_default()));
            CoTaskMemFree(Some(path.0 as _));
        }
        Some(paths)
    }
}

fn choose_color(surface: &Surface, current: Option<RgbColor>) -> Option<RgbColor> {
    let initial = current.map_or(0, |color| {
        u32::from(color.red) | u32::from(color.green) << 8 | u32::from(color.blue) << 16
    });
    let mut custom = [COLORREF(0x00FF_FFFF); 16];
    let mut request = CHOOSECOLORW {
        lStructSize: size_of::<CHOOSECOLORW>() as u32,
        hwndOwner: HWND(surface.owner as _),
        rgbResult: COLORREF(initial),
        lpCustColors: custom.as_mut_ptr(),
        Flags: CC_RGBINIT | CC_FULLOPEN,
        ..Default::default()
    };
    unsafe { ChooseColorW(&mut request) }.as_bool().then(|| {
        let [red, green, blue, _] = request.rgbResult.0.to_le_bytes();
        RgbColor { red, green, blue }
    })
}
