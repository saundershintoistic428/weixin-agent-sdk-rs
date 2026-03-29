//! Echo bot example — demonstrates SDK usage with sync_buf persistence.
//!
//! Usage: `WEIXIN_TOKEN=xxx cargo run --example echo_bot`

use std::path::PathBuf;

use weixin_agent::{MessageContext, MessageHandler, Result, WeixinClient, WeixinConfig};

struct EchoBotHandler {
    state_dir: PathBuf,
}

#[async_trait::async_trait]
impl MessageHandler for EchoBotHandler {
    async fn on_message(&self, ctx: &MessageContext) -> Result<()> {
        if let Some(text) = &ctx.body {
            tracing::info!(from = %ctx.from, text = %text, "received message");
            ctx.reply_text(text).await?;
        }
        Ok(())
    }

    async fn on_sync_buf_updated(&self, sync_buf: &str) -> Result<()> {
        let path = self.state_dir.join("sync_buf.json");
        tokio::fs::write(&path, sync_buf).await?;
        tracing::debug!(path = ?path, "sync_buf persisted");
        Ok(())
    }

    async fn on_start(&self) -> Result<()> {
        tracing::info!("echo bot starting");
        Ok(())
    }

    async fn on_shutdown(&self) -> Result<()> {
        tracing::info!("echo bot shutting down");
        Ok(())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let token = std::env::var("WEIXIN_TOKEN").expect("WEIXIN_TOKEN env var required");
    let state_dir =
        PathBuf::from(std::env::var("STATE_DIR").unwrap_or_else(|_| "/tmp/echo_bot".into()));
    tokio::fs::create_dir_all(&state_dir).await?;

    // Load previous sync_buf if available
    let sync_buf_path = state_dir.join("sync_buf.json");
    let initial_sync_buf = tokio::fs::read_to_string(&sync_buf_path).await.ok();

    let config = WeixinConfig::builder().token(token).build()?;

    let client = WeixinClient::builder(config)
        .on_message(EchoBotHandler {
            state_dir: state_dir.clone(),
        })
        .build()?;

    client.start(initial_sync_buf).await?;
    Ok(())
}
