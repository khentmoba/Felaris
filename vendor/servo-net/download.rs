/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

//! Saves top-level navigation responses that are downloads to a file chosen by the embedder.
//!
//! The navigation itself is answered with an empty `204 No Content` response, which makes
//! the script thread abort it and keep the current document. The original response body,
//! fetched with the page's cookies and request body, is then written to the file.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

use content_security_policy::percent_encoding::percent_decode_str;
use http::StatusCode;
use http::header::{CONTENT_DISPOSITION, CONTENT_LENGTH};
use net_traits::fetch::headers::extract_mime_type_as_mime;
use net_traits::mime_classifier::{MediaType, MimeClassifier};
use net_traits::request::{Destination, Request, RequestMode};
use net_traits::response::{Response, ResponseBody};
use net_traits::{NetworkError, ResourceFetchTiming, ResourceTimingType};
use servo_url::ServoUrl;

use crate::embedder::NetToEmbedderMsg;
use crate::fetch::methods::{Data, DoneChannel, FetchContext, Target};

/// What the embedder is told about a download before choosing where to save it.
#[derive(Clone, Debug)]
pub struct DownloadDetails {
    pub url: ServoUrl,
    /// The file name from `Content-Disposition`, or else from the URL. It is not sanitized.
    pub suggested_filename: String,
    pub mime_type: Option<String>,
    pub content_length: Option<u64>,
    pub cancelled: Arc<DownloadCancellation>,
}

/// Whether `response` to a top-level navigation should be saved instead of displayed: it is
/// sent as an attachment, or Servo cannot display its content type.
pub(crate) fn is_download(request: &Request, response: &Response) -> bool {
    if request.mode != RequestMode::Navigate
        || request.destination != Destination::Document
        || request.target_webview_id.is_none()
        || response.is_network_error()
        || !response.status.in_range(200..=299)
        || response.status.in_range(204..=205)
    {
        return false;
    }
    let headers = &response.actual_response().headers;
    let attachment = headers
        .get(CONTENT_DISPOSITION)
        .map(|value| String::from_utf8_lossy(value.as_bytes()).into_owned())
        .is_some_and(|value| {
            value
                .split(';')
                .next()
                .is_some_and(|kind| kind.trim().eq_ignore_ascii_case("attachment"))
        });
    // A missing or explicitly unknown type is sniffed by the parser, so only a declared type
    // that Servo has no viewer for counts.
    let undisplayable = extract_mime_type_as_mime(headers).is_some_and(|mime| {
        !matches!(
            mime.essence_str(),
            "unknown/unknown" | "application/unknown" | "*/*"
        ) && matches!(
            MimeClassifier::get_media_type(&mime),
            None | Some(MediaType::Font)
        )
    });
    attachment || undisplayable
}

fn details(request: &Request, response: &Response) -> DownloadDetails {
    let headers = &response.actual_response().headers;
    let url = response
        .actual_response()
        .url()
        .cloned()
        .unwrap_or_else(|| request.current_url());
    let disposition = headers
        .get(CONTENT_DISPOSITION)
        .map(|value| String::from_utf8_lossy(value.as_bytes()).into_owned());
    let suggested_filename = disposition
        .as_deref()
        .and_then(disposition_filename)
        .or_else(|| {
            url.path_segments()
                .and_then(|mut segments| segments.next_back())
                .map(|segment| percent_decode_str(segment).decode_utf8_lossy().into_owned())
                .filter(|name| !name.is_empty())
        })
        .unwrap_or_else(|| "download".into());
    DownloadDetails {
        url,
        suggested_filename,
        mime_type: extract_mime_type_as_mime(headers).map(|mime| mime.essence_str().to_owned()),
        content_length: headers
            .get(CONTENT_LENGTH)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse().ok()),
        cancelled: Arc::new(DownloadCancellation::default()),
    }
}

