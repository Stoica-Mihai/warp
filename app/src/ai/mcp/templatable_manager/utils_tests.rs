#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use rmcp::model::{ErrorCode, ErrorData, ServerCapabilities, Tool};

    use crate::ai::mcp::templatable_manager::utils::{query_tools_for, should_query_tools};

    /// Build a `ServerCapabilities` with selected capability flags toggled on.
    /// Each `Some(default)` mirrors how rmcp deserializes a capability the
    /// server advertised with no inner flags set.
    fn caps(tools: bool, resources: bool) -> ServerCapabilities {
        match (tools, resources) {
            (true, true) => ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
            (true, false) => ServerCapabilities::builder().enable_tools().build(),
            (false, true) => ServerCapabilities::builder().enable_resources().build(),
            (false, false) => ServerCapabilities::builder().build(),
        }
    }

    fn test_tool(name: &str) -> Tool {
        serde_json::from_value(serde_json::json!({
            "name": name,
            "description": "test tool",
            "inputSchema": { "type": "object" },
        }))
        .expect("Tool deserialization")
    }

    // ---------- predicate-level tests ----------

    #[test]
    fn each_capability_is_queried_independently() {
        for has_tools in [false, true] {
            for has_resources in [false, true] {
                let c = caps(has_tools, has_resources);
                assert_eq!(
                    should_query_tools(Some(&c)),
                    has_tools,
                    "tools={has_tools}, resources={has_resources}",
                );
            }
        }
        assert!(!should_query_tools(None));
    }

    // ---------- query_tools_for control-flow tests ----------

    /// When `tools` is not advertised, the helper must skip the list call so
    /// we don't waste a round trip and pollute the wire log with a request
    /// that's destined to return `METHOD_NOT_FOUND`.
    #[tokio::test]
    async fn query_tools_for_skips_listing_when_capability_not_advertised() {
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_clone = calls.clone();
        let no_caps = caps(false, false);

        let result = query_tools_for(Some(&no_caps), "srv", || async move {
            calls_clone.fetch_add(1, Ordering::SeqCst);
            Ok(vec![test_tool("never")])
        })
        .await;

        assert!(result.is_empty());
        assert_eq!(
            calls.load(Ordering::SeqCst),
            0,
            "list function must not be called when tools capability is absent",
        );
    }

    /// `None` server info follows the same skip-listing path as "no capability".
    #[tokio::test]
    async fn query_tools_for_skips_listing_when_server_info_is_none() {
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_clone = calls.clone();

        let result = query_tools_for(None, "srv", || async move {
            calls_clone.fetch_add(1, Ordering::SeqCst);
            Ok(vec![test_tool("never")])
        })
        .await;

        assert!(result.is_empty());
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    /// Happy path: tools advertised, list call succeeds, tools returned.
    #[tokio::test]
    async fn query_tools_for_returns_listed_tools_when_capability_advertised() {
        let c = caps(true, false);
        let expected = vec![test_tool("greet"), test_tool("review")];
        let to_return = expected.clone();

        let result = query_tools_for(Some(&c), "srv", || async move { Ok(to_return) }).await;

        assert_eq!(result, expected);
    }

    /// `tools` advertised but server returns an empty list — distinct from the
    /// "skipped" case in that we still made the call.
    #[tokio::test]
    async fn query_tools_for_returns_empty_vec_when_server_lists_no_tools() {
        let c = caps(true, false);
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_clone = calls.clone();

        let result = query_tools_for(Some(&c), "srv", || async move {
            calls_clone.fetch_add(1, Ordering::SeqCst);
            Ok(Vec::new())
        })
        .await;

        assert!(result.is_empty());
        assert_eq!(
            calls.load(Ordering::SeqCst),
            1,
            "list function still called when capability is advertised",
        );
    }

    /// **The fail-soft test the bug ticket implicitly demands.** Transport-
    /// closed errors must not abort server startup; the helper must log and
    /// return an empty vec. This is the regression-protector for #6798's
    /// underlying asymmetry — if anyone re-introduces a `return Err(...)` here,
    /// this test fails.
    #[tokio::test]
    async fn query_tools_for_returns_empty_on_transport_error() {
        let c = caps(true, false);
        let result = query_tools_for(Some(&c), "srv", || async {
            Err(rmcp::ServiceError::TransportClosed)
        })
        .await;
        assert!(result.is_empty());
    }

    /// MCP-protocol errors (e.g. METHOD_NOT_FOUND from a misbehaving server
    /// that advertised the capability but rejects the call) also fail soft,
    /// so the rest of the server surface still comes up.
    #[tokio::test]
    async fn query_tools_for_returns_empty_on_mcp_error() {
        let c = caps(true, false);
        let result = query_tools_for(Some(&c), "srv", || async {
            Err(rmcp::ServiceError::McpError(ErrorData {
                code: ErrorCode::METHOD_NOT_FOUND,
                message: "tools/list not implemented".into(),
                data: None,
            }))
        })
        .await;
        assert!(result.is_empty());
    }

    /// The list function must be called exactly once per query — not zero
    /// (that would be the skip path) and not multiple times (no implicit
    /// retry inside the helper).
    #[tokio::test]
    async fn query_tools_for_calls_list_function_exactly_once() {
        let c = caps(true, false);
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_clone = calls.clone();

        let _ = query_tools_for(Some(&c), "srv", || async move {
            calls_clone.fetch_add(1, Ordering::SeqCst);
            Ok(vec![test_tool("p")])
        })
        .await;

        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    /// Independence: tools-listing decision must not depend on whether
    /// resources are also advertised. Run the full happy-path flow under
    /// every (tools, resources) combination and assert tools come back iff
    /// the tools capability is advertised.
    #[tokio::test]
    async fn query_tools_for_decision_independent_of_other_capabilities() {
        let tools = vec![test_tool("x")];
        for has_tools in [false, true] {
            for has_resources in [false, true] {
                let c = caps(has_tools, has_resources);
                let to_return = tools.clone();
                let result =
                    query_tools_for(Some(&c), "srv", || async move { Ok(to_return) }).await;

                if has_tools {
                    assert_eq!(
                        result, tools,
                        "expected tools when advertised \
                         (tools={has_tools}, resources={has_resources})",
                    );
                } else {
                    assert!(
                        result.is_empty(),
                        "expected empty when tools not advertised \
                         (tools={has_tools}, resources={has_resources})",
                    );
                }
            }
        }
    }

}
