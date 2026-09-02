use super::ContextualUserFragment;
use codex_prompts::SUMMARY_PREFIX;
use codex_protocol::models::ContentItem;
use codex_protocol::models::ContentItemKind;
use codex_protocol::models::ResponseItem;

const COMPACTION_SUMMARY_CONTENT_KIND: &str = "compaction.summary";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompactionSummary {
    summary: String,
}

impl CompactionSummary {
    pub(crate) fn new(summary: impl Into<String>) -> Self {
        Self {
            summary: summary.into(),
        }
    }
}

impl ContextualUserFragment for CompactionSummary {
    fn content_kind(&self) -> ContentItemKind {
        ContentItemKind(COMPACTION_SUMMARY_CONTENT_KIND.to_string())
    }

    fn role(&self) -> &'static str {
        "user"
    }

    fn markers(&self) -> (&'static str, &'static str) {
        Self::type_markers()
    }

    fn type_markers() -> (&'static str, &'static str) {
        ("", "")
    }

    fn body(&self) -> String {
        self.summary.clone()
    }
}

pub(crate) fn is_compaction_summary_text(text: &str) -> bool {
    text.strip_prefix(SUMMARY_PREFIX)
        .is_some_and(|suffix| suffix.starts_with('\n'))
}

pub(crate) fn is_compaction_summary_item(item: &ResponseItem) -> bool {
    let ResponseItem::Message {
        role,
        content,
        internal_chat_message_metadata_passthrough,
        ..
    } = item
    else {
        return false;
    };
    if role != "user" {
        return false;
    }

    match internal_chat_message_metadata_passthrough
        .as_ref()
        .and_then(|metadata| metadata.content_item_kinds.as_deref())
    {
        Some(kinds) if kinds.len() == content.len() => {
            if kinds
                .iter()
                .any(|kind| kind.0 == COMPACTION_SUMMARY_CONTENT_KIND)
            {
                return kinds
                    .iter()
                    .all(|kind| kind.0 == COMPACTION_SUMMARY_CONTENT_KIND);
            }
            if kinds.iter().any(|kind| kind.0 != "unknown") {
                return false;
            }
        }
        Some(_) => return false,
        None => {}
    }

    content.iter().any(|item| {
        matches!(
            item,
            ContentItem::InputText { text } | ContentItem::OutputText { text }
                if is_compaction_summary_text(text)
        )
    })
}

#[cfg(test)]
#[path = "compaction_summary_tests.rs"]
mod tests;
