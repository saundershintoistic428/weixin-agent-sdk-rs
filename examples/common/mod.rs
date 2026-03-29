//! Shared utilities for examples: CLI args, QR login flow, state persistence.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use clap::Parser;
use weixin_agent::{LoginStatus, StandaloneQrLogin, WeixinConfig};

/// Common CLI arguments for example bots.
#[derive(Parser, Debug)]
pub struct BotArgs {
    /// State directory for persisting sync_buf, token, etc. (required)
    #[arg(long)]
    pub state_dir: PathBuf,

    /// Bot token (overrides saved token). Can also use WEIXIN_TOKEN env var.
    #[arg(long, env = "WEIXIN_TOKEN")]
    pub token: Option<String>,

    /// API base URL override.
    #[arg(long)]
    pub base_url: Option<String>,
}

// ── State file paths ────────────────────────────────────────────────

pub fn token_path(dir: &Path) -> PathBuf {
    dir.join("token.txt")
}
pub fn sync_buf_path(dir: &Path) -> PathBuf {
    dir.join("sync_buf.json")
}
pub fn context_tokens_path(dir: &Path) -> PathBuf {
    dir.join("context_tokens.json")
}

// ── Token resolution ────────────────────────────────────────────────

/// Resolve bot token: CLI arg / env > saved file > QR login.
pub async fn resolve_token(args: &BotArgs) -> anyhow::Result<String> {
    if let Some(t) = &args.token {
        return Ok(t.clone());
    }

    let tp = token_path(&args.state_dir);
    if tp.exists() {
        let saved = tokio::fs::read_to_string(&tp).await?.trim().to_owned();
        if !saved.is_empty() {
            tracing::info!(path = %tp.display(), "loaded saved token");
            return Ok(saved);
        }
    }

    tracing::info!("no token found, starting QR login...");
    qr_login(&args.state_dir, args.base_url.as_deref()).await
}

// ── QR login ────────────────────────────────────────────────────────

/// Interactive QR login: display QR in terminal, poll until confirmed, save token.
async fn qr_login(state_dir: &Path, base_url: Option<&str>) -> anyhow::Result<String> {
    let mut builder = WeixinConfig::builder().token("");
    if let Some(url) = base_url {
        builder = builder.base_url(url);
    }
    let config = builder.build()?;
    let qr = StandaloneQrLogin::new(&config);

    let mut session = qr.start(None).await?;
    print_qr(&session.qrcode_img_content);

    let mut refresh_count = 0u32;
    loop {
        match qr.poll_status(&session).await? {
            LoginStatus::Confirmed {
                bot_token,
                ilink_bot_id,
                base_url,
                ilink_user_id,
            } => {
                tracing::info!(bot_id = %ilink_bot_id, user_id = %ilink_user_id, base_url = %base_url, "login confirmed");
                tokio::fs::write(token_path(state_dir), &bot_token).await?;
                return Ok(bot_token);
            }
            LoginStatus::Scanned => {
                tracing::info!("scanned, waiting for confirmation...");
            }
            LoginStatus::Expired => {
                refresh_count += 1;
                if refresh_count >= 3 {
                    anyhow::bail!("QR code expired 3 times, giving up");
                }
                tracing::warn!("QR expired, refreshing ({refresh_count}/3)...");
                session = qr.start(None).await?;
                print_qr(&session.qrcode_img_content);
            }
            LoginStatus::Wait | LoginStatus::ScannedButRedirect { .. } => {}
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}

fn print_qr(content: &str) {
    println!("\n请使用微信扫描以下二维码登录:\n");
    if let Err(e) = qr2term::print_qr(content) {
        eprintln!("无法生成终端二维码: {e}");
        println!("请手动访问: {content}");
    }
    println!();
}

// ── State persistence helpers ───────────────────────────────────────

pub async fn load_sync_buf(state_dir: &Path) -> Option<String> {
    tokio::fs::read_to_string(sync_buf_path(state_dir))
        .await
        .ok()
}

pub async fn load_context_tokens(state_dir: &Path) -> HashMap<String, String> {
    let Ok(data) = tokio::fs::read_to_string(context_tokens_path(state_dir)).await else {
        return HashMap::new();
    };
    serde_json::from_str(&data).unwrap_or_default()
}

pub async fn save_context_tokens(
    state_dir: &Path,
    tokens: &HashMap<String, String>,
) -> anyhow::Result<()> {
    tokio::fs::write(
        context_tokens_path(state_dir),
        serde_json::to_string(tokens)?,
    )
    .await?;
    Ok(())
}
