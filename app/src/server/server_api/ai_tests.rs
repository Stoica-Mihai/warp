use futures::executor::block_on;

use super::super::auth::CLOUD_AGENT_ID_HEADER;
use super::super::ServerApi;
use super::{
    build_list_agent_runs_url,
    AmbientAgentTaskState, Artifact,
    ConnectedSelfHostedWorker,
    ListConnectedSelfHostedWorkersResponse, ListRunsResponse,
    TaskListFilter, CONNECTED_SELF_HOSTED_WORKERS_PATH,
};
use crate::notebooks::NotebookId;

#[test]
fn ambient_agent_headers_for_task_overrides_existing_cloud_agent_header() {
    let server_api = ServerApi::new_for_test();
    let ambient_task_id = "550e8400-e29b-41d4-a716-446655440000".parse().unwrap();
    let task_scoped_id = "123e4567-e89b-12d3-a456-426614174000".parse().unwrap();

    server_api.set_ambient_agent_task_id(Some(ambient_task_id));

    let cloud_agent_headers: Vec<_> =
        block_on(server_api.ambient_agent_headers_for_task(&task_scoped_id))
            .unwrap()
            .into_iter()
            .filter(|(name, _)| *name == CLOUD_AGENT_ID_HEADER)
            .collect();

    assert_eq!(
        cloud_agent_headers,
        vec![(CLOUD_AGENT_ID_HEADER, task_scoped_id.to_string())]
    );
}

#[test]
fn connected_self_hosted_workers_path_uses_public_api_route() {
    assert_eq!(
        CONNECTED_SELF_HOSTED_WORKERS_PATH,
        "agent/connected-self-hosted-workers"
    );
}

#[test]
fn deserialize_connected_self_hosted_workers_response() {
    let json = r#"{
        "workers": [
            {
                "worker_host": "worker-2",
                "connection_count": 2,
                "connected_at": "2026-05-18T19:00:00Z",
                "last_seen_at": "2026-05-18T19:05:00Z"
            },
            {
                "worker_host": "worker-1",
                "connection_count": 1,
                "connected_at": "2026-05-18T18:00:00Z",
                "last_seen_at": "2026-05-18T18:05:00Z"
            }
        ]
    }"#;

    let response: ListConnectedSelfHostedWorkersResponse = serde_json::from_str(json).unwrap();

    assert_eq!(
        response.workers,
        vec![
            ConnectedSelfHostedWorker {
                worker_host: "worker-2".to_string(),
                connection_count: 2,
                connected_at: "2026-05-18T19:00:00Z".to_string(),
                last_seen_at: "2026-05-18T19:05:00Z".to_string(),
            },
            ConnectedSelfHostedWorker {
                worker_host: "worker-1".to_string(),
                connection_count: 1,
                connected_at: "2026-05-18T18:00:00Z".to_string(),
                last_seen_at: "2026-05-18T18:05:00Z".to_string(),
            },
        ]
    );
}

#[test]
fn test_deserialize_plan_artifact() {
    let json = r#"{
        "created_at": "2024-01-15T10:30:00Z",
        "artifact_type": "PLAN",
        "data": {
            "document_uid": "doc-uid-123",
            "notebook_uid": "1234567890123456789012",
            "title": "My Plan"
        }
    }"#;

    let artifact: Artifact = serde_json::from_str(json).unwrap();

    let Artifact::Plan {
        document_uid,
        notebook_uid,
        title,
    } = &artifact
    else {
        panic!("expected Plan artifact");
    };
    assert_eq!(document_uid, "doc-uid-123");
    assert_eq!(
        notebook_uid.as_ref().map(|n| n.to_string()),
        Some("1234567890123456789012".to_string())
    );
    assert_eq!(*title, Some("My Plan".to_string()));
}

#[test]
fn test_deserialize_pull_request_artifact() {
    let json = r#"{
        "created_at": "2024-01-15T10:30:00Z",
        "artifact_type": "PULL_REQUEST",
        "data": {
            "url": "https://github.com/org/repo/pull/42",
            "branch": "feature-branch"
        }
    }"#;

    let artifact: Artifact = serde_json::from_str(json).unwrap();

    let Artifact::PullRequest {
        url,
        branch,
        repo,
        number,
    } = &artifact
    else {
        panic!("expected PullRequest artifact");
    };
    assert_eq!(url, "https://github.com/org/repo/pull/42");
    assert_eq!(branch, "feature-branch");
    assert_eq!(*repo, Some("repo".to_string()));
    assert_eq!(*number, Some(42));
}

