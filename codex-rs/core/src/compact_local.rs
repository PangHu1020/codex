use crate::Prompt;
use crate::client::ModelClientSession;
use crate::client_common::ResponseEvent;
use crate::responses_metadata::CodexResponsesMetadata;
use crate::session::session::Session;
use crate::session::turn_context::TurnContext;
use crate::stream_events_utils::last_assistant_message_from_item;
use codex_protocol::error::CodexErr;
use codex_protocol::error::Result as CodexResult;
use codex_protocol::models::MessagePhase;
use codex_protocol::models::ResponseItem;
use codex_rollout_trace::InferenceTraceContext;
use codex_utils_output_truncation::TruncationPolicy;
use codex_utils_output_truncation::approx_token_count;
use codex_utils_output_truncation::truncate_text;
use futures::StreamExt;

use codex_prompts::SUMMARY_PREFIX;

pub(crate) const LOCAL_COMPACTION_SUMMARY_MAX_TOKENS: usize = 4096;
const LOCAL_COMPACTION_SUMMARY_TRUNCATION_RESERVE_TOKENS: usize = 16;

pub(crate) struct LocalCompactionSummary {
    pub(crate) text: String,
    pub(crate) truncated: bool,
}

pub(crate) struct LocalCompactionOutput {
    pub(crate) response_id: String,
    pub(crate) summary_text: String,
}

pub(crate) fn build_local_compaction_summary(raw_summary: &str) -> LocalCompactionSummary {
    let prefix = format!("{SUMMARY_PREFIX}\n");
    let full_summary = format!("{prefix}{raw_summary}");
    if approx_token_count(&full_summary) <= LOCAL_COMPACTION_SUMMARY_MAX_TOKENS {
        return LocalCompactionSummary {
            text: full_summary,
            truncated: false,
        };
    }

    let mut budget = LOCAL_COMPACTION_SUMMARY_MAX_TOKENS
        .saturating_sub(LOCAL_COMPACTION_SUMMARY_TRUNCATION_RESERVE_TOKENS);
    loop {
        let candidate = truncate_text(&full_summary, TruncationPolicy::Tokens(budget));
        let actual = approx_token_count(&candidate);
        if actual <= LOCAL_COMPACTION_SUMMARY_MAX_TOKENS || budget == 0 {
            return LocalCompactionSummary {
                text: candidate,
                truncated: true,
            };
        }
        budget = budget.saturating_sub(
            actual
                .saturating_sub(LOCAL_COMPACTION_SUMMARY_MAX_TOKENS)
                .max(1),
        );
    }
}

pub(crate) async fn collect_local_compaction_output(
    sess: &Session,
    turn_context: &TurnContext,
    client_session: &mut ModelClientSession,
    responses_metadata: &CodexResponsesMetadata,
    prompt: &Prompt,
) -> CodexResult<LocalCompactionOutput> {
    let mut stream = client_session
        .stream(
            prompt,
            turn_context.model_info(),
            &turn_context.session_telemetry,
            turn_context.reasoning_effort().cloned(),
            turn_context.reasoning_summary(),
            turn_context.config.service_tier.clone(),
            responses_metadata,
            // Rollout tracing currently models remote compaction only; local compaction streams
            // are left untraced until the reducer has a first-class local compaction lifecycle.
            &InferenceTraceContext::disabled(),
        )
        .await?;
    let mut summary_text = None;
    loop {
        let Some(event) = stream.next().await else {
            return Err(CodexErr::Stream(
                "local compaction stream closed before response.completed".into(),
            ));
        };
        match event {
            Ok(ResponseEvent::OutputItemDone(item)) => {
                if !matches!(
                    &item,
                    ResponseItem::Message {
                        phase: Some(MessagePhase::Commentary),
                        ..
                    }
                ) && let Some(message) =
                    last_assistant_message_from_item(&item, /*plan_mode*/ false)
                {
                    summary_text = Some(message);
                }
                sess.record_conversation_items(turn_context, std::slice::from_ref(&item))
                    .await;
            }
            Ok(ResponseEvent::ServerReasoningIncluded(included)) => {
                sess.set_server_reasoning_included(included).await;
            }
            Ok(ResponseEvent::RateLimits(snapshot)) => {
                sess.update_rate_limits(turn_context, snapshot).await;
            }
            Ok(ResponseEvent::Completed {
                response_id,
                token_usage,
                usage_metadata,
                ..
            }) => {
                sess.record_observed_response_completed(
                    turn_context,
                    &response_id,
                    token_usage.as_ref(),
                    usage_metadata.as_ref(),
                )
                .await;
                sess.update_token_usage_info(turn_context, token_usage.as_ref())
                    .await?;
                let Some(summary_text) = summary_text else {
                    return Err(CodexErr::Fatal(
                        "local compaction response.completed contained no visible final assistant summary"
                            .to_string(),
                    ));
                };
                return Ok(LocalCompactionOutput {
                    response_id,
                    summary_text,
                });
            }
            Ok(_) => continue,
            Err(error) => return Err(error),
        }
    }
}

#[cfg(test)]
#[path = "compact_local_tests.rs"]
mod tests;
