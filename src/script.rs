use crate::model::{MockRequest, MockResponse};
use rquickjs::{AsyncContext, AsyncRuntime, Promise};
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;
use tokio::sync::oneshot;

pub struct ScriptJob {
    pub script: String,
    pub timeout: Duration,
    pub req: MockRequest,
    pub respondent: oneshot::Sender<Result<MockResponse, String>>,
}

#[derive(Clone)]
pub struct ScriptEngine {
    tx: tokio::sync::mpsc::Sender<ScriptJob>,
    default_timeout: Duration,
}

impl ScriptEngine {
    pub fn new(default_timeout: Duration) -> Self {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<ScriptJob>(100);

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            rt.block_on(async move {
                // Create QuickJS runtime once and reuse it for all script executions
                let runtime = match AsyncRuntime::new() {
                    Ok(rt) => rt,
                    Err(e) => {
                        tracing::error!("Failed to create QuickJS runtime: {}", e);
                        return;
                    }
                };
                runtime.set_max_stack_size(256 * 1024).await;

                let ctx = match AsyncContext::full(&runtime).await {
                    Ok(ctx) => ctx,
                    Err(e) => {
                        tracing::error!("Failed to create QuickJS context: {}", e);
                        return;
                    }
                };

                let runtime = Rc::new(runtime);
                let ctx = Rc::new(ctx);

                while let Some(job) = rx.recv().await {
                    let script = job.script;
                    let req = job.req;
                    let respondent = job.respondent;

                    let res = run_js_script(&script, &req, &runtime, &ctx).await;

                    let _ = respondent.send(res);
                }
            });
        });

        Self {
            tx,
            default_timeout,
        }
    }

    pub async fn execute(
        &self,
        script: &str,
        custom_timeout_ms: Option<u64>,
        req: &MockRequest,
    ) -> Result<MockResponse, String> {
        let timeout = custom_timeout_ms
            .map(Duration::from_millis)
            .unwrap_or(self.default_timeout);

        let (respondent_tx, respondent_rx) = oneshot::channel();

        let job = ScriptJob {
            script: script.to_string(),
            timeout,
            req: req.clone(),
            respondent: respondent_tx,
        };

        if self.tx.send(job).await.is_err() {
            return Err("Script error: Script engine channel closed".to_string());
        }

        match tokio::time::timeout(timeout, respondent_rx).await {
            Ok(Ok(res)) => res,
            Ok(Err(_)) => Err("Script error: Script worker dropped response".to_string()),
            Err(_) => Err("Script error: Script execution timed out".to_string()),
        }
    }
}