#[test]
fn test_deserialize_pull_request_non_github_url() {
    let json = r#"{
        "created_at": "2024-01-15T10:30:00Z",
        "artifact_type": "PULL_REQUEST",
        "data": {
            "url": "https://gitlab.com/org/repo/merge_requests/42",
            "branch": "feature-branch"
        }
    }"#;

    let artifact: Artifact = serde_json::from_str(json).unwrap();

    let Artifact::PullRequest { repo, number, .. } = &artifact else {
        panic!("expected PullRequest artifact");
    };
    assert_eq!(*repo, None);
    assert_eq!(*number, None);
}

#[test]
fn test_deserialize_plan_artifact_with_optional_fields_missing() {
    let json = r#"{
        "created_at": "2024-01-15T10:30:00Z",
        "artifact_type": "PLAN",
        "data": {
            "document_uid": "doc-uid-123",
            "notebook_uid": "abcdefghijklmnopqrstuv"
        }
    }"#;

    let artifact: Artifact = serde_json::from_str(json).unwrap();

    let Artifact::Plan {
        document_uid,
        notebook_uid,
        title,
    } = &artifact
    else {
        panic!("expected Plan artifact");
    };
    assert_eq!(document_uid, "doc-uid-123");
    assert_eq!(
        notebook_uid.as_ref().map(|n| n.to_string()),
        Some("abcdefghijklmnopqrstuv".to_string())
    );
    assert!(title.is_none());
}

#[test]
fn test_deserialize_list_tasks_response_with_artifacts() {
    let json = r#"{
        "runs": [
            {
                "task_id": "550e8400-e29b-41d4-a716-446655440000",
                "title": "Test Task",
                "state": "SUCCEEDED",
                "prompt": "test prompt",
                "created_at": "2024-01-15T10:00:00Z",
                "updated_at": "2024-01-15T10:30:00Z",
                "is_sandbox_running": true,
                "artifacts": [
                    {
                        "created_at": "2024-01-15T10:20:00Z",
                        "artifact_type": "PLAN",
                        "data": {
                            "document_uid": "doc-1",
                            "notebook_uid": "xyz1234567890123456789",
                            "title": "Plan Title"
                        }
                    },
                    {
                        "created_at": "2024-01-15T10:25:00Z",
                        "artifact_type": "PULL_REQUEST",
                        "data": {
                            "url": "https://github.com/org/repo/pull/1",
                            "branch": "main"
                        }
                    },
                    {
                        "created_at": "2024-01-15T10:27:00Z",
                        "artifact_type": "FILE",
                        "data": {
                            "artifact_uid": "artifact-file-1",
                            "filepath": "outputs/report.txt",
                            "filename": "report.txt",
                            "mime_type": "text/plain",
                            "description": "Daily summary",
                            "size_bytes": 42
                        }
                    }
                ]
            }
        ]
    }"#;

    let response: ListRunsResponse = serde_json::from_str(json).unwrap();

    assert_eq!(response.runs.len(), 1);
    let task = &response.runs[0];
    assert_eq!(
        task.task_id.to_string(),
        "550e8400-e29b-41d4-a716-446655440000"
    );
    assert_eq!(task.artifacts.len(), 3);

    // Check first artifact (Plan)
    let Artifact::Plan {
        document_uid,
        title,
        ..
    } = &task.artifacts[0]
    else {
        panic!("expected Plan artifact");
    };
    assert_eq!(document_uid, "doc-1");
    assert_eq!(*title, Some("Plan Title".to_string()));

    // Check second artifact (PullRequest)
    let Artifact::PullRequest {
        url,
        branch,
        repo,
        number,
        ..
    } = &task.artifacts[1]
    else {
        panic!("expected PullRequest artifact");
    };
    assert_eq!(url, "https://github.com/org/repo/pull/1");
    assert_eq!(branch, "main");
    assert_eq!(*repo, Some("repo".to_string()));
    assert_eq!(*number, Some(1));

    let Artifact::File {
        artifact_uid,
        filepath,
        filename,
        mime_type,
        description,
        size_bytes,
    } = &task.artifacts[2]
    else {
        panic!("expected File artifact");
    };
    assert_eq!(artifact_uid, "artifact-file-1");
    assert_eq!(filepath, "outputs/report.txt");
    assert_eq!(filename, "report.txt");
    assert_eq!(mime_type, "text/plain");
    assert_eq!(*description, Some("Daily summary".to_string()));
    assert_eq!(*size_bytes, Some(42));
}

