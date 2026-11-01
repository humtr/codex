//! Threadless local I/O. Core owns permissions/processes; these records recover
//! operations and output, never Native conversations or model responses.
use crate::{
    config::Config,
    context::digest,
    journal::{private_directory, Journal},
    native_claims::Guard as SharedNativeGuard,
    operation::{self, Operations},
    operator_native::{Native, Pool},
    protocol::Error,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::sync::{mpsc, Mutex, Notify};

// Pages bound the tool reply, not captured process output.
const PAGE_BYTES: usize = 32 * 1024;

struct Output {
    stdout: File,
    stderr: File,
}

struct Process {
    native: Arc<Native>,
    changed: Arc<Notify>,
    effect_root: PathBuf,
}
pub(crate) struct Local {
    pool: Arc<Pool>,
    journal: Arc<Journal>,
    context: Value,
    operations: Arc<Operations>,
    processes: Mutex<BTreeMap<String, Arc<Process>>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Execute {
    context_id: String,
    call_id: String,
    command: Vec<String>,
    cwd: Option<String>,
    env: Option<BTreeMap<String, Option<String>>>,
    #[serde(default)]
    tty: bool,
    #[serde(default)]
    stream_stdin: bool,
    timeout_ms: Option<u64>,
    #[serde(default)]
    disable_timeout: bool,
    size: Option<Size>,
}
#[derive(Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
struct Size {
    rows: u16,
    cols: u16,
}
#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
enum Control {
    Read {
        context_id: String,
        process_id: String,
        #[serde(default)]
        stdout_offset: u64,
        #[serde(default)]
        stderr_offset: u64,
        #[serde(default)]
        wait_ms: u64,
    },
    Write {
        context_id: String,
        process_id: String,
        call_id: String,
        delta_base64: Option<String>,
        #[serde(default)]
        close_stdin: bool,
    },
    Resize {
        context_id: String,
        process_id: String,
        call_id: String,
        size: Size,
    },
    Terminate {
        context_id: String,
        process_id: String,
        call_id: String,
    },
}

impl Local {
    pub(crate) fn new(
        pool: Arc<Pool>,
        journal: Arc<Journal>,
        config: &Config,
    ) -> Option<Arc<Self>> {
        let profile = config.execution.standalone_permission_profile.as_ref()?;
        let context_id = digest(&(
            "standalone-context-v1",
            &journal.owner,
            &pool.home,
            &pool.workspace,
            profile,
        ));
        Some(Arc::new(Self {
            context: json!({"context_id":context_id,"workflow":"standalone",
                "workspace":pool.workspace,"permission_profile":profile,
                "permissions":"configured Core profile; cwd does not restrict file access; no Native turn approval/reviewer",
                "instructions":"Read and follow applicable workspace/ancestor and nested AGENTS.md before effects. Use local_execute for files, patches, commands and tests under this context. Return full public progress and results in this Chat. Do not create Native work, call Native tools or send codex_report/codex_finish for standalone development. Select integrated workflow explicitly when the user requests shared Native work or orchestration."}),
            pool,
            journal,
            operations: Arc::new(Operations::default()),
            processes: Mutex::new(BTreeMap::new()),
        }))
    }
    fn claim_thread(&self) -> String {
        format!(
            "standalone:{}",
            self.context["context_id"].as_str().unwrap()
        )
    }
    fn shared_guard_home(&self) -> Option<PathBuf> {
        // Unit tests share the Broker's private ledger; production shares the
        // account-level OS ledger with Direct MCP and AppHost tools.
        #[cfg(test)]
        {
            Some(self.journal.root.join("native-shared-test"))
        }
        #[cfg(not(test))]
        {
            None
        }
    }
    fn guard_error(reason: String) -> Error {
        match reason.as_str() {
            "native_resource_busy" => Error("native_resource_busy"),
            "native_resource_version_conflict" => Error("native_resource_version_conflict"),
            "native_effect_duplicate_unconfirmed" => Error("native_effect_duplicate_unconfirmed"),
            "native_effect_identity_changed" => Error("native_effect_identity_changed"),
            _ => Error("native_resource_shared_guard_failed"),
        }
    }
    fn reserve_effect(&self, cwd: &Path, id: &str, fingerprint: &str) -> Result<(), Error> {
        SharedNativeGuard::offer_external(
            cwd,
            self.shared_guard_home().as_deref(),
            &self.claim_thread(),
            id,
            "local_execute",
            fingerprint,
        )
        .map_err(Self::guard_error)
    }
    fn abort_effect(&self, cwd: &Path, id: &str, fingerprint: &str) -> Result<(), Error> {
        SharedNativeGuard::abort_unoffered(
            cwd,
            self.shared_guard_home().as_deref(),
            &self.claim_thread(),
            id,
            "local_execute",
            fingerprint,
        )
        .map_err(Self::guard_error)
    }
    fn settle_effect(&self, cwd: &Path, id: &str, result: &Value) -> Result<(), Error> {
        // Neither cancellation ACK, rejected admission, connection loss nor
        // an unparseable Core reply proves that the writer process exited.
        if result["state"] != "exited" {
            return Ok(());
        }
        SharedNativeGuard::settle_external_receipt(
            cwd,
            self.shared_guard_home().as_deref(),
            &self.claim_thread(),
            id,
            "local_execute",
            None,
            result,
        )
        .map_err(Self::guard_error)
    }
    fn verify_context(&self, id: &str) -> Result<(), Error> {
        if self.context["context_id"].as_str() != Some(id) {
            return Err(Error("standalone_context_changed"));
        }
        Ok(())
    }
    fn call_id(id: &str) -> Result<(), Error> {
        if id.is_empty() || id.len() > 256 {
            return Err(Error("standalone_call_id_invalid"));
        }
        Ok(())
    }
    fn output(&self, process: &str, stream: &str) -> std::path::PathBuf {
        self.journal
            .root
            .join("local-output")
            .join(format!("{process}.{stream}"))
    }
    fn record(&self, process: &str) -> Result<Value, Error> {
        let record = self
            .journal
            .read::<Value>(process)?
            .ok_or(Error("standalone_process_unavailable"))?;
        if record["context"]["context_id"] != self.context["context_id"] {
            return Err(Error("standalone_context_changed"));
        }
        Ok(record)
    }
    fn page(&self, process: &str, stream: &str, offset: u64) -> Result<Value, Error> {
        let mut file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(self.output(process, stream))
            .map_err(|_| Error("standalone_output_unavailable"))?;
        let meta = file
            .metadata()
            .map_err(|_| Error("standalone_output_unavailable"))?;
        if !meta.is_file() || meta.uid() != unsafe { libc::geteuid() } || meta.mode() & 0o077 != 0 {
            return Err(Error("standalone_output_not_private"));
        }
        if offset > meta.len() {
            return Err(Error("standalone_output_offset_invalid"));
        }
        file.seek(SeekFrom::Start(offset))
            .map_err(|_| Error("standalone_output_unavailable"))?;
        let mut bytes = vec![0; PAGE_BYTES];
        let count = file
            .read(&mut bytes)
            .map_err(|_| Error("standalone_output_unavailable"))?;
        bytes.truncate(count);
        Ok(
            json!({"text":std::str::from_utf8(&bytes).ok(),"base64":STANDARD.encode(&bytes),
            "offset":offset,"next_offset":offset+count as u64,"more":(offset+count as u64)<meta.len()}),
        )
    }
    async fn status(&self, process: &str, out: u64, err: u64) -> Result<Value, Error> {
        let record = self.record(process)?;
        // Reconcile a crash after the result was journaled but before the
        // physical claim was released. No command is dispatched again.
        if let (Some(result), Some(root)) = (
            record.get("result"),
            record["context"]["effect_workspace"].as_str(),
        ) {
            self.settle_effect(Path::new(root), process, result)?;
        }
        let active = self.processes.lock().await.contains_key(process);
        Ok(
            json!({"context_id":self.context["context_id"],"process_id":process,
            "state":record.get("result").map(|v|v["state"].clone())
                .unwrap_or_else(||json!(if active {"submitted"} else {"delivery_unknown"})),
            "completion":record.get("result"),
            "stdout":self.page(process,"stdout",out)?,"stderr":self.page(process,"stderr",err)?}),
        )
    }
    pub(crate) async fn call(self: &Arc<Self>, name: &str, args: Value) -> Result<Value, Error> {
        match name {
            "local_context" => Ok(self.context.clone()),
            "local_execute" => {
                let input: Execute = serde_json::from_value(args.clone())
                    .map_err(|_| Error("operator_arguments_invalid"))?;
                self.verify_context(&input.context_id)?;
                Self::call_id(&input.call_id)?;
                if input.command.is_empty()
                    || input.command.iter().any(|v| v.contains('\0'))
                    || input
                        .timeout_ms
                        .is_some_and(|v| v == 0 || v > i64::MAX as u64)
                    || input.timeout_ms.is_some() && input.disable_timeout
                    || input
                        .size
                        .as_ref()
                        .is_some_and(|s| !input.tty || s.rows == 0 || s.cols == 0)
                    || input
                        .cwd
                        .as_ref()
                        .is_some_and(|p| !std::path::Path::new(p).is_absolute())
                {
                    return Err(Error("operator_arguments_invalid"));
                }
                let key = digest(&("standalone-exec-v1", &input.context_id, &input.call_id));
                let fingerprint = digest(&args);
                let this = self.clone();
                self.operations.run(key.clone(),fingerprint.clone(),Box::pin(async move {
                    match operation::recover(&this.journal,&key,&fingerprint,"standalone_delivery_unknown") {
                        Ok(Some(_)) | Err(Error("standalone_delivery_unknown")) => return this.status(&key,0,0).await,
                        Ok(None) => (), Err(error) => return Err(error),
                    }
                    let cwd = input.cwd.clone().unwrap_or_else(||this.pool.workspace.to_string_lossy().into());
                    let effect_root = std::fs::canonicalize(&cwd)
                        .map_err(|_| Error("native_workspace_unavailable"))?;
                    let (sender, receiver) = mpsc::channel(64);
                    let native = this.pool.prepare_io(sender).await?;
                    private_directory(&this.journal.root.join("local-output"))?;
                    let open = |stream| OpenOptions::new().write(true).create_new(true).mode(0o600)
                        .custom_flags(libc::O_NOFOLLOW).open(this.output(&key,stream))
                        .map_err(|_|Error("standalone_output_unavailable"));
                    let stdout = open("stdout")?;
                    let stderr = match open("stderr") {
                        Ok(file) => file,
                        Err(error) => {
                            drop(stdout);
                            let _ = std::fs::remove_file(this.output(&key,"stdout"));
                            native.close();
                            return Err(error);
                        }
                    };
                    // This is the same process-shared reservation used by
                    // Broker-mediated Native tools, acquired before Core sees
                    // command/exec. Unknown argv effects remain opaque.
                    if let Err(error) = this.reserve_effect(&effect_root,&key,&fingerprint) {
                        drop(stdout); drop(stderr);
                        let _ = std::fs::remove_file(this.output(&key,"stdout"));
                        let _ = std::fs::remove_file(this.output(&key,"stderr"));
                        native.close();
                        return Err(error);
                    }
                    let mut params = json!({"command":input.command,"processId":key,
                        "cwd":cwd,
                        "tty":input.tty,"streamStdin":input.stream_stdin,"streamStdoutStderr":true,
                        "disableOutputCap":true,"disableTimeout":input.disable_timeout,
                        "permissionProfile":this.context["permission_profile"]});
                    if let Some(env)=input.env { params["env"]=json!(env); }
                    if let Some(timeout)=input.timeout_ms { params["timeoutMs"]=json!(timeout); }
                    if let Some(size)=input.size { params["size"]=json!(size); }
                    if let Err(error) = operation::intent_with_context(&this.journal,&key,&fingerprint,
                        json!({"context_id":input.context_id,"effect_workspace":effect_root})) {
                        this.abort_effect(&effect_root,&key,&fingerprint)?;
                        native.close();
                        return Err(error);
                    }
                    let process = Arc::new(Process {native,changed:Arc::new(Notify::new()),effect_root});
                    this.processes.lock().await.insert(key.clone(),process.clone());
                    let task=this.clone(); let id=key.clone();
                    tokio::spawn(async move {task.run(id,fingerprint,process,params,receiver,Output{stdout,stderr}).await;});
                    this.status(&key,0,0).await
                })).await
            }
            "local_process" => self.control(args).await,
            _ => Err(Error("mcp_tool_unknown")),
        }
    }
    async fn run(
        self: Arc<Self>,
        id: String,
        fingerprint: String,
        process: Arc<Process>,
        params: Value,
        mut output: mpsc::Receiver<Value>,
        mut files: Output,
    ) {
        let result = async {
            let reply = process.native.request("command/exec", params);
            tokio::pin!(reply);
            let settled = loop {
                tokio::select! {
                    biased;
                    event=output.recv()=> match event {
                        Some(event)=>{files.capture(&id,event)?;process.changed.notify_waiters();},
                        None=>break reply.await?,
                    },
                    result=&mut reply=>break result?,
                }
            };
            // The transport enqueues every output delta before the final reply.
            while let Ok(event) = output.try_recv() {
                files.capture(&id, event)?;
            }
            files
                .stdout
                .sync_data()
                .and_then(|_| files.stderr.sync_data())
                .map_err(|_| Error("standalone_output_write_failed"))?;
            Ok::<_, Error>(completion(settled))
        }
        .await;
        process.native.close();
        let result =
            result.unwrap_or_else(|error| json!({"state":"delivery_unknown","error":error.0}));
        match operation::settle(&self.journal, &id, &fingerprint, &result) {
            Ok(()) => {
                if let Err(error) = self.settle_effect(&process.effect_root, &id, &result) {
                    // The settled journal retains enough identity to retry
                    // release on the next local_process observation.
                    eprintln!("standalone_resource_settlement_failed: {}", error.0);
                }
            }
            Err(error) => eprintln!("standalone_result_persistence_failed: {}", error.0),
        }
        self.processes.lock().await.remove(&id);
        process.changed.notify_waiters();
    }
    async fn control(self: &Arc<Self>, args: Value) -> Result<Value, Error> {
        let control: Control = serde_json::from_value(args.clone())
            .map_err(|_| Error("operator_arguments_invalid"))?;
        let (context, process) = match &control {
            Control::Read {
                context_id,
                process_id,
                ..
            }
            | Control::Write {
                context_id,
                process_id,
                ..
            }
            | Control::Resize {
                context_id,
                process_id,
                ..
            }
            | Control::Terminate {
                context_id,
                process_id,
                ..
            } => (context_id.clone(), process_id.clone()),
        };
        self.verify_context(&context)?;
        self.record(&process)?;
        if let Control::Read {
            stdout_offset,
            stderr_offset,
            wait_ms,
            ..
        } = control
        {
            if wait_ms > 20_000 {
                return Err(Error("operator_arguments_invalid"));
            }
            let active = self.processes.lock().await.get(&process).cloned();
            if let Some(active) = active.filter(|_| wait_ms > 0) {
                let notified = active.changed.notified();
                tokio::pin!(notified);
                notified.as_mut().enable();
                let status = self.status(&process, stdout_offset, stderr_offset).await?;
                if status["state"] == "submitted"
                    && status["stdout"]["next_offset"] == stdout_offset
                    && status["stderr"]["next_offset"] == stderr_offset
                {
                    let _ = tokio::time::timeout(Duration::from_millis(wait_ms), notified).await;
                }
            }
            return self.status(&process, stdout_offset, stderr_offset).await;
        }
        let (call, method, params) = match control {
            Control::Write {
                call_id,
                delta_base64,
                close_stdin,
                ..
            } => {
                if delta_base64.is_none() && !close_stdin {
                    return Err(Error("operator_arguments_invalid"));
                }
                if delta_base64
                    .as_ref()
                    .is_some_and(|v| STANDARD.decode(v).is_err())
                {
                    return Err(Error("operator_arguments_invalid"));
                }
                (
                    call_id,
                    "command/exec/write",
                    json!({"processId":process,"deltaBase64":delta_base64,"closeStdin":close_stdin}),
                )
            }
            Control::Resize { call_id, size, .. } => {
                if size.rows == 0 || size.cols == 0 {
                    return Err(Error("operator_arguments_invalid"));
                }
                (
                    call_id,
                    "command/exec/resize",
                    json!({"processId":process,"size":size}),
                )
            }
            Control::Terminate { call_id, .. } => (
                call_id,
                "command/exec/terminate",
                json!({"processId":process}),
            ),
            Control::Read { .. } => unreachable!(),
        };
        Self::call_id(&call)?;
        let key = digest(&("standalone-control-v1", &context, &process, call));
        let fingerprint = digest(&args);
        let this = self.clone();
        let process = process.clone();
        self.operations.run(key.clone(),fingerprint.clone(),Box::pin(async move {
            if let Some(result)=operation::recover(&this.journal,&key,&fingerprint,"standalone_control_delivery_unknown")? {return Ok(result);}
            let active=this.processes.lock().await.get(&process).cloned().ok_or(Error("standalone_process_not_active"))?;
            operation::intent(&this.journal,&key,&fingerprint)?;
            let result=match active.native.request(method,params).await {
                Ok(reply)=>json!({"process_id":process,"native_reply":reply,"exit_confirmed":false}),
                Err(error)=>json!({"process_id":process,"delivery_unknown":true,"error":error.0,"exit_confirmed":false}),
            };
            operation::settle(&this.journal,&key,&fingerprint,&result)?;Ok(result)
        })).await
    }
    pub(crate) async fn close(&self) {
        for process in self.processes.lock().await.values() {
            process.native.close();
        }
    }
}
// A deferred reply must actually match Core's streaming-command completion
// contract. A malformed/changed upstream reply cannot establish process exit.
fn completion(reply: Value) -> Value {
    let result = reply.get("result");
    let error = reply.get("error");
    let state = if result.is_none()
        && error.is_some_and(|e| e["code"].as_i64().is_some() && e["message"].is_string())
    {
        "rejected"
    } else if error.is_none()
        && result.is_some_and(|r| {
            r["exitCode"]
                .as_i64()
                .is_some_and(|n| i32::try_from(n).is_ok())
                && r["stdout"].as_str() == Some("")
                && r["stderr"].as_str() == Some("")
        })
    {
        "exited"
    } else {
        "delivery_unknown"
    };
    let mut value = json!({"state":state,"native_reply":reply});
    if state == "delivery_unknown" {
        value["error"] = json!("native_command_reply_invalid");
    }
    value
}

impl Output {
    fn capture(&mut self, id: &str, event: Value) -> Result<(), Error> {
        let params = &event["params"];
        if params["processId"] != id || params["capReached"] != false {
            return Err(Error("standalone_output_invalid"));
        }
        let bytes = STANDARD
            .decode(
                params["deltaBase64"]
                    .as_str()
                    .ok_or(Error("standalone_output_invalid"))?,
            )
            .map_err(|_| Error("standalone_output_invalid"))?;
        let file = match params["stream"].as_str() {
            Some("stdout") => &mut self.stdout,
            Some("stderr") => &mut self.stderr,
            _ => return Err(Error("standalone_output_invalid")),
        };
        file.write_all(&bytes)
            .map_err(|_| Error("standalone_output_write_failed"))
    }
}

pub(crate) fn tools() -> Vec<Value> {
    let string = json!({"type":"string"});
    let integer = json!({"type":"integer","minimum":0});
    let boolean = json!({"type":"boolean"});
    let size = json!({"type":"object","properties":{"rows":{"type":"integer","minimum":1,"maximum":65535},"cols":{"type":"integer","minimum":1,"maximum":65535}},"required":["rows","cols"],"additionalProperties":false});
    [
        ("local_context", "Standalone development: inspect the configured local workspace and standing Core permission profile without creating a Codex conversation or model request. Use these local tools by default for an unbound local ChatGPT task; use existing integrated work explicitly when requested. Read returned workspace instructions before effects. A context is a grant binding, not a Native work/controller.",json!({}),Vec::<&str>::new()),
        ("local_execute", "Execute argv in standalone context under its configured Core permission profile; use for file reads/edits, patches, commands and tests. No Codex thread/history/completion notification or additional model inference. Read applicable AGENTS.md before changes. Use a stable call_id; exact retries recover this same process and never rerun it. A returned process_id confirms admission, not spawn or exit. Poll local_process for full output/completion. cwd is not an access sandbox. tty enables PTY and stdin; otherwise stream_stdin is opt-in. Core's default timeout applies unless timeout_ms or disable_timeout is selected. Model arguments cannot change permission profile or workflow lock.",json!({"context_id":string,"call_id":string,"command":{"type":"array","items":{"type":"string"},"minItems":1},"cwd":string,"env":{"type":"object","additionalProperties":{"type":["string","null"]}},"tty":boolean,"stream_stdin":boolean,"timeout_ms":integer,"disable_timeout":boolean,"size":size}),vec!["context_id","call_id","command"]),
        ("local_process", "Read standalone output or control its original connection-owned process. read pages exact stdout/stderr bytes with independent offsets (base64 always; text when valid UTF-8); follow next_offset until all output is consumed. wait_ms<=20000 waits for progress without new inference. write/resize/terminate require their own stable call_id, preserve original Core reply and never replay an unknown mutation. An early control may be rejected before Core registers the process; inspect the original rejection. Terminate acknowledgement is not exit confirmation: poll the original command result. A disconnected/restarted handle cannot control a new connection or redispatch the original command.",json!({"context_id":string,"process_id":string,"action":{"type":"string","enum":["read","write","resize","terminate"]},"stdout_offset":integer,"stderr_offset":integer,"wait_ms":integer,"call_id":string,"delta_base64":string,"close_stdin":boolean,"size":size}),vec!["context_id","process_id","action"]),
    ].into_iter().map(|(name,description,properties,required)|json!({"name":name,"description":description,
        "inputSchema":{"type":"object","properties":properties,"required":required,"additionalProperties":false},
        "annotations":crate::mcp::tool_annotations(name)})).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fixture(std::path::PathBuf);
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    fn fixture() -> (Fixture, Arc<Local>) {
        let root = Fixture(
            std::env::temp_dir().join(format!("local-{}", crate::journal::random_id().unwrap())),
        );
        let mut config = Config::default();
        config.execution.core = Some(root.0.join("must-not-launch"));
        config.execution.home = Some(root.0.join("home"));
        config.execution.workspace = Some(root.0.join("workspace"));
        config.execution.standalone_permission_profile = Some("configured-only".into());
        let journal = Arc::new(Journal::open(&root.0).unwrap());
        let pool = Pool::new(
            config.clone(),
            root.0.clone(),
            "http://127.0.0.1:9".into(),
            root.0.join("provider"),
        )
        .unwrap();
        let local = Local::new(pool, journal, &config).unwrap();
        (root, local)
    }
    #[test]
    fn standalone_and_native_share_resource_admission_until_verified_exit() {
        let (fixture, local) = fixture();
        let cwd = fixture.0.join("workspace");
        std::fs::create_dir_all(&cwd).unwrap();
        std::fs::write(cwd.join("target"), "old\n").unwrap();
        let home = local.shared_guard_home();
        let patch = "*** Begin Patch\n*** Update File: target\n@@\n-old\n+new\n*** End Patch\n";

        SharedNativeGuard::offer_external(
            &cwd,
            home.as_deref(),
            "native",
            "patch",
            "functions.apply_patch",
            patch,
        )
        .unwrap();
        assert_eq!(
            local.reserve_effect(&cwd, "standalone", "command-fingerprint"),
            Err(Error("native_resource_busy")),
            "standalone command must not bypass an in-flight mediated write",
        );
        SharedNativeGuard::settle_external_receipt(
            &cwd,
            home.as_deref(),
            "native",
            "patch",
            "functions.apply_patch",
            None,
            &json!({"output":"patched"}),
        )
        .unwrap();

        local
            .reserve_effect(&cwd, "standalone", "command-fingerprint")
            .unwrap();
        local
            .settle_effect(&cwd, "standalone", &json!({"state":"delivery_unknown"}))
            .unwrap();
        assert_eq!(
            SharedNativeGuard::offer_external(
                &cwd,
                home.as_deref(),
                "native",
                "second-patch",
                "functions.apply_patch",
                patch,
            )
            .err()
            .as_deref(),
            Some("native_resource_busy"),
            "an unconfirmed Core reply cannot free the process-shared resource",
        );
        local
            .settle_effect(&cwd, "standalone", &json!({"state":"rejected"}))
            .unwrap();
        assert_eq!(
            SharedNativeGuard::offer_external(
                &cwd,
                home.as_deref(),
                "native",
                "second-patch",
                "functions.apply_patch",
                patch,
            )
            .err()
            .as_deref(),
            Some("native_resource_busy"),
            "a rejected Core response alone cannot prove no process was launched",
        );

        local
            .settle_effect(&cwd, "standalone", &json!({"state":"exited"}))
            .unwrap();
        SharedNativeGuard::offer_external(
            &cwd,
            home.as_deref(),
            "native",
            "second-patch",
            "functions.apply_patch",
            patch,
        )
        .unwrap();
        assert_eq!(
            local.reserve_effect(&cwd, "standalone", "command-fingerprint"),
            Err(Error("native_resource_busy")),
        );
    }

    #[tokio::test]
    async fn persisted_core_exit_reconciles_claim_without_reexecuting_process() {
        let (fixture, local) = fixture();
        let cwd = fixture.0.join("workspace");
        std::fs::create_dir_all(&cwd).unwrap();
        std::fs::write(cwd.join("target"), "old\n").unwrap();
        let call_id = digest(&"recovered-command");
        let call = call_id.as_str();
        let fingerprint = "original-input";
        local.reserve_effect(&cwd, call, fingerprint).unwrap();
        operation::intent_with_context(
            &local.journal,
            call,
            fingerprint,
            json!({"context_id":local.context["context_id"],"effect_workspace":cwd}),
        )
        .unwrap();
        operation::settle(
            &local.journal, call, fingerprint,
            &json!({"state":"exited","native_reply":{"result":{"exitCode":0,"stdout":"","stderr":""}}}),
        )
        .unwrap();
        private_directory(&local.journal.root.join("local-output")).unwrap();
        for stream in ["stdout", "stderr"] {
            OpenOptions::new()
                .create_new(true)
                .write(true)
                .mode(0o600)
                .open(local.output(call, stream))
                .unwrap();
        }
        let home = local.shared_guard_home();
        let patch = "*** Begin Patch\n*** Update File: target\n@@\n-old\n+new\n*** End Patch\n";
        assert_eq!(
            SharedNativeGuard::offer_external(
                &cwd,
                home.as_deref(),
                "integrated",
                "patch",
                "functions.apply_patch",
                patch,
            )
            .err()
            .as_deref(),
            Some("native_resource_busy"),
        );
        let recovered = local.status(call, 0, 0).await.unwrap();
        assert_eq!(recovered["state"], "exited");
        SharedNativeGuard::offer_external(
            &cwd,
            home.as_deref(),
            "integrated",
            "patch",
            "functions.apply_patch",
            patch,
        )
        .unwrap();
    }

    #[tokio::test]
    async fn wrong_context_and_unknown_old_intent_cannot_dispatch_core() {
        let (_root, local) = fixture();
        let mut args =
            json!({"context_id":"another context","call_id":"edit","command":["must-not-run"]});
        assert_eq!(
            local.call("local_execute", args.clone()).await,
            Err(Error("standalone_context_changed"))
        );
        args["context_id"] = local.context["context_id"].clone();
        let key = digest(&("standalone-exec-v1", &args["context_id"], "edit"));
        operation::intent_with_context(
            &local.journal,
            &key,
            &digest(&args),
            json!({"context_id":args["context_id"]}),
        )
        .unwrap();
        private_directory(&local.journal.root.join("local-output")).unwrap();
        for stream in ["stdout", "stderr"] {
            OpenOptions::new()
                .create_new(true)
                .write(true)
                .mode(0o600)
                .open(local.output(&key, stream))
                .unwrap();
        }
        let recovered = local.call("local_execute", args.clone()).await.unwrap();
        assert_eq!(recovered["state"], "delivery_unknown");
        assert_eq!(recovered["process_id"], key);
        args["command"] = json!(["different effect"]);
        assert_eq!(
            local.call("local_execute", args).await,
            Err(Error("operator_call_changed"))
        );
    }
    #[test]
    fn only_a_valid_original_command_reply_establishes_completion() {
        for reply in [
            json!({}),
            json!({"result":{}}),
            json!({"error":true}),
            json!({"result":{"exitCode":0,"stdout":"unstreamed","stderr":""}}),
            json!({"result":{"exitCode":0,"stdout":"","stderr":""},"error":{"code":-32600,"message":"conflicting reply"}}),
        ] {
            let observed = completion(reply.clone());
            assert_eq!(observed["state"], "delivery_unknown");
            assert_eq!(observed["native_reply"], reply);
        }
        assert_eq!(
            completion(json!({"result":{"exitCode":7,"stdout":"","stderr":""}}))["state"],
            "exited"
        );
        assert_eq!(
            completion(json!({"error":{"code":-32600,"message":"unknown permission profile"}}))
                ["state"],
            "rejected"
        );
    }
    #[test]
    fn output_pages_preserve_binary_bytes_and_reject_loss_or_false_offsets() {
        let (_root, local) = fixture();
        private_directory(&local.journal.root.join("local-output")).unwrap();
        let open = |stream| {
            OpenOptions::new()
                .create_new(true)
                .write(true)
                .mode(0o600)
                .open(local.output("process", stream))
                .unwrap()
        };
        let mut output = Output {
            stdout: open("stdout"),
            stderr: open("stderr"),
        };
        let bytes: Vec<u8> = (0..PAGE_BYTES + 19).map(|i| i as u8).collect();
        let event = json!({"params":{"processId":"process","stream":"stdout","capReached":false,"deltaBase64":STANDARD.encode(&bytes)}});
        output.capture("process", event.clone()).unwrap();
        let first = local.page("process", "stdout", 0).unwrap();
        assert_eq!(
            STANDARD.decode(first["base64"].as_str().unwrap()).unwrap(),
            bytes[..PAGE_BYTES]
        );
        assert_eq!(first["more"], true);
        assert!(first["text"].is_null());
        let tail = local.page("process", "stdout", PAGE_BYTES as u64).unwrap();
        assert_eq!(
            STANDARD.decode(tail["base64"].as_str().unwrap()).unwrap(),
            bytes[PAGE_BYTES..]
        );
        assert_eq!(tail["more"], false);
        assert_eq!(
            local.page("process", "stdout", bytes.len() as u64 + 1),
            Err(Error("standalone_output_offset_invalid"))
        );
        let mut lost = event.clone();
        lost["params"]["capReached"] = json!(true);
        assert_eq!(
            output.capture("process", lost),
            Err(Error("standalone_output_invalid"))
        );
        assert_eq!(
            output.capture("another-process", event.clone()),
            Err(Error("standalone_output_invalid"))
        );
        // Inject a real write failure at the capture sink, without relying on
        // Android permitting access to /dev/full.
        output.stdout = File::open(local.output("process", "stdout")).unwrap();
        assert_eq!(
            output.capture("process", event),
            Err(Error("standalone_output_write_failed"))
        );
    }
}
