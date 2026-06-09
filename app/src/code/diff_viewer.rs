use warpui::View;

/// Shared trait for views that display an inline diff.
pub trait DiffViewer
where
    Self: Sized + View,
{
    fn reject_diff(&mut self, _ctx: &mut warpui::ViewContext<Self>) {}
}
