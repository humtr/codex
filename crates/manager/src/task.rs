//! Active upstream work assistance. No history index, credentials or writer mutation.
use super::*;
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::io::IsTerminal;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::os::unix::net::UnixStream;
use tungstenite::{client::client_with_config, protocol::WebSocketConfig, Message, WebSocket};

const ERR_TASK: ManagerError = ManagerError::operation("codex termux task: owner unavailable, ambiguous, unsafe or unresponsive; no transfer performed");
const LIMIT: usize = 256 * 1024;
const MAX_SERVERS: usize = 64;
const MAX_TASKS: usize = 1024;
const SCAN_TIME: Duration = Duration::from_secs(10);
const CANCELLED: ManagerError = ManagerError {
    class: ErrorClass::Cancelled,
    message: "",
};

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Action {
    Choose,
    Status,
    Reconnect,
    Stop,
    Takeover,
}
#[derive(Debug)]
pub(super) struct TaskCommand {
    action: Action,
    id: Option<String>,
    profile: Option<ProfileTarget>,
    force: Option<String>,
}

pub(super) fn parse(args: &[OsString]) -> Result<TaskCommand, ManagerError> {
    let mut command = TaskCommand {
        action: Action::Choose,
        id: None,
        profile: None,
        force: None,
    };
    let mut index = 0;
    if let Some(arg) = args.first().and_then(|s| s.to_str()) {
        command.action = match arg {
            "status" => Action::Status,
            "reconnect" => Action::Reconnect,
            "stop" => Action::Stop,
            "takeover" => Action::Takeover,
            _ if canonical_session_id(arg) => Action::Choose,
            _ => return Err(ERR_USAGE),
        };
        if command.action != Action::Choose {
            index += 1;
        }
    }
    if let Some(id) = args.get(index).and_then(|s| s.to_str()) {
        if !canonical_session_id(id) {
            return Err(ERR_USAGE);
        }
        command.id = Some(id.to_owned());
        index += 1;
    }
    while index < args.len() {
        if command.action != Action::Takeover {
            return Err(ERR_USAGE);
        }
        let value = args.get(index + 1).ok_or(ERR_USAGE)?;
        match args[index].to_str() {
            Some("--profile") if command.profile.is_none() => {
                command.profile = Some(parse_target(value)?)
            }
            Some("--force-server") if command.force.is_none() => {
                let token = value.to_str().ok_or(ERR_USAGE)?;
                let (pid, start) = token.split_once(':').ok_or(ERR_USAGE)?;
                if pid.parse::<u32>().ok().filter(|p| *p > 1).is_none()
                    || start.is_empty()
                    || !start.bytes().all(|b| b.is_ascii_digit())
                {
                    return Err(ERR_USAGE);
                }
                command.force = Some(token.to_owned());
            }
            _ => return Err(ERR_USAGE),
        }
        index += 2;
    }
    if !matches!(command.action, Action::Choose | Action::Status) && command.id.is_none() {
        return Err(ERR_USAGE);
    }
    Ok(command)
}

#[derive(Debug, Clone)]
struct Server {
    home: PathBuf,
    program: PathBuf,
    socket: PathBuf,
    pid: u32,
    start: String,
}
impl Server {
    fn token(&self) -> String {
        format!("{}:{}", self.pid, self.start)
    }
    fn identity(&self) -> Result<(), ManagerError> {
        if process_start(self.pid)? != self.start
            || fs::read_link(format!("/proc/{}/exe", self.pid)).map_err(|_| ERR_TASK)?
                != self.program
        {
            return Err(ERR_TASK);
        }
        Ok(())
    }
    fn stream(&self, deadline: Instant) -> Result<UnixStream, ManagerError> {
        self.identity()?;
        let mut address: libc::sockaddr_un = unsafe { std::mem::zeroed() };
        address.sun_family = libc::AF_UNIX as libc::sa_family_t;
        let bytes = self.socket.as_os_str().as_bytes();
        if bytes.len() >= address.sun_path.len() || bytes.contains(&0) {
            return Err(ERR_TASK);
        }
        for (target, byte) in address.sun_path.iter_mut().zip(bytes) {
            *target = *byte as libc::c_char;
        }
        let raw = unsafe {
            libc::socket(
                libc::AF_UNIX,
                libc::SOCK_STREAM | libc::SOCK_CLOEXEC | libc::SOCK_NONBLOCK,
                0,
            )
        };
        if raw < 0 {
            return Err(ERR_TASK);
        }
        let stream = unsafe { UnixStream::from_raw_fd(raw) };
        loop {
            if Instant::now() >= deadline {
                return Err(ERR_TASK);
            }
            if unsafe {
                libc::connect(
                    raw,
                    (&address as *const libc::sockaddr_un).cast(),
                    (std::mem::offset_of!(libc::sockaddr_un, sun_path) + bytes.len() + 1)
                        as libc::socklen_t,
                )
            } == 0
            {
                break;
            }
            match io::Error::last_os_error().raw_os_error() {
                Some(libc::EAGAIN | libc::EINTR) => thread::sleep(Duration::from_millis(10)),
                _ => return Err(ERR_TASK),
            }
        }
        stream.set_nonblocking(false).map_err(|_| ERR_TASK)?;
        let mut peer: libc::ucred = unsafe { std::mem::zeroed() };
        let mut len = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
        if unsafe {
            libc::getsockopt(
                stream.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_PEERCRED,
                (&mut peer as *mut libc::ucred).cast(),
                &mut len,
            )
        } != 0
            || peer.pid != self.pid as i32
            || peer.uid != unsafe { libc::geteuid() }
        {
            return Err(ERR_TASK);
        }
        self.identity()?;
        Ok(stream)
    }
    fn connect(&self, deadline: Instant) -> Result<Rpc, ManagerError> {
        let stream = self.stream(deadline)?;
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .ok_or(ERR_TASK)?;
        stream
            .set_read_timeout(Some(remaining))
            .map_err(|_| ERR_TASK)?;
        stream
            .set_write_timeout(Some(remaining))
            .map_err(|_| ERR_TASK)?;
        let config = WebSocketConfig::default()
            .max_message_size(Some(LIMIT))
            .max_frame_size(Some(LIMIT));
        let (socket, _) =
            client_with_config("ws://localhost/", stream, Some(config)).map_err(|_| ERR_TASK)?;
        self.identity()?;
        let mut rpc = Rpc {
            socket,
            next: 0,
            deadline,
        };
        rpc.call("initialize", json!({"clientInfo":{"name":"codex_termux_task","version":"1.0"},"capabilities":{"experimentalApi":true}}))?;
        rpc.socket
            .send(Message::Text(
                json!({"method":"initialized"}).to_string().into(),
            ))
            .map_err(|_| ERR_TASK)?;
        Ok(rpc)
    }
}

