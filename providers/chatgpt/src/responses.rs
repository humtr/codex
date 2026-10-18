//! Native public transcript projection. Host reasoning is never synthesized.
use crate::protocol::{Call, Error};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    Text {
        id: String,
        phase: String,
        delta: String,
    },
    Tool(Call),
    Completed {
        input_tokens: u64,
        output_tokens: u64,
    },
}

pub struct Projection {
    id: String,
    model: String,
    sequence: u64,
    items: Vec<Value>,
    texts: BTreeMap<String, usize>,
    closed: usize,
    done: bool,
}
impl Projection {
    pub fn new(id: String, model: String) -> Self {
        Self {
            id,
            model,
            sequence: 0,
            items: Vec::new(),
            texts: BTreeMap::new(),
            closed: 0,
            done: false,
        }
    }
    fn frame(&mut self, mut value: Value) -> Value {
        value["sequence_number"] = json!(self.sequence);
        self.sequence += 1;
        value
    }
    fn response(&self, status: &str) -> Value {
        json!({"id":self.id,"object":"response","created_at":0,"status":status,"model":self.model,"output":self.items,"parallel_tool_calls":true,"error":null})
    }
    pub fn start(&mut self) -> Vec<Value> {
        let response = self.response("in_progress");
        vec![
            self.frame(json!({"type":"response.created","response":response})),
            self.frame(json!({"type":"response.in_progress","response":response})),
        ]
    }
    fn close_items(&mut self) -> Vec<Value> {
        let mut out = Vec::new();
        while self.closed < self.items.len() {
            let index = self.closed;
            self.items[index]["status"] = json!("completed");
            let item = self.items[index].clone();
            if item["type"] == "message" {
                let text = &item["content"][0]["text"];
                out.push(self.frame(json!({"type":"response.output_text.done","item_id":item["id"],"output_index":index,"content_index":0,"text":text})));
                out.push(self.frame(json!({"type":"response.content_part.done","item_id":item["id"],"output_index":index,"content_index":0,"part":item["content"][0]})));
            }
            out.push(self.frame(
                json!({"type":"response.output_item.done","output_index":index,"item":item}),
            ));
            self.closed += 1;
        }
        out
    }
    pub fn push(&mut self, event: Event) -> Result<Vec<Value>, Error> {
        if self.done {
            return Err(Error("response_already_completed"));
        }
        let mut out = Vec::new();
        match event {
            Event::Text { id, phase, delta } => {
                if !matches!(phase.as_str(), "commentary" | "final_answer") {
                    return Err(Error("nonpublic_host_message"));
                }
                let index = if let Some(index) = self.texts.get(&id) {
                    if *index < self.closed || self.items[*index]["phase"] != phase {
                        return Err(Error("nonmonotonic_public_message"));
                    }
                    *index
                } else {
                    out.extend(self.close_items());
                    let index = self.items.len();
                    let item = json!({"id":id,"type":"message","role":"assistant","status":"in_progress","phase":phase,"content":[{"type":"output_text","text":"","annotations":[]}]});
                    self.items.push(item.clone());
                    self.texts.insert(id.clone(), index);
                    out.push(self.frame(json!({"type":"response.output_item.added","output_index":index,"item":item})));
                    out.push(self.frame(json!({"type":"response.content_part.added","item_id":id,"output_index":index,"content_index":0,"part":item["content"][0]})));
                    index
                };
                let text = self.items[index]["content"][0]["text"]
                    .as_str()
                    .unwrap()
                    .to_owned()
                    + &delta;
                self.items[index]["content"][0]["text"] = json!(text);
                out.push(self.frame(json!({"type":"response.output_text.delta","item_id":id,"output_index":index,"content_index":0,"delta":delta})));
            }
            Event::Tool(call) => {
                out.extend(self.close_items());
                if self.items.iter().any(|v| v["id"] == call.id) {
                    return Err(Error("duplicate_host_call"));
                }
                let full = call.output_item()?;
                let index = self.items.len();
                let (field, kind) = match call.tool.kind {
                    crate::protocol::ToolKind::Function => ("arguments", "function_call_arguments"),
                    crate::protocol::ToolKind::Custom => ("input", "custom_tool_call_input"),
                    crate::protocol::ToolKind::Search => {
                        ("arguments", "tool_search_call_arguments")
                    }
                };
                let mut initial = full.clone();
                initial["status"] = json!("in_progress");
                initial[field] = if call.tool.kind == crate::protocol::ToolKind::Search {
                    json!({})
                } else {
                    json!("")
                };
                out.push(self.frame(json!({"type":"response.output_item.added","output_index":index,"item":initial})));
                out.push(self.frame(json!({"type":format!("response.{kind}.delta"),"item_id":call.id,"output_index":index,"delta":call.input})));
                out.push(self.frame(json!({"type":format!("response.{kind}.done"),"item_id":call.id,"output_index":index,(field):full[field]})));
                self.items.push(full);
                out.extend(self.close_items());
            }
            Event::Completed {
                input_tokens,
                output_tokens,
            } => {
                out.extend(self.close_items());
                let mut response = self.response("completed");
                // Estimates are labeled. These counters are not Chat allowance deductions.
                response["usage"] = json!({"input_tokens":input_tokens,"output_tokens":output_tokens,"total_tokens":input_tokens.saturating_add(output_tokens),"input_tokens_details":{"cached_tokens":0},"output_tokens_details":{"reasoning_tokens":0},"estimated":true});
                out.push(self.frame(json!({"type":"response.completed","response":response})));
                self.done = true;
            }
        }
        Ok(out)
    }
    pub fn fail(&mut self, error: Error) -> Value {
        self.done = true;
        self.frame(json!({"type":"error","code":error.0,"message":error.0,"param":null}))
    }
    pub fn completed(&self) -> bool {
        self.done
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn public_phases_and_tools_become_one_ordered_native_response() {
        let mut p = Projection::new("resp-owned".into(), "configured".into());
        let mut frames = p.start();
        frames.extend(
            p.push(Event::Text {
                id: "first".into(),
                phase: "commentary".into(),
                delta: "Inspecting".into(),
            })
            .unwrap(),
        );
        frames.extend(
            p.push(Event::Text {
                id: "first".into(),
                phase: "commentary".into(),
                delta: " files".into(),
            })
            .unwrap(),
        );
        frames.extend(
            p.push(Event::Tool(Call {
                id: "original".into(),
                host_id: "original".into(),
                tool: crate::protocol::Tool {
                    name: "exec".into(),
                    namespace: Some("functions".into()),
                    kind: crate::protocol::ToolKind::Function,
                    declaration: json!({}),
                },
                input: "{malformed".into(),
            }))
            .unwrap(),
        );
        frames.extend(
            p.push(Event::Completed {
                input_tokens: 5,
                output_tokens: 3,
            })
            .unwrap(),
        );
        let response = &frames.last().unwrap()["response"];
        assert_eq!(
            response["output"][0]["content"][0]["text"],
            "Inspecting files"
        );
        assert_eq!(response["output"][1]["arguments"], "{malformed");
        assert_eq!(response["output"][1]["call_id"], "original");
        for (index, frame) in frames.iter().enumerate() {
            assert_eq!(frame["sequence_number"], index as u64);
        }
        assert!(p
            .push(Event::Completed {
                input_tokens: 0,
                output_tokens: 0
            })
            .is_err());
    }
    #[test]
    fn private_reasoning_is_never_projected() {
        let mut p = Projection::new("r".into(), "m".into());
        assert_eq!(
            p.push(Event::Text {
                id: "x".into(),
                phase: "analysis".into(),
                delta: "private".into()
            })
            .unwrap_err(),
            Error("nonpublic_host_message")
        );
        assert_eq!(p.items.len(), 0);
    }
}
