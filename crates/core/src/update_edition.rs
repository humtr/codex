//! Explicit edition selection; signed admission and rollback remain unchanged.
use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum Edition {
    #[default]
    Preserve,
    Core,
    Full,
}

impl Edition {
    pub(super) fn parse(args: Vec<OsString>) -> Result<(Self, Vec<OsString>), &'static str> {
        let mut edition = Self::Preserve;
        let mut rest = Vec::new();
        let mut index = 0;
        while index < args.len() {
            let arg = &args[index];
            let selected = match arg.to_str() {
                Some("--core") => Some(Self::Core),
                Some("--full") => Some(Self::Full),
                _ => None,
            };
            if let Some(selected) = selected {
                if edition != Self::Preserve {
                    return Err("select exactly one update edition: --core or --full");
                }
                edition = selected;
            } else {
                rest.push(arg.clone());
                if matches!(arg.to_str(), Some("--local" | "--remote")) {
                    index += 1;
                    if let Some(value) = args.get(index) {
                        rest.push(value.clone());
                    }
                }
            }
            index += 1;
        }
        if edition != Self::Preserve
            && rest
                .iter()
                .any(|v| v == "--rollback" || v == "--force" || v == "--help")
        {
            return Err("edition selection cannot be combined with rollback, force or help");
        }
        Ok((edition, rest))
    }

    pub(super) fn index_url(self, roots: &LocalCoreRoots) -> Result<String, LocalProductError> {
        // An explicit operator URL remains accepted, but its authenticated
        // manifest still has to match the requested edition before acquisition.
        if std::env::var_os(UPDATE_INDEX_URL_ENV).is_some() || self == Self::Preserve {
            return configured_update_index_url(roots);
        }
        Ok(match self {
            Self::Core => BASE_UPDATE_INDEX_URL,
            Self::Full => DEFAULT_UPDATE_INDEX_URL,
            Self::Preserve => unreachable!(),
        }
        .to_owned())
    }

    pub(super) fn validate(self, manifest: &LocalReleaseManifest) -> Result<(), LocalProductError> {
        let manager = manifest.files.iter().any(|f| f.relative_path == "manager");
        let tui = manifest
            .files
            .iter()
            .any(|f| f.relative_path == "helpers/2");
        if (self == Self::Core && (manager || tui)) || (self == Self::Full && !(manager && tui)) {
            return Err(LocalProductError::Release(
                "authenticated release does not match the selected Core/Full edition",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_selection_is_unique_and_does_not_consume_source_arguments() {
        let (edition, rest) = Edition::parse(vec!["--core".into(), "--details".into()]).unwrap();
        assert_eq!(edition, Edition::Core);
        assert_eq!(rest, [OsString::from("--details")]);
        assert_eq!(
            Edition::parse(vec!["--local".into(), "--full".into()])
                .unwrap()
                .0,
            Edition::Preserve
        );
        for args in [
            vec!["--core", "--full"],
            vec!["--core", "--core"],
            vec!["--full", "--rollback"],
            vec!["--core", "--force"],
        ] {
            assert!(Edition::parse(args.into_iter().map(OsString::from).collect()).is_err());
        }
    }
    #[test]
    fn selected_edition_checks_inventory_not_generation_label() {
        let mut manifest = LocalReleaseManifest {
            format: LocalReleaseFormat::CoordinatedV4,
            generation_id: "core-v1.0.0-mislabel-s2".into(),
            release_sequence: 2,
            channel: "stable".into(),
            expected_platform: "android".into(),
            expected_architecture: "aarch64".into(),
            core_api_identity: "core-api-v1".into(),
            persistent_schema_identity: "schema-v1".into(),
            release_public_key: ReleasePublicKey([1; 32]),
            files: vec![],
        };
        assert!(Edition::Core.validate(&manifest).is_ok());
        assert!(Edition::Full.validate(&manifest).is_err());
        for path in ["manager", "helpers/2"] {
            manifest.files.push(ReleaseFileEntry {
                relative_path: path.into(),
                sha256: "a".repeat(64),
                mode: 0o755,
            });
        }
        assert!(Edition::Core.validate(&manifest).is_err());
        assert!(Edition::Full.validate(&manifest).is_ok());
        assert!(Edition::Preserve.validate(&manifest).is_ok());
    }
}
