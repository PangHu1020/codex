use super::*;
use crate::context::CompactionSummary;
use chrono::TimeZone;

#[test]
fn compaction_summary_does_not_trigger_after_user_boundary_reminder() {
    let mut state = CurrentTimeReminderState {
        last_window_id: Some("window".to_string()),
        ..Default::default()
    };
    state.note_recorded_items(&[ContextualUserFragment::into(CompactionSummary::new(
        "checkpoint",
    ))]);

    assert!(!state.take_reminder_due(
        "window",
        Utc.timestamp_opt(1, 0).single().expect("valid timestamp"),
        /*interval_seconds*/ 0,
        CurrentTimeReminderDeliveryMode::AfterUserOrToolOutput,
    ));
}
