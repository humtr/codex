//! Provider-owned model capabilities. This describes adapter support, not account entitlement.
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ModelCapability {
    pub id: &'static str,
    pub family: &'static str,
    pub display_name: &'static str,
    pub efforts: &'static [&'static str],
    pub pro: bool,
    pub execution_unverified: bool,
}

const GPT6_EFFORTS: &[&str] = &["none", "medium", "high", "xhigh"];
const PRO_EFFORTS: &[&str] = &["none"];

pub const CATALOG: &[ModelCapability] = &[
    ModelCapability {
        id: "chatgpt-desktop/gpt-6",
        family: "GPT-6",
        display_name: "ChatGPT · GPT-6",
        efforts: GPT6_EFFORTS,
        pro: false,
        execution_unverified: false,
    },
    ModelCapability {
        id: "chatgpt-desktop/gpt-6-pro",
        family: "GPT-6",
        display_name: "ChatGPT · GPT-6 Pro",
        efforts: PRO_EFFORTS,
        pro: true,
        execution_unverified: true,
    },
];

pub fn find(id: &str) -> Option<&'static ModelCapability> {
    CATALOG.iter().find(|model| model.id == id)
}

pub fn supports(id: &str, effort: &str) -> bool {
    find(id).is_some_and(|model| model.efforts.contains(&effort))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_contains_only_currently_admitted_rust_models() {
        assert_eq!(CATALOG.len(), 2);
        assert!(find("chatgpt-desktop/gpt-6").is_some());
        assert!(find("chatgpt-desktop/gpt-5.6-sol").is_none());
        assert!(find("chatgpt-desktop/gpt-6-sol").is_none());
        assert!(
            find("chatgpt-desktop/gpt-6-pro")
                .unwrap()
                .execution_unverified
        );
    }

    #[test]
    fn effort_combinations_match_public_app_controls() {
        for effort in GPT6_EFFORTS {
            assert!(supports("chatgpt-desktop/gpt-6", effort));
        }
        assert!(!supports("chatgpt-desktop/gpt-6", "instant"));
        assert!(supports("chatgpt-desktop/gpt-6-pro", "none"));
        assert!(!supports("chatgpt-desktop/gpt-6-pro", "high"));
        assert!(!supports("chatgpt-desktop/gpt-6", "toString"));
    }
}
