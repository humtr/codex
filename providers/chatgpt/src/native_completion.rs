//! Typed facts emitted by the qualified Native harness. Parsing is not
//! admission: every consumer must still bind the original Owner/call/session.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// Reject duplicate keys before serde_json::Value can erase them. Canonical
/// harness output never needs last-key-wins interpretation for authority.
pub(crate) fn strict_json(bytes: &[u8]) -> Result<Value, serde_json::Error> {
    struct Unique(Value);
    impl<'de> Deserialize<'de> for Unique {
        fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            struct Read;
            impl<'de> serde::de::Visitor<'de> for Read {
                type Value = Unique;
                fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                    f.write_str("unique-key JSON")
                }
                fn visit_unit<E: serde::de::Error>(self) -> Result<Unique, E> {
                    Ok(Unique(Value::Null))
                }
                fn visit_bool<E: serde::de::Error>(self, v: bool) -> Result<Unique, E> {
                    Ok(Unique(Value::Bool(v)))
                }
                fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Unique, E> {
                    Ok(Unique(v.into()))
                }
                fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Unique, E> {
                    Ok(Unique(v.into()))
                }
                fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Unique, E> {
                    serde_json::Number::from_f64(v)
                        .map(Value::Number)
                        .map(Unique)
                        .ok_or_else(|| E::custom("invalid number"))
                }
                fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Unique, E> {
                    Ok(Unique(v.into()))
                }
                fn visit_string<E: serde::de::Error>(self, v: String) -> Result<Unique, E> {
                    Ok(Unique(v.into()))
                }
                fn visit_seq<A: serde::de::SeqAccess<'de>>(
                    self,
                    mut a: A,
                ) -> Result<Unique, A::Error> {
                    let mut values = Vec::new();
                    while let Some(Unique(value)) = a.next_element()? {
                        values.push(value);
                    }
                    Ok(Unique(Value::Array(values)))
                }
                fn visit_map<A: serde::de::MapAccess<'de>>(
                    self,
                    mut a: A,
                ) -> Result<Unique, A::Error> {
                    let mut fields = serde_json::Map::new();
                    while let Some((key, Unique(value))) = a.next_entry::<String, Unique>()? {
                        if fields.insert(key, value).is_some() {
                            return Err(serde::de::Error::custom("duplicate JSON key"));
                        }
                    }
                    Ok(Unique(Value::Object(fields)))
                }
            }
            d.deserialize_any(Read)
        }
    }
    serde_json::from_slice::<Unique>(bytes).map(|Unique(value)| value)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Outcome {
    PreBackendRejected,
    WaitCompleted,
    SessionRunning,
}

