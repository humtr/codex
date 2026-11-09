// Semantic qualification fixture for Core's sandbox precheck.
// Compile against the actual codex-utils-cli and clap artifacts from the
// qualified upstream source, rather than copying its argument parser or macro.
// Verified upstream: 086ccc833bc9cf01024ad2dd6af7dbd421b6bf9a.
use clap::Parser;
use codex_utils_cli::{CliConfigOverrides, SharedCliOptions};

#[derive(Parser)]
struct Probe {
    #[command(flatten)]
    shared: SharedCliOptions,
}

fn main() {
    for flag in ["--approve-for-me", "--not-so-yolo"] {
        let mut parsed = Probe::try_parse_from(["codex", flag]).expect("upstream accepts alias");
        assert!(parsed.shared.auto_review);
        let mut overrides = CliConfigOverrides::default();
        parsed
            .shared
            .take_auto_review_config_overrides(&mut overrides);
        assert_eq!(
            overrides.raw_overrides,
            [
                r#"approvals_reviewer="auto_review""#,
                r#"approval_policy="on-request""#,
                r#"sandbox_mode="workspace-write""#,
            ]
        );
        assert!(!parsed.shared.auto_review);
        println!("{flag}: workspace-write");
    }
}