#[test]
fn test_deserialize_list_tasks_response_empty_artifacts() {
    let json = r#"{
        "runs": [
            {
                "task_id": "550e8400-e29b-41d4-a716-446655440001",
                "title": "Test Task",
                "state": "INPROGRESS",
                "prompt": "test prompt",
                "created_at": "2024-01-15T10:00:00Z",
                "updated_at": "2024-01-15T10:30:00Z",
                "is_sandbox_running": true,
                "artifacts": []
            }
        ]
    }"#;

    let response: ListRunsResponse = serde_json::from_str(json).unwrap();

    assert_eq!(response.runs.len(), 1);
    assert!(response.runs[0].artifacts.is_empty());
}

#[test]
fn test_deserialize_list_tasks_response_missing_artifacts_field() {
    // Server may not include artifacts field at all for older responses
    let json = r#"{
        "runs": [
            {
                "task_id": "550e8400-e29b-41d4-a716-446655440002",
                "title": "Test Task",
                "state": "QUEUED",
                "prompt": "test prompt",
                "created_at": "2024-01-15T10:00:00Z",
                "updated_at": "2024-01-15T10:30:00Z",
                "is_sandbox_running": true
            }
        ]
    }"#;

    let response: ListRunsResponse = serde_json::from_str(json).unwrap();

    assert_eq!(response.runs.len(), 1);
    assert!(response.runs[0].artifacts.is_empty());
}

#[test]
fn test_deserialize_artifacts_skips_invalid_items() {
    // deserialize_artifacts should skip invalid items and keep valid ones
    let json = r#"{
        "runs": [
            {
                "task_id": "550e8400-e29b-41d4-a716-446655440000",
                "title": "Test Task",
                "state": "SUCCEEDED",
                "prompt": "test prompt",
                "created_at": "2024-01-15T10:00:00Z",
                "updated_at": "2024-01-15T10:30:00Z",
                "is_sandbox_running": true,
                "artifacts": [
                    {
                        "created_at": "2024-01-15T10:20:00Z",
                        "artifact_type": "PLAN",
                        "data": {
                            "document_uid": "valid-doc",
                            "notebook_uid": "validnotebook123456789",
                            "title": "Valid Plan"
                        }
                    },
                    {
                        "created_at": "2024-01-15T10:25:00Z",
                        "artifact_type": "UNKNOWN_TYPE",
                        "data": {
                            "some_field": "value"
                        }
                    },
                    {
                        "created_at": "2024-01-15T10:30:00Z",
                        "artifact_type": "PULL_REQUEST",
                        "data": {
                            "url": "https://github.com/org/repo/pull/1",
                            "branch": "main"
                        }
                    }
                ]
            }
        ]
    }"#;

    let response: ListRunsResponse = serde_json::from_str(json).unwrap();

    assert_eq!(response.runs.len(), 1);
    // Invalid artifact skipped, valid ones kept
    assert_eq!(response.runs[0].artifacts.len(), 2);
    assert!(matches!(
        response.runs[0].artifacts[0],
        Artifact::Plan { .. }
    ));
    assert!(matches!(
        response.runs[0].artifacts[1],
        Artifact::PullRequest { .. }
    ));
}

#[test]
fn test_deserialize_artifacts_all_invalid_returns_empty() {
    // When all artifacts are invalid, result should be empty vec
    let json = r#"{
        "runs": [
            {
                "task_id": "550e8400-e29b-41d4-a716-446655440000",
                "title": "Test Task",
                "state": "SUCCEEDED",
                "prompt": "test prompt",
                "created_at": "2024-01-15T10:00:00Z",
                "updated_at": "2024-01-15T10:30:00Z",
                "is_sandbox_running": true,
                "artifacts": [
                    {
                        "created_at": "2024-01-15T10:20:00Z",
                        "artifact_type": "UNKNOWN_TYPE",
                        "data": {}
                    }
                ]
            }
        ]
    }"#;

    let response: ListRunsResponse = serde_json::from_str(json).unwrap();

    assert_eq!(response.runs.len(), 1);
    assert!(response.runs[0].artifacts.is_empty());
}

