use warpui::ViewContext;

pub fn has_feature_gated_anonymous_user_reached_notebook_limit<V: warpui::View>(
    _ctx: &mut ViewContext<V>,
) -> bool {
    false
}

pub fn has_feature_gated_anonymous_user_reached_workflow_limit<V: warpui::View>(
    _ctx: &mut ViewContext<V>,
) -> bool {
    false
}

pub fn has_feature_gated_anonymous_user_reached_env_var_limit<V: warpui::View>(
    _ctx: &mut ViewContext<V>,
) -> bool {
    false
}
