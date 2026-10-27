use mock_server::{
    live_feed::LiveFeed,
    matching::MockIndex,
    mock_server::{create_app_router, MockServerState},
    model::*,
    responder::SequenceState,
    script::ScriptEngine,
    store::memory::InMemoryMockStore,
    store::MockStore,
};
use reqwest::Client;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

async fn spawn_test_server() -> String {
    let store = Arc::new(InMemoryMockStore::new());
    let sequence_state = SequenceState::new();
    let script_engine = ScriptEngine::new(Duration::from_millis(2000));
    let live_feed = LiveFeed::new();

    let initial_mocks = store.list().await.unwrap_or_default();
    let mock_cache = Arc::new(tokio::sync::RwLock::new(initial_mocks.clone()));
    let mock_index = Arc::new(tokio::sync::RwLock::new(MockIndex::build(&initial_mocks)));

    let server_state = MockServerState {
        store,
        mock_cache,
        mock_index,
        sequence_state,
        script_engine: Some(script_engine),
        live_feed,
    };

    let app = create_app_router(server_state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    format!("http://127.0.0.1:{}", port)
}

#[tokio::test]
async fn test_admin_api_and_static_mock_lifecycle() {
    let base_url = spawn_test_server().await;
    let client = Client::new();

    // 1. Health check
    let res = client
        .get(format!("{}/__admin/health", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);

    // 2. Create static mock via Admin API
    let mut headers = HashMap::new();
    headers.insert("X-Custom-Header".to_string(), "test-value".to_string());

    let mock = Mock::builder()
        .id("")
        .name("Static Ping Mock")
        .method("GET")
        .path("/api/ping")
        .priority(1)
        .response(MockResponse {
            status: 200,
            headers: Some(headers),
            body: Some(r#"{"pong": true}"#.to_string()),
            content_type: Some("application/json".to_string()),
        })
        .created_at("")
        .updated_at("")
        .build();

    let create_res = client
        .post(format!("{}/__admin/mocks", base_url))
        .json(&mock)
        .send()
        .await
        .unwrap();
    assert_eq!(create_res.status(), 201);
    let created_mock: Mock = create_res.json().await.unwrap();
    assert!(!created_mock.id.is_empty());
    let mock_id = created_mock.id;

    // 3. Call mock surface
    let mock_res = client
        .get(format!("{}/api/ping", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(mock_res.status(), 200);
    assert_eq!(
        mock_res.headers().get("content-type").unwrap(),
        "application/json"
    );
    assert_eq!(
        mock_res.headers().get("x-custom-header").unwrap(),
        "test-value"
    );
    let body = mock_res.text().await.unwrap();
    assert_eq!(body, r#"{"pong": true}"#);

    // 4. List mocks
    let list_res = client
        .get(format!("{}/__admin/mocks", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(list_res.status(), 200);
    let mocks: Vec<Mock> = list_res.json().await.unwrap();
    assert_eq!(mocks.len(), 1);

    // 5. Get mock by ID
    let get_res = client
        .get(format!("{}/__admin/mocks/{}", base_url, mock_id))
        .send()
        .await
        .unwrap();
    assert_eq!(get_res.status(), 200);

    // 6. Update mock
    let update_payload = json!({
        "priority": 50,
        "response": {
            "status": 201,
            "body": "updated body",
            "contentType": "text/plain"
        }
    });
    let update_res = client
        .put(format!("{}/__admin/mocks/{}", base_url, mock_id))
        .json(&update_payload)
        .send()
        .await
        .unwrap();
    assert_eq!(update_res.status(), 200);

    let updated_call = client
        .get(format!("{}/api/ping", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(updated_call.status(), 201);
    assert_eq!(updated_call.text().await.unwrap(), "updated body");

    // 7. Delete mock
    let del_res = client
        .delete(format!("{}/__admin/mocks/{}", base_url, mock_id))
        .send()
        .await
        .unwrap();
    assert_eq!(del_res.status(), 200);

    // 8. Call mock after deletion -> 404
    let not_found_res = client
        .get(format!("{}/api/ping", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(not_found_res.status(), 404);
}

#[tokio::test]
async fn test_content_type_precedence() {
    let base_url = spawn_test_server().await;
    let client = Client::new();

    // 1. Explicit Header has the highest precedence
    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), "application/xml".to_string());

    let mock1 = Mock::builder()
        .id("mock-ct-1")
        .name("CT 1")
        .method("GET")
        .path("/ct-test-1")
        .response(MockResponse {
            status: 200,
            headers: Some(headers),
            body: Some("<xml/>".to_string()),
            content_type: Some("application/json".to_string()),
        })
        .build();
    client
        .post(format!("{}/__admin/mocks", base_url))
        .json(&mock1)
        .send()
        .await
        .unwrap();

    let res1 = client
        .get(format!("{}/ct-test-1", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(
        res1.headers().get("content-type").unwrap(),
        "application/xml"
    );

    // 2. contentType field fallback
    let mock2 = Mock::builder()
        .id("mock-ct-2")
        .name("CT 2")
        .method("GET")
        .path("/ct-test-2")
        .response(MockResponse {
            status: 200,
            headers: None,
            body: Some("{}".to_string()),
            content_type: Some("application/json".to_string()),
        })
        .build();
    client
        .post(format!("{}/__admin/mocks", base_url))
        .json(&mock2)
        .send()
        .await
        .unwrap();

    let res2 = client
        .get(format!("{}/ct-test-2", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(
        res2.headers().get("content-type").unwrap(),
        "application/json"
    );

    // 3. Default text/plain
    let mock3 = Mock::builder()
        .id("mock-ct-3")
        .name("CT 3")
        .method("GET")
        .path("/ct-test-3")
        .response(MockResponse {
            status: 200,
            headers: None,
            body: Some("hello".to_string()),
            content_type: None,
        })
        .build();
    client
        .post(format!("{}/__admin/mocks", base_url))
        .json(&mock3)
        .send()
        .await
        .unwrap();

    let res3 = client
        .get(format!("{}/ct-test-3", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(res3.headers().get("content-type").unwrap(), "text/plain");
}

#[tokio::test]
async fn test_matching_priority_and_url_pattern() {
    let base_url = spawn_test_server().await;
    let client = Client::new();

    // Mock low priority
    let mock_low = Mock::builder()
        .id("low-priority")
        .name("Low Priority")
        .method("GET")
        .path("/users/:id")
        .priority(1)
        .response(MockResponse {
            status: 200,
            body: Some("low".to_string()),
            ..Default::default()
        })
        .created_at("2026-01-01T00:00:00.000Z")
        .updated_at("2026-01-01T00:00:00.000Z")
        .build();

    // Mock high priority
    let mock_high = Mock::builder()
        .id("high-priority")
        .name("High Priority")
        .method("GET")
        .path("/users/:id")
        .priority(10)
        .response(MockResponse {
            status: 200,
            body: Some("high".to_string()),
            ..Default::default()
        })
        .created_at("2026-01-01T00:00:00.000Z")
        .updated_at("2026-01-01T00:00:00.000Z")
        .build();

    client
        .post(format!("{}/__admin/mocks", base_url))
        .json(&mock_low)
        .send()
        .await
        .unwrap();
    client
        .post(format!("{}/__admin/mocks", base_url))
        .json(&mock_high)
        .send()
        .await
        .unwrap();

    let res = client
        .get(format!("{}/users/42", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    assert_eq!(res.text().await.unwrap(), "high");
}

#[tokio::test]
async fn test_conditional_responder() {
    let base_url = spawn_test_server().await;
    let client = Client::new();

    let cond_mock = Mock::builder()
        .id("cond-mock")
        .name("Conditional Route")
        .method("POST")
        .path("/auth/login")
        .mock_type(MockType::Conditional)
        .priority(5)
        .conditions(vec![
            Condition {
                source: ConditionSource::Header,
                key: "X-Role".to_string(),
                op: ConditionOp::Equals,
                value: Some("admin".to_string()),
                response: MockResponse {
                    status: 200,
                    body: Some("Welcome Admin".to_string()),
                    ..Default::default()
                },
            },
            Condition {
                source: ConditionSource::Body,
                key: "username".to_string(),
                op: ConditionOp::Equals,
                value: Some("guest".to_string()),
                response: MockResponse {
                    status: 200,
                    body: Some("Welcome Guest".to_string()),
                    ..Default::default()
                },
            },
            Condition {
                source: ConditionSource::Query,
                key: "secret".to_string(),
                op: ConditionOp::Exists,
                value: None,
                response: MockResponse {
                    status: 200,
                    body: Some("Secret VIP".to_string()),
                    ..Default::default()
                },
            },
        ])
        .else_response(MockResponse {
            status: 403,
            body: Some("Access Denied".to_string()),
            ..Default::default()
        })
        .build();

    client
        .post(format!("{}/__admin/mocks", base_url))
        .json(&cond_mock)
        .send()
        .await
        .unwrap();

    // 1. Match header condition
    let res_admin = client
        .post(format!("{}/auth/login", base_url))
        .header("X-Role", "admin")
        .body(r#"{"username": "other"}"#)
        .send()
        .await
        .unwrap();
    assert_eq!(res_admin.status(), 200);
    assert_eq!(res_admin.text().await.unwrap(), "Welcome Admin");

    // 2. Match body condition
    let res_guest = client
        .post(format!("{}/auth/login", base_url))
        .header("content-type", "application/json")
        .body(r#"{"username": "guest"}"#)
        .send()
        .await
        .unwrap();
    assert_eq!(res_guest.status(), 200);
    assert_eq!(res_guest.text().await.unwrap(), "Welcome Guest");

    // 3. Match query condition
    let res_query = client
        .post(format!("{}/auth/login?secret=123", base_url))
        .body(r#"{}"#)
        .send()
        .await
        .unwrap();
    assert_eq!(res_query.status(), 200);
    assert_eq!(res_query.text().await.unwrap(), "Secret VIP");

    // 4. Match else response
    let res_else = client
        .post(format!("{}/auth/login", base_url))
        .body(r#"{"username": "stranger"}"#)
        .send()
        .await
        .unwrap();
    assert_eq!(res_else.status(), 403);
    assert_eq!(res_else.text().await.unwrap(), "Access Denied");
}

#[tokio::test]
async fn test_sequence_responder_cycle_and_stop_at_end() {
    let base_url = spawn_test_server().await;
    let client = Client::new();

    // 1. Cycle mode
    let seq_cycle = Mock::builder()
        .id("seq-cycle")
        .name("Sequence Cycle")
        .method("GET")
        .path("/seq/cycle")
        .mock_type(MockType::Sequence)
        .sequence(vec![
            MockResponse {
                status: 200,
                body: Some("1".to_string()),
                ..Default::default()
            },
            MockResponse {
                status: 200,
                body: Some("2".to_string()),
                ..Default::default()
            },
        ])
        .sequence_mode(SequenceMode::Cycle)
        .build();

    client
        .post(format!("{}/__admin/mocks", base_url))
        .json(&seq_cycle)
        .send()
        .await
        .unwrap();

    let r1 = client
        .get(format!("{}/seq/cycle", base_url))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    let r2 = client
        .get(format!("{}/seq/cycle", base_url))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    let r3 = client
        .get(format!("{}/seq/cycle", base_url))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert_eq!(r1, "1");
    assert_eq!(r2, "2");
    assert_eq!(r3, "1");

    // Reset sequence via admin endpoint
    client
        .post(format!("{}/__admin/mocks/seq-cycle/reset", base_url))
        .send()
        .await
        .unwrap();
    let r4 = client
        .get(format!("{}/seq/cycle", base_url))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert_eq!(r4, "1");

    // 2. StopAtEnd mode
    let seq_stop = Mock::builder()
        .id("seq-stop")
        .name("Sequence Stop")
        .method("GET")
        .path("/seq/stop")
        .mock_type(MockType::Sequence)
        .sequence(vec![
            MockResponse {
                status: 200,
                body: Some("A".to_string()),
                ..Default::default()
            },
            MockResponse {
                status: 200,
                body: Some("B".to_string()),
                ..Default::default()
            },
        ])
        .sequence_mode(SequenceMode::StopAtEnd)
        .build();

    client
        .post(format!("{}/__admin/mocks", base_url))
        .json(&seq_stop)
        .send()
        .await
        .unwrap();

    let s1 = client
        .get(format!("{}/seq/stop", base_url))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    let s2 = client
        .get(format!("{}/seq/stop", base_url))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    let s3 = client
        .get(format!("{}/seq/stop", base_url))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert_eq!(s1, "A");
    assert_eq!(s2, "B");
    assert_eq!(s3, "B");
}

#[tokio::test]
async fn test_script_responder() {
    let base_url = spawn_test_server().await;
    let client = Client::new();

    let script_mock = Mock::builder()
        .id("script-1")
        .name("Script Mock")
        .method("POST")
        .path("/api/calculate/:operation")
        .mock_type(MockType::Script)
        .script(
            r#"
            const op = context.pathParams.operation;
            const a = context.body.a;
            const b = context.body.b;
            let result = 0;
            if (op === 'add') result = a + b;
            if (op === 'multiply') result = a * b;
            return {
                status: 200,
                headers: { "x-computed-by": "deno-script" },
                body: { operation: op, result: result }
            };
        "#,
        )
        .script_timeout_ms(2000)
        .build();

    client
        .post(format!("{}/__admin/mocks", base_url))
        .json(&script_mock)
        .send()
        .await
        .unwrap();

    let calc_res = client
        .post(format!("{}/api/calculate/multiply", base_url))
        .json(&json!({ "a": 6, "b": 7 }))
        .send()
        .await
        .unwrap();

    assert_eq!(calc_res.status(), 200);
    assert_eq!(
        calc_res.headers().get("content-type").unwrap(),
        "application/json"
    );
    assert_eq!(
        calc_res.headers().get("x-computed-by").unwrap(),
        "deno-script"
    );
    let calc_body: serde_json::Value = calc_res.json().await.unwrap();
    assert_eq!(calc_body["result"], 42);
}

#[tokio::test]
async fn test_rest_mock_complete_crud_and_reset() {
    let base_url = spawn_test_server().await;
    let client = Client::new();

    let initial_items = vec![
        json!({ "id": "1", "name": "Item One", "price": 10 }),
        json!({ "id": "2", "name": "Item Two", "price": 20 }),
    ];

    let rest_mock = Mock::builder()
        .id("rest-items")
        .name("Items REST API")
        .method("*")
        .path("/api/items/:id")
        .mock_type(MockType::Rest)
        .priority(10)
        .rest_id_field("id")
        .rest_initial_state(initial_items.clone())
        .build();

    let create_res = client
        .post(format!("{}/__admin/mocks", base_url))
        .json(&rest_mock)
        .send()
        .await
        .unwrap();
    assert_eq!(create_res.status(), 201);

    // 1. Collection GET
    let col_get = client
        .get(format!("{}/api/items", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(col_get.status(), 200);
    let items_list: Vec<serde_json::Value> = col_get.json().await.unwrap();
    assert_eq!(items_list.len(), 2);

    // 2. Item GET
    let item_get = client
        .get(format!("{}/api/items/1", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(item_get.status(), 200);
    let item: serde_json::Value = item_get.json().await.unwrap();
    assert_eq!(item["name"], "Item One");

    // 3. Collection POST (create with auto UUID or specified id)
    let post_res = client
        .post(format!("{}/api/items", base_url))
        .json(&json!({ "name": "Item Three", "price": 30 }))
        .send()
        .await
        .unwrap();
    assert_eq!(post_res.status(), 201);
    let created_item: serde_json::Value = post_res.json().await.unwrap();
    assert!(created_item["id"].is_string());
    assert_eq!(created_item["name"], "Item Three");

    // Verify collection has 3 items
    let col_get3 = client
        .get(format!("{}/api/items", base_url))
        .send()
        .await
        .unwrap();
    let items_list3: Vec<serde_json::Value> = col_get3.json().await.unwrap();
    assert_eq!(items_list3.len(), 3);

    // 4. Item PATCH
    let patch_res = client
        .patch(format!("{}/api/items/1", base_url))
        .json(&json!({ "price": 99 }))
        .send()
        .await
        .unwrap();
    assert_eq!(patch_res.status(), 200);
    let patched_item: serde_json::Value = patch_res.json().await.unwrap();
    assert_eq!(patched_item["price"], 99);
    assert_eq!(patched_item["name"], "Item One"); // preserved name

    // 5. Item DELETE
    let del_item = client
        .delete(format!("{}/api/items/2", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(del_item.status(), 204);

    let get_deleted = client
        .get(format!("{}/api/items/2", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(get_deleted.status(), 404);

    // 6. Reset endpoint restores original initial state
    let reset_res = client
        .post(format!("{}/__admin/mocks/rest-items/reset", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(reset_res.status(), 200);

    let col_after_reset = client
        .get(format!("{}/api/items", base_url))
        .send()
        .await
        .unwrap();
    let items_after_reset: Vec<serde_json::Value> = col_after_reset.json().await.unwrap();
    assert_eq!(items_after_reset.len(), 2);
    assert_eq!(items_after_reset[0]["price"], 10);
}

#[tokio::test]
async fn test_ui_and_static_routes() {
    let base_url = spawn_test_server().await;
    let client = Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();

    // 1. Root redirect
    let root_res = client.get(&base_url).send().await.unwrap();
    assert!(root_res.status().is_redirection());
    assert_eq!(
        root_res.headers().get("location").unwrap(),
        "/ui/index.html"
    );

    // 2. Static files
    let html_res = client
        .get(format!("{}/ui/index.html", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(html_res.status(), 200);

    let js_res = client
        .get(format!("{}/ui/app.js", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(js_res.status(), 200);

    let css_res = client
        .get(format!("{}/ui/styles.css", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(css_res.status(), 200);

    let fav_res = client
        .get(format!("{}/favicon.svg", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(fav_res.status(), 200);
}

#[tokio::test]
async fn test_rest_mock_custom_id_field() {
    let base_url = spawn_test_server().await;
    let client = Client::new();

    let custom_rest = Mock::builder()
        .id("rest-sku")
        .name("Products by SKU")
        .method("*")
        .path("/products/:id")
        .mock_type(MockType::Rest)
        .rest_id_field("sku")
        .rest_initial_state(vec![json!({ "sku": "PROD-100", "title": "Keyboard" })])
        .build();

    client
        .post(format!("{}/__admin/mocks", base_url))
        .json(&custom_rest)
        .send()
        .await
        .unwrap();

    // 1. GET item by SKU
    let get_item = client
        .get(format!("{}/products/PROD-100", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(get_item.status(), 200);
    let item_json: serde_json::Value = get_item.json().await.unwrap();
    assert_eq!(item_json["title"], "Keyboard");

    // 2. PUT item by SKU
    let put_item = client
        .put(format!("{}/products/PROD-100", base_url))
        .json(&json!({ "title": "Mechanical Keyboard" }))
        .send()
        .await
        .unwrap();
    assert_eq!(put_item.status(), 200);
    let updated_json: serde_json::Value = put_item.json().await.unwrap();
    assert_eq!(updated_json["sku"], "PROD-100");
    assert_eq!(updated_json["title"], "Mechanical Keyboard");
}

#[tokio::test]
async fn test_admin_validations() {
    let base_url = spawn_test_server().await;
    let client = Client::new();

    // 1. Missing name
    let invalid_mock = json!({
        "name": "",
        "method": "GET",
        "path": "/test",
        "type": "static",
        "response": { "status": 200, "body": "ok" }
    });
    let res = client
        .post(format!("{}/__admin/mocks", base_url))
        .json(&invalid_mock)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 400);

    // 2. Path starting with /__admin
    let admin_path_mock = json!({
        "name": "Invalid Admin Path",
        "method": "GET",
        "path": "/__admin/custom",
        "type": "static",
        "response": { "status": 200, "body": "ok" }
    });
    let res2 = client
        .post(format!("{}/__admin/mocks", base_url))
        .json(&admin_path_mock)
        .send()
        .await
        .unwrap();
    assert_eq!(res2.status(), 400);

    // 3. Static without response
    let no_resp_mock = json!({
        "name": "No response",
        "method": "GET",
        "path": "/no-resp",
        "type": "static"
    });
    let res3 = client
        .post(format!("{}/__admin/mocks", base_url))
        .json(&no_resp_mock)
        .send()
        .await
        .unwrap();
    assert_eq!(res3.status(), 400);
}

#[tokio::test]
async fn test_404_not_found_standard_payload() {
    let base_url = spawn_test_server().await;
    let client = Client::new();

    let res = client
        .get(format!("{}/non-existent/route", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 404);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["error"], "Not Found");
    assert_eq!(
        body["message"],
        "No mock matched for GET /non-existent/route"
    );
}