fn present_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Proof {
    pub schema: String,
    pub call_id: String,
    #[serde(deserialize_with = "present_option")]
    pub session_id: Option<u64>,
    pub outcome: Outcome,
    #[serde(deserialize_with = "present_option")]
    pub exit_code: Option<i64>,
    pub hooks: String,
}
impl Proof {
    pub(crate) fn parse(value: &Value) -> Option<Self> {
        let fields = value.as_object()?;
        if fields.len() != 6
            || ![
                "schema",
                "call_id",
                "session_id",
                "outcome",
                "exit_code",
                "hooks",
            ]
            .iter()
            .all(|key| fields.contains_key(*key))
        {
            return None;
        }
        let proof: Self = serde_json::from_value(value.clone()).ok()?;
        proof.valid().then_some(proof)
    }
    pub(crate) fn valid(&self) -> bool {
        self.schema == "codex-native-completion-v1"
            && self.hooks == "absent"
            && self.session_id.is_none_or(|id| id <= i32::MAX as u64)
            && !self.call_id.is_empty()
            && self.call_id.len() <= 512
            && !self.call_id.chars().any(char::is_control)
            && match self.outcome {
                Outcome::PreBackendRejected => {
                    self.exit_code.is_none() && self.session_id.is_none()
                }
                Outcome::WaitCompleted => {
                    self.exit_code.is_none_or(|code| (0..=255).contains(&code))
                }
                Outcome::SessionRunning => self.session_id.is_some() && self.exit_code.is_none(),
            }
    }
    pub(crate) fn belongs_to(&self, call: &str, session: Option<u64>) -> bool {
        self.valid()
            && self.call_id == call
            && session.is_none_or(|session| self.session_id == Some(session))
    }
    pub(crate) fn wait_completed(&self) -> bool {
        self.valid() && self.outcome == Outcome::WaitCompleted
    }
    pub(crate) fn waited(&self) -> Option<i64> {
        (self.valid() && self.outcome == Outcome::WaitCompleted)
            .then_some(self.exit_code)
            .flatten()
    }
    pub(crate) fn running(&self) -> Option<u64> {
        (self.valid() && self.outcome == Outcome::SessionRunning)
            .then_some(self.session_id)
            .flatten()
    }
    pub(crate) fn not_started(&self) -> bool {
        self.valid() && self.outcome == Outcome::PreBackendRejected
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Presence {
    Absent,
    /// The new producer ran, but supplied no completion authority.
    Unconfirmed,
    Invalid,
    Present(Proof),
}

/// Only the actual one-text Native result projection is accepted. This does
/// not search arbitrary stdout or stringify objects into apparent headers.
pub(crate) fn result_text(value: &Value) -> Option<&str> {
    fn read(value: &Value, remaining: usize) -> Option<&str> {
        if remaining == 0 {
            return None;
        }
        match value {
            Value::String(text) => Some(text),
            Value::Array(values) if values.len() == 1 => read(&values[0], remaining - 1),
            Value::Array(values) => values
                .first()
                .filter(|item| matches!(item["type"].as_str(), Some("input_text" | "text")))
                .and_then(|item| item["text"].as_str())
                .filter(|text| text.starts_with("Native completion: ")),
            Value::Object(fields) => {
                let mut candidates = ["body", "text", "output", "content"]
                    .iter()
                    .filter_map(|name| fields.get(*name));
                let value = candidates.next()?;
                if candidates.next().is_some() {
                    return None;
                }
                read(value, remaining - 1)
            }
            _ => None,
        }
    }
    read(value, 6)
}

pub(crate) fn result_proof(value: &Value) -> Presence {
    fn parse(text: &str, decoded: bool) -> Presence {
        if let Ok(object) = strict_json(text.as_bytes()) {
            if let Some(text) = object.as_str().filter(|_| !decoded) {
                return parse(text, true);
            }
            return match object.get("native_completion") {
                Some(value) if value.is_null() => Presence::Unconfirmed,
                Some(value) => Proof::parse(value).map_or(Presence::Invalid, Presence::Present),
                None => Presence::Absent,
            };
        }
        if text.trim_start().starts_with(['{', '[', '"']) {
            return Presence::Invalid;
        }
        let header = text
            .split_once("\nOutput:\n")
            .map_or(text, |(header, _)| header);
        let mut lines = header.lines();
        let Some(first) = lines.next() else {
            return Presence::Absent;
        };
        if !first.starts_with("Native completion:") {
            return if lines.any(|line| line.starts_with("Native completion:")) {
                Presence::Invalid
            } else {
                Presence::Absent
            };
        }
        if lines.any(|line| line.starts_with("Native completion:")) {
            return Presence::Invalid;
        }
        let Some(raw) = first.strip_prefix("Native completion: ") else {
            return Presence::Invalid;
        };
        let Ok(value) = strict_json(raw.as_bytes()) else {
            return Presence::Invalid;
        };
        if value.is_null() {
            Presence::Unconfirmed
        } else {
            Proof::parse(&value).map_or(Presence::Invalid, Presence::Present)
        }
    }
    result_text(value).map_or(Presence::Absent, |text| parse(text, false))
}

/// A session-owned raw End row can add completion evidence without changing
/// the already returned function output or the materialized UI item.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct EndWitness {
    pub call_id: String,
    pub turn_id: String,
    pub session_id: Option<u64>,
    pub proof: Presence,
    original: Value,
}
impl EndWitness {
    pub(crate) fn matches_canonical_command(&self, completed: &Value) -> bool {
        let item = &completed["item"];
        self.call_id == item["id"].as_str().unwrap_or("")
            && self.turn_id == completed["turn_id"].as_str().unwrap_or("")
            && event_session(item.get("process_id")) == Some(self.session_id)
            && self.original["completed_at_ms"] == completed["completed_at_ms"]
            && [
                "command",
                "cwd",
                "parsed_cmd",
                "source",
                "plugin_id",
                "script_path",
                "aggregated_output",
                "exit_code",
                "duration",
                "status",
            ]
            .iter()
            .all(|key| self.original[*key] == item[*key])
            && self.original["interaction_input"] == item["interaction_input"]
    }
}
fn event_session(value: Option<&Value>) -> Option<Option<u64>> {
    match value {
        None | Some(Value::Null) => Some(None),
        Some(Value::Number(number)) => number.as_u64().map(Some),
        Some(Value::String(value))
            if !value.is_empty() && value.bytes().all(|v| v.is_ascii_digit()) =>
        {
            value.parse().ok().map(Some)
        }
        _ => None,
    }
}
pub(crate) fn end_witnesses(rows: &[Value]) -> Result<BTreeMap<String, EndWitness>, &'static str> {
    let mut result: BTreeMap<String, EndWitness> = BTreeMap::new();
    for row in rows {
        let payload = &row["payload"];
        if row["type"] != "event_msg"
            || payload["type"] != "exec_command_end"
            || payload.get("native_completion").is_none()
        {
            continue;
        }
        let call = payload["call_id"]
            .as_str()
            .filter(|v| !v.is_empty())
            .ok_or("native_completion_identity_invalid")?;
        let turn = payload["turn_id"]
            .as_str()
            .filter(|v| !v.is_empty())
            .ok_or("native_completion_identity_invalid")?;
        let session =
            event_session(payload.get("process_id")).ok_or("native_completion_identity_invalid")?;
        let value = &payload["native_completion"];
        let proof = if value.is_null() {
            Presence::Unconfirmed
        } else {
            Proof::parse(value)
                .filter(|p| {
                    p.belongs_to(call, session) && p.session_id == session && p.wait_completed()
                })
                .map_or(Presence::Invalid, Presence::Present)
        };
        let mut original = payload.clone();
        original
            .as_object_mut()
            .ok_or("native_completion_identity_invalid")?
            .remove("native_completion");
        let next = EndWitness {
            call_id: call.into(),
            turn_id: turn.into(),
            session_id: session,
            proof,
            original,
        };
        if let Some(previous) = result.get(call) {
            // The actual waiter may supplement exactly one already published
            // unknown End. Conflicting/duplicate terminal evidence is unsafe.
            if previous.original != next.original
                || previous.proof != Presence::Unconfirmed
                || !matches!(next.proof, Presence::Present(_))
            {
                return Err("native_completion_identity_changed");
            }
        }
        result.insert(call.into(), next);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn wait() -> Value {
        json!({"schema":"codex-native-completion-v1","call_id":"original","session_id":41,
            "outcome":"wait_completed","exit_code":137,"hooks":"absent"})
    }
    #[test]
    fn actual_wait_with_unknown_numeric_exit_is_distinct_from_missing_wait_proof() {
        let mut value = wait();
        value["exit_code"] = Value::Null;
        let proof =
            Proof::parse(&value).expect("explicit typed wait is positive even without a number");
        assert!(proof.wait_completed());
        assert_eq!(proof.waited(), None);
        assert!(!proof.not_started());
        assert_eq!(proof.running(), None);
        let mut missing = value.clone();
        missing.as_object_mut().unwrap().remove("exit_code");
        assert!(Proof::parse(&missing).is_none());
        assert!(
            serde_json::from_value::<Proof>(missing.clone()).is_err(),
            "durable typed proof also requires the explicit nullable key"
        );
        let mut no_session = value.clone();
        no_session.as_object_mut().unwrap().remove("session_id");
        assert!(serde_json::from_value::<Proof>(no_session).is_err());
        assert_eq!(
            result_proof(
                &json!({"output":"Native completion: null\nProcess exited with code -1\nOutput:\n"})
            ),
            Presence::Unconfirmed
        );
        let end = json!({"type":"event_msg","payload":{"type":"exec_command_end","call_id":"original",
            "turn_id":"turn","process_id":"41","exit_code":-1,"native_completion":value}});
        assert!(matches!(
            end_witnesses(&[end]).unwrap()["original"].proof,
            Presence::Present(_)
        ));
    }

    #[test]
    fn typed_completion_binds_exact_original_call_session_and_real_wait_code() {
        let proof = Proof::parse(&wait()).unwrap();
        assert!(proof.belongs_to("original", Some(41)));
        assert!(!proof.belongs_to("other", Some(41)));
        assert!(!proof.belongs_to("original", Some(42)));
        assert_eq!(proof.waited(), Some(137));
        let header = json!({"type":"function_call_output","call_id":"original",
            "output":format!("Native completion: {}\nProcess exited with code 124\nOutput:\nunchanged timeout body",wait())});
        assert_eq!(result_proof(&header), Presence::Present(proof.clone()));
        let code = json!({"output":json!({"wall_time_seconds":1.0,"exit_code":124,
            "native_completion":wait(),"output":"unchanged"}).to_string()});
        assert_eq!(result_proof(&code), Presence::Present(proof));
    }
    #[test]
    fn completion_unknown_missing_malformed_duplicate_and_child_stdout_are_not_proof() {
        let good = wait();
        for (key, value) in [
            ("schema", json!("future-v9")),
            ("session_id", json!(-1)),
            ("exit_code", json!(-1)),
            ("hooks", json!("unknown")),
            ("call_id", json!("")),
            ("outcome", json!("declined")),
        ] {
            let mut wrong = good.clone();
            wrong[key] = value;
            assert!(Proof::parse(&wrong).is_none());
        }
        for key in [
            "schema",
            "call_id",
            "session_id",
            "outcome",
            "exit_code",
            "hooks",
        ] {
            let mut missing = good.clone();
            missing.as_object_mut().unwrap().remove(key);
            assert!(Proof::parse(&missing).is_none());
        }
        let stdout = json!({"output":format!("Process running with session ID 41\nOutput:\nNative completion: {good}")});
        assert_eq!(result_proof(&stdout), Presence::Absent);
        let duplicate = json!({"output":format!("Native completion: {good}\nNative completion: {good}\nOutput:\n")});
        assert_eq!(result_proof(&duplicate), Presence::Invalid);
        assert_eq!(
            result_proof(&json!({"output":"Native completion: broken\nOutput:\n"})),
            Presence::Invalid
        );
        assert!(result_text(&json!({"output":"one","body":"other"})).is_none());
    }
    #[test]
    fn explicit_null_is_not_a_legacy_generation_or_a_completion_proof() {
        for output in [
            "Native completion: null\nProcess exited with code 0\nOutput:\n".to_owned(),
            json!({"output":"","wall_time_seconds":0.1,"exit_code":0,
                "native_completion":null})
            .to_string(),
        ] {
            assert_eq!(
                result_proof(&json!({"output":output})),
                Presence::Unconfirmed
            );
        }
        assert_eq!(
            result_proof(&json!({"output":"Process exited with code 0\nOutput:\n"})),
            Presence::Absent
        );
    }
    #[test]
    fn duplicate_json_keys_cannot_change_the_completion_authority() {
        let duplicate = serde_json::to_string(&wait()).unwrap().replacen(
            "\"call_id\":\"original\"",
            "\"call_id\":\"wrong\",\"call_id\":\"original\"",
            1,
        );
        let header = json!({"output":format!("Native completion: {duplicate}\nOutput:\n")});
        assert_eq!(result_proof(&header), Presence::Invalid);
        let code = json!({"output":format!("{{\"output\":\"\",\"exit_code\":0,\"wall_time_seconds\":0,\"native_completion\":null,\"native_completion\":{}}}",wait())});
        assert_eq!(result_proof(&code), Presence::Invalid);
        assert!(
            strict_json(br#"{"payload":{"native_completion":null,"native_completion":null}}"#)
                .is_err()
        );
    }
    #[test]
    fn canonical_code_mode_string_and_first_content_header_preserve_new_unknown() {
        let string = json!({"output":serde_json::to_string(
            "Native completion: null\nfeedback wrapper").unwrap()});
        assert_eq!(result_proof(&string), Presence::Unconfirmed);
        let contents = json!({"output":[
            {"type":"input_text","text":"Native completion: null\n"},
            {"type":"input_text","text":format!("Native completion: {}",wait())},
            {"type":"input_image","image_url":"owned-fixture"}
        ]});
        assert_eq!(result_proof(&contents), Presence::Unconfirmed);
        let child = json!({"output":[{"type":"input_text","text":"child output"},
            {"type":"input_text","text":format!("Native completion: {}",wait())}]});
        assert_eq!(result_proof(&child), Presence::Absent);
    }
    #[test]
    fn canonical_command_preserves_nonempty_interaction_input_and_rejects_alias_or_mutation() {
        let end = json!({"type":"event_msg","payload":{"type":"exec_command_end",
            "call_id":"original","turn_id":"turn","process_id":"41","completed_at_ms":42,
            "exit_code":124,"status":"failed","aggregated_output":"original timeout body",
            "interaction_input":"owned input\n","native_completion":wait()}});
        let witnesses = end_witnesses(&[end.clone()]).unwrap();
        let mut item = end["payload"].clone();
        item["id"] = json!("original");
        item["type"] = json!("command_execution");
        let canonical =
            json!({"thread_id":"owned-thread","turn_id":"turn","completed_at_ms":42,"item":item});
        assert!(witnesses["original"].matches_canonical_command(&canonical));
        let mut changed = canonical.clone();
        changed["item"]["interaction_input"] = json!("different input");
        assert!(!witnesses["original"].matches_canonical_command(&changed));
        let mut alias = canonical;
        alias["item"]
            .as_object_mut()
            .unwrap()
            .remove("interaction_input");
        alias["item"]["input"] = json!("owned input\n");
        assert!(!witnesses["original"].matches_canonical_command(&alias));
    }
    #[test]
    fn late_wait_supplements_only_the_identical_unknown_end() {
        let end = json!({"type":"event_msg","payload":{"type":"exec_command_end",
            "call_id":"original","turn_id":"turn","process_id":"41","exit_code":124,
            "status":"failed","aggregated_output":"original timeout body","native_completion":null}});
        let mut waited = end.clone();
        waited["payload"]["native_completion"] = wait();
        let witnesses = end_witnesses(&[end.clone(), waited.clone()]).unwrap();
        assert_eq!(
            witnesses["original"].proof,
            Presence::Present(Proof::parse(&wait()).unwrap())
        );
        assert!(end_witnesses(&[waited.clone(), waited.clone()]).is_err());
        assert!(end_witnesses(&[end.clone(), end.clone()]).is_err());
        for key in ["turn_id", "process_id", "aggregated_output", "exit_code"] {
            let mut changed = waited.clone();
            changed["payload"][key] = json!("different");
            assert!(end_witnesses(&[end.clone(), changed]).is_err(), "{key}");
        }
        let mut invalid = waited;
        invalid["payload"]["native_completion"]["call_id"] = json!("other");
        assert_eq!(
            end_witnesses(&[invalid]).unwrap()["original"].proof,
            Presence::Invalid
        );
    }
    #[test]
    fn typed_header_is_first_harness_line_never_an_embedded_error_or_feedback() {
        let proof = wait();
        for prefix in [
            "ordinary error",
            "PostToolUse feedback",
            "Process exited with code 0",
            " ",
        ] {
            let result =
                json!({"output":format!("{prefix}\nNative completion: {proof}\nOutput:\n")});
            assert_eq!(result_proof(&result), Presence::Invalid, "{prefix}");
        }
        let legitimate = json!({"output":format!("Native completion: {proof}\nordinary original body without a second header")});
        assert!(matches!(result_proof(&legitimate), Presence::Present(_)));
        let child = json!({"output":format!("Native completion: null\nOutput:\nNative completion: {proof}")});
        assert_eq!(result_proof(&child), Presence::Unconfirmed);
    }

    #[test]
    fn running_continuation_proof_is_not_terminal_and_cannot_be_a_raw_end() {
        let mut value = wait();
        value["call_id"] = json!("poll");
        value["outcome"] = json!("session_running");
        value["exit_code"] = Value::Null;
        let proof = Proof::parse(&value).unwrap();
        assert_eq!(proof.running(), Some(41));
        assert_eq!(proof.waited(), None);
        assert!(!proof.not_started());
        let end = json!({"type":"event_msg","payload":{"type":"exec_command_end","call_id":"poll",
            "turn_id":"turn","process_id":"41","native_completion":value}});
        assert_eq!(
            end_witnesses(&[end]).unwrap()["poll"].proof,
            Presence::Invalid
        );
        for (key, bad) in [("session_id", Value::Null), ("exit_code", json!(0))] {
            let mut wrong = value.clone();
            wrong[key] = bad;
            assert!(Proof::parse(&wrong).is_none());
        }
    }
    #[test]
    fn no_backend_proof_requires_absent_hooks_no_process_and_no_exit() {
        let mut value = wait();
        value["outcome"] = json!("pre_backend_rejected");
        value["session_id"] = Value::Null;
        value["exit_code"] = Value::Null;
        let proof = Proof::parse(&value).unwrap();
        assert!(proof.not_started());
        assert!(!proof.belongs_to("original", Some(41)));
        let end = json!({"type":"event_msg","payload":{"type":"exec_command_end","call_id":"original",
            "turn_id":"turn","process_id":null,"native_completion":value}});
        assert_eq!(
            end_witnesses(&[end]).unwrap()["original"].proof,
            Presence::Invalid
        );
        value["session_id"] = json!(41);
        assert!(Proof::parse(&value).is_none());
        value["session_id"] = Value::Null;
        value["exit_code"] = json!(0);
        assert!(Proof::parse(&value).is_none());
    }
}
