use axum::{
    http::header,
    response::{IntoResponse, Redirect},
};

pub async fn redirect_to_ui() -> impl IntoResponse {
    Redirect::temporary("/ui/index.html")
}

pub async fn serve_index() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        include_str!("../ui/index.html"),
    )
}

pub async fn serve_app_js() -> impl IntoResponse {
    (
        [(
            header::CONTENT_TYPE,
            "application/javascript; charset=utf-8",
        )],
        include_str!("../ui/app.js"),
    )
}

pub async fn serve_styles_css() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        include_str!("../ui/styles.css"),
    )
}

pub async fn serve_favicon_svg() -> impl IntoResponse {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><text y=".9em" font-size="90">⚡</text></svg>"#;
    ([(header::CONTENT_TYPE, "image/svg+xml")], svg)
}

pub async fn serve_favicon_ico() -> impl IntoResponse {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><text y=".9em" font-size="90">⚡</text></svg>"#;
    ([(header::CONTENT_TYPE, "image/svg+xml")], svg)
}
