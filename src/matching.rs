use crate::model::{Mock, MockRequest, MockType};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct MatchResult {
    pub matched_mock: Option<Mock>,
    pub path_params: HashMap<String, String>,
}

pub fn match_path_pattern(
    pattern: &str,
    path: &str,
    is_rest: bool,
) -> Option<HashMap<String, String>> {
    let clean_pattern = pattern.trim_matches('/');
    let clean_path = path.trim_matches('/');

    let pat_segments: Vec<&str> = if clean_pattern.is_empty() {
        Vec::new()
    } else {
        clean_pattern.split('/').collect()
    };

    let path_segments: Vec<&str> = if clean_path.is_empty() {
        Vec::new()
    } else {
        clean_path.split('/').collect()
    };

    if is_rest {
        // For REST mocks: support both collection (e.g. /items) and item (e.g. /items/123)
        let mut base_pat_segments = pat_segments.clone();
        let mut id_param_name = "id".to_string();

        if let Some(last) = base_pat_segments.last() {
            if last.starts_with(':') {
                id_param_name = last
                    .trim_start_matches(':')
                    .trim_end_matches('?')
                    .to_string();
                base_pat_segments.pop();
            }
        }

        // Case A: Path matches base segments exactly (Collection)
        if segments_match_exact(&base_pat_segments, &path_segments) {
            let mut params = HashMap::new();
            for (p_seg, u_seg) in base_pat_segments.iter().zip(path_segments.iter()) {
                if p_seg.starts_with(':') {
                    let key = p_seg.trim_start_matches(':').trim_end_matches('?');
                    params.insert(key.to_string(), u_seg.to_string());
                }
            }
            return Some(params);
        }

        // Case B: Path matches base segments + 1 segment (Item)
        if path_segments.len() == base_pat_segments.len() + 1
            && segments_match_exact(
                &base_pat_segments,
                &path_segments[..base_pat_segments.len()],
            )
        {
            let mut params = HashMap::new();
            for (p_seg, u_seg) in base_pat_segments
                .iter()
                .zip(path_segments[..base_pat_segments.len()].iter())
            {
                if p_seg.starts_with(':') {
                    let key = p_seg.trim_start_matches(':').trim_end_matches('?');
                    params.insert(key.to_string(), u_seg.to_string());
                }
            }
            let item_id = path_segments[base_pat_segments.len()];
            params.insert(id_param_name, item_id.to_string());
            return Some(params);
        }
    }

    match_generic_pattern(&pat_segments, &path_segments)
}

fn segments_match_exact(pat_segments: &[&str], path_segments: &[&str]) -> bool {
    if pat_segments.len() != path_segments.len() {
        return false;
    }
    for (p, u) in pat_segments.iter().zip(path_segments.iter()) {
        if p.starts_with(':') || *p == "*" {
            continue;
        }
        if p != u {
            return false;
        }
    }
    true
}

fn match_generic_pattern(
    pat_segments: &[&str],
    path_segments: &[&str],
) -> Option<HashMap<String, String>> {
    let mut params = HashMap::new();
    let mut p_idx = 0;
    let mut u_idx = 0;

    while p_idx < pat_segments.len() && u_idx < path_segments.len() {
        let p_seg = pat_segments[p_idx];
        let u_seg = path_segments[u_idx];

        if p_seg == "*" {
            // Wildcard matches rest or single
            if p_idx == pat_segments.len() - 1 {
                return Some(params);
            }
            p_idx += 1;
            u_idx += 1;
            continue;
        }

        if p_seg.starts_with(':') {
            let param_name = p_seg.trim_start_matches(':').trim_end_matches('?');
            params.insert(param_name.to_string(), u_seg.to_string());
            p_idx += 1;
            u_idx += 1;
            continue;
        }

        if p_seg == u_seg {
            p_idx += 1;
            u_idx += 1;
            continue;
        }

        return None;
    }

    // Check trailing optional parameters in pattern
    while p_idx < pat_segments.len() {
        let p_seg = pat_segments[p_idx];
        if (p_seg.starts_with(':') && p_seg.ends_with('?')) || p_seg == "*" {
            p_idx += 1;
        } else {
            break;
        }
    }

    if p_idx == pat_segments.len() && u_idx == path_segments.len() {
        Some(params)
    } else {
        None
    }
}

