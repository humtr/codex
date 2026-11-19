#[cfg(unix)]
mod cli;
#[cfg(unix)]
mod daemon_settings;
#[cfg(unix)]
mod doctor;
#[cfg(unix)]
mod error;
#[cfg(unix)]
mod extensions;
#[cfg(unix)]
mod fs_util;
#[cfg(unix)]
mod generation;
#[cfg(unix)]
mod json;
#[cfg(unix)]
mod maintenance;
#[cfg(unix)]
mod proc_exec;
#[cfg(unix)]
mod progress;
#[cfg(unix)]
mod qualification;
#[cfg(unix)]
mod remote_download;
#[cfg(unix)]
mod runtime_env;
#[cfg(unix)]
mod shared_home;
#[cfg(unix)]
mod state;
#[cfg(unix)]
mod system_config;
#[cfg(unix)]
mod timezone;
#[cfg(unix)]
mod update;
#[cfg(unix)]
mod upstream_metadata;
#[cfg(unix)]
mod wrapper_cli;

#[cfg(unix)]
use {
    cli::{plan_public_dispatch, PublicDispatchRoute, UpstreamArgScan},
    error::{LocalProductError, PublicDispatchExecutionError},
    generation::{
        inspect_activated_generation, load_activated_generation, with_qualified_loaded_runtime,
        GenerationLayout, LocalWrapperRoots, QualifiedRuntimeAssets,
    },
    runtime_env::{capture_termux_process_env, launch_upstream_runtime, TermuxProcessEnvSnapshot},
    std::ffi::{OsStr, OsString},
    std::path::Path,
    wrapper_cli::run_wrapper_command,
};

#[cfg(unix)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct LocalPublicDispatchContext<'context, 'runtime_selection, 'runtime_asset> {
    pub(crate) runtime_assets: QualifiedRuntimeAssets<'runtime_selection, 'runtime_asset>,
    pub(crate) process_env: &'context TermuxProcessEnvSnapshot,
    pub(crate) cert_file: &'context OsStr,
    pub(crate) cert_dir: Option<&'context OsStr>,
    pub(crate) resolver_path: &'context Path,
    pub(crate) config_dir: &'context Path,
    pub(crate) state_root: &'context Path,
    pub(crate) wrapper_doctor_status: doctor::WrapperDoctorStatus,
    pub(crate) generation_id: &'context str,
    pub(crate) generation_layout: GenerationLayout,
    pub(crate) has_code_mode_host: bool,
}
#[cfg(unix)]
#[derive(Debug, Clone, PartialEq, Eq)]
enum PublicDispatchCompletion {
    Doctor(doctor::DoctorCommandOutcome),
}

#[cfg(unix)]
fn execute_activated_route(
    route: PublicDispatchRoute,
    roots: &LocalWrapperRoots,
    process_env: &TermuxProcessEnvSnapshot,
    scan: &UpstreamArgScan,
) -> Result<PublicDispatchCompletion, LocalProductError> {
    let _lease = maintenance::launch_lease(&roots.generation_root).map_err(|source| {
        LocalProductError::Io {
            operation: "lease installed generation during launch",
            source,
        }
    })?;
    let loaded = if matches!(route, PublicDispatchRoute::Doctor(_)) {
        inspect_activated_generation(roots)?
    } else {
        load_activated_generation(roots)?.1
    };
    with_qualified_loaded_runtime(&loaded, |runtime_assets| match route {
        PublicDispatchRoute::Doctor(args) => {
            let context = LocalPublicDispatchContext {
                runtime_assets,
                process_env,
                cert_file: roots.cert_file.as_os_str(),
                cert_dir: Some(roots.cert_dir.as_os_str()),
                resolver_path: &roots.resolver_path,
                config_dir: &roots.config_dir,
                state_root: &roots.state_root,
                wrapper_doctor_status: doctor::WrapperDoctorStatus::Healthy,
                generation_id: &loaded.generation_id,
                generation_layout: loaded.generation_layout,
                has_code_mode_host: loaded.code_mode_host_path.is_some(),
            };
            let outcome = doctor::run_local_doctor_command(args, context)
                .map_err(PublicDispatchExecutionError::Doctor)
                .map_err(LocalProductError::Dispatch)?;
            Ok(PublicDispatchCompletion::Doctor(outcome))
        }
        PublicDispatchRoute::Upstream(args) => {
            let err = launch_upstream_runtime(runtime_assets, process_env, roots, &args, scan);
            Err(LocalProductError::Dispatch(
                PublicDispatchExecutionError::Upstream(err),
            ))
        }
        PublicDispatchRoute::Update(_) | PublicDispatchRoute::Wrapper(_) => {
            unreachable!("handled before execute_activated_route")
        }
    })
}

#[cfg(unix)]
fn run_public_main<I, S>(args: I) -> i32
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let original: Vec<OsString> = args.into_iter().map(Into::into).collect();
    let scan = UpstreamArgScan::new(&original);
    let route = match plan_public_dispatch(original, &scan) {
        Ok(route) => route,
        Err(err) => {
            eprintln!("codex: {err}");
            return 2;
        }
    };
    match route {
        PublicDispatchRoute::Update(args) => {
            let action = match update::parse_update_args(&args) {
                Ok(action) => action,
                Err(err) => {
                    eprintln!("{err}");
                    return 2;
                }
            };
            update::run_update_action(action)
        }
        PublicDispatchRoute::Wrapper(args) => run_wrapper_command(args),
        route @ (PublicDispatchRoute::Doctor(_) | PublicDispatchRoute::Upstream(_)) => {
            let doctor_mode = if let PublicDispatchRoute::Doctor(args) = &route {
                match doctor::doctor_output_mode(args.iter().cloned()) {
                    Ok(mode) => Some(mode),
                    Err(err) => {
                        eprintln!("codex: {err}");
                        return 2;
                    }
                }
            } else {
                None
            };
            let result = LocalWrapperRoots::from_environment().and_then(|roots| {
                if matches!(route, PublicDispatchRoute::Upstream(_)) && !scan.help_or_version {
                    warn_unavailable_addons(&roots);
                }
                execute_activated_route(route, &roots, &capture_termux_process_env(), &scan)
            });
            let outcome = match result {
                Ok(PublicDispatchCompletion::Doctor(outcome)) => outcome,
                Err(err) => {
                    let Some(mode) = doctor_mode else {
                        eprintln!("codex: {err}");
                        return 1;
                    };
                    doctor::doctor_preparation_failure(mode, &err.to_string())
                }
            };
            print!("{}", outcome.output);
            doctor::doctor_exit_code(outcome.exit_class)
        }
    }
}

#[cfg(unix)]
fn warn_unavailable_addons(roots: &LocalWrapperRoots) {
    match extensions::WrapperSettings::read(&roots.state_root) {
        Ok(settings) => {
            if let Some(modules) = settings.unavailable_addons() {
                eprintln!("codex: optional module(s) {modules} are unavailable; continuing with standalone upstream");
            }
        }
        Err(err) => {
            eprintln!(
                "codex: warning: ignoring invalid wrapper.toml ({err}); using bare standalone mode"
            );
        }
    }
}

#[cfg(unix)]
fn entry() -> ! {
    let mut args = std::env::args_os();
    let _ = args.next();
    std::process::exit(run_public_main(args));
}
#[cfg(unix)]
fn main() {
    entry();
}

#[cfg(not(unix))]
fn main() {
    eprintln!("codex: this build requires a Unix/Termux target");
    std::process::exit(1);
}

#[cfg(all(test, unix))]
pub(crate) mod tests;