async fn run_js_script(
    script: &str,
    req: &MockRequest,
    _runtime: &Rc<AsyncRuntime>,
    ctx: &Rc<AsyncContext>,
) -> Result<MockResponse, String> {
    let mut lower_headers = HashMap::new();
    for (k, v) in &req.headers {
        lower_headers.insert(k.to_lowercase(), v.clone());
    }

    let context_val = serde_json::json!({
        "method": req.method,
        "path": req.path,
        "pathParams": req.path_params,
        "headers": lower_headers,
        "query": req.query,
        "body": req.json_body
    });

    let context_json =
        serde_json::to_string(&context_val).map_err(|e| format!("Script error: {}", e))?;

    let wrapped_code = format!(
        r#"
        (async function() {{
            const context = {context_json};

            let fnResult;
            try {{
                const userFactory = new Function('context', `
                    {script}
                `);
                fnResult = userFactory(context);
                if (fnResult && typeof fnResult.then === 'function') {{
                    fnResult = await fnResult;
                }}
            }} catch (err) {{
                throw err;
            }}

            if (fnResult === undefined) {{
                if (typeof handle === 'function') {{
                    fnResult = handle(context);
                    if (fnResult && typeof fnResult.then === 'function') {{
                        fnResult = await fnResult;
                    }}
                }} else if (typeof main === 'function') {{
                    fnResult = main(context);
                    if (fnResult && typeof fnResult.then === 'function') {{
                        fnResult = await fnResult;
                    }}
                }}
            }}

            if (fnResult === undefined || fnResult === null) {{
                return JSON.stringify({{ status: 200, body: "" }});
            }}

            let status = 200;
            if (typeof fnResult.status === 'number') {{
                status = fnResult.status;
            }}

            let headers = undefined;
            if (fnResult.headers && typeof fnResult.headers === 'object') {{
                headers = {{}};
                for (const [k, v] of Object.entries(fnResult.headers)) {{
                    headers[k] = String(v);
                }}
            }}

            let contentType = undefined;
            if (typeof fnResult.contentType === 'string') {{
                contentType = fnResult.contentType;
            }}

            let body = undefined;
            if (fnResult.body !== undefined && fnResult.body !== null) {{
                if (typeof fnResult.body === 'string') {{
                    body = fnResult.body;
                }} else if (typeof fnResult.body === 'number' || typeof fnResult.body === 'boolean') {{
                    body = String(fnResult.body);
                }} else if (typeof fnResult.body === 'object') {{
                    body = JSON.stringify(fnResult.body);
                    if (!contentType && (!headers || (!headers['content-type'] && !headers['Content-Type']))) {{
                        contentType = 'application/json';
                    }}
                }}
            }}

            return JSON.stringify({{
                status: status,
                headers: headers,
                body: body,
                contentType: contentType
            }});
        }})()
        "#
    );

    ctx.async_with(async |ctx| {
        // Evaluate the wrapped script — the IIFE returns a Promise that resolves to a JSON string
        let promise: Promise<'_> = ctx
            .eval(wrapped_code.as_str())
            .map_err(|e| format!("Script error: {}", e))?;

        // Resolve the promise to get the result string
        let json_str: String = promise
            .into_future()
            .await
            .map_err(|e| format!("Script error: {}", e))?;

        // Parse as MockResponse
        let resp: MockResponse = serde_json::from_str(&json_str)
            .map_err(|e| format!("Script error: Failed to parse result: {}", e))?;
        Ok(resp)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_script_engine_basic() {
        let engine = ScriptEngine::new(Duration::from_millis(2000));
        let req = MockRequest {
            method: "POST".to_string(),
            path: "/calc".to_string(),
            path_params: HashMap::new(),
            headers: HashMap::new(),
            query: HashMap::new(),
            body: r#"{"a": 10, "b": 20}"#.to_string(),
            json_body: Some(serde_json::json!({ "a": 10, "b": 20 })),
        };

        let script = r#"
            return {
                status: 200,
                body: { sum: context.body.a + context.body.b }
            };
        "#;

        let res = engine.execute(script, None, &req).await.unwrap();
        assert_eq!(res.status, 200);
        assert_eq!(res.content_type.as_deref(), Some("application/json"));
        assert!(res.body.unwrap().contains("30"));
    }

    #[tokio::test]
    async fn test_script_engine_timeout() {
        let engine = ScriptEngine::new(Duration::from_millis(100));
        let req = MockRequest {
            method: "GET".to_string(),
            path: "/timeout".to_string(),
            path_params: HashMap::new(),
            headers: HashMap::new(),
            query: HashMap::new(),
            body: "".to_string(),
            json_body: None,
        };

        let script = r#"
            while (true) {}
        "#;

        let res = engine.execute(script, None, &req).await;
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("timed out"));
    }

    #[tokio::test]
    async fn test_script_engine_syntax_error() {
        let engine = ScriptEngine::new(Duration::from_millis(2000));
        let req = MockRequest {
            method: "GET".to_string(),
            path: "/err".to_string(),
            path_params: HashMap::new(),
            headers: HashMap::new(),
            query: HashMap::new(),
            body: "".to_string(),
            json_body: None,
        };

        let script = r#"invalid syntax ;;; }}}"#;

        let res = engine.execute(script, None, &req).await;
        assert!(res.is_err());
        assert!(res.unwrap_err().starts_with("Script error: "));
    }
}