struct Rpc {
    socket: WebSocket<UnixStream>,
    next: u64,
    deadline: Instant,
}
impl Rpc {
    fn call(&mut self, method: &str, params: Value) -> Result<Value, ManagerError> {
        self.next += 1;
        let id = self.next;
        let remaining = self
            .deadline
            .checked_duration_since(Instant::now())
            .ok_or(ERR_TASK)?;
        self.socket
            .get_ref()
            .set_read_timeout(Some(remaining))
            .map_err(|_| ERR_TASK)?;
        self.socket
            .get_ref()
            .set_write_timeout(Some(remaining))
            .map_err(|_| ERR_TASK)?;
        self.socket
            .send(Message::Text(
                json!({"id":id,"method":method,"params":params})
                    .to_string()
                    .into(),
            ))
            .map_err(|_| ERR_TASK)?;
        let mut consumed = 0;
        loop {
            let remaining = self
                .deadline
                .checked_duration_since(Instant::now())
                .ok_or(ERR_TASK)?;
            self.socket
                .get_ref()
                .set_read_timeout(Some(remaining))
                .map_err(|_| ERR_TASK)?;
            let message = self.socket.read().map_err(|_| ERR_TASK)?;
            consumed += message.len();
            if consumed > 4 * LIMIT {
                return Err(ERR_TASK);
            }
            let Message::Text(text) = message else {
                if matches!(message, Message::Ping(_) | Message::Pong(_)) {
                    continue;
                }
                return Err(ERR_TASK);
            };
            let response: Value = serde_json::from_str(&text).map_err(|_| ERR_TASK)?;
            if response.get("id").is_none() {
                continue;
            }
            if response["id"].as_u64() != Some(id) || response.get("error").is_some() {
                return Err(ERR_TASK);
            }
            return response.get("result").cloned().ok_or(ERR_TASK);
        }
    }
    fn loaded(&mut self) -> Result<Vec<String>, ManagerError> {
        let mut ids = BTreeSet::new();
        let mut cursor = Value::Null;
        loop {
            let page = self.call("thread/loaded/list", json!({"limit":100,"cursor":cursor}))?;
            for id in page["data"].as_array().ok_or(ERR_TASK)? {
                let id = id
                    .as_str()
                    .filter(|s| canonical_session_id(s))
                    .ok_or(ERR_TASK)?;
                if !ids.insert(id.to_owned()) || ids.len() > MAX_TASKS {
                    return Err(ERR_TASK);
                }
            }
            let next = page.get("nextCursor").ok_or(ERR_TASK)?;
            if next.is_null() {
                break;
            }
            let next = next
                .as_str()
                .filter(|s| canonical_session_id(s))
                .ok_or(ERR_TASK)?;
            if cursor.as_str() == Some(next) {
                return Err(ERR_TASK);
            }
            cursor = Value::String(next.to_owned());
        }
        Ok(ids.into_iter().collect())
    }
    fn state(&mut self, id: &str) -> Result<String, ManagerError> {
        let result = self.call("thread/read", json!({"threadId":id,"includeTurns":false}))?;
        if result["thread"]["id"].as_str() != Some(id) {
            return Err(ERR_TASK);
        }
        let status = result["thread"]["status"]["type"]
            .as_str()
            .ok_or(ERR_TASK)?;
        if !matches!(status, "active" | "idle" | "systemError" | "notLoaded") {
            return Err(ERR_TASK);
        }
        Ok(status.to_owned())
    }
}

fn process_start(pid: u32) -> Result<String, ManagerError> {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat")).map_err(|_| ERR_TASK)?;
    let fields: Vec<_> = stat
        .rsplit_once(')')
        .ok_or(ERR_TASK)?
        .1
        .split_whitespace()
        .collect();
    if matches!(fields.first(), Some(&"Z" | &"X")) {
        return Err(ERR_TASK);
    }
    let start = fields.get(19).ok_or(ERR_TASK)?;
    if !start.bytes().all(|b| b.is_ascii_digit()) {
        return Err(ERR_TASK);
    }
    Ok((*start).to_owned())
}
fn process_exited(pid: u32) -> Result<bool, ManagerError> {
    let stat = match fs::read_to_string(format!("/proc/{pid}/stat")) {
        Ok(stat) => stat,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(true),
        Err(_) => return Err(ERR_TASK),
    };
    let fields: Vec<_> = stat
        .rsplit_once(')')
        .ok_or(ERR_TASK)?
        .1
        .split_whitespace()
        .collect();
    Ok(matches!(fields.first(), Some(&"Z" | &"X")) && fields.get(17) == Some(&"1"))
}
fn private(path: &Path, directory: bool) -> Result<(), ManagerError> {
    if inspect_path(path)? != PathPresence::Present {
        return Err(ERR_TASK);
    }
    let m = fs::symlink_metadata(path).map_err(|_| ERR_TASK)?;
    if m.uid() != unsafe { libc::geteuid() }
        || m.permissions().mode() & 0o7777 != if directory { 0o700 } else { 0o600 }
        || if directory { !m.is_dir() } else { !m.is_file() }
    {
        return Err(ERR_TASK);
    }
    Ok(())
}
fn record(path: &Path) -> Result<Vec<u8>, ManagerError> {
    private(path, false)?;
    let mut file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
        .map_err(|_| ERR_TASK)?;
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take(16385)
        .read_to_end(&mut bytes)
        .map_err(|_| ERR_TASK)?;
    if bytes.len() > 16384 {
        return Err(ERR_TASK);
    }
    Ok(bytes)
}
fn namespace(home: &Path, program: &Path) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for b in home
        .as_os_str()
        .as_bytes()
        .iter()
        .chain([0].iter())
        .chain(program.as_os_str().as_bytes())
    {
        hash = (hash ^ u64::from(*b)).wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}
