use chrono::Utc;
use warpui::{App, SingletonEntity};

use super::*;
use crate::auth::AuthStateProvider;
use crate::test_util::settings::initialize_settings_for_tests;
use crate::workspaces::user_workspaces::UserWorkspaces;

fn add_ai_enablement_dependencies_for_test(app: &mut App) {
    app.add_singleton_model(|_| AuthStateProvider::new_for_test());
    app.add_singleton_model(UserWorkspaces::default_mock);
}

// FocusedTerminalInfo Tests

#[test]
fn test_update_both_values_changed() {
    App::test((), |mut app| async move {
        // Create FocusedTerminalInfo with default values (false, false)
        let model_handle = app.add_model(|_| FocusedTerminalInfo::default());

        // Setup event tracking
        let (sender, receiver) = async_channel::unbounded();
        let model_handle_clone = model_handle.clone();
        model_handle.update(&mut app, move |_, ctx| {
            let sender = sender.clone();
            ctx.subscribe_to_model(
                &model_handle_clone,
                move |_, event: &FocusedTerminalInfoEvent, _| match event {
                    FocusedTerminalInfoEvent::TerminalInfoUpdated => {
                        let _ = sender.try_send(());
                    }
                },
            );
        });

        // Update both values to (true, false)
        model_handle.update(&mut app, |model, ctx| {
            model.update(true, false, ctx);
        });

        // Verify model state
        model_handle.read(&app, |model, _| {
            assert!(model.contains_any_remote_blocks());
            assert!(!model.contains_any_restored_remote_blocks());
        });

        // Verify event was emitted exactly once
        let mut count = 0;
        while receiver.try_recv().is_ok() {
            count += 1;
        }
        assert_eq!(count, 1);
    });
}

#[test]
fn test_update_additional_value_changed() {
    App::test((), |mut app| async move {
        // Create FocusedTerminalInfo with default values (false, false)
        let model_handle = app.add_model(|_| FocusedTerminalInfo::default());

        // Setup event tracking
        let (sender, receiver) = async_channel::unbounded();
        let model_handle_clone = model_handle.clone();
        model_handle.update(&mut app, move |_, ctx| {
            let sender = sender.clone();
            ctx.subscribe_to_model(
                &model_handle_clone,
                move |_, event: &FocusedTerminalInfoEvent, _| match event {
                    FocusedTerminalInfoEvent::TerminalInfoUpdated => {
                        let _ = sender.try_send(());
                    }
                },
            );
        });

        // First update to (true, false)
        model_handle.update(&mut app, |model, ctx| {
            model.update(true, false, ctx);
        });

        // Clear events by draining the channel
        while receiver.try_recv().is_ok() {}

        // Now update to (true, true) - only changing restored blocks
        model_handle.update(&mut app, |model, ctx| {
            model.update(true, true, ctx);
        });

        // Verify model state
        model_handle.read(&app, |model, _| {
            assert!(model.contains_any_remote_blocks());
            assert!(model.contains_any_restored_remote_blocks());
        });

        // Verify event was emitted exactly once
        let mut count = 0;
        while receiver.try_recv().is_ok() {
            count += 1;
        }
        assert_eq!(count, 1);
    });
}

#[test]
fn test_update_no_change() {
    App::test((), |mut app| async move {
        // Create FocusedTerminalInfo with default values (false, false)
        let model_handle = app.add_model(|_| FocusedTerminalInfo::default());

        // Setup event tracking
        let (sender, receiver) = async_channel::unbounded();
        let model_handle_clone = model_handle.clone();
        model_handle.update(&mut app, move |_, ctx| {
            let sender = sender.clone();
            ctx.subscribe_to_model(
                &model_handle_clone,
                move |_, event: &FocusedTerminalInfoEvent, _| match event {
                    FocusedTerminalInfoEvent::TerminalInfoUpdated => {
                        let _ = sender.try_send(());
                    }
                },
            );
        });

        // First update to (true, true)
        model_handle.update(&mut app, |model, ctx| {
            model.update(true, true, ctx);
        });

        // Clear events by draining the channel
        while receiver.try_recv().is_ok() {}

        // Update with same values (true, true)
        model_handle.update(&mut app, |model, ctx| {
            model.update(true, true, ctx);
        });

        // Verify model state remains the same
        model_handle.read(&app, |model, _| {
            assert!(model.contains_any_remote_blocks());
            assert!(model.contains_any_restored_remote_blocks());
        });

        // Verify no event was emitted
        let mut count = 0;
        while receiver.try_recv().is_ok() {
            count += 1;
        }
        assert_eq!(count, 0);
    });
}

