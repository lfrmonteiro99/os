use std::io::{Read, Write};
use std::net::TcpListener;

const INDEX_HTML: &str = include_str!("../../../desktop/index.html");
const STYLES_CSS: &str = include_str!("../../../desktop/styles.css");
const APP_JS: &str = include_str!("../../../desktop/app.js");

fn response(status: &str, content_type: &str, body: &str) -> String {
    format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

fn route(path: &str) -> (&'static str, &'static str, &'static str) {
    match path {
        "/" | "/index.html" => ("200 OK", "text/html; charset=utf-8", INDEX_HTML),
        "/styles.css" => ("200 OK", "text/css; charset=utf-8", STYLES_CSS),
        "/app.js" => ("200 OK", "application/javascript; charset=utf-8", APP_JS),
        _ => ("404 Not Found", "text/plain; charset=utf-8", "Not Found"),
    }
}

fn parse_path(request: &str) -> &str {
    request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("/")
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:4173").expect("failed to bind 127.0.0.1:4173");

    println!("AuroraOS visual preview: http://127.0.0.1:4173");
    println!("Press Ctrl+C to stop.");

    for incoming in listener.incoming() {
        let Ok(mut stream) = incoming else {
            continue;
        };

        let mut buffer = [0_u8; 2048];
        let Ok(read) = stream.read(&mut buffer) else {
            continue;
        };
        if read == 0 {
            continue;
        }

        let req = String::from_utf8_lossy(&buffer[..read]);
        let path = parse_path(&req);
        let (status, content_type, body) = route(path);
        let res = response(status, content_type, body);
        let _ = stream.write_all(res.as_bytes());
        let _ = stream.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── response formatting ─────────────────────────────

    #[test]
    fn response_has_status_line() {
        let res = response("200 OK", "text/html", "hello");
        assert!(res.starts_with("HTTP/1.1 200 OK\r\n"));
    }

    #[test]
    fn response_has_content_type() {
        let res = response("200 OK", "text/css; charset=utf-8", "body{}");
        assert!(res.contains("Content-Type: text/css; charset=utf-8"));
    }

    #[test]
    fn response_has_correct_content_length() {
        let body = "hello world";
        let res = response("200 OK", "text/plain", body);
        assert!(res.contains(&format!("Content-Length: {}", body.len())));
    }

    #[test]
    fn response_body_follows_headers() {
        let res = response("200 OK", "text/plain", "test");
        let parts: Vec<&str> = res.splitn(2, "\r\n\r\n").collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[1], "test");
    }

    #[test]
    fn response_has_connection_close() {
        let res = response("404 Not Found", "text/plain", "nope");
        assert!(res.contains("Connection: close"));
    }

    // ── path parsing ────────────────────────────────────

    #[test]
    fn parse_path_get_root() {
        assert_eq!(parse_path("GET / HTTP/1.1\r\nHost: localhost"), "/");
    }

    #[test]
    fn parse_path_get_css() {
        assert_eq!(parse_path("GET /styles.css HTTP/1.1\r\n"), "/styles.css");
    }

    #[test]
    fn parse_path_get_js() {
        assert_eq!(parse_path("GET /app.js HTTP/1.1\r\n"), "/app.js");
    }

    #[test]
    fn parse_path_empty_defaults_to_root() {
        assert_eq!(parse_path(""), "/");
    }

    #[test]
    fn parse_path_unknown() {
        assert_eq!(parse_path("GET /unknown HTTP/1.1"), "/unknown");
    }

    // ── routing ─────────────────────────────────────────

    #[test]
    fn route_root_returns_html() {
        let (status, ct, _body) = route("/");
        assert_eq!(status, "200 OK");
        assert!(ct.contains("text/html"));
    }

    #[test]
    fn route_index_returns_html() {
        let (status, ct, _body) = route("/index.html");
        assert_eq!(status, "200 OK");
        assert!(ct.contains("text/html"));
    }

    #[test]
    fn route_css_returns_css() {
        let (status, ct, _body) = route("/styles.css");
        assert_eq!(status, "200 OK");
        assert!(ct.contains("text/css"));
    }

    #[test]
    fn route_js_returns_javascript() {
        let (status, ct, _body) = route("/app.js");
        assert_eq!(status, "200 OK");
        assert!(ct.contains("javascript"));
    }

    #[test]
    fn route_unknown_returns_404() {
        let (status, _, body) = route("/nope");
        assert_eq!(status, "404 Not Found");
        assert_eq!(body, "Not Found");
    }

    // ── embedded content ────────────────────────────────

    #[test]
    fn index_html_is_not_empty() {
        assert!(!INDEX_HTML.is_empty());
        assert!(INDEX_HTML.contains("<!doctype html>") || INDEX_HTML.contains("<!DOCTYPE html>"));
    }

    #[test]
    fn styles_css_is_not_empty() {
        assert!(!STYLES_CSS.is_empty());
        assert!(STYLES_CSS.contains("wallpaper"));
    }

    #[test]
    fn app_js_is_not_empty() {
        assert!(!APP_JS.is_empty());
        assert!(APP_JS.contains("function"));
    }

    #[test]
    fn html_references_css() {
        assert!(INDEX_HTML.contains("styles.css"));
    }

    #[test]
    fn html_references_js() {
        assert!(INDEX_HTML.contains("app.js"));
    }
}