pub(super) fn profile_in_use(context: &Context, home: &Path) -> Result<bool, ManagerError> {
    Ok(servers(context)?.iter().any(|server| server.home == home))
}

fn servers(context: &Context) -> Result<Vec<Server>, ManagerError> {
    let root = context.home.join(".local/share/codex/core/servers");
    if inspect_path(&root)? == PathPresence::Missing {
        return Ok(Vec::new());
    }
    private(&root, true)?;
    let mut result = Vec::new();
    for (count, entry) in fs::read_dir(&root).map_err(|_| ERR_TASK)?.enumerate() {
        if count >= MAX_SERVERS {
            return Err(ERR_TASK);
        }
        let dir = entry.map_err(|_| ERR_TASK)?.path();
        private(&dir, true)?;
        let binding = record(&dir.join("owner"))?;
        let mut fields = binding.split(|b| *b == 0);
        let home = PathBuf::from(OsString::from_vec(fields.next().ok_or(ERR_TASK)?.to_vec()));
        let program = PathBuf::from(OsString::from_vec(fields.next().ok_or(ERR_TASK)?.to_vec()));
        if fields.next().is_some()
            || dir.file_name() != Some(OsStr::new(&namespace(&home, &program)))
        {
            return Err(ERR_TASK);
        }
        let generations = context.home.join(".local/lib/codex/core/generations");
        if program.file_name() != Some(OsStr::new("runtime"))
            || program.parent().and_then(Path::parent) != Some(generations.as_path())
        {
            return Err(ERR_TASK);
        }
        let text = String::from_utf8(record(&dir.join("pid"))?).map_err(|_| ERR_TASK)?;
        let pid = text
            .trim()
            .parse::<u32>()
            .ok()
            .filter(|p| *p > 1)
            .ok_or(ERR_TASK)?;
        let start = match process_start(pid) {
            Ok(start) => start,
            Err(_) if process_exited(pid)? => continue,
            Err(_) => return Err(ERR_TASK),
        };
        // Retired records survive homes and generations. Only live owners need
        // those execution files; exited records are ignored without repairing them.
        private(&home, true)?;
        if !is_existing_executable(&program)? {
            return Err(ERR_TASK);
        }
        let m = fs::metadata(&program).map_err(|_| ERR_TASK)?;
        if m.uid() != unsafe { libc::geteuid() } || m.mode() & 0o022 != 0 {
            return Err(ERR_TASK);
        }
        let socket = dir.join("s");
        let m = fs::symlink_metadata(&socket).map_err(|_| ERR_TASK)?;
        let physical = if m.file_type().is_symlink() {
            fs::read_link(&socket).map_err(|_| ERR_TASK)?
        } else {
            socket.clone()
        };
        if !is_safe_absolute_path(&physical) || inspect_path(&physical)? != PathPresence::Present {
            return Err(ERR_TASK);
        }
        let m = fs::symlink_metadata(&physical).map_err(|_| ERR_TASK)?;
        if !m.file_type().is_socket()
            || m.uid() != unsafe { libc::geteuid() }
            || m.mode() & 0o077 != 0
        {
            return Err(ERR_TASK);
        }
        result.push(Server {
            home,
            program,
            socket: physical,
            pid,
            start,
        });
    }
    result.sort_by_key(|s| (s.home.clone(), s.pid));
    Ok(result)
}

#[derive(Debug)]
struct Task {
    id: String,
    server: Server,
    state: String,
}
fn discover(context: &Context, filter: Option<&str>) -> Result<Vec<Task>, ManagerError> {
    let deadline = Instant::now() + SCAN_TIME;
    let mut tasks = Vec::new();
    let mut seen = BTreeSet::new();
    for server in servers(context)? {
        let mut rpc = match server.connect(deadline) {
            Ok(rpc) => rpc,
            Err(_) => {
                if let Some(id) = filter {
                    if owns_writer(context, id, &server)?
                        && server
                            .stream(Instant::now() + Duration::from_secs(2))
                            .is_ok()
                    {
                        if !seen.insert(id.to_owned()) {
                            return Err(ERR_TASK);
                        }
                        tasks.push(Task {
                            id: id.to_owned(),
                            server: server.clone(),
                            state: "unresponsive".to_owned(),
                        });
                        continue;
                    }
                }
                return Err(ERR_TASK);
            }
        };
        for id in rpc.loaded()? {
            if filter.is_some_and(|f| f != id) {
                continue;
            }
            let state = rpc.state(&id)?;
            // Loaded read-only copies and former writers are not current owners.
            if !owns_writer(context, &id, &server)? {
                continue;
            }
            if !seen.insert(id.clone()) || tasks.len() >= MAX_TASKS {
                return Err(ERR_TASK);
            }
            tasks.push(Task {
                id,
                server: server.clone(),
                state,
            });
        }
    }
    tasks.sort_by(|a, b| a.id.cmp(&b.id));
    if let Some(id) = filter {
        if tasks.is_empty() && !writer_free(context, id)? {
            return Err(ERR_TASK);
        }
    }
    Ok(tasks)
}
fn account(context: &Context, home: &Path) -> Result<String, ManagerError> {
    if let Some(id) = profile_view::registered_id(context, home)? {
        return Ok(id);
    }
    // Execution home is non-secret identity, escaped to prevent terminal control injection.
    Ok(format!(
        "CODEX_HOME={}",
        home.display().to_string().escape_default()
    ))
}
pub(super) fn snapshot(context: &Context) -> Result<String, ManagerError> {
    let mut records = Vec::new();
    for task in discover(context, None)? {
        records.push(json!({
            "id": task.id,
            "owner_profile": profile_view::registered_id(context, &task.server.home)?,
            "state": task.state,
            "server_token": task.server.token(),
        }));
    }
    Ok(format!(
        "{}\n",
        json!({"schema": "codex-manager-tasks-v1", "tasks": records})
    ))
}

