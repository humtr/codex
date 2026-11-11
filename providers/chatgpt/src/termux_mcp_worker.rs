//! Independent Termux app-UID MCP worker. No Native/Core or running Provider
//! service is required for its single termux_shell tool, including receipt replay.
use std::path::PathBuf;

#[tokio::main]
async fn main() {
    let mut args = std::env::args_os().skip(1);
    let valid = args.next().as_deref() == Some(std::ffi::OsStr::new("--state"));
    let state = args.next().map(PathBuf::from);
    if !valid || state.is_none() || args.next().is_some() {
        eprintln!("usage: codex-termux-mcp-worker --state ABSOLUTE_PROVIDER_STATE");
        std::process::exit(2);
    }
    let state = state.unwrap();
    if let Err(error) = codex_chatgpt_provider::runtime::mcp_stdio_worker(&state, true).await {
        eprintln!("termux_mcp_worker_failed: {}", error.0);
        std::process::exit(1);
    }
}
