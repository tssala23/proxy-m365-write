use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use openssl::sha::sha256;
use proxy_m365_write::config::Config;
use proxy_m365_write::{AppState, handle};
use tokio::net::TcpListener;

async fn graph(request: Request<Incoming>) -> Result<Response<Full<Bytes>>, Infallible> {
    assert_eq!(request.method(), "POST");
    assert_eq!(request.uri().path(), "/v1.0/me/messages");
    assert_eq!(
        request.headers()["authorization"],
        "Bearer graph-placeholder"
    );
    let body = request.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["subject"], "Review me");
    assert!(json.get("attachments").is_none());
    Ok(Response::builder()
        .status(StatusCode::CREATED)
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from_static(
            br#"{"id":"draft-1","isDraft":true}"#,
        )))
        .unwrap())
}

async fn spawn_graph() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move {
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            tokio::spawn(async move {
                http1::Builder::new()
                    .serve_connection(TokioIo::new(stream), service_fn(graph))
                    .await
                    .unwrap();
            });
        }
    });
    address
}

async fn spawn_proxy(graph_address: SocketAddr) -> SocketAddr {
    let state = Arc::new(AppState::new(Config {
        inter_vm_bearer_sha256: sha256(b"write-secret"),
        graph_access_token_placeholder: "graph-placeholder".into(),
        listen_addr: "127.0.0.1:0".parse().unwrap(),
        graph_api_base: format!("http://{graph_address}"),
    }));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move {
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            let state = state.clone();
            tokio::spawn(async move {
                http1::Builder::new()
                    .serve_connection(
                        TokioIo::new(stream),
                        service_fn(move |request| handle(state.clone(), request)),
                    )
                    .await
                    .unwrap();
            });
        }
    });
    address
}

async fn request(
    address: SocketAddr,
    method: &str,
    path: &str,
    token: Option<&str>,
    body: &str,
) -> reqwest::Response {
    let client = reqwest::Client::new();
    let mut builder = client
        .request(method.parse().unwrap(), format!("http://{address}{path}"))
        .header("content-type", "application/json")
        .body(body.to_owned());
    if let Some(token) = token {
        builder = builder.header("x-m365-write-bearer", token);
    }
    builder.send().await.unwrap()
}

#[tokio::test]
async fn creates_only_a_validated_draft() {
    let graph = spawn_graph().await;
    let proxy = spawn_proxy(graph).await;
    let response = request(
        proxy,
        "POST",
        "/v1.0/me/messages",
        Some("write-secret"),
        r#"{"subject":"Review me","body":{"contentType":"Text","content":"Safe draft"}}"#,
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let body = response.bytes().await.unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&body).unwrap()["id"],
        "draft-1"
    );
}

#[tokio::test]
async fn rejects_wrong_credentials_paths_methods_and_schema() {
    let graph = spawn_graph().await;
    let proxy = spawn_proxy(graph).await;
    let valid = r#"{"subject":"Review me","body":{"contentType":"Text","content":"Safe draft"}}"#;
    assert_eq!(
        request(proxy, "POST", "/v1.0/me/messages", None, valid)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        request(
            proxy,
            "POST",
            "/v1.0/me/sendMail",
            Some("write-secret"),
            valid
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        request(proxy, "GET", "/v1.0/me/messages", Some("write-secret"), "")
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    let attachment = r#"{"subject":"Review me","body":{"contentType":"Text","content":"Safe draft"},"attachments":[]}"#;
    assert_eq!(
        request(
            proxy,
            "POST",
            "/v1.0/me/messages",
            Some("write-secret"),
            attachment
        )
        .await
        .status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
}