fn format_tasks(context: &Context, tasks: &[Task]) -> Result<String, ManagerError> {
    let mut output = String::new();
    for task in tasks {
        output.push_str(&format!(
            "{}\towner={}\tstate={}\tserver={}\n",
            task.id,
            account(context, &task.server.home)?,
            task.state,
            task.server.token()
        ));
    }
    if tasks.is_empty() {
        output.push_str("No current task writers found.\n");
    }
    Ok(output)
}

fn resume_command(
    context: &Context,
    home: &Path,
    id: &str,
    remote: Option<&Path>,
) -> Result<Command, ManagerError> {
    private(home, true)?;
    let mut command = Command::new(&context.core_entrypoint);
    command
        .env_remove(CORE_API_ENV)
        .env_remove(CORE_ENTRYPOINT_ENV)
        .env_remove(CODEX_SQLITE_HOME_ENV);
    command.env(CODEX_HOME_ENV, home);
    if let Some(socket) = remote {
        let mut address = OsString::from("unix://");
        address.push(socket);
        command.arg("--remote").arg(address);
    }
    command.arg("resume").arg(id);
    Ok(command)
}
fn reconnect(context: &Context, task: &Task) -> Result<(), ManagerError> {
    // Verify this exact old/current server immediately before the final exec.
    let mut rpc = task.server.connect(Instant::now() + SCAN_TIME)?;
    if !rpc.loaded()?.iter().any(|id| id == &task.id)
        || !owns_writer(context, &task.id, &task.server)?
    {
        return Err(ERR_TASK);
    }
    drop(rpc);
    let _ = resume_command(
        context,
        &task.server.home,
        &task.id,
        Some(&task.server.socket),
    )?
    .exec();
    Err(ERR_LAUNCH)
}
fn native_file(path: &Path) -> Result<Option<File>, ManagerError> {
    if inspect_path(path)? == PathPresence::Missing {
        return Ok(None);
    }
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
        .map_err(|_| ERR_TASK)?;
    let m = file.metadata().map_err(|_| ERR_TASK)?;
    if !m.is_file() || m.uid() != unsafe { libc::geteuid() } || m.mode() & 0o022 != 0 {
        return Err(ERR_TASK);
    }
    Ok(Some(file))
}
fn writer_free(context: &Context, id: &str) -> Result<bool, ManagerError> {
    let directory = context.home.join(".codex/thread-writer-locks");
    if inspect_path(&directory)? == PathPresence::Missing {
        return Ok(true);
    }
    let Some(coordination) = native_file(&directory.join(".coordination.lock"))? else {
        return if inspect_path(&directory.join(format!("{id}.lock")))? == PathPresence::Missing {
            Ok(true)
        } else {
            Err(ERR_TASK)
        };
    };
    match coordination.try_lock() {
        Ok(()) => {}
        Err(std::fs::TryLockError::WouldBlock) => return Ok(false),
        Err(_) => return Err(ERR_TASK),
    }
    let Some(writer) = native_file(&directory.join(format!("{id}.lock")))? else {
        return Ok(true);
    };
    match writer.try_lock() {
        Ok(()) => Ok(true),
        Err(std::fs::TryLockError::WouldBlock) => Ok(false),
        Err(_) => Err(ERR_TASK),
    }
}
fn stop_one(
    rpc: &mut Rpc,
    context: &Context,
    server: &Server,
    id: &str,
) -> Result<(), ManagerError> {
    let deadline = rpc.deadline;
    if !rpc.loaded()?.iter().any(|loaded| loaded == id) {
        return Ok(());
    }
    let goal = rpc.call("thread/goal/get", json!({"threadId":id}))?;
    let goal_status = match goal.get("goal") {
        Some(Value::Null) => None,
        Some(goal) => Some(
            goal["status"]
                .as_str()
                .filter(|s| {
                    matches!(
                        *s,
                        "active"
                            | "paused"
                            | "blocked"
                            | "usageLimited"
                            | "budgetLimited"
                            | "complete"
                    )
                })
                .ok_or(ERR_TASK)?,
        ),
        None => return Err(ERR_TASK),
    };
    if goal_status == Some("active") {
        if !owns_writer(context, id, server)? {
            return Err(ERR_TASK);
        }
        rpc.call("thread/goal/set", json!({"threadId":id,"status":"paused"}))?;
    }
    if rpc.state(id)? == "active" {
        let turns = rpc.call(
            "thread/turns/list",
            json!({"threadId":id,"limit":1,"sortDirection":"desc","itemsView":"notLoaded"}),
        )?;
        let data = turns["data"].as_array().ok_or(ERR_TASK)?;
        let turn_id = match data.first() {
            Some(turn) if turn["status"].as_str() == Some("inProgress") => {
                turn["id"].as_str().ok_or(ERR_TASK)?
            }
            _ => "",
        };
        if !owns_writer(context, id, server)? {
            return Err(ERR_TASK);
        }
        rpc.call("turn/interrupt", json!({"threadId":id,"turnId":turn_id}))?;
    }
    // Stop uses native state as completion evidence, not the interrupt submission.
    loop {
        if !rpc.loaded()?.iter().any(|loaded| loaded == id) {
            break;
        }
        match rpc.state(id)?.as_str() {
            "idle" => {
                rpc.call("thread/backgroundTerminals/clean", json!({"threadId":id}))?;
                break;
            }
            "notLoaded" => break,
            "active" if Instant::now() < deadline => thread::sleep(Duration::from_millis(25)),
            _ => return Err(ERR_TASK),
        }
    }
    Ok(())
}

