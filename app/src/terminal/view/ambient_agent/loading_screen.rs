//! Loading screen UI for cloud mode initialization.

use markdown_parser::{FormattedText, FormattedTextFragment, FormattedTextLine};
use warp_core::ui::appearance::Appearance;
use warp_core::ui::theme::AnsiColorIdentifier;
use warp_core::ui::Icon;
use warpui::elements::shimmering_text::ShimmeringTextStateHandle;
use warpui::elements::{
    Align, Border, ConstrainedBox, Container, CrossAxisAlignment, Element, Expanded, Flex,
    FormattedTextElement, MainAxisAlignment, MainAxisSize, ParentElement,
    SelectableArea, SelectionHandle, Text,
};
use warpui::fonts::{Properties, Weight};
use warpui::prelude::{CornerRadius, Radius};
use warpui::text_layout::TextAlignment;
use warpui::{AppContext, SingletonEntity};

use crate::ai::loading::shimmering_warp_loading_text;
use crate::ui_components::blended_colors;
use crate::workspaces::user_workspaces::UserWorkspaces;

/// Icon size for the error icon
const ERROR_ICON_SIZE: f32 = 24.;

/// Renders the cloud mode loading screen with shimmering warp logo.
pub fn render_cloud_mode_loading_screen(
    message: &str,
    appearance: &Appearance,
    shimmer_handle: &ShimmeringTextStateHandle,
    app: &AppContext,
) -> Box<dyn Element> {
    let loading_font_size = appearance.monospace_font_size() + 2.;

    // Create the shimmering warp loading text element
    let shimmer_element =
        shimmering_warp_loading_text(message, loading_font_size, shimmer_handle.clone(), app);

    // Get tier info for the concurrency limits footer
    let tier_footer_element = render_tier_limits_footer(appearance, app);

    // Vertical layout with centered main content and footer at bottom
    Flex::column()
        .with_main_axis_size(MainAxisSize::Max)
        .with_cross_axis_alignment(CrossAxisAlignment::Center)
        .with_child(
            Expanded::new(
                1.,
                // Align centers content both horizontally and vertically within the Expanded area
                Align::new(
                    Flex::column()
                        .with_cross_axis_alignment(CrossAxisAlignment::Center)
                        .with_child(
                            Container::new(shimmer_element)
                                .with_horizontal_padding(4.)
                                .finish(),
                        )
                        .finish(),
                )
                .finish(),
            )
            .finish(),
        )
        // Footer anchored at bottom (only if we have tier info to show)
        .with_children(
            tier_footer_element
                .into_iter()
                .map(|element| {
                    Container::new(element)
                        .with_horizontal_padding(16.)
                        .with_vertical_padding(12.)
                        .finish()
                })
                .collect::<Vec<_>>(),
        )
        .finish()
}

/// Renders the tier limits footer showing concurrency limits and upgrade suggestions.
/// Returns None if there are no specs to display.
fn render_tier_limits_footer(
    appearance: &Appearance,
    app: &AppContext,
) -> Option<Box<dyn Element>> {
    let theme = appearance.theme();
    let footer_font_size = appearance.monospace_font_size() - 2.;

    // Get tier info and billing metadata from UserWorkspaces
    let workspace = UserWorkspaces::as_ref(app).current_workspace()?;
    let policy = workspace.billing_metadata.tier.ambient_agents_policy?;

    let shape = policy.instance_shape.as_ref()?;
    let specs = format!("{}CPU, {}GB", shape.vcpus, shape.memory_gb);

    // If there's no way to upgrade, don't render the footer at all
    // (Build Max users can still upgrade to Business plans)
    if !workspace.billing_metadata.can_upgrade_to_build_plan()
        && !workspace.billing_metadata.can_upgrade_to_build_max_plan()
        && !workspace.billing_metadata.is_on_build_max_plan()
    {
        return None;
    }

    let mut fragments = vec![FormattedTextFragment::plain_text(format!(
        "Your agent is currently running on a {} machine. ",
        specs
    ))];

    // Get the upgrade URL for the current team
    let upgrade_url = UserWorkspaces::as_ref(app)
        .current_team()
        .map(|team| UserWorkspaces::upgrade_link_for_team(team.uid))?;

    fragments.push(FormattedTextFragment::hyperlink("Upgrade", upgrade_url));
    fragments.push(FormattedTextFragment::plain_text(
        " for more powerful cloud agents.",
    ));

    let formatted_text = FormattedText::new(vec![FormattedTextLine::Line(fragments)]);

    let text_element = FormattedTextElement::new(
        formatted_text,
        footer_font_size,
        appearance.ui_font_family(),
        appearance.monospace_font_family(),
        blended_colors::text_sub(theme, theme.surface_1()),
        Default::default(),
    )
    .with_alignment(TextAlignment::Center)
    .with_hyperlink_font_color(theme.accent().into())
    .register_default_click_handlers_with_action_support(|link, _evt, app| {
        use warpui::elements::HyperlinkLens;
        if let HyperlinkLens::Url(url) = link {
            app.open_url(url);
        }
    })
    .finish();

    // Create info icon
    let icon_size = footer_font_size;
    let info_icon = ConstrainedBox::new(
        Icon::Info
            .to_warpui_icon(blended_colors::text_sub(theme, theme.surface_1()).into())
            .finish(),
    )
    .with_width(icon_size)
    .with_height(icon_size)
    .finish();

    Some(
        Flex::row()
            .with_main_axis_alignment(MainAxisAlignment::Center)
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_spacing(6.)
            .with_child(info_icon)
            .with_child(text_element)
            .finish(),
    )
}