/// The `filename*` (RFC 8187, UTF-8 only) or `filename` parameter of a `Content-Disposition`
/// value. `filename*` wins when both are present.
fn disposition_filename(value: &str) -> Option<String> {
    let mut plain = None;
    let mut extended = None;
    for parameter in value.split(';').skip(1) {
        let Some((name, value)) = parameter.split_once('=') else {
            continue;
        };
        let value = value.trim();
        match name.trim().to_ascii_lowercase().as_str() {
            "filename*" => {
                let mut parts = value.splitn(3, '\'');
                if let (Some(charset), Some(_language), Some(encoded)) =
                    (parts.next(), parts.next(), parts.next())
                    && charset.eq_ignore_ascii_case("utf-8")
                {
                    extended = percent_decode_str(encoded)
                        .decode_utf8()
                        .ok()
                        .map(|name| name.into_owned());
                }
            }
            "filename" => {
                plain = Some(
                    value
                        .strip_prefix('"')
                        .and_then(|value| value.strip_suffix('"'))
                        .unwrap_or(value)
                        .replace("\\\"", "\""),
                );
            }
            _ => {}
        }
    }
    extended.or(plain).filter(|name| !name.trim().is_empty())
}

/// Cancellation and successful completion share one terminal-state transition.
#[derive(Debug, Default)]
pub struct DownloadCancellation {
    state: AtomicU8,
    notification: tokio::sync::Notify,
}

impl DownloadCancellation {
    pub fn cancel(&self) {
        if self
            .state
            .compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            self.notification.notify_one();
        }
    }

    async fn wait(&self) {
        loop {
            let notified = self.notification.notified();
            if self.state.load(Ordering::Acquire) == 1 {
                return;
            }
            notified.await;
        }
    }

    fn complete(&self) -> Result<(), String> {
        self.state
            .compare_exchange(0, 2, Ordering::AcqRel, Ordering::Acquire)
            .map(|_| ())
            .map_err(|_| "The download was cancelled.".into())
    }
}

/// An already-created file. The writer never reopens or removes its path.
pub struct DownloadDestination {
    pub path: PathBuf,
    file: Option<File>,
    error: Option<String>,
    completed: bool,
}

impl DownloadDestination {
    pub fn new(path: PathBuf, file: std::io::Result<File>) -> Self {
        let (file, error) = match file {
            Ok(file) => (Some(file), None),
            Err(error) => (None, Some(error.to_string())),
        };
        Self {
            path,
            file,
            error,
            completed: false,
        }
    }
}

#[allow(unsafe_code)]
impl Drop for DownloadDestination {
    fn drop(&mut self) {
        if self.completed {
            return;
        }
        // Delete the opened file itself, never a potentially replaced path.
        #[cfg(windows)]
        if let Some(file) = self.file.as_ref() {
            use std::os::windows::io::AsRawHandle;
            #[link(name = "kernel32")]
            unsafe extern "system" {
                fn SetFileInformationByHandle(
                    file: *mut std::ffi::c_void,
                    class: i32,
                    information: *const std::ffi::c_void,
                    size: u32,
                ) -> i32;
            }
            let delete: u8 = 1;
            // The borrowed File owns this live handle; `delete` remains valid for the
            // synchronous call and has FILE_DISPOSITION_INFO's BOOLEAN layout.
            let removed = unsafe {
                SetFileInformationByHandle(
                    file.as_raw_handle(),
                    4,
                    (&delete as *const u8).cast(),
                    1,
                )
            };
            if removed == 0 {
                log::warn!(
                    "Could not remove failed download: {}",
                    std::io::Error::last_os_error()
                );
            }
        }
    }
}