pub fn matches_mock(mock: &Mock, req: &MockRequest) -> Option<HashMap<String, String>> {
    let method_matches = mock.method == "*" || mock.method.eq_ignore_ascii_case(&req.method);
    if !method_matches {
        return None;
    }

    let is_rest = mock.mock_type == MockType::Rest;
    match_path_pattern(&mock.path, &req.path, is_rest)
}

pub fn find_best_match(mocks: &[Mock], req: &MockRequest) -> MatchResult {
    let mut candidates: Vec<(Mock, HashMap<String, String>)> = Vec::new();

    for mock in mocks {
        if let Some(params) = matches_mock(mock, req) {
            candidates.push((mock.clone(), params));
        }
    }

    candidates.sort_by(|(a, _), (b, _)| {
        b.priority
            .cmp(&a.priority)
            .then_with(|| b.created_at.cmp(&a.created_at))
    });

    if let Some((best_mock, params)) = candidates.into_iter().next() {
        MatchResult {
            matched_mock: Some(best_mock),
            path_params: params,
        }
    } else {
        MatchResult {
            matched_mock: None,
            path_params: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_param_matching() {
        let res = match_path_pattern("/users/:id", "/users/42", false);
        assert!(res.is_some());
        let params = res.unwrap();
        assert_eq!(params.get("id").unwrap(), "42");
    }

    #[test]
    fn test_multi_param_matching() {
        let res = match_path_pattern(
            "/api/v1/:section/items/:item_id",
            "/api/v1/books/items/99",
            false,
        );
        assert!(res.is_some());
        let params = res.unwrap();
        assert_eq!(params.get("section").unwrap(), "books");
        assert_eq!(params.get("item_id").unwrap(), "99");
    }

    #[test]
    fn test_rest_collection_and_item() {
        let col_res = match_path_pattern("/items/:id", "/items", true);
        assert!(col_res.is_some());
        assert!(col_res.unwrap().get("id").is_none());

        let item_res = match_path_pattern("/items/:id", "/items/123", true);
        assert!(item_res.is_some());
        assert_eq!(item_res.unwrap().get("id").unwrap(), "123");

        let col_res2 = match_path_pattern("/items", "/items", true);
        assert!(col_res2.is_some());

        let item_res2 = match_path_pattern("/items", "/items/456", true);
        assert!(item_res2.is_some());
        assert_eq!(item_res2.unwrap().get("id").unwrap(), "456");
    }

    #[test]
    fn test_wildcard_matching() {
        let res = match_path_pattern("/api/*", "/api/users", false);
        assert!(res.is_some());

        let res = match_path_pattern("/api/*", "/api/users/123/orders", false);
        assert!(res.is_some());
    }

    #[test]
    fn test_no_match() {
        let res = match_path_pattern("/users", "/orders", false);
        assert!(res.is_none());

        let res = match_path_pattern("/users/:id", "/users", false);
        assert!(res.is_none());
    }

    #[test]
    fn test_exact_match() {
        let res = match_path_pattern("/api/health", "/api/health", false);
        assert!(res.is_some());
        assert!(res.unwrap().is_empty());
    }

    #[test]
    fn test_optional_params() {
        let res = match_path_pattern("/users/:id/:format?", "/users/42", false);
        assert!(res.is_some());
        let params = res.unwrap();
        assert_eq!(params.get("id").unwrap(), "42");

        let res = match_path_pattern("/users/:id/:format?", "/users/42/json", false);
        assert!(res.is_some());
        let params = res.unwrap();
        assert_eq!(params.get("id").unwrap(), "42");
        assert_eq!(params.get("format").unwrap(), "json");
    }

    #[test]
    fn test_root_path() {
        let res = match_path_pattern("/", "/", false);
        assert!(res.is_some());
    }

    #[test]
    fn test_matches_mock_method_wildcard() {
        let mock = Mock {
            id: "1".to_string(),
            name: "Wildcard".to_string(),
            method: "*".to_string(),
            path: "/api/test".to_string(),
            mock_type: MockType::Static,
            priority: 0,
            response: None,
            conditions: None,
            else_response: None,
            sequence: None,
            sequence_mode: None,
            script: None,
            script_timeout_ms: None,
            rest_id_field: None,
            rest_initial_state: None,
            state: None,
            created_at: "2026-01-01T00:00:00.000Z".to_string(),
            updated_at: "2026-01-01T00:00:00.000Z".to_string(),
        };
        let req = MockRequest {
            method: "DELETE".to_string(),
            path: "/api/test".to_string(),
            path_params: HashMap::new(),
            headers: HashMap::new(),
            query: HashMap::new(),
            body: "".to_string(),
            json_body: None,
        };
        assert!(matches_mock(&mock, &req).is_some());
    }

    #[test]
    fn test_matches_mock_method_case_insensitive() {
        let mock = Mock {
            id: "1".to_string(),
            name: "Get Mock".to_string(),
            method: "GET".to_string(),
            path: "/api/test".to_string(),
            mock_type: MockType::Static,
            priority: 0,
            response: None,
            conditions: None,
            else_response: None,
            sequence: None,
            sequence_mode: None,
            script: None,
            script_timeout_ms: None,
            rest_id_field: None,
            rest_initial_state: None,
            state: None,
            created_at: "2026-01-01T00:00:00.000Z".to_string(),
            updated_at: "2026-01-01T00:00:00.000Z".to_string(),
        };
        let req = MockRequest {
            method: "get".to_string(),
            path: "/api/test".to_string(),
            path_params: HashMap::new(),
            headers: HashMap::new(),
            query: HashMap::new(),
            body: "".to_string(),
            json_body: None,
        };
        assert!(matches_mock(&mock, &req).is_some());
    }

    #[test]
    fn test_matches_mock_method_mismatch() {
        let mock = Mock {
            id: "1".to_string(),
            name: "Post Mock".to_string(),
            method: "POST".to_string(),
            path: "/api/test".to_string(),
            mock_type: MockType::Static,
            priority: 0,
            response: None,
            conditions: None,
            else_response: None,
            sequence: None,
            sequence_mode: None,
            script: None,
            script_timeout_ms: None,
            rest_id_field: None,
            rest_initial_state: None,
            state: None,
            created_at: "2026-01-01T00:00:00.000Z".to_string(),
            updated_at: "2026-01-01T00:00:00.000Z".to_string(),
        };
        let req = MockRequest {
            method: "GET".to_string(),
            path: "/api/test".to_string(),
            path_params: HashMap::new(),
            headers: HashMap::new(),
            query: HashMap::new(),
            body: "".to_string(),
            json_body: None,
        };
        assert!(matches_mock(&mock, &req).is_none());
    }

    #[test]
    fn test_find_best_match_by_priority() {
        let mocks = vec![
            Mock {
                id: "low".to_string(),
                name: "Low Priority".to_string(),
                method: "GET".to_string(),
                path: "/api/test".to_string(),
                mock_type: MockType::Static,
                priority: 1,
                response: None,
                conditions: None,
                else_response: None,
                sequence: None,
                sequence_mode: None,
                script: None,
                script_timeout_ms: None,
                rest_id_field: None,
                rest_initial_state: None,
                state: None,
                created_at: "2026-01-01T00:00:00.000Z".to_string(),
                updated_at: "2026-01-01T00:00:00.000Z".to_string(),
            },
            Mock {
                id: "high".to_string(),
                name: "High Priority".to_string(),
                method: "GET".to_string(),
                path: "/api/test".to_string(),
                mock_type: MockType::Static,
                priority: 10,
                response: None,
                conditions: None,
                else_response: None,
                sequence: None,
                sequence_mode: None,
                script: None,
                script_timeout_ms: None,
                rest_id_field: None,
                rest_initial_state: None,
                state: None,
                created_at: "2026-01-01T00:00:00.000Z".to_string(),
                updated_at: "2026-01-01T00:00:00.000Z".to_string(),
            },
        ];
        let req = MockRequest {
            method: "GET".to_string(),
            path: "/api/test".to_string(),
            path_params: HashMap::new(),
            headers: HashMap::new(),
            query: HashMap::new(),
            body: "".to_string(),
            json_body: None,
        };
        let result = find_best_match(&mocks, &req);
        assert!(result.matched_mock.is_some());
        assert_eq!(result.matched_mock.unwrap().id, "high");
    }

    #[test]
    fn test_find_best_match_no_match() {
        let mocks = vec![Mock {
            id: "1".to_string(),
            name: "Mock".to_string(),
            method: "GET".to_string(),
            path: "/api/other".to_string(),
            mock_type: MockType::Static,
            priority: 0,
            response: None,
            conditions: None,
            else_response: None,
            sequence: None,
            sequence_mode: None,
            script: None,
            script_timeout_ms: None,
            rest_id_field: None,
            rest_initial_state: None,
            state: None,
            created_at: "2026-01-01T00:00:00.000Z".to_string(),
            updated_at: "2026-01-01T00:00:00.000Z".to_string(),
        }];
        let req = MockRequest {
            method: "GET".to_string(),
            path: "/api/test".to_string(),
            path_params: HashMap::new(),
            headers: HashMap::new(),
            query: HashMap::new(),
            body: "".to_string(),
            json_body: None,
        };
        let result = find_best_match(&mocks, &req);
        assert!(result.matched_mock.is_none());
    }

    #[test]
    fn test_find_best_match_by_created_at() {
        let mocks = vec![
            Mock {
                id: "older".to_string(),
                name: "Older".to_string(),
                method: "GET".to_string(),
                path: "/api/test".to_string(),
                mock_type: MockType::Static,
                priority: 5,
                response: None,
                conditions: None,
                else_response: None,
                sequence: None,
                sequence_mode: None,
                script: None,
                script_timeout_ms: None,
                rest_id_field: None,
                rest_initial_state: None,
                state: None,
                created_at: "2026-01-01T00:00:00.000Z".to_string(),
                updated_at: "2026-01-01T00:00:00.000Z".to_string(),
            },
            Mock {
                id: "newer".to_string(),
                name: "Newer".to_string(),
                method: "GET".to_string(),
                path: "/api/test".to_string(),
                mock_type: MockType::Static,
                priority: 5,
                response: None,
                conditions: None,
                else_response: None,
                sequence: None,
                sequence_mode: None,
                script: None,
                script_timeout_ms: None,
                rest_id_field: None,
                rest_initial_state: None,
                state: None,
                created_at: "2026-02-01T00:00:00.000Z".to_string(),
                updated_at: "2026-02-01T00:00:00.000Z".to_string(),
            },
        ];
        let req = MockRequest {
            method: "GET".to_string(),
            path: "/api/test".to_string(),
            path_params: HashMap::new(),
            headers: HashMap::new(),
            query: HashMap::new(),
            body: "".to_string(),
            json_body: None,
        };
        let result = find_best_match(&mocks, &req);
        assert!(result.matched_mock.is_some());
        assert_eq!(result.matched_mock.unwrap().id, "newer");
    }

    #[test]
    fn test_find_best_match_returns_path_params() {
        let mocks = vec![Mock {
            id: "1".to_string(),
            name: "Mock".to_string(),
            method: "GET".to_string(),
            path: "/users/:id".to_string(),
            mock_type: MockType::Static,
            priority: 0,
            response: None,
            conditions: None,
            else_response: None,
            sequence: None,
            sequence_mode: None,
            script: None,
            script_timeout_ms: None,
            rest_id_field: None,
            rest_initial_state: None,
            state: None,
            created_at: "2026-01-01T00:00:00.000Z".to_string(),
            updated_at: "2026-01-01T00:00:00.000Z".to_string(),
        }];
        let req = MockRequest {
            method: "GET".to_string(),
            path: "/users/42".to_string(),
            path_params: HashMap::new(),
            headers: HashMap::new(),
            query: HashMap::new(),
            body: "".to_string(),
            json_body: None,
        };
        let result = find_best_match(&mocks, &req);
        assert!(result.matched_mock.is_some());
        assert_eq!(result.path_params.get("id").unwrap(), "42");
    }

    #[test]
    fn test_rest_custom_id_param() {
        let res = match_path_pattern("/products/:productId", "/products/abc-123", true);
        assert!(res.is_some());
        assert_eq!(res.unwrap().get("productId").unwrap(), "abc-123");
    }

    #[test]
    fn test_segments_match_exact_with_wildcard() {
        let result = segments_match_exact(&["api", "*"], &["api", "anything"]);
        assert!(result);
    }

    #[test]
    fn test_segments_match_exact_mismatch() {
        let result = segments_match_exact(&["api", "users"], &["api", "orders"]);
        assert!(!result);
    }

    #[test]
    fn test_segments_match_exact_different_lengths() {
        let result = segments_match_exact(&["api"], &["api", "users"]);
        assert!(!result);
    }
}
