//! Text message construction and sending.

use std::sync::Arc;

use crate::api::client::HttpApiClient;
use crate::error::Result;
use crate::messaging::inbound::SendResult;
use crate::types::{
    MessageItem, MessageItemType, MessageState, MessageType, SendMessageRequest, TextItem,
    WeixinMessage, build_base_info,
};
use crate::util::random::generate_id;

/// Generate a client ID for outbound messages.
pub fn generate_client_id() -> String {
    generate_id("weixin-agent")
}

/// Build a `SendMessageRequest` for a text message.
pub fn build_text_message(to: &str, text: &str, context_token: Option<&str>) -> SendMessageRequest {
    let item_list = if text.is_empty() {
        None
    } else {
        Some(vec![MessageItem {
            item_type: Some(MessageItemType::Text),
            text_item: Some(TextItem {
                text: Some(text.to_owned()),
            }),
            ..Default::default()
        }])
    };

    SendMessageRequest {
        msg: WeixinMessage {
            from_user_id: Some(String::new()),
            to_user_id: Some(to.to_owned()),
            client_id: Some(generate_client_id()),
            message_type: Some(MessageType::Bot),
            message_state: Some(MessageState::Finish),
            item_list,
            context_token: context_token.map(String::from),
            ..Default::default()
        },
        base_info: build_base_info(),
    }
}

/// Send a text message and return the client ID.
pub(crate) async fn send_text(
    api: &Arc<HttpApiClient>,
    to: &str,
    text: &str,
    context_token: Option<&str>,
) -> Result<SendResult> {
    let req = build_text_message(to, text, context_token);
    let message_id = req.msg.client_id.clone().unwrap_or_default();
    api.send_message(&req).await?;
    Ok(SendResult { message_id })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_text_message_structure() {
        let req = build_text_message("user123", "hi", None);
        let msg = &req.msg;
        assert_eq!(msg.to_user_id.as_deref(), Some("user123"));
        assert_eq!(msg.message_type, Some(MessageType::Bot));
        assert_eq!(msg.message_state, Some(MessageState::Finish));
        let items = msg.item_list.as_ref().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].item_type, Some(MessageItemType::Text));
        assert_eq!(
            items[0].text_item.as_ref().unwrap().text.as_deref(),
            Some("hi")
        );
    }

    #[test]
    fn build_text_message_empty_text() {
        let req = build_text_message("user123", "", None);
        assert!(req.msg.item_list.is_none());
    }

    #[test]
    fn build_text_message_with_context_token() {
        let req = build_text_message("u", "t", Some("ctx_tok"));
        assert_eq!(req.msg.context_token.as_deref(), Some("ctx_tok"));
    }

    #[test]
    fn generate_client_id_format() {
        let id = generate_client_id();
        assert!(id.starts_with("weixin-agent:"));
    }
}
