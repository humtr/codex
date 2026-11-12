//! Conservative completion proof for an unloaded Native thread. The public
//! history omits transient starts, so terminal items alone are insufficient.
use super::original_tool_results;
use crate::native_claims::completion::{self, Presence};
use crate::protocol::Error;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

fn unsettled() -> Error {
    Error("native_unloaded_effects_unsettled")
}

fn session(value: &Value) -> Result<Option<u64>, Error> {
    match value {
        Value::Null => Ok(None),
        Value::Number(number) => number
            .as_u64()
            .filter(|id| *id <= i32::MAX as u64)
            .map(Some)
            .ok_or_else(unsettled),
        Value::String(text)
            if !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit()) =>
        {
            text.parse::<u64>()
                .ok()
                .filter(|id| *id <= i32::MAX as u64)
                .map(Some)
                .ok_or_else(unsettled)
        }
        _ => Err(unsettled()),
    }
}

fn exit(value: &Value) -> Result<i64, Error> {
    let code = value.as_i64().ok_or_else(unsettled)?;
    // Native synthesizes -1 when no wait result exists and 124 after requesting
    // timeout termination. Neither proves a completed OS wait, even when a
    // matching terminal item exists. Preserve the original result unchanged.
    if !crate::native_claims::confirmed_exit_code(code) {
        return Err(unsettled());
    }
    Ok(code)
}

#[derive(Debug, PartialEq)]
struct Output {
    exit: Option<i64>,
    session: Option<u64>,
}

fn output(value: &Value) -> Result<Output, Error> {
    let text = value.as_str().ok_or_else(unsettled)?;
    if let Ok(value) = completion::strict_json(text.as_bytes()) {
        if !value["output"].is_string()
            || !value
                .get("exit_code")
                .is_none_or(|v| v.is_null() || v.is_i64())
            || !value
                .get("session_id")
                .is_none_or(|v| v.is_null() || v.as_u64().is_some_and(|id| id <= i32::MAX as u64))
        {
            return Err(unsettled());
        }
        let result = Output {
            exit: value["exit_code"].as_i64(),
            session: value["session_id"].as_u64(),
        };
        if result.exit.is_none() && result.session.is_none() {
            return Err(unsettled());
        }
        return Ok(result);
    }
    // Parse Native's envelope only, never text written by the child command.
    let (header, _) = text.split_once("\nOutput:\n").ok_or_else(unsettled)?;
    let mut result = Output {
        exit: None,
        session: None,
    };
    for line in header.lines() {
        if let Some(code) = line.strip_prefix("Process exited with code ") {
            if result
                .exit
                .replace(code.parse().map_err(|_| unsettled())?)
                .is_some()
            {
                return Err(unsettled());
            }
        } else if let Some(id) = line.strip_prefix("Process running with session ID ") {
            if result
                .session
                .replace(id.parse().map_err(|_| unsettled())?)
                .is_some()
            {
                return Err(unsettled());
            }
        } else if line.starts_with("Native completion: ") {
            continue;
        } else if ![
            "Chunk ID: ",
            "Wall time: ",
            "Original token count: ",
            "Final output:",
        ]
        .iter()
        .any(|prefix| line.starts_with(prefix))
        {
            return Err(unsettled());
        }
    }
    if result.exit.is_none() && result.session.is_none() {
        return Err(unsettled());
    }
    Ok(result)
}