#[test]
fn test_deserialize_artifact_missing_data_field() {
    let json = r#"{
        "created_at": "2024-01-15T10:30:00Z",
        "artifact_type": "PLAN"
    }"#;

    let result = serde_json::from_str::<Artifact>(json);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("missing field"));
}

#[test]
fn test_deserialize_artifact_invalid_plan_data() {
    // Missing required `document_uid` field should fail deserialization
    let json = r#"{
        "created_at": "2024-01-15T10:30:00Z",
        "artifact_type": "PLAN",
        "data": {
            "title": "Only title, no document_uid"
        }
    }"#;

    let result = serde_json::from_str::<Artifact>(json);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("missing field"));
}

#[test]
fn test_deserialize_artifact_invalid_pr_data() {
    let json = r#"{
        "created_at": "2024-01-15T10:30:00Z",
        "artifact_type": "PULL_REQUEST",
        "data": {
            "url": "https://github.com/org/repo/pull/1"
        }
    }"#;

    let result = serde_json::from_str::<Artifact>(json);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("missing field"));
}

#[test]
fn test_deserialize_artifact_unknown_variant() {
    let json = r#"{
        "created_at": "2024-01-15T10:30:00Z",
        "artifact_type": "UNKNOWN_TYPE",
        "data": {
            "some_field": "value"
        }
    }"#;

    let result = serde_json::from_str::<Artifact>(json);
    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("unknown variant"));
}

// ---------------------------------------------------------------------------------------------------------------------
//  Tests for resilient task list deserialization (skipping malformed tasks while tolerating unknown states)
// ---------------------------------------------------------------------------------------------------------------------

#[test]
fn test_deserialize_list_tasks_skips_invalid_task() {
    // One valid task and one invalid task (missing required field)
    let json = r#"{
        "runs": [
            {
                "task_id": "550e8400-e29b-41d4-a716-446655440000",
                "title": "Valid Task",
                "state": "SUCCEEDED",
                "prompt": "test prompt",
                "created_at": "2024-01-15T10:00:00Z",
                "updated_at": "2024-01-15T10:30:00Z",
                "is_sandbox_running": true
            },
            {
                "task_id": "550e8400-e29b-41d4-a716-446655440001",
                "title": "Invalid Task",
                "state": "INPROGRESS"
            }
        ]
    }"#;

    let response: ListRunsResponse = serde_json::from_str(json).unwrap();

    // Should only have the valid task
    assert_eq!(response.runs.len(), 1);
    assert_eq!(
        response.runs[0].task_id.to_string(),
        "550e8400-e29b-41d4-a716-446655440000"
    );
    assert_eq!(response.runs[0].title, "Valid Task");
}

#[test]
fn test_deserialize_list_tasks_error_and_blocked_states() {
    let json = r#"{
        "runs": [
            {
                "task_id": "550e8400-e29b-41d4-a716-446655440000",
                "title": "Errored Task",
                "state": "ERROR",
                "prompt": "test prompt",
                "created_at": "2024-01-15T10:00:00Z",
                "updated_at": "2024-01-15T10:30:00Z",
                "is_sandbox_running": false
            },
            {
                "task_id": "550e8400-e29b-41d4-a716-446655440001",
                "title": "Blocked Task",
                "state": "BLOCKED",
                "prompt": "test prompt",
                "created_at": "2024-01-15T10:00:00Z",
                "updated_at": "2024-01-15T10:30:00Z",
                "is_sandbox_running": false
            }
        ]
    }"#;

    let response: ListRunsResponse = serde_json::from_str(json).unwrap();
    assert_eq!(response.runs.len(), 2);
    assert_eq!(response.runs[0].state, AmbientAgentTaskState::Error);
    assert_eq!(response.runs[1].state, AmbientAgentTaskState::Blocked);
}

#[test]
fn test_deserialize_list_tasks_all_tasks_invalid_returns_empty() {
    // All tasks are missing required fields
    let json = r#"{
        "runs": [
            {
                "task_id": "550e8400-e29b-41d4-a716-446655440000",
                "title": "Missing State"
            },
            {
                "task_id": "550e8400-e29b-41d4-a716-446655440001",
                "state": "SUCCEEDED"
            }
        ]
    }"#;

    let response: ListRunsResponse = serde_json::from_str(json).unwrap();

    // Should return empty list, not fail
    assert_eq!(response.runs.len(), 0);
}

