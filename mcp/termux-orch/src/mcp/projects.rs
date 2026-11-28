use super::*;

impl OrchMcpHandler {
    pub(super) fn resolve_front_chat_by_marker(
        &self,
        marker: &str,
        project_id: Option<&str>,
    ) -> Result<String, String> {
        let now = now_unix();
        let mut candidates = Vec::new();
        for source in project_id
            .into_iter()
            .map(Some)
            .chain(std::iter::once(None))
        {
            let page = self.owners.chatgpt_recent_chats(source)?;
            let rows = page["chats"].as_array().ok_or_else(|| {
                page["error"]
                    .as_str()
                    .unwrap_or("invalid recent chat list")
                    .to_owned()
            })?;
            for row in rows.iter().take(2) {
                if let (Some(id), Some(updated)) =
                    (row["id"].as_str(), row["update_time_unix"].as_u64())
                {
                    if updated >= now.saturating_sub(600) && updated <= now {
                        candidates.push((updated, id.to_owned()));
                    }
                }
            }
        }
        candidates.sort_unstable_by(|a, b| b.cmp(a));
        let mut seen = std::collections::HashSet::new();
        let mut matched = None;
        for (_, cid) in candidates
            .into_iter()
            .filter(|(_, cid)| seen.insert(cid.clone()))
            .take(2)
        {
            let chat = self.owners.chatgpt_read_chat(&cid, 6)?;
            if crate::owners::is_rate_limited(&chat) {
                return Err(chat["error"]
                    .as_str()
                    .unwrap_or("front_marker_rate_limited")
                    .to_owned());
            }
            if chat["state"] == "ok"
                && chat["messages"].as_array().is_some_and(|messages| {
                    messages.iter().any(|message| {
                        message["role"] == "user"
                            && message["text"]
                                .as_str()
                                .is_some_and(|text| text.contains(marker))
                    })
                })
            {
                if matched.is_some() {
                    return Err(
                        "front_marker_ambiguous: multiple recent chats contain the marker"
                            .to_owned(),
                    );
                }
                matched = Some(cid);
            }
        }
        matched.ok_or_else(|| {
            "front_marker_not_found: no recent chat contained the user message marker (pass front_chat_id directly or ensure the marker is in your latest message)".to_owned()
        })
    }

    pub(super) fn current_account_id(&self) -> Option<String> {
        let status = self.owners.chatgpt_status().ok()?;
        crate::owners::chatgpt_account_identity(&status).map(ToOwned::to_owned)
    }

    pub(super) fn bind_project(
        &self,
        scope: &str,
        label: Option<&str>,
        explicit: Option<&str>,
    ) -> Result<(String, String, String), String> {
        let account = self.owners.chatgpt_account_id()?;
        let key = sha256_hex(json!([account, scope]).to_string().as_bytes());
        let (_lock, path) = self
            .ledger
            .lock_project_scope(&sha256_hex(account.as_bytes()), &key)
            .map_err(|e| e.to_string())?;
        let previous: Option<ProjectBinding> = match read_private_file(&path, 8192) {
            Ok(bytes) => Some(serde_json::from_slice(&bytes).map_err(|e| e.to_string())?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(e.to_string()),
        };
        if previous
            .as_ref()
            .zip(label)
            .is_some_and(|(bound, given)| bound.name != given)
        {
            return Err("project_scope is already bound to a different project_name".to_owned());
        }
        let previous_id = previous.as_ref().and_then(|binding| binding.id.as_deref());
        if previous_id
            .zip(explicit)
            .is_some_and(|(bound, given)| bound != given)
        {
            return Err("project_scope is already bound to another project_id".to_owned());
        }
        let name = previous
            .as_ref()
            .map(|b| b.name.as_str())
            .or(label)
            .unwrap_or(scope);
        let id = previous_id.or(explicit);
        let persist = |binding: &ProjectBinding| {
            let data = serde_json::to_vec(binding).map_err(|e| e.to_string())?;
            write_atomic_private(&path, &data).map_err(|e| e.to_string())
        };
        let require_account = || {
            if self.owners.chatgpt_account_id()? == account {
                Ok(())
            } else {
                Err("ChatGPT account changed during project binding".to_owned())
            }
        };
        require_account()?;
        let (id, name) = match self.owners.chatgpt_find_project(name, id)? {
            Some((id, actual)) => {
                if label.is_some_and(|given| given != actual) {
                    return Err(
                        "explicit project_name does not match the selected project".to_owned()
                    );
                }
                (id, actual)
            }
            None if id.is_some() => {
                return Err("project_id is unavailable in the pinned account".to_owned())
            }
            None if previous.is_some() => return Err(
                "project creation is unresolved; supply its project_id after checking the account"
                    .to_owned(),
            ),
            None => {
                require_account()?;
                persist(&ProjectBinding {
                    name: name.to_owned(),
                    id: None,
                })?;
                let created = self.owners.chatgpt_create_project(name, None)?;
                if crate::owners::is_rate_limited(&created) {
                    // The admission gate rejected before project creation. Release only this
                    // known-unsubmitted claim, retaining transport-uncertain claims.
                    std::fs::remove_file(&path).map_err(|error| error.to_string())?;
                    std::fs::File::open(path.parent().expect("project binding directory"))
                        .and_then(|dir| dir.sync_all())
                        .map_err(|error| error.to_string())?;
                    return Err(created["error"]
                        .as_str()
                        .unwrap_or("project_rate_limited")
                        .to_owned());
                }
                let id = created["project_id"]
                    .as_str()
                    .filter(|id| !id.is_empty())
                    .ok_or("project creation response lacks an ID")?;
                (
                    id.to_owned(),
                    created["name"].as_str().unwrap_or(name).to_owned(),
                )
            }
        };
        require_account()?;
        persist(&ProjectBinding {
            name: name.clone(),
            id: Some(id.clone()),
        })?;
        Ok((account, id, name))
    }
}