fn descendants(rpc: &mut Rpc, id: &str) -> Result<Vec<String>, ManagerError> {
    let mut cursor = Value::Null;
    let mut ids = BTreeSet::new();
    loop {
        let page=rpc.call("thread/list",json!({"ancestorThreadId":id,"useStateDbOnly":true,"sourceKinds":["subAgent","subAgentReview","subAgentCompact","subAgentThreadSpawn","subAgentOther"],"limit":100,"cursor":cursor}))?;
        for thread in page["data"].as_array().ok_or(ERR_TASK)? {
            let child = thread["id"]
                .as_str()
                .filter(|s| canonical_session_id(s) && *s != id)
                .ok_or(ERR_TASK)?;
            if !ids.insert(child.to_owned()) || ids.len() > MAX_TASKS {
                return Err(ERR_TASK);
            }
        }
        let next = page.get("nextCursor").ok_or(ERR_TASK)?;
        if next.is_null() {
            break;
        }
        if next.as_str().is_none() || next == &cursor {
            return Err(ERR_TASK);
        }
        cursor = next.clone();
    }
    let loaded = rpc.loaded()?;
    Ok(ids.into_iter().filter(|id| loaded.contains(id)).collect())
}
fn stop(context: &Context, task: &Task) -> Result<(), ManagerError> {
    if !owns_writer(context, &task.id, &task.server)? {
        return Err(ERR_TASK);
    }
    let mut rpc = task.server.connect(Instant::now() + SCAN_TIME)?;
    let mut children = descendants(&mut rpc, &task.id)?;
    if !owns_writer(context, &task.id, &task.server)? {
        return Err(ERR_TASK);
    }
    stop_one(&mut rpc, context, &task.server, &task.id)?;
    // Recheck ancestry after cancelling the parent to include children spawned in the race.
    children.extend(descendants(&mut rpc, &task.id)?);
    children.sort();
    children.dedup();
    for child in children {
        if owns_writer(context, &child, &task.server)? {
            stop_one(&mut rpc, context, &task.server, &child)?;
        }
    }
    Ok(())
}

fn owns_writer(context: &Context, id: &str, server: &Server) -> Result<bool, ManagerError> {
    server.identity()?;
    let path = context
        .home
        .join(format!(".codex/thread-writer-locks/{id}.lock"));
    let Some(file) = native_file(&path)? else {
        return Ok(false);
    };
    let target = file.metadata().map_err(|_| ERR_TASK)?;
    let dir = PathBuf::from(format!("/proc/{}/fd", server.pid));
    for (count, entry) in fs::read_dir(dir).map_err(|_| ERR_TASK)?.enumerate() {
        if count >= 4096 {
            return Err(ERR_TASK);
        }
        let entry = entry.map_err(|_| ERR_TASK)?;
        let m = match fs::metadata(entry.path()) {
            Ok(m) => m,
            Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
            Err(_) => return Err(ERR_TASK),
        };
        if m.ino() != target.ino() || m.dev() != target.dev() {
            continue;
        }
        let info = fs::read_to_string(format!(
            "/proc/{}/fdinfo/{}",
            server.pid,
            entry.file_name().to_string_lossy()
        ))
        .map_err(|_| ERR_TASK)?;
        if info.lines().any(|line| {
            let fields: Vec<_> = line.split_whitespace().collect();
            fields.len() >= 6
                && fields[0] == "lock:"
                && fields[2] == "FLOCK"
                && fields[4] == "WRITE"
                && fields[5] == server.pid.to_string()
        }) {
            server.identity()?;
            return Ok(true);
        }
    }
    Ok(false)
}
fn destination(
    context: &Context,
    profile: Option<&ProfileTarget>,
) -> Result<PathBuf, ManagerError> {
    let home = match profile {
        Some(ProfileTarget::Default) => context.home.join(".codex"),
        Some(ProfileTarget::Custom(id)) => {
            let dirs = existing_manager_profiles(context)?.ok_or(ERR_PROFILE)?;
            if !profile_complete(&dirs, id) {
                return Err(ERR_PROFILE);
            }
            profile_home_path(&dirs, id)
        }
        None => match context
            .inherited_codex_home
            .as_ref()
            .filter(|h| h.to_str().is_some_and(|h| !h.is_empty()))
        {
            Some(home) => PathBuf::from(home),
            None => {
                return destination(
                    context,
                    Some(&super::profile::read_default(context)?.unwrap_or(ProfileTarget::Default)),
                )
            }
        },
    };
    private(&home, true)?;
    Ok(home)
}
fn force_server(context: &Context, task: &Task, token: &str) -> Result<(), ManagerError> {
    if token != task.server.token() || !owns_writer(context, &task.id, &task.server)? {
        return Err(ERR_TASK);
    }
    drop(
        task.server
            .stream(Instant::now() + Duration::from_secs(2))?,
    );
    // Hold a kernel identity before signalling; never fall back to kill(pid).
    let raw = unsafe { libc::syscall(libc::SYS_pidfd_open, task.server.pid, 0) };
    if raw < 0 {
        return Err(ManagerError::operation(
            "codex termux task: PID-stable force termination is unavailable; owner preserved",
        ));
    }
    let handle = unsafe { File::from_raw_fd(raw as i32) };
    task.server.identity()?;
    let affected = task
        .server
        .connect(Instant::now() + Duration::from_secs(2))
        .and_then(|mut r| r.loaded());
    match affected {
        Ok(ids) => eprintln!(
            "Whole-server termination {} affects: {}",
            token,
            ids.join(", ")
        ),
        Err(_) => eprintln!(
            "Whole-server termination {}: other affected tasks are unknown (server unresponsive).",
            token
        ),
    }
    // Scope discovery may take time; an earlier snapshot cannot authorize a signal.
    if !owns_writer(context, &task.id, &task.server)? {
        return Err(ERR_TASK);
    }
    for (signal, wait) in [
        (libc::SIGTERM, Duration::from_secs(2)),
        (libc::SIGKILL, Duration::from_secs(3)),
    ] {
        if unsafe {
            libc::syscall(
                libc::SYS_pidfd_send_signal,
                handle.as_raw_fd(),
                signal,
                std::ptr::null::<libc::siginfo_t>(),
                0,
            )
        } < 0
        {
            return Err(ERR_TASK);
        }
        let deadline = Instant::now() + wait;
        loop {
            let mut poll = libc::pollfd {
                fd: handle.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            };
            let result = unsafe { libc::poll(&mut poll, 1, 25) };
            if result > 0 && poll.revents & libc::POLLIN != 0 {
                return Ok(());
            }
            if result < 0 && io::Error::last_os_error().kind() != io::ErrorKind::Interrupted {
                return Err(ERR_TASK);
            }
            if Instant::now() >= deadline {
                break;
            }
        }
    }
    Err(ERR_TASK)
}
fn takeover(context: &Context, task: &Task, command: &TaskCommand) -> Result<(), ManagerError> {
    let home = destination(context, command.profile.as_ref())?;
    if let Some(token) = &command.force {
        force_server(context, task, token)?;
    } else {
        stop(context, task)?;
    }
    let deadline = Instant::now() + SCAN_TIME;
    while !writer_free(context, &task.id)? {
        if Instant::now() >= deadline {
            return Err(ManagerError::operation("codex termux task: writer is still owned; reconnect or explicitly confirm whole-server termination; no new writer started"));
        }
        thread::sleep(Duration::from_millis(25));
    }
    let _ = resume_command(context, &home, &task.id, None)?.exec();
    Err(ERR_LAUNCH)
}
fn answer(prompt: &str) -> Result<String, ManagerError> {
    print!("{prompt}");
    io::stdout().flush().map_err(|_| ERR_TASK)?;
    let mut input = io::stdin().lock();
    let mut bytes = Vec::new();
    loop {
        let mut byte = [0];
        if input.read(&mut byte).map_err(|_| ERR_TASK)? == 0 || byte[0] == b'\n' {
            break;
        }
        if bytes.len() >= 127 {
            return Err(ERR_USAGE);
        }
        bytes.push(byte[0]);
    }
    String::from_utf8(bytes)
        .map(|s| s.trim().to_owned())
        .map_err(|_| ERR_USAGE)
}