/// Reads a download without keeping a response body or a DevTools copy.
async fn write_body(
    url: &ServoUrl,
    response: &Response,
    done_chan: &mut DoneChannel,
    context: &FetchContext,
    cancelled: &DownloadCancellation,
    file: &File,
) -> Result<(), String> {
    let mut writer = BufWriter::new(file);
    let result = async {
        #[cfg(windows)]
        mark_from_internet(file, url.as_str()).map_err(|error| error.to_string())?;
        #[cfg(not(windows))]
        let _ = url;
        if let Some((sender, mut receiver)) = done_chan.take() {
            drop(sender);
            loop {
                let data = tokio::select! {
                    biased;
                    _ = cancelled.wait() => return Err("The download was cancelled.".into()),
                    _ = context.cancellation_listener.wait_for_cancellation() => {
                        return Err("The download was cancelled.".into());
                    },
                    data = receiver.recv() => data,
                };
                match data {
                    Some(Data::Payload(bytes)) => {
                        writer.write_all(&bytes).map_err(|e| e.to_string())?
                    }
                    Some(Data::DownloadPayload(bytes, _permit)) => {
                        writer.write_all(&bytes).map_err(|e| e.to_string())?;
                    }
                    Some(Data::ContentLength(_)) => {}
                    Some(Data::Done) => break,
                    Some(Data::Cancelled) => return Err("The download was cancelled.".into()),
                    Some(Data::Error(error)) => return Err(format!("{error:?}")),
                    None => return Err("The download stream ended before completion.".into()),
                }
            }
        } else {
            match &*response.actual_response().body.lock() {
                ResponseBody::Done(bytes) => writer.write_all(bytes).map_err(|e| e.to_string())?,
                ResponseBody::Empty => {}
                _ => return Err("The download body is not complete.".into()),
            }
        }
        writer.flush().map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        if response.is_network_error() {
            return Err(format!("{:?}", response.get_network_error()));
        }
        if response.aborted.load(Ordering::Acquire) || context.cancellation_listener.cancelled() {
            return Err("The download was cancelled.".into());
        }
        cancelled.complete()
    }
    .await;
    // Do not retry buffered writes when dropping a failed or cancelled writer.
    let _ = writer.into_parts();
    if result.is_err() {
        context.cancellation_listener.cancel();
    }
    result
}

#[cfg(windows)]
#[allow(unsafe_code)]
fn mark_from_internet(file: &File, url: &str) -> std::io::Result<()> {
    use std::os::windows::io::AsRawHandle;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn BackupWrite(
            file: *mut std::ffi::c_void,
            buffer: *const u8,
            size: u32,
            written: *mut u32,
            abort: i32,
            process_security: i32,
            context: *mut *mut std::ffi::c_void,
        ) -> i32;
    }
    let name: Vec<u8> = ":Zone.Identifier:$DATA"
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect();
    let data = format!("[ZoneTransfer]\r\nZoneId=3\r\nHostUrl={url}\r\n");
    // WIN32_STREAM_ID has a 20-byte fixed header before the UTF-16 stream name.
    let mut stream = Vec::new();
    stream.extend_from_slice(&4u32.to_le_bytes()); // BACKUP_ALTERNATE_DATA
    stream.extend_from_slice(&0u32.to_le_bytes());
    stream.extend_from_slice(&(data.len() as i64).to_le_bytes());
    stream.extend_from_slice(&(name.len() as u32).to_le_bytes());
    stream.extend_from_slice(&name);
    stream.extend_from_slice(data.as_bytes());
    let size = u32::try_from(stream.len())
        .map_err(|_| std::io::Error::other("Download URL is too long."))?;
    let mut context = std::ptr::null_mut();
    let mut written = 0;
    // File owns the handle for both calls. The stream, written count and backup context
    // remain live throughout BackupWrite; the final call releases its context.
    let success = unsafe {
        BackupWrite(
            file.as_raw_handle(),
            stream.as_ptr(),
            size,
            &mut written,
            0,
            0,
            &mut context,
        )
    };
    let result = if success == 0 {
        Err(std::io::Error::last_os_error())
    } else if written != size {
        Err(std::io::Error::new(
            std::io::ErrorKind::WriteZero,
            "Could not mark the downloaded file.",
        ))
    } else {
        Ok(())
    };
    unsafe {
        BackupWrite(
            file.as_raw_handle(),
            std::ptr::null(),
            0,
            &mut written,
            1,
            0,
            &mut context,
        );
    }
    result
}