#[test]
fn test_deserialize_list_tasks_invalid_state_enum() {
    // Task with an unknown state enum value
    let json = r#"{
        "runs": [
            {
                "task_id": "550e8400-e29b-41d4-a716-446655440000",
                "title": "Valid Task",
                "state": "SUCCEEDED",
                "prompt": "test prompt",
                "created_at": "2024-01-15T10:00:00Z",
                "updated_at": "2024-01-15T10:30:00Z",
                "is_sandbox_running": true
            },
            {
                "task_id": "550e8400-e29b-41d4-a716-446655440001",
                "title": "Task with Invalid State",
                "state": "INVALID_STATE",
                "prompt": "test prompt",
                "created_at": "2024-01-15T10:00:00Z",
                "updated_at": "2024-01-15T10:30:00Z",
                "is_sandbox_running": true
            }
        ]
    }"#;

    let response: ListRunsResponse = serde_json::from_str(json).unwrap();

    // Unknown states should deserialize to AmbientAgentTaskState::Unknown.
    assert_eq!(response.runs.len(), 2);
    assert_eq!(response.runs[0].title, "Valid Task");
    assert_eq!(response.runs[1].title, "Task with Invalid State");
    assert_eq!(response.runs[1].state, AmbientAgentTaskState::Unknown);
}

#[test]
fn test_deserialize_list_tasks_corrupted_json_in_middle() {
    // Mix of valid and completely malformed JSON
    let json = r#"{
        "runs": [
            {
                "task_id": "550e8400-e29b-41d4-a716-446655440000",
                "title": "First Valid Task",
                "state": "SUCCEEDED",
                "prompt": "test prompt",
                "created_at": "2024-01-15T10:00:00Z",
                "updated_at": "2024-01-15T10:30:00Z",
                "is_sandbox_running": true
            },
            {
                "task_id": 12345,
                "title": 999,
                "state": true
            },
            {
                "task_id": "550e8400-e29b-41d4-a716-446655440002",
                "title": "Second Valid Task",
                "state": "INPROGRESS",
                "prompt": "test prompt 2",
                "created_at": "2024-01-15T11:00:00Z",
                "updated_at": "2024-01-15T11:30:00Z",
                "is_sandbox_running": false
            }
        ]
    }"#;

    let response: ListRunsResponse = serde_json::from_str(json).unwrap();

    // Should have both valid tasks, malformed one skipped
    assert_eq!(response.runs.len(), 2);
    assert_eq!(response.runs[0].title, "First Valid Task");
    assert_eq!(response.runs[1].title, "Second Valid Task");
}

#[test]
fn test_deserialize_list_tasks_empty_tasks_array() {
    // Empty tasks array should work fine
    let json = r#"{
        "runs": []
    }"#;

    let response: ListRunsResponse = serde_json::from_str(json).unwrap();
    assert_eq!(response.runs.len(), 0);
}

#[test]
fn test_deserialize_list_tasks_all_tasks_valid() {
    // Ensure we don't break the happy path
    let json = r#"{
        "runs": [
            {
                "task_id": "550e8400-e29b-41d4-a716-446655440000",
                "title": "Task 1",
                "state": "SUCCEEDED",
                "prompt": "test prompt",
                "created_at": "2024-01-15T10:00:00Z",
                "updated_at": "2024-01-15T10:30:00Z",
                "is_sandbox_running": true
            },
            {
                "task_id": "550e8400-e29b-41d4-a716-446655440001",
                "title": "Task 2",
                "state": "INPROGRESS",
                "prompt": "test prompt 2",
                "created_at": "2024-01-15T11:00:00Z",
                "updated_at": "2024-01-15T11:30:00Z",
                "is_sandbox_running": false
            },
            {
                "task_id": "550e8400-e29b-41d4-a716-446655440002",
                "title": "Task 3",
                "state": "FAILED",
                "prompt": "test prompt 3",
                "created_at": "2024-01-15T12:00:00Z",
                "updated_at": "2024-01-15T12:30:00Z",
                "is_sandbox_running": false
            }
        ]
    }"#;

    let response: ListRunsResponse = serde_json::from_str(json).unwrap();

    // All tasks should be present
    assert_eq!(response.runs.len(), 3);
    assert_eq!(response.runs[0].title, "Task 1");
    assert_eq!(response.runs[1].title, "Task 2");
    assert_eq!(response.runs[2].title, "Task 3");
}

// ---------------------------------------------------------------------------------------------------------------------
//  We test roundtripping serialize and deserialize since we use this for persisting artifacts for local conversations.
// ---------------------------------------------------------------------------------------------------------------------