fn choose(context: &Context, tasks: &[Task]) -> Result<Option<String>, ManagerError> {
    if tasks.is_empty() {
        return Ok(Some("No current task writers found.\n".to_owned()));
    }
    print!("{}", format_tasks(context, tasks)?);
    let task = if tasks.len() == 1 {
        &tasks[0]
    } else {
        for (index, task) in tasks.iter().enumerate() {
            println!(
                "{}: {} ({})",
                index + 1,
                task.id,
                account(context, &task.server.home)?
            );
        }
        let selected = answer("Task number (Enter cancels): ")?;
        if selected.is_empty() {
            return Err(CANCELLED);
        }
        let index = selected
            .parse::<usize>()
            .ok()
            .and_then(|i| i.checked_sub(1))
            .ok_or(ERR_USAGE)?;
        tasks.get(index).ok_or(ERR_USAGE)?
    };
    match answer("1 Reconnect to owner  2 Stop  3 Stop and resume in current account  4 Force whole server and resume  Enter Cancel: ")?.as_str() {
        ""=>Err(CANCELLED),
        "1"=>{reconnect(context,task)?;Ok(None)},
        "2"=>{stop(context,task)?;Ok(Some(if writer_free(context,&task.id)? {"Task stopped; writer released.\n"} else {"Task stopped; writer still owned by another connection.\n"}.to_owned()))},
        "3"=>{takeover(context,task,&TaskCommand {action:Action::Takeover,id:Some(task.id.clone()),profile:None,force:None})?;Ok(None)},
        "4"=>{
            let scope=task.server.connect(Instant::now()+Duration::from_secs(2)).and_then(|mut r|r.loaded());
            match scope {Ok(ids)=>println!("This stops the whole server, including: {}",ids.join(", ")),Err(_)=>println!("This stops the whole server. Other affected tasks are unknown.")};
            let token=task.server.token();
            if answer(&format!("Type {token} to terminate this server (Enter cancels): "))?!=token { return Err(CANCELLED); }
            takeover(context,task,&TaskCommand {action:Action::Takeover,id:Some(task.id.clone()),profile:None,force:Some(token)})?;Ok(None)
        },
        _=>Err(ERR_USAGE),
    }
}

