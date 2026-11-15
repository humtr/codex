//! App-home registrations. The official app owns every credential inside a home.
use crate::{
    app::Identity,
    context::digest,
    journal::{private_directory, random_id, Journal},
    protocol::Error,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    os::fd::AsRawFd,
    path::{Path, PathBuf},
};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Account {
    directory: String,
    enabled: bool,
    identity: Option<Identity>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Accounts {
    selected: String,
    accounts: BTreeMap<String, Account>,
}
fn key() -> String {
    digest(&"provider-app-accounts-v1")
}
fn label(value: &str) -> Result<(), Error> {
    if value.is_empty()
        || value.len() > 64
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
    {
        return Err(Error("app_profile_name_invalid"));
    }
    Ok(())
}
impl Accounts {
    pub(crate) fn load(journal: &Journal) -> Result<Self, Error> {
        let value = journal.read::<Self>(&key())?.unwrap_or_else(|| Self {
            selected: "default".into(),
            accounts: BTreeMap::from([(
                "default".into(),
                Account {
                    directory: "home".into(),
                    enabled: true,
                    identity: None,
                },
            )]),
        });
        let mut homes = BTreeSet::new();
        for (name, account) in &value.accounts {
            label(name)?;
            if !homes.insert(&account.directory)
                || !(account.directory == "home"
                    || account.directory.len() == 48
                        && account.directory.bytes().all(|b| b.is_ascii_hexdigit()))
            {
                return Err(Error("app_profile_record_invalid"));
            }
            if let Some(identity) = &account.identity {
                identity.validate()?;
            }
        }
        value.get(&value.selected)?;
        Ok(value)
    }
    pub(crate) fn save(&self, journal: &Journal) -> Result<(), Error> {
        journal.write(&key(), self)
    }
    fn get(&self, name: &str) -> Result<&Account, Error> {
        label(name)?;
        self.accounts
            .get(name)
            .filter(|a| a.enabled)
            .ok_or(Error("app_profile_not_registered"))
    }
    pub(crate) fn selected(&self) -> &str {
        &self.selected
    }
    pub(crate) fn home(&self, state: &Path, name: &str) -> Result<PathBuf, Error> {
        label(name)?;
        let account = self
            .accounts
            .get(name)
            .ok_or(Error("app_profile_not_registered"))?;
        Ok(if account.directory == "home" {
            state.join("desktop/home")
        } else {
            state.join("desktop/accounts").join(&account.directory)
        })
    }
    pub(crate) fn expected_identity(&self, name: &str) -> Result<Option<&Identity>, Error> {
        let account = self.get(name)?;
        match &account.identity {
            None if account.directory != "home" => Err(Error("app_profile_login_required")),
            expected => Ok(expected.as_ref()),
        }
    }
    pub(crate) fn confirm(&mut self, name: &str, identity: Identity) -> Result<(), Error> {
        self.get(name)?;
        if self.accounts[name]
            .identity
            .as_ref()
            .is_some_and(|old| !old.account_matches(&identity))
        {
            return Err(Error("app_profile_account_mismatch"));
        }
        self.accounts.get_mut(name).unwrap().identity = Some(identity);
        Ok(())
    }
    pub(crate) fn list(&self) -> Value {
        json!({"selected":self.selected,"profiles":self.accounts.iter().map(|(name,a)|json!({"profile":name,"registered":a.enabled,"selected":name==&self.selected,"identity":a.identity,"verified_now":false})).collect::<Vec<_>>(),"inference_submitted":false})
    }
    pub(crate) fn add(&mut self, name: &str) -> Result<(), Error> {
        label(name)?;
        if let Some(account) = self.accounts.get_mut(name) {
            if account.enabled {
                return Err(Error("app_profile_already_registered"));
            }
            account.enabled = true;
        } else {
            self.accounts.insert(
                name.into(),
                Account {
                    directory: random_id()?,
                    enabled: true,
                    identity: None,
                },
            );
        }
        Ok(())
    }
    pub(crate) fn rename(&mut self, old: &str, new: &str) -> Result<(), Error> {
        self.get(old)?;
        label(new)?;
        if self.accounts.contains_key(new) {
            return Err(Error("app_profile_already_registered"));
        }
        let account = self.accounts.remove(old).unwrap();
        self.accounts.insert(new.into(), account);
        if self.selected == old {
            self.selected = new.into();
        }
        Ok(())
    }
    pub(crate) fn select(&mut self, name: &str) -> Result<(), Error> {
        self.get(name)?;
        self.selected = name.into();
        Ok(())
    }
    pub(crate) fn remove(&mut self, state: &Path, name: &str, purge: bool) -> Result<(), Error> {
        label(name)?;
        if !self.accounts.contains_key(name) {
            return Err(Error("app_profile_not_registered"));
        }
        if self.selected == name {
            return Err(Error("app_profile_selected"));
        }
        if purge {
            let home = self.home(state, name)?;
            match std::fs::symlink_metadata(&home) {
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                Err(_) => return Err(Error("app_profile_home_unavailable")),
                Ok(_) => {
                    private_directory(&state.join("desktop"))?;
                    if self.accounts[name].directory != "home" {
                        private_directory(&state.join("desktop/accounts"))?;
                    }
                    private_directory(&home)?;
                    let lease =
                        File::open(&home).map_err(|_| Error("app_profile_home_unavailable"))?;
                    if unsafe { libc::flock(lease.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0
                    {
                        return Err(Error("host_profile_busy"));
                    }
                    std::fs::remove_dir_all(&home)
                        .map_err(|_| Error("app_profile_purge_failed"))?;
                }
            }
            self.accounts.remove(name);
        } else {
            self.accounts.get_mut(name).unwrap().enabled = false;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn purge_rejects_live_or_redirected_homes_and_preserves_other_data() {
        use std::os::unix::fs::symlink;
        let root = std::env::temp_dir().join(format!("accounts-{}", random_id().unwrap()));
        let journal = Journal::open(&root).unwrap();
        let mut accounts = Accounts::load(&journal).unwrap();
        accounts.add("second").unwrap();
        let home = accounts.home(&root, "second").unwrap();
        private_directory(home.parent().unwrap()).unwrap();
        let outside = root.join("unrelated");
        private_directory(&outside).unwrap();
        std::fs::write(outside.join("unique"), "keep").unwrap();
        symlink(&outside, &home).unwrap();
        assert!(accounts.remove(&root, "second", true).is_err());
        assert_eq!(
            std::fs::read_to_string(outside.join("unique")).unwrap(),
            "keep"
        );
        std::fs::remove_file(&home).unwrap();
        private_directory(&home).unwrap();
        let lease = File::open(&home).unwrap();
        assert_eq!(
            unsafe { libc::flock(lease.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) },
            0
        );
        assert!(accounts.remove(&root, "second", true).is_err());
        drop(lease);
        accounts.remove(&root, "second", false).unwrap();
        accounts.remove(&root, "second", true).unwrap();
        assert!(!home.exists());
        assert!(outside.join("unique").is_file());
        drop(journal);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn legacy_home_and_removed_login_survive_registration_changes() {
        let root = std::env::temp_dir().join(format!("accounts-{}", random_id().unwrap()));
        let journal = Journal::open(&root).unwrap();
        let mut accounts = Accounts::load(&journal).unwrap();
        let legacy = accounts.home(&root, "default").unwrap();
        private_directory(&legacy).unwrap();
        std::fs::write(legacy.join("auth-fixture"), "preserve-original").unwrap();
        accounts.rename("default", "first").unwrap();
        assert_eq!(accounts.home(&root, "first").unwrap(), legacy);
        accounts.add("second").unwrap();
        let second = accounts.home(&root, "second").unwrap();
        assert_ne!(legacy, second);
        private_directory(&second).unwrap();
        std::fs::write(second.join("auth-fixture"), "preserve-second").unwrap();
        accounts.remove(&root, "second", false).unwrap();
        assert!(accounts.select("second").is_err());
        accounts.add("second").unwrap();
        assert_eq!(accounts.home(&root, "second").unwrap(), second);
        accounts.save(&journal).unwrap();
        let restored = Accounts::load(&journal).unwrap();
        assert_eq!(
            std::fs::read_to_string(restored.home(&root, "first").unwrap().join("auth-fixture"))
                .unwrap(),
            "preserve-original"
        );
        assert_eq!(
            std::fs::read_to_string(restored.home(&root, "second").unwrap().join("auth-fixture"))
                .unwrap(),
            "preserve-second"
        );
        assert!(accounts.remove(&root, "first", true).is_err());
        drop(journal);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn identity_binding_rejects_unconfirmed_or_wrong_account() {
        let root = std::env::temp_dir().join(format!("accounts-{}", random_id().unwrap()));
        let journal = Journal::open(&root).unwrap();
        let mut accounts = Accounts::load(&journal).unwrap();
        accounts.add("second").unwrap();
        let one=Identity::parse(&json!({"generation":"first","account":"account-a","user":"user-a","structure":"personal"})).unwrap();
        let other=Identity::parse(&json!({"generation":"second","account":"account-b","user":"user-b","structure":"personal"})).unwrap();
        assert!(accounts.expected_identity("second").is_err());
        accounts.confirm("second", one.clone()).unwrap();
        assert!(accounts
            .expected_identity("second")
            .unwrap()
            .unwrap()
            .account_matches(&one));
        assert!(accounts.confirm("second", other).is_err());
        assert!(accounts.add("../escape").is_err());
        drop(journal);
        std::fs::remove_dir_all(root).unwrap();
    }
}
