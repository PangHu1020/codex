use super::LOCAL_COMPACTION_SUMMARY_MAX_TOKENS;
use super::build_local_compaction_summary;
use codex_prompts::SUMMARY_PREFIX;
use codex_utils_output_truncation::approx_token_count;
use pretty_assertions::assert_eq;

#[test]
fn short_local_compaction_summary_is_byte_identical() {
    let summary = build_local_compaction_summary("short summary");

    assert_eq!(summary.text, format!("{SUMMARY_PREFIX}\nshort summary"));
    assert!(!summary.truncated);
}

#[test]
fn long_local_compaction_summary_caps_the_complete_prefixed_message() {
    let summary = build_local_compaction_summary(&format!(
        "retained start {}retained end",
        "middle ".repeat(20_000)
    ));

    assert!(summary.truncated);
    assert!(approx_token_count(&summary.text) <= LOCAL_COMPACTION_SUMMARY_MAX_TOKENS);
    assert!(summary.text.starts_with(&format!("{SUMMARY_PREFIX}\n")));
    assert!(summary.text.ends_with("retained end"));
}
