#[cfg(test)]
mod tests {
    use crate::api::{AppState, build_router};
    use std::net::SocketAddr;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use tspm_engine::ProcessManager;
    use tspm_monitor::StatsCollector;
    use serde_json::json;

    fn test_state() -> Arc<AppState> {
        Arc::new(AppState {
            manager: Arc::new(Mutex::new(ProcessManager::new())),
            stats: StatsCollector::new(),
        })
    }

    async fn start_test_server(state: Arc<AppState>) -> SocketAddr {
        let app = build_router().with_state(state);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap(); });
        addr
    }

    async fn get(addr: SocketAddr, path: &str) -> (u16, serde_json::Value) {
        let url = format!("http://{addr}{path}");
        let resp = reqwest::get(&url).await.unwrap();
        (resp.status().as_u16(), resp.json().await.unwrap_or_default())
    }

    async fn post(addr: SocketAddr, path: &str, body: &serde_json::Value) -> (u16, serde_json::Value) {
        let url = format!("http://{addr}{path}");
        let resp = reqwest::Client::new().post(&url).json(body).send().await.unwrap();
        (resp.status().as_u16(), resp.json().await.unwrap_or_default())
    }

    async fn delete(addr: SocketAddr, path: &str) -> (u16, serde_json::Value) {
        let url = format!("http://{addr}{path}");
        let resp = reqwest::Client::new().delete(&url).send().await.unwrap();
        (resp.status().as_u16(), resp.json().await.unwrap_or_default())
    }

    #[tokio::test]
    async fn test_create_list_delete() {
        let addr = start_test_server(test_state()).await;

        let (s, j) = get(addr, "/api/v1/processes").await;
        assert_eq!(s, 200);
        assert!(j["data"].as_array().unwrap().is_empty());

        let (s, j) = post(addr, "/api/v1/processes", &json!({"name":"p1","script":"sleep","args":["5"]})).await;
        assert_eq!(s, 200, "{j:?}");

        let (s, j) = get(addr, "/api/v1/processes").await;
        assert!(!j["data"].as_array().unwrap().is_empty());

        let _ = post(addr, "/api/v1/processes/p1/stop", &json!({})).await;
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;

        let (s, j) = delete(addr, "/api/v1/processes/p1").await;
        assert_eq!(s, 200, "{j:?}");

        let (_, j) = get(addr, "/api/v1/processes").await;
        assert!(j["data"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_validation() {
        let addr = start_test_server(test_state()).await;
        let (s, _) = post(addr, "/api/v1/processes", &json!({"script":"x"})).await;
        assert!(s >= 400);
        let (s, _) = post(addr, "/api/v1/processes", &json!({"name":"x"})).await;
        assert!(s >= 400);
    }

    #[tokio::test]
    async fn test_ports_list() {
        let addr = start_test_server(test_state()).await;
        let (s, j) = get(addr, "/api/v1/ports").await;
        assert_eq!(s, 200);
        assert!(j["data"].is_array());
    }

    #[tokio::test]
    async fn test_kill_nonexistent_port() {
        let addr = start_test_server(test_state()).await;
        let (s, j) = post(addr, "/api/v1/ports/0", &json!({})).await;
        assert_eq!(s, 404, "{j:?}");
    }

    #[tokio::test]
    async fn test_kill_invalid_port() {
        let addr = start_test_server(test_state()).await;
        let (s, _) = post(addr, "/api/v1/ports/abc", &json!({})).await;
        assert_eq!(s, 400);
    }

    #[tokio::test]
    async fn test_status_health() {
        let addr = start_test_server(test_state()).await;
        let (s, j) = get(addr, "/api/v1/status").await;
        assert_eq!(s, 200);
        assert!(j["data"]["processes"].is_array());
        let (s, j) = get(addr, "/api/v1/health").await;
        assert_eq!(s, 200);
        assert_eq!(j["status"], "healthy");
    }

    #[tokio::test]
    async fn test_dump() {
        let addr = start_test_server(test_state()).await;
        let (s, j) = get(addr, "/api/v1/dump").await;
        assert_eq!(s, 200);
        assert!(j["data"]["processes"].is_array());
        let (s, j) = delete(addr, "/api/v1/dump/").await;
        assert_eq!(s, 200, "{j:?}");
    }

    #[tokio::test]
    async fn test_autocomplete() {
        let addr = start_test_server(test_state()).await;
        let (s, j) = post(addr, "/api/v1/autocomplete", &json!({"prefix":"ls","cwd":"."})).await;
        assert_eq!(s, 200);
        let v = j["suggestions"].as_array().unwrap();
        assert!(v.iter().any(|x| x.as_str() == Some("ls")));
    }

    #[tokio::test]
    async fn test_install_build_no_script() {
        let addr = start_test_server(test_state()).await;
        post(addr, "/api/v1/processes", &json!({"name":"nb","script":"sleep","args":["1"]})).await;
        let (s, j) = post(addr, "/api/v1/processes/nb/install", &json!({})).await;
        assert_eq!(s, 200);
        assert!(j["message"].as_str().unwrap().contains("No install"));
        let (s, j) = post(addr, "/api/v1/processes/nb/build", &json!({})).await;
        assert_eq!(s, 200);
        assert!(j["message"].as_str().unwrap().contains("No build"));
        delete(addr, "/api/v1/processes/nb").await;
    }

    #[tokio::test]
    async fn test_delete_nonexistent() {
        let addr = start_test_server(test_state()).await;
        let (s, _) = delete(addr, "/api/v1/processes/does-not-exist").await;
        assert_eq!(s, 404, "Expected 404 for non-existent process");
    }

    /// Ensures the router builds without panicking (catches {*name} vs {name} conflicts)
    #[tokio::test]
    async fn test_router_builds_without_panic() {
        let state = test_state();
        let app = build_router().with_state(state);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        // Spawning the server is the real test — it panics if routes conflict
        let handle = tokio::spawn(async move { axum::serve(listener, app).await.unwrap(); });

        // Hit health endpoint to confirm the server is actually running
        let (s, _) = get(addr, "/api/v1/health").await;
        assert_eq!(s, 200, "Server should be healthy after router construction");
        handle.abort();
    }

    /// Test that a process created with a path-like name can be found by its basename
    #[tokio::test]
    async fn test_process_name_with_path() {
        let state = test_state();
        let addr = start_test_server(state.clone()).await;

        // Create a process whose name looks like a filesystem path
        let body = json!({
            "name": "/home/user/projects/myApp",
            "script": "sleep",
            "args": ["10"]
        });
        let (s, j) = post(addr, "/api/v1/processes", &body).await;
        assert_eq!(s, 200, "create failed: {j:?}");

        // The list should show the process with a display name of "myApp"
        let (s, j) = get(addr, "/api/v1/processes").await;
        assert_eq!(s, 200);
        let procs = j["data"].as_array().unwrap();
        assert_eq!(procs.len(), 1);
        assert_eq!(procs[0]["name"], "myApp", "Should display basename, got {:?}", procs[0]["name"]);

        // GET by basename should find it
        let (s, _) = get(addr, "/api/v1/processes/myApp").await;
        assert_eq!(s, 200, "GET by basename should find the process");

        // Stop and DELETE by basename should work
        let _ = post(addr, "/api/v1/processes/myApp/stop", &json!({})).await;
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        let (s, j) = delete(addr, "/api/v1/processes/myApp").await;
        assert_eq!(s, 200, "DELETE by basename should work: {j:?}");

        // Should be gone
        let (_, j) = get(addr, "/api/v1/processes").await;
        assert!(j["data"].as_array().unwrap().is_empty(), "Process should be removed");
    }
}
