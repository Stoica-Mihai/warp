use std::sync::Arc;

use warpui::elements::SecretRange;

use crate::terminal::model::secrets::{SecretLevel, SecretsRegex, SECRETS_REGEX};

pub const SECRET_REDACTION_REPLACEMENT_CHARACTER: &str = "*";

pub fn find_secrets_in_text(text: &str) -> Vec<SecretRange> {
    find_secrets_in_text_with_levels(text)
        .into_iter()
        .map(|(range, _level)| range)
        .collect()
}

pub fn find_secrets_in_text_with_levels(text: &str) -> Vec<(SecretRange, SecretLevel)> {
    let secrets_regex: Arc<SecretsRegex> = { SECRETS_REGEX.lock().clone() };
    find_secrets_in_text_with_levels_using_regex(text, &secrets_regex)
}

pub fn find_secrets_in_text_with_levels_using_regex(
    text: &str,
    secrets_regex: &SecretsRegex,
) -> Vec<(SecretRange, SecretLevel)> {
    let SecretsRegex {
        regex,
        level_metadata,
        ..
    } = secrets_regex;

    let mut secret_ranges = vec![];
    let mut byte_to_char_index = vec![0; text.len() + 1];

    let mut char_index = 0;
    for (byte_index, _) in text.char_indices() {
        byte_to_char_index[byte_index] = char_index;
        char_index += 1;
    }
    byte_to_char_index[text.len()] = char_index;

    for mat in regex.find_iter(text) {
        let start_byte = mat.start();
        let end_byte = mat.end();
        let start_char = byte_to_char_index[start_byte];
        let end_char = byte_to_char_index[end_byte];

        let pattern_id = mat.pattern().as_usize();
        let total_patterns = level_metadata.enterprise_count + level_metadata.user_count;
        if pattern_id >= total_patterns {
            log::error!("Secret level not found for pattern ID {pattern_id}");
            continue;
        }
        let secret_level = if pattern_id < level_metadata.enterprise_count {
            SecretLevel::Enterprise
        } else {
            SecretLevel::User
        };

        secret_ranges.push((
            SecretRange {
                char_range: start_char..end_char,
                byte_range: start_byte..end_byte,
            },
            secret_level,
        ));
    }

    merge_sorted_ranges_with_levels(secret_ranges)
}

fn merge_sorted_ranges_with_levels(
    ranges: Vec<(SecretRange, SecretLevel)>,
) -> Vec<(SecretRange, SecretLevel)> {
    if ranges.is_empty() {
        return ranges;
    }

    let mut merged_ranges = vec![];
    let mut current_range = ranges[0].0.clone();
    let mut current_level = ranges[0].1;

    for (range, level) in ranges.into_iter().skip(1) {
        if range.char_range.start <= current_range.char_range.end {
            current_range.extend_range_end(&range);
            if level.priority() > current_level.priority() {
                current_level = level;
            }
        } else {
            merged_ranges.push((current_range, current_level));
            current_range = range;
            current_level = level;
        }
    }

    merged_ranges.push((current_range, current_level));
    merged_ranges
}
