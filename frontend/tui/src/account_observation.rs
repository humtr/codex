//! Account presentation is separate from Native connection and model admission.
use crate::app_server_session::AppServerSession;
use crate::app_server_session::ThreadParamsMode;
use crate::legacy_core::config::Config;
use codex_app_server_protocol::Account;
use codex_app_server_protocol::GetAccountResponse;
use codex_app_server_protocol::WorkspaceRouting;

pub(crate) enum AccountObservation {
    Available(GetAccountResponse),
    Unavailable {
        requires_openai_auth: bool,
        error: String,
    },
}

impl From<GetAccountResponse> for AccountObservation {
    fn from(account: GetAccountResponse) -> Self {
        Self::Available(account)
    }
}

impl AccountObservation {
    pub(crate) fn account(&self) -> Option<&Account> {
        match self {
            Self::Available(response) => response.account.as_ref(),
            Self::Unavailable { .. } => None,
        }
    }

    pub(crate) fn requires_openai_auth(&self) -> bool {
        match self {
            Self::Available(response) => response.requires_openai_auth,
            Self::Unavailable {
                requires_openai_auth,
                ..
            } => *requires_openai_auth,
        }
    }

    pub(crate) fn into_presentation(
        self,
    ) -> (
        Option<Account>,
        Option<WorkspaceRouting>,
        bool,
        Option<String>,
    ) {
        match self {
            Self::Available(response) => (
                response.account,
                response.workspace_routing,
                response.requires_openai_auth,
                None,
            ),
            Self::Unavailable {
                requires_openai_auth,
                error,
            } => (None, None, requires_openai_auth, Some(error)),
        }
    }
}

/// A valid installed independent route allows the user to choose a provider
/// before repairing OpenAI sign-in. This neither changes nor authenticates one.
pub(crate) fn independent_provider_available(config: &Config) -> bool {
    #[cfg(unix)]
    {
        crate::termux_chat_models::installed_command(config).is_some()
            && crate::termux_chat_models::augment(config, Vec::new())
                .iter()
                .any(|model| model.model.starts_with("chatgpt-desktop/"))
    }
    #[cfg(not(unix))]
    {
        let _ = config;
        false
    }
}

pub(crate) fn can_defer_initial_login(target: &crate::AppServerTarget, config: &Config) -> bool {
    #[cfg(unix)]
    {
        crate::profile_transport::core_owned(target, crate::termux_permissions::enabled())
            && independent_provider_available(config)
    }
    #[cfg(not(unix))]
    {
        let _ = (target, config);
        false
    }
}

fn observe(
    result: color_eyre::Result<GetAccountResponse>,
    independent_provider: bool,
    requires_openai_auth: bool,
) -> color_eyre::Result<AccountObservation> {
    match result {
        Ok(account) => Ok(AccountObservation::Available(account)),
        Err(error)
            if independent_provider
                && error.chain().any(|cause| {
                    matches!(
                        cause.downcast_ref::<codex_app_server_client::TypedRequestError>(),
                        Some(codex_app_server_client::TypedRequestError::Server { method, source })
                            if method == "account/read" && source.code == -32603
                                && source.data.as_ref().and_then(|data| data.get("reason"))
                                    .and_then(serde_json::Value::as_str)
                                    == Some("accountAuthenticationUnavailable")
                    )
                }) =>
        {
            Ok(AccountObservation::Unavailable {
                requires_openai_auth,
                error: format!(
                    "OpenAI account information is unavailable: {error}. Use /login to repair sign-in or /provider to choose another installed provider."
                ),
            })
        }
        Err(error) => Err(error),
    }
}

pub(crate) async fn read(
    server: &mut AppServerSession,
    config: &Config,
) -> color_eyre::Result<AccountObservation> {
    let independent_provider = server.thread_params_mode() == ThreadParamsMode::Embedded
        && matches!(
            server.request_handle(),
            codex_app_server_client::AppServerRequestHandle::Remote(_)
        )
        && independent_provider_available(config);
    observe(
        server.read_account().await,
        independent_provider,
        config.model_provider.requires_openai_auth,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rejected_account(method: &str) -> color_eyre::Report {
        codex_app_server_client::TypedRequestError::Server {
            method: method.into(),
            source: codex_app_server_protocol::JSONRPCErrorError {
                code: -32603,
                message: "The refresh token was revoked.".into(),
                data: Some(serde_json::json!({"reason":"accountAuthenticationUnavailable"})),
            },
        }
        .into()
    }

    #[test]
    fn account_observation_preserves_unavailable_auth_without_fabricating_an_account() {
        for required in [false, true] {
            let observation =
                observe(Err(rejected_account("account/read")), true, required).unwrap();
            assert!(observation.account().is_none());
            assert_eq!(observation.requires_openai_auth(), required);
            let (account, routing, requires_auth, error) = observation.into_presentation();
            assert!(account.is_none());
            assert!(routing.is_none());
            assert_eq!(requires_auth, required);
            let error = error.unwrap();
            assert!(error.contains("refresh token was revoked"));
            assert!(error.contains("/provider"));
            assert!(error.contains("/login"));
        }
    }

    #[test]
    fn account_observation_never_hides_connection_policy_or_other_operation_errors() {
        for error in [
            rejected_account("model/list"),
            rejected_account("configRequirements/read"),
            codex_app_server_client::TypedRequestError::Transport {
                method: "account/read".into(),
                source: std::io::Error::new(
                    std::io::ErrorKind::ConnectionAborted,
                    "connection closed",
                ),
            }
            .into(),
        ] {
            assert!(observe(Err(error), true, true).is_err());
        }
        assert!(observe(Err(rejected_account("account/read")), false, true).is_err());
        for data in [
            None,
            Some(serde_json::json!({"reason":"requirementsUnavailable"})),
        ] {
            let error = codex_app_server_client::TypedRequestError::Server {
                method: "account/read".into(),
                source: codex_app_server_protocol::JSONRPCErrorError {
                    code: -32603,
                    message: "cloud config: authentication failed".into(),
                    data,
                },
            };
            assert!(observe(Err(error.into()), true, true).is_err());
        }
    }
}