#[test]
fn test_artifact_plan_serialize_deserialize_roundtrip() {
    let original = Artifact::Plan {
        document_uid: "doc-123".to_string(),
        notebook_uid: Some(NotebookId::from("notebook12345678901234".to_string())),
        title: Some("My Plan".to_string()),
    };

    let serialized = serde_json::to_string(&original).unwrap();
    let deserialized: Artifact = serde_json::from_str(&serialized).unwrap();

    assert_eq!(original, deserialized);
}

#[test]
fn test_artifact_plan_serialize_deserialize_roundtrip_no_notebook_uid() {
    let original = Artifact::Plan {
        document_uid: "doc-123".to_string(),
        notebook_uid: None,
        title: Some("My Plan".to_string()),
    };

    let serialized = serde_json::to_string(&original).unwrap();
    let deserialized: Artifact = serde_json::from_str(&serialized).unwrap();

    assert_eq!(original, deserialized);
}

#[test]
fn test_artifact_pr_serialize_deserialize_roundtrip() {
    let original = Artifact::PullRequest {
        url: "https://github.com/org/repo/pull/42".to_string(),
        branch: "feature-branch".to_string(),
        repo: Some("repo".to_string()),
        number: Some(42),
    };

    let serialized = serde_json::to_string(&original).unwrap();
    let deserialized: Artifact = serde_json::from_str(&serialized).unwrap();

    // repo/number are re-derived from URL on deserialize, so should match
    assert_eq!(original, deserialized);
}

#[test]
fn test_artifact_file_serialize_deserialize_roundtrip() {
    let original = Artifact::File {
        artifact_uid: "artifact-file-1".to_string(),
        filepath: "outputs/report.txt".to_string(),
        filename: "report.txt".to_string(),
        mime_type: "text/plain".to_string(),
        description: Some("Daily summary".to_string()),
        size_bytes: Some(42),
    };

    let serialized = serde_json::to_string(&original).unwrap();
    let deserialized: Artifact = serde_json::from_str(&serialized).unwrap();

    assert_eq!(original, deserialized);
}

#[test]
fn test_artifact_vec_serialize_deserialize_roundtrip() {
    let original = vec![
        Artifact::Plan {
            document_uid: "doc-1".to_string(),
            notebook_uid: None,
            title: Some("Plan 1".to_string()),
        },
        Artifact::PullRequest {
            url: "https://github.com/org/repo/pull/1".to_string(),
            branch: "main".to_string(),
            repo: Some("repo".to_string()),
            number: Some(1),
        },
        Artifact::File {
            artifact_uid: "artifact-file-1".to_string(),
            filepath: "outputs/report.txt".to_string(),
            filename: "report.txt".to_string(),
            mime_type: "text/plain".to_string(),
            description: Some("Daily summary".to_string()),
            size_bytes: Some(42),
        },
    ];

    let serialized = serde_json::to_string(&original).unwrap();
    let deserialized: Vec<Artifact> = serde_json::from_str(&serialized).unwrap();

    assert_eq!(original, deserialized);
}

#[test]
fn build_list_agent_runs_url_repeats_state_filter() {
    let filter = TaskListFilter {
        states: Some(vec![
            AmbientAgentTaskState::Queued,
            AmbientAgentTaskState::InProgress,
            AmbientAgentTaskState::Succeeded,
        ]),
        ..TaskListFilter::default()
    };
    let url = build_list_agent_runs_url(5, &filter);
    assert_eq!(
        url,
        "agent/runs?limit=5&state=QUEUED&state=INPROGRESS&state=SUCCEEDED"
    );
}

#[test]
fn build_list_agent_runs_url_skips_unknown_state() {
    // The deserializer keeps `Unknown` for forward compatibility, but we shouldn't send it to
    // the server as a filter value.
    let filter = TaskListFilter {
        states: Some(vec![
            AmbientAgentTaskState::Unknown,
            AmbientAgentTaskState::Succeeded,
        ]),
        ..TaskListFilter::default()
    };
    let url = build_list_agent_runs_url(1, &filter);
    assert_eq!(url, "agent/runs?limit=1&state=SUCCEEDED");
}

#[test]
fn build_list_agent_runs_url_routes_to_runs_not_tasks() {
    let url = build_list_agent_runs_url(10, &TaskListFilter::default());
    assert!(url.starts_with("agent/runs?"));
    assert!(!url.starts_with("agent/tasks"));
}


