/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::path::PathBuf;

use cookie::Cookie;
use embedder_traits::{
    AuthenticationResponse, EmbedderControlId, FilePickerRequest, WebResourceRequest,
    WebResourceResponseMsg,
};
use net_traits::CookieOperationId;
use servo_base::id::WebViewId;
use servo_url::ServoUrl;

use crate::download::{DownloadDestination, DownloadDetails};
use tokio::sync::mpsc::UnboundedSender as TokioSender;
use tokio::sync::oneshot::Sender as TokioOneshotSender;

/// Messages sent from the network threads to the embedder.
pub enum NetToEmbedderMsg {
    /// Open file dialog to select files. Set boolean flag to true allows to select multiple files.
    SelectFiles(
        EmbedderControlId,
        FilePickerRequest,
        TokioOneshotSender<Option<Vec<PathBuf>>>,
    ),
    WebResourceRequested(
        Option<WebViewId>,
        WebResourceRequest,
        TokioSender<WebResourceResponseMsg>,
    ),
    /// Request authentication for a load or navigation from the embedder.
    RequestAuthentication(
        WebViewId,
        ServoUrl,
        bool, /* for proxy */
        TokioOneshotSender<Option<AuthenticationResponse>>,
    ),
    /// Ask where to save a download from a top-level navigation. `None` discards it.
    RequestDownload(
        WebViewId,
        DownloadDetails,
        TokioOneshotSender<Option<DownloadDestination>>,
    ),
    /// A download accepted through `RequestDownload` finished. Failed files are cleaned up by handle.
    DownloadFinished(WebViewId, PathBuf, Result<(), String>),
    /// Response to an asynchronous request from SiteDataManager with cookies result.
    EmbedderCookieOperationResponseWithCookies(CookieOperationId, Vec<Cookie<'static>>),
    /// Response to an asynchronous request from SiteDataManager.
    EmbedderCookieOperationResponse(CookieOperationId),
}