pub(super) fn validate(
    thread: &str,
    expected_turn: &str,
    history: &Value,
    rows: &[Value],
    receipts: &super::UnloadedReceipts,
) -> Result<(), Error> {
    let saved = &history["thread"];
    let turns = saved["turns"].as_array().ok_or_else(unsettled)?;
    if saved["id"] != thread
        || saved["status"]["type"] != "notLoaded"
        || !turns.last().is_some_and(|turn| {
            turn["id"] == expected_turn && turn["status"] == "completed" && turn["error"].is_null()
        })
    {
        return Err(unsettled());
    }
    let mut turn_ids = BTreeSet::new();
    let mut commands = BTreeMap::new();
    for turn in turns {
        let turn_id = turn["id"].as_str().ok_or_else(unsettled)?;
        if !turn_ids.insert(turn_id)
            || !matches!(
                turn["status"].as_str(),
                Some("completed" | "failed" | "interrupted")
            )
        {
            return Err(unsettled());
        }
        for item in turn["items"].as_array().ok_or_else(unsettled)? {
            if item["type"] != "commandExecution" {
                continue;
            }
            let id = item["id"].as_str().ok_or_else(unsettled)?;
            if !item["exitCode"].is_i64()
                || !matches!(
                    item["status"].as_str(),
                    Some("completed" | "failed" | "declined")
                )
                || !item
                    .get("aggregatedOutput")
                    .is_some_and(|v| v.is_null() || v.is_string())
                || commands.insert(id, (turn_id, item)).is_some()
            {
                return Err(unsettled());
            }
        }
    }
    let witnesses = completion::end_witnesses(rows).map_err(|_| unsettled())?;
    let mut call_turns = BTreeMap::new();
    let mut raw_turn = String::new();
    let mut raw_commands = BTreeMap::new();
    let mut calls = BTreeMap::new();
    let mut order = Vec::new();
    let mut result_ids = BTreeSet::new();
    for row in rows {
        let item = &row["payload"];
        if row["type"] == "turn_context"
            || row["type"] == "event_msg" && item["type"] == "task_started"
        {
            raw_turn = item["turn_id"].as_str().unwrap_or("").to_owned();
        }
        if row["type"] == "event_msg"
            && item["type"] == "item_completed"
            && item["item"]["type"] == "CommandExecution"
        {
            let command = &item["item"];
            let id = command["id"].as_str().ok_or_else(unsettled)?;
            let Some((turn, public)) = commands.get(id) else {
                return Err(unsettled());
            };
            if item["thread_id"] != thread
                || item["turn_id"] != *turn
                || command["status"] != public["status"]
                || command["exit_code"] != public["exitCode"]
                || session(&command["process_id"])? != session(&public["processId"])?
                || raw_commands.insert(id, item).is_some()
            {
                return Err(unsettled());
            }
        }
        if row["type"] != "response_item" {
            continue;
        }
        match item["type"].as_str() {
            Some("function_call" | "custom_tool_call" | "tool_search_call") => {
                let id = item["call_id"]
                    .as_str()
                    .filter(|id| !id.is_empty())
                    .ok_or_else(unsettled)?;
                if calls.insert(id.to_owned(), item.clone()).is_some() {
                    return Err(unsettled());
                }
                call_turns.insert(id.to_owned(), raw_turn.clone());
                order.push(id.to_owned());
            }
            Some("function_call_output" | "custom_tool_call_output" | "tool_search_output") => {
                let id = item["call_id"].as_str().ok_or_else(unsettled)?;
                if !result_ids.insert(id.to_owned()) {
                    return Err(unsettled());
                }
            }
            Some(
                "message" | "reasoning" | "compaction" | "compaction_summary"
                | "context_compaction" | "compaction_trigger" | "additional_tools",
            ) => (),
            // Unknown/custom/local-shell envelopes cannot establish absence of
            // a detached session through this narrow read-only fallback.
            _ => return Err(unsettled()),
        }
    }
    if raw_commands.len() != commands.len() || result_ids != calls.keys().cloned().collect() {
        return Err(unsettled());
    }
    let results: BTreeMap<_, _> = original_tool_results(rows, &calls)?.into_iter().collect();
    let mut sessions = BTreeMap::new();
    let mut matched = BTreeSet::new();
    for id in order {
        let call = &calls[&id];
        let result = &results[&id];
        if call["type"] == "tool_search_call" {
            if result["status"] != "completed" {
                return Err(unsettled());
            }
            continue;
        }
        if receipts.non_commands.contains(&id) {
            if commands.contains_key(id.as_str()) {
                return Err(unsettled());
            }
            continue;
        }
        if call["type"] != "function_call"
            || !call["namespace"].is_null() && call["namespace"] != "functions"
        {
            return Err(unsettled());
        }
        let response = completion::result_text(result)
            .and_then(|text| output(&Value::String(text.into())).ok());
        let presence = completion::result_proof(result);
        if presence == Presence::Invalid {
            return Err(unsettled());
        }
        match call["name"].as_str() {
            Some("exec_command" | "shell" | "shell_command") => {
                let command = commands.get(id.as_str()).copied();
                let event = witnesses.get(&id);
                if event.is_some_and(|event| {
                    event.turn_id != call_turns[&id]
                        || !turn_ids.contains(event.turn_id.as_str())
                        || raw_commands
                            .get(id.as_str())
                            .is_some_and(|raw| !event.matches_canonical_command(raw))
                }) {
                    return Err(unsettled());
                }
                let function = crate::native_claims::process_observation(result, &id, None);
                let mut proof = function.proof;
                if let Some(event) = event {
                    match &event.proof {
                        Presence::Present(terminal) => {
                            if proof.as_ref().is_some_and(|prior| prior != terminal) {
                                return Err(unsettled());
                            }
                            proof = Some(terminal.clone());
                        }
                        Presence::Invalid => return Err(unsettled()),
                        _ => (),
                    }
                }
                if let Some(proof) = proof {
                    if !proof.belongs_to(&id, None) {
                        return Err(unsettled());
                    }
                    if proof.not_started() {
                        if response
                            .as_ref()
                            .is_some_and(|output| output.session.is_some())
                        {
                            return Err(unsettled());
                        }
                        if let Some((_, item)) = command {
                            if session(&item["processId"])?.is_some()
                                || !matches!(item["status"].as_str(), Some("failed" | "declined"))
                            {
                                return Err(unsettled());
                            }
                            matched.insert(id.clone());
                        }
                        continue;
                    }
                    if !proof.wait_completed() {
                        return Err(unsettled());
                    }
                    let code = proof.waited();
                    if let Some(response) = &response {
                        if response
                            .session
                            .is_some_and(|session| proof.session_id != Some(session))
                            || response
                                .exit
                                .filter(|code| crate::native_claims::confirmed_exit_code(*code))
                                .is_some_and(|original| {
                                    code.is_some_and(|actual| original != actual)
                                })
                        {
                            return Err(unsettled());
                        }
                    }
                    if let Some((turn, item)) = command {
                        if event.is_some_and(|event| event.turn_id != turn)
                            || session(&item["processId"])? != proof.session_id
                        {
                            return Err(unsettled());
                        }
                        matched.insert(id.clone());
                    }
                    if let Some(process) = proof.session_id {
                        if sessions.insert(process, code).is_some() {
                            return Err(unsettled());
                        }
                    }
                    continue;
                }
                // Neither a new unknown/unsupported result nor a new raw
                // unknown End can borrow an older success-code interpretation.
                if presence != Presence::Absent
                    || event.is_some()
                    || !receipts.legacy_processes.contains(&id)
                {
                    return Err(unsettled());
                }
                let response = response.ok_or_else(unsettled)?;
                let (_, command) = command.ok_or_else(unsettled)?;
                let code = exit(&command["exitCode"])?;
                if command["status"] != if code == 0 { "completed" } else { "failed" } {
                    return Err(unsettled());
                }
                let process = session(&command["processId"])?;
                if response.exit.is_some_and(|exit| exit != code)
                    || response.session.is_some_and(|id| process != Some(id))
                    || !matched.insert(id.clone())
                {
                    return Err(unsettled());
                }
                if let Some(process) = process {
                    if sessions.insert(process, Some(code)).is_some() {
                        return Err(unsettled());
                    }
                }
            }
            Some("write_stdin") => {
                let arguments: Value = completion::strict_json(
                    call["arguments"].as_str().ok_or_else(unsettled)?.as_bytes(),
                )
                .map_err(|_| unsettled())?;
                let process = arguments["session_id"]
                    .as_u64()
                    .filter(|id| *id <= i32::MAX as u64)
                    .ok_or_else(unsettled)?;
                let code = *sessions.get(&process).ok_or_else(unsettled)?;
                let observed =
                    crate::native_claims::process_observation(result, &id, Some(process));
                if observed.not_spawned {
                    // This exact poll was rejected before hooks/dispatch. Its
                    // original session still needed the terminal proof above.
                    if commands.contains_key(id.as_str()) {
                        return Err(unsettled());
                    }
                    continue;
                }
                if observed.proof.is_some() {
                    if observed.running != Some(process) && !observed.terminal
                        || observed.terminal
                            && observed
                                .exit_code
                                .zip(code)
                                .is_some_and(|(actual, original)| actual != original)
                    {
                        return Err(unsettled());
                    }
                } else {
                    let response = response.ok_or_else(unsettled)?;
                    if !receipts.legacy_processes.contains(&id) || presence != Presence::Absent {
                        return Err(unsettled());
                    }
                    if response
                        .exit
                        .zip(code)
                        .is_some_and(|(exit, original)| exit != original)
                        || response.session.is_some_and(|id| id != process)
                    {
                        return Err(unsettled());
                    }
                }
            }
            _ => return Err(unsettled()),
        }
    }
    if matched.len() != commands.len() {
        return Err(unsettled());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn closed() -> super::super::UnloadedReceipts {
        super::super::UnloadedReceipts {
            non_commands: BTreeSet::new(),
            legacy_processes: BTreeSet::from(["exec".into(), "poll".into()]),
        }
    }
    #[test]
    fn unloaded_legacy_command_needs_original_receipt_hook_proof() {
        let (history, rows) = fixture();
        assert!(validate(
            "thread",
            "final",
            &history,
            &rows,
            &super::super::UnloadedReceipts::default()
        )
        .is_err());
        validate("thread", "final", &history, &rows, &closed()).unwrap();
    }
    fn fixture() -> (Value, Vec<Value>) {
        let command = json!({"id":"exec","type":"commandExecution","processId":"41",
            "status":"completed","exitCode":0,"aggregatedOutput":null});
        let history = json!({"thread":{"id":"thread","status":{"type":"notLoaded"},
            "turns":[{"id":"first","status":"completed","error":null,"items":[command]},
            {"id":"final","status":"completed","error":null,"items":[]}]}});
        let rows = vec![
            json!({"type":"session_meta","payload":{"id":"thread"}}),
            json!({"type":"response_item","payload":{"type":"function_call","call_id":"exec",
                "name":"exec_command","arguments":"{\"cmd\":\"owned compiler\"}"}}),
            json!({"type":"response_item","payload":{"type":"function_call_output","call_id":"exec",
                "output":"Chunk ID: original\nWall time: 0.1 seconds\nProcess running with session ID 41\nOutput:\n"}}),
            json!({"type":"event_msg","payload":{"type":"item_completed","thread_id":"thread","turn_id":"first",
                "item":{"type":"CommandExecution","id":"exec","process_id":41,"status":"completed","exit_code":0}}}),
            json!({"type":"response_item","payload":{"type":"function_call","call_id":"poll","name":"write_stdin",
                "arguments":"{\"session_id\":41,\"chars\":\"\"}"}}),
            json!({"type":"response_item","payload":{"type":"function_call_output","call_id":"poll",
                "output":"Chunk ID: original\nWall time: 0.1 seconds\nProcess exited with code 0\nOutput:\n"}}),
        ];
        (history, rows)
    }

    #[test]
    fn unloaded_current_poll_needs_its_own_positive_proof_even_after_original_wait() {
        let (history, mut rows) = fixture();
        rows[5]["payload"]["output"] =
            json!("Native completion: null\nProcess running with session ID 41\nOutput:\n");
        assert!(
            validate("thread", "final", &history, &rows, &closed()).is_err(),
            "new unknown invocation cannot borrow saved legacy hook facts"
        );
        for outcome in ["session_running", "pre_backend_rejected"] {
            let proof = json!({"schema":"codex-native-completion-v1","call_id":"poll",
                "session_id":if outcome == "session_running" {json!(41)} else {Value::Null},
                "outcome":outcome,"exit_code":null,"hooks":"absent"});
            let status = if outcome == "session_running" {
                "Process running with session ID 41\n"
            } else {
                ""
            };
            rows[5]["payload"]["output"] =
                json!(format!("Native completion: {proof}\n{status}Output:\n"));
            validate("thread", "final", &history, &rows, &closed()).unwrap();
            let mut active = history.clone();
            active["thread"]["turns"][0]["items"][0]["status"] = json!("inProgress");
            assert!(validate("thread", "final", &active, &rows, &closed()).is_err());
        }
    }

    #[test]
    fn unloaded_typed_wait_preserves_public_timeout_and_original_result() {
        let (mut history, mut rows) = fixture();
        rows.insert(
            1,
            json!({"type":"turn_context","payload":{"turn_id":"first"}}),
        );
        history["thread"]["turns"][0]["items"][0]["exitCode"] = json!(124);
        history["thread"]["turns"][0]["items"][0]["status"] = json!("failed");
        rows[4]["payload"]["item"]["exit_code"] = json!(124);
        rows[4]["payload"]["item"]["status"] = json!("failed");
        rows[4]["payload"]["item"]["aggregated_output"] = json!("original timeout");
        rows[3]["payload"]["output"] = json!(
            "Native completion: null\nProcess exited with code 124\nOutput:\noriginal timeout"
        );
        // No poll result is fabricated; this one-shot call's actual waiter is
        // a separate immutable End witness after the public timeout.
        rows.truncate(5);
        let end = json!({"type":"event_msg","payload":{"type":"exec_command_end","call_id":"exec",
            "turn_id":"first","process_id":"41","exit_code":124,"status":"failed",
            "aggregated_output":"original timeout","native_completion":null}});
        rows.push(end.clone());
        let before = rows.clone();
        assert!(validate("thread", "final", &history, &rows, &closed()).is_err());
        let mut late = end;
        late["payload"]["native_completion"] = json!({"schema":"codex-native-completion-v1",
            "call_id":"exec","session_id":41,"outcome":"wait_completed","exit_code":137,"hooks":"absent"});
        rows.push(late.clone());
        validate("thread", "final", &history, &rows, &closed()).unwrap();
        let mut platform_signal = rows.clone();
        platform_signal.last_mut().unwrap()["payload"]["native_completion"]["exit_code"] =
            Value::Null;
        validate("thread", "final", &history, &platform_signal, &closed()).unwrap();
        assert_eq!(&platform_signal[..before.len()], before.as_slice());
        let mut missing_numeric_key = platform_signal.clone();
        missing_numeric_key.last_mut().unwrap()["payload"]["native_completion"]
            .as_object_mut()
            .unwrap()
            .remove("exit_code");
        assert!(validate("thread", "final", &history, &missing_numeric_key, &closed()).is_err());
        assert_eq!(&rows[..before.len()], before.as_slice());
        for fault in [
            "unknown",
            "wrong_session",
            "wrong_turn",
            "duplicate",
            "changed_payload",
            "changed_canonical_output",
            "changed_canonical_command",
        ] {
            let mut bad = rows.clone();
            match fault {
                "unknown" => {
                    bad.last_mut().unwrap()["payload"]["native_completion"]["schema"] =
                        json!("unknown")
                }
                "wrong_session" => {
                    bad.last_mut().unwrap()["payload"]["native_completion"]["session_id"] =
                        json!(42)
                }
                "wrong_turn" => bad.last_mut().unwrap()["payload"]["turn_id"] = json!("other"),
                "duplicate" => bad.push(late.clone()),
                "changed_payload" => {
                    bad.last_mut().unwrap()["payload"]["aggregated_output"] = json!("changed")
                }
                "changed_canonical_output" => {
                    bad[4]["payload"]["item"]["aggregated_output"] = json!("foreign")
                }
                "changed_canonical_command" => {
                    bad[4]["payload"]["item"]["command"] = json!(["foreign"])
                }
                _ => unreachable!(),
            }
            assert!(
                validate("thread", "final", &history, &bad, &closed()).is_err(),
                "{fault}"
            );
        }
    }
    #[test]
    fn unloaded_new_unknown_does_not_reuse_legacy_zero_item() {
        let (history, mut rows) = fixture();
        rows[2]["payload"]["output"] =
            json!("Native completion: null\nProcess running with session ID 41\nOutput:\n");
        assert!(validate("thread", "final", &history, &rows, &closed()).is_err());
        rows[2]["payload"]["output"] =
            json!("Native completion: null\nProcess exited with code 0\nOutput:\n");
        assert!(validate("thread", "final", &history, &rows, &closed()).is_err());
    }
    #[test]
    fn unloaded_terminal_proof_binds_original_session_results_and_final_turn() {
        let (history, rows) = fixture();
        validate("thread", "final", &history, &rows, &closed()).unwrap();
        assert!(validate("other", "final", &history, &rows, &closed()).is_err());
        assert!(validate("thread", "old", &history, &rows, &closed()).is_err());
        let mut foreign = rows.clone();
        foreign[4]["payload"]["arguments"] = json!("{\"session_id\":42}");
        assert!(validate("thread", "final", &history, &foreign, &closed()).is_err());
        let mut missing = rows.clone();
        missing.remove(2);
        assert!(validate("thread", "final", &history, &missing, &closed()).is_err());
    }

    #[test]
    fn omitted_transient_start_and_other_historical_command_remain_unsettled() {
        let (history, rows) = fixture();
        let mut hidden = history.clone();
        hidden["thread"]["turns"][0]["items"] = json!([]);
        let no_end: Vec<_> = rows
            .iter()
            .filter(|row| row["type"] != "event_msg")
            .cloned()
            .collect();
        assert!(validate("thread", "final", &hidden, &no_end, &closed()).is_err());
        let mut mixed = rows.clone();
        mixed.push(
            json!({"type":"response_item","payload":{"type":"function_call","call_id":"orphan",
            "name":"exec_command","arguments":"{\"cmd\":\"still running\"}"}}),
        );
        mixed.push(json!({"type":"response_item","payload":{"type":"function_call_output","call_id":"orphan",
            "output":"Wall time: 0.1 seconds\nProcess running with session ID 99\nOutput:\n"}}));
        assert!(validate("thread", "final", &history, &mixed, &closed()).is_err());
    }

    #[test]
    fn ordinary_nonzero_is_terminal_but_synthetic_wait_codes_and_output_text_are_not() {
        let (history, rows) = fixture();
        for code in [7, -1, 124] {
            let mut history = history.clone();
            let mut rows = rows.clone();
            history["thread"]["turns"][0]["items"][0]["exitCode"] = json!(code);
            history["thread"]["turns"][0]["items"][0]["status"] = json!("failed");
            rows[3]["payload"]["item"]["exit_code"] = json!(code);
            rows[3]["payload"]["item"]["status"] = json!("failed");
            rows[5]["payload"]["output"] = json!(format!(
                "Wall time: 0 seconds\nProcess exited with code {code}\nOutput:\n"
            ));
            assert_eq!(
                validate("thread", "final", &history, &rows, &closed()).is_ok(),
                code == 7
            );
        }
        assert!(output(&json!(
            "Wall time: 0 seconds\nOutput:\nProcess exited with code 0"
        ))
        .is_err());
        assert_eq!(output(&json!("Wall time: 0 seconds\nProcess running with session ID 41\nOutput:\nProcess exited with code 0")).unwrap(),
            Output { exit:None, session:Some(41) });
    }

    #[test]
    fn historical_in_progress_unknown_calls_and_reused_sessions_are_not_absence() {
        let (history, rows) = fixture();
        let mut in_progress = history.clone();
        in_progress["thread"]["turns"][0]["status"] = json!("inProgress");
        assert!(validate("thread", "final", &in_progress, &rows, &closed()).is_err());
        let mut unknown = rows.clone();
        unknown[1]["payload"]["name"] = json!("unknown_background_tool");
        assert!(validate("thread", "final", &history, &unknown, &closed()).is_err());
        let mut repeated = rows.clone();
        repeated.extend(rows[1..4].iter().cloned());
        assert!(validate("thread", "final", &history, &repeated, &closed()).is_err());
        let mut wrong = rows.clone();
        wrong[3]["payload"]["item"]["process_id"] = json!(42);
        assert!(validate("thread", "final", &history, &wrong, &closed()).is_err());
    }
    #[test]
    fn unloaded_code_mode_optional_fields_follow_actual_native_projection() {
        for (value, expected) in [
            (
                json!({"wall_time_seconds":0.1,"exit_code":0,"output":""}),
                Output {
                    exit: Some(0),
                    session: None,
                },
            ),
            (
                json!({"wall_time_seconds":0.1,"session_id":41,"output":""}),
                Output {
                    exit: None,
                    session: Some(41),
                },
            ),
            (
                json!({"wall_time_seconds":0.1,"session_id":null,"exit_code":7,"output":""}),
                Output {
                    exit: Some(7),
                    session: None,
                },
            ),
        ] {
            assert_eq!(output(&json!(value.to_string())).unwrap(), expected);
        }
        for bad in [
            json!({"output":""}),
            json!({"output":"","exit_code":null,"session_id":null}),
            json!({"output":"","exit_code":"0"}),
            json!({"output":"","session_id":-1}),
        ] {
            assert!(output(&json!(bad.to_string())).is_err());
        }
        let (history, mut rows) = fixture();
        rows[2]["payload"]["output"] = json!(json!({"output":"","session_id":41}).to_string());
        rows[5]["payload"]["output"] = json!(json!({"output":"","exit_code":0}).to_string());
        validate("thread", "final", &history, &rows, &closed()).unwrap();
    }

    #[test]
    fn unloaded_malformed_process_identity_never_collapses_to_safe_null() {
        let (history, rows) = fixture();
        for (raw, public) in [
            (json!("bad-a"), json!("bad-b")),
            (json!(-1), Value::Null),
            (json!("18446744073709551616"), json!("18446744073709551616")),
            (json!({}), json!({})),
            (json!(41), json!("42")),
        ] {
            let mut history = history.clone();
            let mut rows = rows.clone();
            rows[3]["payload"]["item"]["process_id"] = raw;
            history["thread"]["turns"][0]["items"][0]["processId"] = public;
            assert!(validate("thread", "final", &history, &rows, &closed()).is_err());
        }
    }
}