/// Asks the embedder where to save a download, ends the navigation without replacing the
/// page, and writes the body to the chosen file. Declining discards the response.
pub(crate) async fn divert(
    request: &Request,
    response: &mut Response,
    target: Target<'_>,
    done_chan: &mut DoneChannel,
    context: &FetchContext,
) {
    let Some(webview_id) = request.target_webview_id else {
        return;
    };
    let details = details(request, response);
    let cancelled = details.cancelled.clone();
    let url = details.url.clone();
    let (sender, receiver) = tokio::sync::oneshot::channel();
    context
        .state
        .embedder_proxy
        .send(NetToEmbedderMsg::RequestDownload(
            webview_id, details, sender,
        ));
    let destination = receiver.await.ok().flatten();

    let mut no_content = Response::new(
        request.current_url(),
        ResourceFetchTiming::new(ResourceTimingType::Navigation),
    );
    no_content.status = StatusCode::NO_CONTENT.into();
    *no_content.body.lock() = ResponseBody::Empty;
    target.process_response(request, &no_content);
    target.process_response_eof(request, &no_content);

    let Some(mut destination) = destination else {
        context.cancellation_listener.cancel();
        *done_chan = None;
        return;
    };
    let result = match destination.file.as_ref() {
        Some(file) => write_body(&url, response, done_chan, context, &cancelled, file).await,
        None => Err(destination
            .error
            .take()
            .unwrap_or_else(|| "No download file was provided.".into())),
    };
    if result.is_err() {
        context.cancellation_listener.cancel();
        *done_chan = None;
        response.set_network_error(NetworkError::LoadCancelled);
    }
    destination.completed = result.is_ok();
    let path = destination.path.clone();
    drop(destination);
    context
        .state
        .embedder_proxy
        .send(NetToEmbedderMsg::DownloadFinished(webview_id, path, result));
}

#[cfg(test)]
mod tests {
    use super::{DownloadCancellation, disposition_filename};

    #[test]
    fn cancellation_and_completion_have_one_winner() {
        let cancelled = DownloadCancellation::default();
        cancelled.cancel();
        assert!(cancelled.complete().is_err());

        let completed = DownloadCancellation::default();
        assert!(completed.complete().is_ok());
        completed.cancel();
        assert_eq!(
            completed.state.load(std::sync::atomic::Ordering::Acquire),
            2
        );
    }

    #[test]
    fn cancellation_wakes_a_stalled_consumer() {
        use std::future::Future;
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::task::{Context, Poll, Wake, Waker};

        #[derive(Default)]
        struct WakeCount(AtomicUsize);
        impl Wake for WakeCount {
            fn wake(self: Arc<Self>) {
                self.0.fetch_add(1, Ordering::Relaxed);
            }
        }
        let cancelled = DownloadCancellation::default();
        let count = Arc::new(WakeCount::default());
        let waker = Waker::from(count.clone());
        let mut context = Context::from_waker(&waker);
        let mut waiting = Box::pin(cancelled.wait());
        assert_eq!(waiting.as_mut().poll(&mut context), Poll::Pending);
        cancelled.cancel();
        assert!(count.0.load(Ordering::Relaxed) > 0);
        assert_eq!(waiting.as_mut().poll(&mut context), Poll::Ready(()));
        assert_eq!(
            Box::pin(cancelled.wait()).as_mut().poll(&mut context),
            Poll::Ready(())
        );
    }

    #[test]
    fn reads_content_disposition_filenames() {
        assert_eq!(
            disposition_filename("attachment; filename=\"report.pdf\"").as_deref(),
            Some("report.pdf")
        );
        assert_eq!(
            disposition_filename("attachment; filename=plain.txt").as_deref(),
            Some("plain.txt")
        );
        assert_eq!(
            disposition_filename(
                "attachment; filename=\"fallback.txt\"; filename*=UTF-8''na%C3%AFve%20file.txt"
            )
            .as_deref(),
            Some("naïve file.txt")
        );
        assert_eq!(disposition_filename("attachment"), None);
    }
}
