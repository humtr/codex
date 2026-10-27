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
    pub fn call_id(&self, account_ref: &str) -> String {
        format!(
            "chat-source:{}",
            digest(&(account_ref, &self.conversation, &self.message))
        )
    }
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum End {
    Active,
    Completed,
    Interrupted { anchor: String },
}
pub(crate) struct Turn {
    pub account_ref: String,
    pub anchor: String,
    pub input: Input,
    pub text: String,
    pub messages: Vec<Value>,
    pub tool_messages: Vec<Value>,
    pub end: End,
}

pub(crate) async fn read(
    bridge: Arc<dyn Bridge>,
    conversation: &str,
    anchor: &str,
    input: Option<&Input>,
    require_single_branch: bool,
    selection: Option<(&str, &[String])>,
) -> Result<Option<Turn>, Error> {
    let source = Chats::new(bridge);
    let page = source.read_complete_source(conversation).await?;
    if require_single_branch
        && page["branches"]
            .as_array()
            .is_none_or(|branches| branches.len() != 1)
    {
        return Err(Error("source_selection_required"));
    }
    let account_ref = page["account"]
        .as_str()
        .ok_or(Error("app_identity_unavailable"))?;
    let messages = page["messages"]
        .as_array()
        .ok_or(Error("app_chat_response_invalid"))?;
    let anchor = if messages.iter().any(|message| message["id"] == anchor) {
        anchor
    } else if let Some((_, committed)) = selection {
        messages
            .iter()
            .rev()
            .find_map(|message| {
                let id = message["id"].as_str()?;
                committed.iter().any(|known| known == id).then_some(id)
            })
            .ok_or(Error("bound_chat_branch_changed"))?
    } else {
        return Err(Error("bound_chat_branch_changed"));
    };
    let turn = select(account_ref, conversation, anchor, input, messages)?;
    if selection.is_some_and(|(id, _)| turn.as_ref().is_none_or(|turn| turn.input.message != id)) {
        return Err(Error("source_selection_required"));
    }
    Ok(turn)
}

fn select(
    account_ref: &str,
    conversation: &str,
    anchor: &str,
    expected: Option<&Input>,
    messages: &[Value],
) -> Result<Option<Turn>, Error> {
    let source_anchor = anchor;
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
            "channel":message["channel"],"status":message["status"],"end_turn":message["end_turn"],"recipient":message["recipient"],
            "metadata":{"model_slug":message["model"]}}))
    }).collect::<Vec<_>>();
    let completed = public.iter().any(crate::app::committed_final);
    let tool_messages = messages[start + 1..end].iter().filter(|message|
        message["role"]=="assistant" && message["recipient"]=="api_tool.call_tool"
            && message["unavailable"].is_null() && message["content"]["content_type"]=="code"
            && message["status"]=="finished_successfully")
        .map(|message|json!({"id":message["id"],"author":{"role":"assistant"},"recipient":message["recipient"],
            "status":message["status"],"content":message["content"]})).collect();
    let ended = if completed {
        End::Completed
    } else if end < messages.len() {
        End::Interrupted {
            anchor: messages[end - 1]["id"]
                .as_str()
                .ok_or(Error("app_chat_response_invalid"))?
                .into(),
        }
    } else {
        End::Active
    };
    Ok(Some(Turn {
        account_ref: account_ref.into(),
        anchor: source_anchor.into(),
        input,
        text,
        messages: public,
        tool_messages,
        end: ended,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn interrupted_source_preserves_partial_text_and_next_original_input() {
        let messages = vec![
            json!({"id":"anchor","role":"assistant"}),
            json!({"id":"u1","role":"user","content":{"content_type":"text","parts":["first input"]}}),
            json!({"id":"partial","role":"assistant","channel":"commentary","status":"finished_partial_completion","end_turn":false,"content":{"content_type":"text","parts":["Original partial output"]}}),
            json!({"id":"u2","role":"user","content":{"content_type":"text","parts":["next input"]}}),
        ];
        let first = select("test-account", "chat", "anchor", None, &messages)
            .unwrap()
            .unwrap();
        assert_eq!(first.input.message, "u1");
        assert_eq!(
            first.end,
            End::Interrupted {
                anchor: "partial".into()
            }
        );
        assert_eq!(
            first.messages[0]["content"]["parts"][0],
            "Original partial output"
        );
        assert!(!first.messages.iter().any(crate::app::committed_final));
        let next = select("test-account", "chat", "partial", None, &messages)
            .unwrap()
            .unwrap();
        assert_eq!(next.input.message, "u2");
        assert_eq!(next.end, End::Active);
        assert_eq!(next.text, "next input");
        let empty = vec![
            messages[0].clone(),
            messages[1].clone(),
            messages[3].clone(),
        ];
        assert_eq!(
            select("test-account", "chat", "anchor", None, &empty)
                .unwrap()
                .unwrap()
                .end,
            End::Interrupted {
                anchor: "u1".into()
            }
        );
    }

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
        let first = select("test-account", "chat", "anchor", None, &messages)
            .unwrap()
            .unwrap();
        assert_eq!(first.input.message, "u1");
        assert_eq!(first.text, "identical follow-up");
        assert_eq!(first.end, End::Completed);
        assert_eq!(first.messages.len(), 1);
        assert_eq!(first.messages[0]["id"], "a1");
        let second = select("test-account", "chat", "a1", None, &messages)
            .unwrap()
            .unwrap();
        assert_eq!(second.input.message, "u2");
        assert_ne!(
            first.input.call_id("test-account"),
            second.input.call_id("test-account")
        );
        assert_ne!(
            first.input.call_id("account-one"),
            first.input.call_id("account-two")
        );
        assert!(!serde_json::to_string(&first.messages)
            .unwrap()
            .contains("PRIVATE"));
        assert!(select(
            "test-account",
            "chat",
            "anchor",
            Some(&second.input),
            &messages
        )
        .is_err());
        assert!(select("test-account", "chat", "unrelated-branch", None, &messages).is_err());
        let mut changed = messages.clone();
        changed[1]["content"]["parts"] = json!(["changed source"]);
        assert!(select(
            "test-account",
            "chat",
            "anchor",
            Some(&first.input),
            &changed
        )
        .is_err());
    }
}