/// Renders the cloud mode error screen.
pub fn render_cloud_mode_error_screen(
    error_message: &str,
    appearance: &Appearance,
    selection_handle: &SelectionHandle,
    selected_text: &std::rc::Rc<parking_lot::RwLock<Option<String>>>,
    _app: &AppContext,
) -> Box<dyn Element> {
    let theme = appearance.theme();
    let error_color = AnsiColorIdentifier::Red.to_ansi_color(&theme.terminal_colors().normal);

    // Error icon with fixed size constraints - using AlertTriangle icon
    let error_icon = ConstrainedBox::new(
        Icon::AlertTriangle
            .to_warpui_icon(error_color.into())
            .finish(),
    )
    .with_width(ERROR_ICON_SIZE)
    .with_height(ERROR_ICON_SIZE)
    .finish();

    // Error title text
    let title_text = Text::new(
        "Failed to start environment",
        appearance.ui_font_family(),
        appearance.monospace_font_size() + 2.,
    )
    .with_style(Properties::default().weight(Weight::Bold))
    .with_color(error_color.into())
    .finish();

    // Error message wrapped in SelectableArea to make it selectable for easy copying
    let error_text = Text::new(
        error_message.to_string(),
        appearance.ui_font_family(),
        appearance.monospace_font_size(),
    )
    .with_color(error_color.into())
    .with_selectable(true)
    .soft_wrap(true)
    .finish();

    // Wrap error text in SelectableArea to enable text selection
    let selected_text = selected_text.clone();
    let selectable_error_text = SelectableArea::new(
        selection_handle.clone(),
        move |selection_args, _, _| {
            *selected_text.write() = selection_args.selection.filter(|s| !s.is_empty());
        },
        error_text,
    )
    .finish();

    // Content column with icon, title, and message stacked vertically
    let content = Flex::column()
        .with_cross_axis_alignment(CrossAxisAlignment::Center)
        .with_spacing(12.)
        .with_child(error_icon)
        .with_child(title_text)
        .with_child(selectable_error_text)
        .finish();

    // Red bordered container with 10% opacity background
    let error_background = warp_core::ui::color::coloru_with_opacity(error_color.into(), 10);

    let error_container = Container::new(content)
        .with_background(error_background)
        .with_border(Border::all(1.).with_border_color(error_color.into()))
        .with_corner_radius(CornerRadius::with_all(Radius::Pixels(8.)))
        .with_horizontal_padding(24.)
        .with_vertical_padding(16.)
        .finish();

    // Constrain error container to max 400px width
    let constrained_error = ConstrainedBox::new(error_container)
        .with_max_width(400.)
        .finish();

    // Center the error container in the view
    Flex::column()
        .with_main_axis_alignment(MainAxisAlignment::Center)
        .with_main_axis_size(MainAxisSize::Max)
        .with_cross_axis_alignment(CrossAxisAlignment::Center)
        .with_child(constrained_error)
        .finish()
}