#[test]
fn test_update_only_remote_toggles() {
    App::test((), |mut app| async move {
        // Create FocusedTerminalInfo with default values (false, false)
        let model_handle = app.add_model(|_| FocusedTerminalInfo::default());

        // Setup event tracking
        let (sender, receiver) = async_channel::unbounded();
        let model_handle_clone = model_handle.clone();
        model_handle.update(&mut app, move |_, ctx| {
            let sender = sender.clone();
            ctx.subscribe_to_model(
                &model_handle_clone,
                move |_, event: &FocusedTerminalInfoEvent, _| match event {
                    FocusedTerminalInfoEvent::TerminalInfoUpdated => {
                        let _ = sender.try_send(());
                    }
                },
            );
        });

        // First update to (true, true)
        model_handle.update(&mut app, |model, ctx| {
            model.update(true, true, ctx);
        });

        // Clear events by draining the channel
        while receiver.try_recv().is_ok() {}

        // Update with (false, true) - only remote blocks changes
        model_handle.update(&mut app, |model, ctx| {
            model.update(false, true, ctx);
        });

        // Verify model state
        model_handle.read(&app, |model, _| {
            assert!(!model.contains_any_remote_blocks());
            assert!(model.contains_any_restored_remote_blocks());
        });

        // Verify event was emitted exactly once
        let mut count = 0;
        while receiver.try_recv().is_ok() {
            count += 1;
        }
        assert_eq!(count, 1);
    });
}

#[test]
fn test_update_only_restored_toggles() {
    App::test((), |mut app| async move {
        // Create FocusedTerminalInfo with default values (false, false)
        let model_handle = app.add_model(|_| FocusedTerminalInfo::default());

        // Setup event tracking
        let (sender, receiver) = async_channel::unbounded();
        let model_handle_clone = model_handle.clone();
        model_handle.update(&mut app, move |_, ctx| {
            let sender = sender.clone();
            ctx.subscribe_to_model(
                &model_handle_clone,
                move |_, event: &FocusedTerminalInfoEvent, _| match event {
                    FocusedTerminalInfoEvent::TerminalInfoUpdated => {
                        let _ = sender.try_send(());
                    }
                },
            );
        });

        // First update to (true, true)
        model_handle.update(&mut app, |model, ctx| {
            model.update(true, true, ctx);
        });

        // Clear events by draining the channel
        while receiver.try_recv().is_ok() {}

        // Update with (true, false) - only restored blocks changes
        model_handle.update(&mut app, |model, ctx| {
            model.update(true, false, ctx);
        });

        // Verify model state
        model_handle.read(&app, |model, _| {
            assert!(model.contains_any_remote_blocks());
            assert!(!model.contains_any_restored_remote_blocks());
        });

        // Verify event was emitted exactly once
        let mut count = 0;
        while receiver.try_recv().is_ok() {
            count += 1;
        }
        assert_eq!(count, 1);
    });
}

#[test]
fn orchestration_v2_enables_orchestration_when_ai_is_enabled() {
    let _orchestration_v2_flag = FeatureFlag::OrchestrationV2.override_enabled(true);

    App::test((), |mut app| async move {
        initialize_settings_for_tests(&mut app);
        add_ai_enablement_dependencies_for_test(&mut app);

        AISettings::handle(&app).read(&app, |settings, ctx| {
            assert!(settings.is_orchestration_enabled(ctx));
        });
    });
}

#[test]
fn orchestration_v2_disabled_disables_orchestration() {
    let _orchestration_v2_flag = FeatureFlag::OrchestrationV2.override_enabled(false);

    App::test((), |mut app| async move {
        initialize_settings_for_tests(&mut app);
        add_ai_enablement_dependencies_for_test(&mut app);

        AISettings::handle(&app).read(&app, |settings, ctx| {
            assert!(!settings.is_orchestration_enabled(ctx));
        });
    });
}
#[test]
fn test_mark_quota_banner_as_dismissed() {
    App::test((), |mut app| async move {
        initialize_settings_for_tests(&mut app);

        let now = Utc::now();

        // Create test cycles: two expired cycles and one future cycle
        let expired_cycle_1 = CycleInfo {
            end_date: now - chrono::Duration::days(30), // 30 days ago
            was_quota_exceeded: true,
            banner_state: BannerState { dismissed: false },
        };

        let expired_cycle_2 = CycleInfo {
            end_date: now - chrono::Duration::days(15), // 15 days ago
            was_quota_exceeded: true,
            banner_state: BannerState { dismissed: false },
        };

        let future_cycle = CycleInfo {
            end_date: now + chrono::Duration::days(15), // 15 days in future
            was_quota_exceeded: false,
            banner_state: BannerState { dismissed: false },
        };

        let cycle_history = vec![expired_cycle_1, expired_cycle_2, future_cycle];

        // Set up initial state
        AISettings::handle(&app).update(&mut app, |settings, ctx| {
            settings
                .ai_request_quota_info
                .set_value(AIRequestQuotaInfo { cycle_history }, ctx)
                .unwrap();
        });

        // Mark expired cycles as dismissed
        AISettings::handle(&app).update(&mut app, |settings, ctx| {
            settings.mark_quota_banner_as_dismissed(ctx);
        });

        // Verify the results
        AISettings::handle(&app).read(&app, |settings, _ctx| {
            let cycle_history = &settings.ai_request_quota_info.cycle_history;
            assert_eq!(cycle_history.len(), 3);

            // First cycle (oldest expired) should be dismissed
            assert!(cycle_history[0].banner_state.dismissed);
            // Second cycle (more recent expired) should be dismissed
            assert!(cycle_history[1].banner_state.dismissed);
            // Future cycle should not be dismissed
            assert!(!cycle_history[2].banner_state.dismissed);
        });
    });
}
