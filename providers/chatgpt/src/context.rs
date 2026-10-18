//! Ordered native context import and warm-prefix admission. Visibility changes
//! never elevate user text to developer authority.
use crate::protocol::{Error, Request};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

pub fn digest(value: &impl Serialize) -> String {
    Sha256::digest(
        serde_json::to_vec(value).expect("JSON serialization is infallible for these values"),
    )
    .iter()
    .map(|b| format!("{b:02x}"))
    .collect()
}

pub fn text(item: &Value) -> Result<String, Error> {
    match &item["content"] {
        Value::String(s) => Ok(s.clone()),
        Value::Array(parts) => parts
            .iter()
            .map(|p| {
                p["text"]
                    .as_str()
                    .map(str::to_owned)
                    .ok_or(Error("text_only"))
            })
            .collect::<Result<Vec<_>, _>>()
            .map(|p| p.join("\n")),
        _ => Err(Error("text_only")),
    }
}

fn canonical(item: &Value) -> Value {
    match item["type"].as_str().unwrap_or("message") {
        "message" => json!({"type":"message","role":item["role"],"text":text(item).ok()}),
        "function_call" | "custom_tool_call" | "tool_search_call" => {
            json!({"type":item["type"],"call_id":item["call_id"],"name":item["name"],"namespace":item["namespace"],"arguments":item["arguments"],"input":item["input"]})
        }
        _ => item.clone(),
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Prefix {
    pub count: usize,
    pub digest: String,
    pub instructions: String,
    pub tools: String,
}
impl Prefix {
    pub fn accept(&mut self, request: &Request, output: &[Value]) {
        let values = request
            .input
            .iter()
            .chain(output.iter())
            .map(canonical)
            .collect::<Vec<_>>();
        self.count = values.len();
        self.digest = digest(&values);
        self.instructions = digest(&request.instructions);
        self.tools = digest(&(&request.tools, &request.host_tools));
    }
    pub fn delta<'a>(&self, request: &'a Request) -> Result<&'a [Value], Error> {
        if self.count == 0 {
            return Ok(&request.input);
        }
        let prefix = request
            .input
            .iter()
            .take(self.count)
            .map(canonical)
            .collect::<Vec<_>>();
        if prefix.len() != self.count || digest(&prefix) != self.digest {
            return Err(Error("native_context_prefix_changed"));
        }
        Ok(&request.input[self.count..])
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Message {
    pub role: String,
    pub text: String,
    pub hidden: bool,
}

fn internal_visibility_hint(value: &str) -> bool {
    let value = value.trim_start();
    [
        "<environment_context>",
        "<permissions instructions>",
        "<collaboration_mode>",
    ]
    .iter()
    .any(|prefix| value.starts_with(prefix))
}

/// Preserve role/order, but expose only the newest ordinary user request.
/// Native control-shaped text stays user authority even if its UI is hidden.
pub fn messages(input: &[Value]) -> Result<Vec<Message>, Error> {
    let mut result = Vec::new();
    for item in input {
        match item["type"].as_str().unwrap_or("message") {
            "message" => {
                let role = item["role"]
                    .as_str()
                    .ok_or(Error("unsupported_message_role"))?;
                result.push(Message {
                    role: if role == "system" { "developer" } else { role }.into(),
                    text: text(item)?,
                    hidden: true,
                });
            }
            "context_compaction" if item.get("encrypted_content").is_none() => (),
            "compaction" | "compaction_summary" | "context_compaction" => {
                let encoded = item["encrypted_content"]
                    .as_str()
                    .ok_or(Error("opaque_context_unavailable"))?;
                let encoded = encoded
                    .strip_prefix("ocx1:")
                    .ok_or(Error("opaque_context_unavailable"))?;
                let bytes = STANDARD
                    .decode(encoded)
                    .map_err(|_| Error("invalid_context_checkpoint"))?;
                if STANDARD.encode(&bytes) != encoded {
                    return Err(Error("invalid_context_checkpoint"));
                }
                let text =
                    String::from_utf8(bytes).map_err(|_| Error("invalid_context_checkpoint"))?;
                if text.trim().is_empty() {
                    return Err(Error("invalid_context_checkpoint"));
                }
                result.push(Message {
                    role: "user".into(),
                    text,
                    hidden: true,
                });
            }
            "additional_tools" => (),
            "reasoning" => (), // Opaque reasoning is not a fabricated public transcript.
            "compaction_trigger" => return Err(Error("compaction_plan_required")),
            _ => result.push(Message {
                role: "user".into(),
                text: format!("Native tool transcript item:\n{}", item),
                hidden: true,
            }),
        }
    }
    if let Some(index) = result.iter().rposition(|m| {
        m.role == "user"
            && !internal_visibility_hint(&m.text)
            && !m.text.starts_with("Native tool transcript item:\n")
    }) {
        result[index].hidden = false;
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    #[test]
    fn environment_visibility_keeps_user_role_and_original_order() {
        let values = json!([{"role":"user","content":"Do the task"},{"role":"user","content":"<environment_context>private path</environment_context>"}]);
        let projected = messages(values.as_array().unwrap()).unwrap();
        assert!(!projected[0].hidden);
        assert!(projected[1].hidden);
        assert_eq!(projected[1].role, "user");
        assert!(projected[1].text.contains("private path"));
    }
    #[test]
    fn resident_assistant_output_and_setup_are_not_imported_again() {
        let mut body = json!({"model":"chatgpt-desktop/gpt-6","stream":true,"input":"first"});
        let request = Request::parse(body.clone(), &Config::default(), Some("native")).unwrap();
        let mut prefix = Prefix::default();
        prefix.accept(&request,&[json!({"type":"message","role":"assistant","content":[{"type":"output_text","text":"done"}],"phase":"final_answer","id":"response-local"})]);
        body["input"] = json!([{"role":"user","content":[{"type":"input_text","text":"first"}]},{"role":"assistant","content":[{"type":"output_text","text":"done"}]},{"role":"user","content":"next"}]);
        let next = Request::parse(body.clone(), &Config::default(), Some("native")).unwrap();
        assert_eq!(prefix.delta(&next).unwrap(), &next.input[2..]);
        body["input"][0]["content"][0]["text"] = json!("changed");
        assert_eq!(
            prefix
                .delta(&Request::parse(body, &Config::default(), Some("native")).unwrap())
                .unwrap_err(),
            Error("native_context_prefix_changed")
        );
    }
    #[test]
    fn opaque_native_checkpoints_cannot_be_relabelled_as_text() {
        assert_eq!(
            messages(&[json!({"type":"compaction","encrypted_content":"native-opaque"})])
                .unwrap_err(),
            Error("opaque_context_unavailable")
        );
        let valid = STANDARD.encode("retained public summary");
        assert_eq!(
            messages(&[json!({"type":"compaction","encrypted_content":format!("ocx1:{valid}")})])
                .unwrap()[0]
                .text,
            "retained public summary"
        );
    }
}
