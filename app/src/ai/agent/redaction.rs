use crate::secret_redaction::{find_secrets_in_text, SECRET_REDACTION_REPLACEMENT_CHARACTER};
use crate::ai::agent::{
    AIAgentAttachment, AIAgentContext, AskUserQuestionAnswerItem, AskUserQuestionResult, BlockContext,
};

/// Redact all detected secrets in-place within the given string.
pub(crate) fn redact_secrets(input: &mut String) {
    let mut secrets: Vec<_> = find_secrets_in_text(input)
        .into_iter()
        .map(|r| r.byte_range)
        .collect();
    // Replace from the end to preserve indices
    secrets.sort_by_key(|range| range.start);
    for range in secrets.into_iter().rev() {
        let replacement =
            SECRET_REDACTION_REPLACEMENT_CHARACTER.repeat(range.end.saturating_sub(range.start));
        input.replace_range(range.start..range.end, &replacement);
    }
}



fn redact_ask_user_question_result(result: &mut AskUserQuestionResult) {
    match result {
        AskUserQuestionResult::Success { answers } => {
            for answer in answers {
                if let AskUserQuestionAnswerItem::Answered { other_text, .. } = answer {
                    redact_secrets(other_text);
                }
            }
        }
        AskUserQuestionResult::SkippedByAutoApprove { .. } => {}
        AskUserQuestionResult::Error(message) => redact_secrets(message),
        AskUserQuestionResult::Cancelled => {}
    }
}
fn redact_context(context: &mut [AIAgentContext]) {
    for context_item in context {
        match context_item {
            AIAgentContext::Block(context) => {
                redact_secrets(&mut context.command);
                redact_secrets(&mut context.output);
            }
            AIAgentContext::SelectedText(text) => {
                redact_secrets(text);
            }
            // Other context types don't contain user-provided text that needs redaction
            AIAgentContext::Directory { .. }
            | AIAgentContext::ExecutionEnvironment(_)
            | AIAgentContext::CurrentTime { .. }
            | AIAgentContext::Image(_)
            | AIAgentContext::Codebase { .. }
            | AIAgentContext::ProjectRules { .. }
            | AIAgentContext::Git { .. }
            | AIAgentContext::File(_)
            | AIAgentContext::Skills { .. } => {}
        }
    }
}

fn redact_attachment(attachment: &mut AIAgentAttachment) {
    match attachment {
        AIAgentAttachment::PlainText(text) => {
            redact_secrets(text);
        }
        AIAgentAttachment::Block(BlockContext {
            command, output, ..
        }) => {
            redact_secrets(command);
            redact_secrets(output);
        }
        AIAgentAttachment::DriveObject { payload, .. } => {
            if let Some(drive_payload) = payload {
                match drive_payload {
                    crate::ai::agent::DriveObjectPayload::Workflow {
                        name,
                        description,
                        command,
                    } => {
                        redact_secrets(name);
                        redact_secrets(description);
                        redact_secrets(command);
                    }
                    crate::ai::agent::DriveObjectPayload::Notebook { title, content } => {
                        redact_secrets(title);
                        redact_secrets(content);
                    }
                    crate::ai::agent::DriveObjectPayload::GenericStringObject {
                        payload, ..
                    } => {
                        redact_secrets(payload);
                    }
                }
            }
        }
        AIAgentAttachment::DiffHunk {
            file_path,
            diff_content,
            ..
        } => {
            redact_secrets(file_path);
            redact_secrets(diff_content);
        }
        AIAgentAttachment::DiffSet { file_diffs, .. } => {
            for hunks in file_diffs.values_mut() {
                for hunk in hunks {
                    redact_secrets(&mut hunk.diff_content);
                }
            }
        }
        AIAgentAttachment::DocumentContent { content, .. } => {
            redact_secrets(content);
        }
        // FilePathReference only contains a file ID and filename, no user secrets.
        AIAgentAttachment::FilePathReference { .. } => {}
    }
}
