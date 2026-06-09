use ai::diff_validation::DiffType;
use warpui::{View, ViewHandle};

use super::editor::view::CodeEditorView;

/// Shared trait for views that display an inline diff.
pub trait DiffViewer
where
    Self: Sized + View,
{
    fn editor(&self) -> &ViewHandle<CodeEditorView>;
    fn diff(&self) -> Option<&DiffType>;

    fn reject_diff(&mut self, _ctx: &mut warpui::ViewContext<Self>) {}
}
