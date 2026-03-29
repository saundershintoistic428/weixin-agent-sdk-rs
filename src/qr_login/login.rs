//! QR code login API — only HTTP calls, no credential persistence.

use std::time::Duration;

use crate::api::client::HttpApiClient;
use crate::error::Result;
use crate::types::{
    DEFAULT_ILINK_BOT_TYPE, DEFAULT_QR_GET_TIMEOUT_MS, DEFAULT_QR_POLL_TIMEOUT_MS, QrCodeResponse,
    QrStatusResponse,
};

/// QR login session returned by [`QrLoginApi::start`].
#[derive(Debug, Clone)]
pub struct QrLoginSession {
    /// QR code token string.
    pub qrcode: String,
    /// QR code image URL.
    pub qrcode_img_content: String,
}

/// Login status returned by [`QrLoginApi::poll_status`].
#[derive(Debug, Clone)]
pub enum LoginStatus {
    /// Waiting for scan.
    Wait,
    /// QR code scanned, awaiting confirmation.
    Scanned,
    /// Scanned but needs IDC redirect.
    ScannedButRedirect {
        /// New host to redirect polling to.
        redirect_host: String,
    },
    /// Login confirmed.
    Confirmed {
        /// Bot authentication token.
        bot_token: String,
        /// Bot ID.
        ilink_bot_id: String,
        /// API base URL.
        base_url: String,
        /// User ID of the person who scanned.
        ilink_user_id: String,
    },
    /// QR code expired.
    Expired,
}

/// QR login API wrapper.
pub struct QrLoginApi<'a> {
    api: &'a HttpApiClient,
}

impl<'a> QrLoginApi<'a> {
    /// Create a new QR login API handle.
    pub(crate) fn new(api: &'a HttpApiClient) -> Self {
        Self { api }
    }

    /// Fetch a new QR code. `bot_type` defaults to `"3"`.
    pub async fn start(&self, bot_type: Option<&str>) -> Result<QrLoginSession> {
        let bt = bot_type.unwrap_or(DEFAULT_ILINK_BOT_TYPE);
        let endpoint = format!(
            "ilink/bot/get_bot_qrcode?bot_type={}",
            urlencoding::encode(bt)
        );
        let raw = self
            .api
            .api_get(&endpoint, Duration::from_millis(DEFAULT_QR_GET_TIMEOUT_MS))
            .await?;
        let resp: QrCodeResponse = serde_json::from_str(&raw)?;
        Ok(QrLoginSession {
            qrcode: resp.qrcode,
            qrcode_img_content: resp.qrcode_img_content,
        })
    }

    /// Poll the login status for a QR session.
    pub async fn poll_status(&self, session: &QrLoginSession) -> Result<LoginStatus> {
        let endpoint = format!(
            "ilink/bot/get_qrcode_status?qrcode={}",
            urlencoding::encode(&session.qrcode)
        );
        let raw = match self
            .api
            .api_get(&endpoint, Duration::from_millis(DEFAULT_QR_POLL_TIMEOUT_MS))
            .await
        {
            Ok(r) => r,
            Err(crate::error::Error::Http(e)) if e.is_timeout() => {
                return Ok(LoginStatus::Wait);
            }
            Err(e) => return Err(e),
        };

        let resp: QrStatusResponse = serde_json::from_str(&raw)?;
        Ok(match resp.status.as_str() {
            "scaned" => LoginStatus::Scanned,
            "scaned_but_redirect" => LoginStatus::ScannedButRedirect {
                redirect_host: resp.redirect_host.unwrap_or_default(),
            },
            "confirmed" => LoginStatus::Confirmed {
                bot_token: resp.bot_token.unwrap_or_default(),
                ilink_bot_id: resp.ilink_bot_id.unwrap_or_default(),
                base_url: resp.baseurl.unwrap_or_default(),
                ilink_user_id: resp.ilink_user_id.unwrap_or_default(),
            },
            "expired" => LoginStatus::Expired,
            _ => LoginStatus::Wait,
        })
    }
}
