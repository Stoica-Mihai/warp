//! File-content types used when converting proto file payloads into skill context.

use std::fmt::Display;
use std::ops::Range;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use warp_multi_agent_api as api;

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub enum AnyFileContent {
    StringContent(String),
    BinaryContent(Vec<u8>),
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct FileContext {
    pub file_name: String,
    pub content: AnyFileContent,
    pub line_range: Option<Range<usize>>,
    pub last_modified: Option<SystemTime>,
    pub line_count: usize,
}

impl FileContext {
    // create a new FileContext and autocalculate number of lines in the given file
    pub fn new(
        file_name: String,
        content: AnyFileContent,
        line_range: Option<Range<usize>>,
        last_modified: Option<SystemTime>,
    ) -> Self {
        let string_content = if let AnyFileContent::StringContent(content) = content.clone() {
            content
        } else {
            return Self {
                file_name,
                content,
                line_range,
                last_modified,
                line_count: 0,
            };
        };

        let line_count = string_content.lines().count();

        Self {
            file_name,
            content: AnyFileContent::StringContent(string_content),
            line_range,
            last_modified,
            line_count,
        }
    }
}

impl Display for FileContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.line_range {
            None => write!(f, "{}", self.file_name),
            Some(range) => write!(f, "{} ({}-{})", self.file_name, range.start, range.end),
        }
    }
}

impl From<warp_multi_agent_api::FileContent> for FileContext {
    fn from(content: warp_multi_agent_api::FileContent) -> Self {
        let line_range = content.line_range.map(|r| r.start as usize..r.end as usize);

        FileContext::new(
            content.file_path,
            AnyFileContent::StringContent(content.content),
            line_range,
            None,
        )
    }
}

impl From<warp_multi_agent_api::AnyFileContent> for FileContext {
    fn from(content: warp_multi_agent_api::AnyFileContent) -> Self {
        match content.content {
            Some(api::any_file_content::Content::BinaryContent(binary_content)) => {
                FileContext::new(
                    binary_content.file_path,
                    AnyFileContent::BinaryContent(binary_content.data),
                    None,
                    None,
                )
            }
            Some(api::any_file_content::Content::TextContent(text_content)) => {
                let line_range = text_content
                    .line_range
                    .map(|r| r.start as usize..r.end as usize);

                FileContext::new(
                    text_content.file_path,
                    AnyFileContent::StringContent(text_content.content),
                    line_range,
                    None,
                )
            }
            None => unreachable!("AnyFileContent should always have a content"),
        }
    }
}
