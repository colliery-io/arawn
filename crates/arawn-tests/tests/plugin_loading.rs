//! Integration test: load a real compiled plugin dylib and call its methods.
//!
//! This test requires the web-fetch plugin to be pre-built:
//!   cd crates/arawn-tests/fixtures/arawn-plugin-web-fetch && cargo build

use arawn_tool_plugin::ToolExecuteOutput;
use fidius_host::{PluginHandle, PluginHost};

/// Path to the pre-built web-fetch dylib (debug profile).
fn web_fetch_dylib_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../fixtures/arawn-plugin-web-fetch/target/debug")
}

#[test]
fn load_web_fetch_plugin_and_read_metadata() {
    let dylib_dir = web_fetch_dylib_dir();
    if !dylib_dir.exists() {
        eprintln!(
            "SKIP: web-fetch plugin not built. Run: cd crates/arawn-tests/fixtures/arawn-plugin-web-fetch && cargo build"
        );
        return;
    }

    let host = PluginHost::builder()
        .search_path(&dylib_dir)
        .build()
        .expect("failed to build PluginHost");

    let discovered = host.discover().expect("failed to discover plugins");
    assert!(
        !discovered.is_empty(),
        "no plugins discovered in {}",
        dylib_dir.display()
    );

    let loaded = host
        .load("WebFetchTool")
        .expect("failed to load WebFetchTool");
    let handle = PluginHandle::from_loaded(loaded);

    // Call name()
    use arawn_tool_plugin::__fidius_ArawnTool::*;
    let name: String = handle.call_method(METHOD_NAME, &()).expect("name() failed");
    assert_eq!(name, "web_fetch");

    // Call description()
    let desc: String = handle
        .call_method(METHOD_DESCRIPTION, &())
        .expect("description() failed");
    assert!(
        desc.contains("URL"),
        "description should mention URL: {desc}"
    );

    // Call parameters_schema()
    let schema: String = handle
        .call_method(METHOD_PARAMETERS_SCHEMA, &())
        .expect("schema() failed");
    let parsed: serde_json::Value =
        serde_json::from_str(&schema).expect("schema is not valid JSON");
    assert_eq!(parsed["type"], "object");
    assert!(parsed["properties"]["url"].is_object());
}

#[test]
fn web_fetch_plugin_execute_fetches_url() {
    let dylib_dir = web_fetch_dylib_dir();
    if !dylib_dir.exists() {
        eprintln!("SKIP: web-fetch plugin not built");
        return;
    }

    let host = PluginHost::builder()
        .search_path(&dylib_dir)
        .build()
        .unwrap();

    let loaded = host.load("WebFetchTool").unwrap();
    let handle = PluginHandle::from_loaded(loaded);

    use arawn_tool_plugin::__fidius_ArawnTool::*;

    let context_json = serde_json::json!({
        "working_dir": "/tmp",
        "session_id": "test",
        "workstream_name": "test"
    })
    .to_string();

    let params_json = serde_json::json!({
        "url": "https://httpbin.org/get",
        "max_bytes": 1024
    })
    .to_string();

    let result: ToolExecuteOutput = handle
        .call_method(METHOD_EXECUTE, &(context_json, params_json))
        .expect("execute() failed");

    assert!(
        !result.is_error,
        "execute returned error: {}",
        result.content
    );
    assert!(!result.content.is_empty(), "content should not be empty");
    // httpbin.org/get returns JSON with "url" field
    assert!(
        result.content.contains("httpbin"),
        "expected httpbin in response: {}",
        &result.content[..200.min(result.content.len())]
    );
}

/// Path to the pre-built web-search dylib (debug profile).
fn web_search_dylib_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../fixtures/arawn-plugin-web-search/target/debug")
}

#[test]
fn load_web_search_plugin_and_read_metadata() {
    let dylib_dir = web_search_dylib_dir();
    if !dylib_dir.exists() {
        eprintln!("SKIP: web-search plugin not built");
        return;
    }

    let host = PluginHost::builder()
        .search_path(&dylib_dir)
        .build()
        .unwrap();

    let loaded = host.load("WebSearchTool").unwrap();
    let handle = PluginHandle::from_loaded(loaded);

    use arawn_tool_plugin::__fidius_ArawnTool::*;

    let name: String = handle.call_method(METHOD_NAME, &()).unwrap();
    assert_eq!(name, "web_search");

    let schema: String = handle.call_method(METHOD_PARAMETERS_SCHEMA, &()).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&schema).unwrap();
    assert!(parsed["properties"]["query"].is_object());
}

#[test]
fn web_search_plugin_execute_searches() {
    let dylib_dir = web_search_dylib_dir();
    if !dylib_dir.exists() {
        eprintln!("SKIP: web-search plugin not built");
        return;
    }

    let host = PluginHost::builder()
        .search_path(&dylib_dir)
        .build()
        .unwrap();

    let loaded = host.load("WebSearchTool").unwrap();
    let handle = PluginHandle::from_loaded(loaded);

    use arawn_tool_plugin::__fidius_ArawnTool::*;

    let context_json = serde_json::json!({
        "working_dir": "/tmp",
        "session_id": "test",
        "workstream_name": "test"
    })
    .to_string();

    let params_json = serde_json::json!({
        "query": "rust programming language",
        "num_results": 3
    })
    .to_string();

    let result: ToolExecuteOutput = handle
        .call_method(METHOD_EXECUTE, &(context_json, params_json))
        .unwrap();

    assert!(
        !result.is_error,
        "execute returned error: {}",
        result.content
    );
    // Should have some results (DuckDuckGo might return different formats but should have content)
    assert!(
        !result.content.is_empty(),
        "search results should not be empty"
    );
}