pub(super) fn run(context: &Context, command: TaskCommand) -> Result<Option<String>, ManagerError> {
    if command.action == Action::Choose
        && !(io::stdin().is_terminal() && io::stdout().is_terminal())
    {
        return Err(ERR_USAGE);
    }
    // Validate destination and grammar before contacting or stopping any owner.
    if command.action == Action::Takeover {
        destination(context, command.profile.as_ref())?;
    }
    let tasks = discover(context, command.id.as_deref())?;
    if command.action == Action::Status {
        return Ok(Some(format_tasks(context, &tasks)?));
    }
    if command.action == Action::Choose {
        return choose(context, &tasks);
    }
    if command.action == Action::Takeover && tasks.is_empty() && command.force.is_none() {
        if !writer_free(context, command.id.as_deref().ok_or(ERR_USAGE)?)? {
            return Err(ERR_TASK);
        }
        let home = destination(context, command.profile.as_ref())?;
        let _ = resume_command(
            context,
            &home,
            command.id.as_deref().ok_or(ERR_USAGE)?,
            None,
        )?
        .exec();
        return Err(ERR_LAUNCH);
    }
    if tasks.len() != 1 {
        return Err(ERR_TASK);
    }
    let task = &tasks[0];
    match command.action {
        Action::Reconnect => {
            reconnect(context, task)?;
            Ok(None)
        }
        Action::Stop => {
            stop(context, task)?;
            Ok(Some(if writer_free(context,&task.id)? { "Task stopped; writer released.\n" } else { "Task stopped; writer still owned by another connection. Use reconnect or explicitly confirmed whole-server takeover.\n" }.to_owned()))
        }
        Action::Takeover => {
            takeover(context, task, &command)?;
            Ok(None)
        }
        _ => Err(ERR_USAGE),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn args(values: &[&str]) -> Vec<OsString> {
        values.iter().map(OsString::from).collect()
    }
    const ID: &str = "12345678-1234-1234-1234-123456789abc";
    #[test]
    fn local_connect_full_backlog_obeys_deadline_and_preserves_listener() {
        let root = std::env::temp_dir().join(format!("hc{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        let path = root.join("s");
        let listener = std::os::unix::net::UnixListener::bind(&path).unwrap();
        assert_eq!(unsafe { libc::listen(listener.as_raw_fd(), 0) }, 0);
        let pending = UnixStream::connect(&path).unwrap();
        let server = Server {
            home: root.clone(),
            program: std::env::current_exe().unwrap(),
            socket: path,
            pid: std::process::id(),
            start: process_start(std::process::id()).unwrap(),
        };
        let before = Instant::now();
        assert!(server.stream(before + Duration::from_millis(50)).is_err());
        assert!(before.elapsed() < Duration::from_secs(1));
        drop(listener.accept().unwrap());
        let stream = server
            .stream(Instant::now() + Duration::from_secs(1))
            .unwrap();
        drop(stream);
        drop(pending);
        drop(listener);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn task_grammar_is_exact_and_rejects_unsafe_or_duplicate_options() {
        for values in [
            vec![],
            vec!["status"],
            vec![ID],
            vec!["status", ID],
            vec!["reconnect", ID],
            vec!["stop", ID],
            vec![
                "takeover",
                ID,
                "--profile",
                "default",
                "--force-server",
                "10:123",
            ],
        ] {
            assert!(parse(&args(&values)).is_ok());
        }
        for values in [
            vec!["reconnect"],
            vec!["status", "bad"],
            vec!["stop", ID, "--profile", "x"],
            vec!["takeover", ID, "--force-server", "1:12"],
            vec!["takeover", ID, "--force-server", "12:x"],
            vec!["takeover", ID, "--profile", "x", "--profile", "y"],
            vec!["status", ID, "extra"],
        ] {
            assert!(parse(&args(&values)).is_err());
        }
    }
    #[test]
    fn rpc_uses_native_websocket_and_bounds_identity_status_and_pagination() {
        use std::os::unix::net::UnixListener;
        let root = std::env::temp_dir().join(format!("ht{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        let path = root.join("s");
        let listener = UnixListener::bind(&path).unwrap();
        let worker = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut ws = tungstenite::accept(stream).unwrap();
            for expected in [
                "initialize",
                "initialized",
                "thread/loaded/list",
                "thread/read",
            ] {
                let value: Value =
                    serde_json::from_str(ws.read().unwrap().to_text().unwrap()).unwrap();
                assert_eq!(value["method"], expected);
                if expected == "initialized" {
                    continue;
                }
                let result = match expected {
                    "thread/loaded/list" => json!({"data":[ID],"nextCursor":null}),
                    "thread/read" => {
                        assert_eq!(value["params"]["includeTurns"], false);
                        json!({"thread":{"id":ID,"status":{"type":"active"},"preview":"do not expose"}})
                    }
                    _ => json!({}),
                };
                ws.send(Message::Text(
                    json!({"id":value["id"],"result":result}).to_string().into(),
                ))
                .unwrap();
            }
        });
        let server = Server {
            home: root.clone(),
            program: std::env::current_exe().unwrap(),
            socket: path,
            pid: std::process::id(),
            start: process_start(std::process::id()).unwrap(),
        };
        let mut rpc = server.connect(Instant::now() + SCAN_TIME).unwrap();
        assert_eq!(rpc.loaded().unwrap(), [ID]);
        assert_eq!(rpc.state(ID).unwrap(), "active");
        drop(rpc);
        worker.join().unwrap();
        fs::remove_dir_all(root).unwrap();
        let bad = Server {
            start: "0".to_owned(),
            ..server
        };
        assert!(bad.identity().is_err());
    }
    #[test]
    fn task_status_missing_state_is_read_only_and_private_records_reject_substitution() {
        let root = std::env::temp_dir().join(format!("hm{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        set_mode(&root, 0o700).unwrap();
        let context = Context {
            home: root.clone(),
            inherited_codex_home: None,
            core_entrypoint: std::env::current_exe().unwrap(),
        };
        assert_eq!(
            run(&context, parse(&args(&["status"])).unwrap())
                .unwrap()
                .unwrap(),
            "No current task writers found.\n"
        );
        assert_eq!(fs::read_dir(&root).unwrap().count(), 0);
        let record_path = root.join("record");
        fs::write(&record_path, b"owned").unwrap();
        set_mode(&record_path, 0o600).unwrap();
        assert_eq!(record(&record_path).unwrap(), b"owned");
        set_mode(&record_path, 0o644).unwrap();
        assert!(record(&record_path).is_err());
        set_mode(&record_path, 0o600).unwrap();
        fs::write(&record_path, vec![0; 16385]).unwrap();
        assert!(record(&record_path).is_err());
        let link = root.join("link");
        std::os::unix::fs::symlink(&record_path, &link).unwrap();
        assert!(record(&link).is_err());
        assert_eq!(
            account(&context, &context.home.join(".codex")).unwrap(),
            "default"
        );
        assert!(!account(&context, &root.join("external\u{1b}account"))
            .unwrap()
            .contains('\u{1b}'));
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn rpc_rejects_error_wrong_id_invalid_json_duplicates_and_oversize() {
        use std::os::unix::net::UnixStream;
        for response in [
            "not-json".to_owned(),
            json!({"id":2,"result":{}}).to_string(),
            json!({"id":1,"error":{"message":"private"}}).to_string(),
            json!({"id":1,"result":{"data":[ID,ID],"nextCursor":null}}).to_string(),
            "x".repeat(LIMIT + 1),
        ] {
            let (client, server) = UnixStream::pair().unwrap();
            let worker = thread::spawn(move || {
                let mut ws = tungstenite::accept(server).unwrap();
                let _ = ws.read();
                let _ = ws.send(Message::Text(response.into()));
            });
            let (socket, _) = client_with_config(
                "ws://localhost/",
                client,
                Some(
                    WebSocketConfig::default()
                        .max_message_size(Some(LIMIT))
                        .max_frame_size(Some(LIMIT)),
                ),
            )
            .unwrap();
            let mut rpc = Rpc {
                socket,
                next: 0,
                deadline: Instant::now() + SCAN_TIME,
            };
            assert!(rpc.loaded().is_err());
            drop(rpc);
            worker.join().unwrap();
        }
    }
    #[test]
    fn reconnect_command_preserves_exact_owner_remote_uuid_and_exec_boundary() {
        let root = std::env::temp_dir().join(format!("he{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        set_mode(&root, 0o700).unwrap();
        let context = Context {
            home: root.clone(),
            inherited_codex_home: None,
            core_entrypoint: std::env::current_exe().unwrap(),
        };
        let socket = root.join("s");
        let command = resume_command(&context, &root, ID, Some(&socket)).unwrap();
        let args: Vec<_> = command.get_args().collect();
        assert_eq!(
            args,
            [
                OsStr::new("--remote"),
                OsStr::new(&format!("unix://{}", socket.display())),
                OsStr::new("resume"),
                OsStr::new(ID)
            ]
        );
        let env: std::collections::BTreeMap<_, _> = command.get_envs().collect();
        assert_eq!(
            env.get(OsStr::new(CODEX_HOME_ENV)),
            Some(&Some(root.as_os_str()))
        );
        assert_eq!(env.get(OsStr::new(CODEX_SQLITE_HOME_ENV)), Some(&None));
        assert_eq!(env.get(OsStr::new(CORE_API_ENV)), Some(&None));
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn native_writer_probe_never_creates_unlinks_or_steals_held_writer() {
        let root = std::env::temp_dir().join(format!("hl{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        set_mode(&root, 0o700).unwrap();
        let context = Context {
            home: root.clone(),
            inherited_codex_home: None,
            core_entrypoint: std::env::current_exe().unwrap(),
        };
        assert!(writer_free(&context, ID).unwrap());
        assert!(!root.join(".codex").exists());
        let dir = root.join(".codex/thread-writer-locks");
        fs::create_dir_all(&dir).unwrap();
        set_mode(&dir, 0o700).unwrap();
        let lock = dir.join(format!("{ID}.lock"));
        fs::write(&lock, b"native-bytes").unwrap();
        set_mode(&lock, 0o600).unwrap();
        assert!(writer_free(&context, ID).is_err());
        let coordination = dir.join(".coordination.lock");
        fs::write(&coordination, b"").unwrap();
        set_mode(&coordination, 0o600).unwrap();
        let writer = File::open(&lock).unwrap();
        writer.try_lock().unwrap();
        assert!(!writer_free(&context, ID).unwrap());
        assert_eq!(fs::read(&lock).unwrap(), b"native-bytes");
        drop(writer);
        assert!(writer_free(&context, ID).unwrap());
        assert_eq!(fs::read(&lock).unwrap(), b"native-bytes");
        let coordinator = File::open(&coordination).unwrap();
        coordinator.try_lock().unwrap();
        assert!(!writer_free(&context, ID).unwrap());
        drop(coordinator);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn stop_does_not_confuse_an_unloaded_target_with_another_loaded_thread() {
        let (client, server) = UnixStream::pair().unwrap();
        let worker = thread::spawn(move || {
            let mut ws = tungstenite::accept(server).unwrap();
            let msg: Value = serde_json::from_str(ws.read().unwrap().to_text().unwrap()).unwrap();
            assert_eq!(msg["method"], "thread/loaded/list");
            ws.send(Message::Text(json!({"id":msg["id"],"result":{"data":["12345678-1234-1234-1234-123456789abd"],"nextCursor":null}}).to_string().into())).unwrap();
        });
        let (socket, _) = client_with_config("ws://localhost/", client, None).unwrap();
        let mut rpc = Rpc {
            socket,
            next: 0,
            deadline: Instant::now() + SCAN_TIME,
        };
        let root = std::env::temp_dir();
        let server = Server {
            home: root.clone(),
            program: std::env::current_exe().unwrap(),
            socket: root.join("unused"),
            pid: std::process::id(),
            start: process_start(std::process::id()).unwrap(),
        };
        let context = Context {
            home: root,
            inherited_codex_home: None,
            core_entrypoint: server.program.clone(),
        };
        assert!(stop_one(&mut rpc, &context, &server, ID).is_ok());
        drop(rpc);
        worker.join().unwrap();
    }
}
