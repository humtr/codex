//! One saved public Chat turn, observed without another primary model request.
//! Source IDs and branch ancestry, never matching prose, identify a follow-up.
use crate::{app::Bridge, chats::Chats, context::digest, protocol::Error};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Input {
    pub conversation: String,
    pub message: String,
    pub fingerprint: String,
}
impl Input {
    pub fn validate(&self) -> Result<(), Error> {
        if [&self.conversation, &self.message]
            .into_iter()
            .any(|id| id.is_empty() || id.len() > 256 || id.chars().any(char::is_control))
            || self.fingerprint.len() != 64
            || !self
                .fingerprint
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(Error("bound_chat_input_invalid"));
        }
        Ok(())
    }
    pub fn call_id(&self) -> String {
        format!(
            "chat-source:{}",
            digest(&(&self.conversation, &self.message))
        )
    }
}
pub(crate) struct Turn {
    pub input: Input,
    pub text: String,
    pub messages: Vec<Value>,
    pub ended: bool,
}

pub(crate) async fn read(
    bridge: Arc<dyn Bridge>,
    conversation: &str,
    anchor: &str,
    input: Option<&Input>,
) -> Result<Option<Turn>, Error> {
    let source = Chats::new(bridge);
    let mut after = Value::Null;
    let mut messages = Vec::new();
    let mut bytes = 0;
    loop {
        let mut args = json!({"action":"read","conversation_id":conversation,"limit":200});
        if !after.is_null() {
            args["after"] = after.clone();
        }
        let page = source.read_source(args).await?;
        let items = page["messages"]
            .as_array()
            .ok_or(Error("app_chat_response_invalid"))?;
        bytes += serde_json::to_vec(items).unwrap().len();
        if bytes > crate::config::MAX_DECODED_BYTES as usize {
            return Err(Error("bound_chat_history_read_bound"));
        }
        messages.extend_from_slice(items);
        if page["complete"] == true {
            break;
        }
        if page["cursor"] == after || items.is_empty() {
            return Err(Error("chat_pagination_incomplete"));
        }
        after = page["cursor"].clone();
    }
    select(conversation, anchor, input, &messages)
}

fn select(
    conversation: &str,
    anchor: &str,
    expected: Option<&Input>,
    messages: &[Value],
) -> Result<Option<Turn>, Error> {
    let anchor = messages
        .iter()
        .position(|message| message["id"] == anchor)
        .ok_or(Error("bound_chat_branch_changed"))?;
    let Some(start) = messages
        .iter()
        .enumerate()
        .skip(anchor + 1)
        .find_map(|(index, message)| (message["role"] == "user").then_some(index))
    else {
        return Ok(None);
    };
    let message = &messages[start];
    if !message["unavailable"].is_null() || message["content"]["content_type"] != "text" {
        return Err(Error("bound_chat_text_input_required"));
    }
    let text = message["content"]["parts"]
        .as_array()
        .ok_or(Error("app_chat_response_invalid"))?
        .iter()
        .map(|part| {
            part.as_str()
                .map(str::to_owned)
                .ok_or(Error("bound_chat_text_input_required"))
        })
        .collect::<Result<Vec<_>, _>>()?
        .join("\n");
    if text.is_empty() || text.len() > 256 * 1024 {
        return Err(Error("bound_chat_text_input_invalid"));
    }
    let input = Input {
        conversation: conversation.into(),
        message: message["id"]
            .as_str()
            .ok_or(Error("app_chat_response_invalid"))?
            .into(),
        fingerprint: digest(&text),
    };
    input.validate()?;
    if expected.is_some_and(|expected| *expected != input) {
        return Err(Error("bound_chat_input_changed"));
    }
    let end = messages
        .iter()
        .enumerate()
        .skip(start + 1)
        .find_map(|(index, message)| (message["role"] == "user").then_some(index))
        .unwrap_or(messages.len());
    let public = messages[start + 1..end].iter().filter_map(|message| {
        if message["role"] != "assistant" || !message["unavailable"].is_null()
            || message["content"]["content_type"] != "text"
            || !matches!(message["channel"].as_str(), None | Some("commentary" | "final"))
            || message["recipient"].as_str().is_some_and(|recipient| recipient != "all") { return None }
        Some(json!({"id":message["id"],"author":{"role":"assistant"},"content":message["content"],
            "channel":message["channel"],"status":message["status"],"end_turn":message["end_turn"],"recipient":message["recipient"]}))
    }).collect::<Vec<_>>();
    let completed = public.iter().any(crate::app::committed_final);
    if end < messages.len() && !completed {
        return Err(Error("bound_chat_turn_interrupted"));
    }
    Ok(Some(Turn {
        input,
        text,
        messages: public,
        ended: completed,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_identity_separates_same_text_inputs_and_complete_prior_answers() {
        let user = |id| json!({"id":id,"role":"user","content":{"content_type":"text","parts":["identical follow-up"]}});
        let answer = |id, channel, text| json!({"id":id,"role":"assistant","channel":channel,"recipient":"all","status":"finished_successfully","end_turn":true,"content":{"content_type":"text","parts":[text]}});
        let messages = vec![
            answer("anchor", "final", "previous native final"),
            user("u1"),
            answer("secret", "analysis", "PRIVATE"),
            answer("a1", "final", "complete first answer"),
            user("u2"),
            answer("a2", "final", "complete second answer"),
        ];
        let first = select("chat", "anchor", None, &messages).unwrap().unwrap();
        assert_eq!(first.input.message, "u1");
        assert_eq!(first.text, "identical follow-up");
        assert!(first.ended);
        assert_eq!(first.messages.len(), 1);
        assert_eq!(first.messages[0]["id"], "a1");
        let second = select("chat", "a1", None, &messages).unwrap().unwrap();
        assert_eq!(second.input.message, "u2");
        assert_ne!(first.input.call_id(), second.input.call_id());
        assert!(!serde_json::to_string(&first.messages)
            .unwrap()
            .contains("PRIVATE"));
        assert!(select("chat", "anchor", Some(&second.input), &messages).is_err());
        assert!(select("chat", "unrelated-branch", None, &messages).is_err());
        let mut changed = messages.clone();
        changed[1]["content"]["parts"] = json!(["changed source"]);
        assert!(select("chat", "anchor", Some(&first.input), &changed).is_err());
    }
}
