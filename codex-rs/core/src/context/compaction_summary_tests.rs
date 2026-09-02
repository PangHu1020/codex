use super::*;
use codex_protocol::models::InternalChatMessageMetadataPassthrough;
use pretty_assertions::assert_eq;

fn user_message(text: &str, content_item_kinds: Option<Vec<&str>>) -> ResponseItem {
    ResponseItem::Message {
        id: None,
        role: "user".to_string(),
        content: vec![ContentItem::InputText {
            text: text.to_string(),
        }],
        phase: None,
        internal_chat_message_metadata_passthrough: content_item_kinds.map(|kinds| {
            InternalChatMessageMetadataPassthrough {
                content_item_kinds: Some(
                    kinds
                        .into_iter()
                        .map(|kind| ContentItemKind(kind.to_string()))
                        .collect(),
                ),
                ..Default::default()
            }
        }),
    }
}

#[test]
fn compaction_summary_classifier_prefers_valid_metadata_and_falls_back_for_legacy_messages() {
    let prefix_text = format!("{SUMMARY_PREFIX}\ncheckpoint");
    let mut mixed_metadata_message = user_message(
        &prefix_text,
        Some(vec![COMPACTION_SUMMARY_CONTENT_KIND, "user.text"]),
    );
    let ResponseItem::Message { content, .. } = &mut mixed_metadata_message else {
        panic!("expected user message");
    };
    content.push(ContentItem::InputText {
        text: "real user text".to_string(),
    });
    let cases = [
        (
            user_message(
                "metadata summary",
                Some(vec![COMPACTION_SUMMARY_CONTENT_KIND]),
            ),
            true,
        ),
        (user_message(&prefix_text, Some(vec!["user.text"])), false),
        (
            user_message(
                &prefix_text,
                Some(vec![COMPACTION_SUMMARY_CONTENT_KIND, "user.text"]),
            ),
            false,
        ),
        (mixed_metadata_message, false),
        (user_message(&prefix_text, Some(vec!["unknown"])), true),
        (user_message(&prefix_text, None), true),
        (user_message("## Objective\nnot a checkpoint", None), false),
    ];

    assert_eq!(
        cases
            .iter()
            .map(|(item, _)| is_compaction_summary_item(item))
            .collect::<Vec<_>>(),
        cases
            .iter()
            .map(|(_, expected)| *expected)
            .collect::<Vec<_>>()
    );
}
