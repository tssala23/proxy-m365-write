pub mod config;
pub mod draft;
pub mod proxy;

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use bytes::{BufMut, Bytes, BytesMut};
use config::Config;
use draft::Draft;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::{Method, Request, Response, StatusCode};
use openssl::sha::sha256;

const MAX_REQUEST_BYTES: usize = 64 * 1024;

pub struct AppState {
    pub config: Config,
    pub client: reqwest::Client,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            client: reqwest::Client::builder()
                .use_native_tls()
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .expect("failed to initialize native TLS client"),
        }
    }
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    let mut difference = left.len() ^ right.len();
    let max_len = left.len().max(right.len());
    for index in 0..max_len {
        difference |= usize::from(
            left.get(index).copied().unwrap_or(0) ^ right.get(index).copied().unwrap_or(0),
        );
    }
    difference == 0
}

fn authenticated(request: &Request<Incoming>, expected: &[u8; 32]) -> bool {
    let Some(value) = request
        .headers()
        .get("x-m365-write-bearer")
        .and_then(|value| value.to_str().ok())
    else {
        return false;
    };
    constant_time_eq(&sha256(value.trim().as_bytes()), expected)
}

fn response(status: StatusCode, message: impl Into<Bytes>) -> Response<Full<Bytes>> {
    Response::builder()
        .status(status)
        .header("content-type", "text/plain")
        .body(Full::new(message.into()))
        .expect("static response")
}

fn log_denial(method: &Method, path: &str, reason: &str) {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    eprintln!("[DENY] ts={timestamp} method={method} path={path} reason=\"{reason}\"");
}

async fn read_limited(mut body: Incoming) -> Result<Vec<u8>, &'static str> {
    let mut bytes = BytesMut::new();
    while let Some(frame) = body.frame().await {
        let frame = frame.map_err(|_| "failed to read request body")?;
        if let Ok(data) = frame.into_data() {
            if bytes.len() + data.len() > MAX_REQUEST_BYTES {
                return Err("request body exceeds 65536 bytes");
            }
            bytes.put(data);
        }
    }
    Ok(bytes.to_vec())
}

fn send_draft_id(path: &str) -> Option<&str> {
    let id = path
        .strip_prefix("/v1.0/me/messages/")?
        .strip_suffix("/send")?;
    if id.is_empty()
        || id.len() > 512
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_.=%".contains(&byte))
    {
        return None;
    }
    Some(id)
}

pub async fn handle(
    state: Arc<AppState>,
    request: Request<Incoming>,
) -> Result<Response<Full<Bytes>>, std::convert::Infallible> {
    let method = request.method().clone();
    let path = request.uri().path().to_string();
    if !authenticated(&request, &state.config.inter_vm_bearer_sha256) {
        log_denial(&method, &path, "invalid inter-VM bearer");
        return Ok(response(StatusCode::UNAUTHORIZED, "unauthorized"));
    }
    let is_create = path == "/v1.0/me/messages";
    let send_id = send_draft_id(&path);
    if method != Method::POST
        || (!is_create && send_id.is_none())
        || request.uri().query().is_some()
    {
        log_denial(
            &method,
            &path,
            "only create-draft and send-draft POSTs are permitted",
        );
        return Ok(response(StatusCode::FORBIDDEN, "forbidden"));
    }
    if let Some(send_id) = send_id {
        let body = match read_limited(request.into_body()).await {
            Ok(body) => body,
            Err(message) => return Ok(response(StatusCode::PAYLOAD_TOO_LARGE, message)),
        };
        if !body.is_empty() {
            return Ok(response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "send request body must be empty",
            ));
        }
        let upstream = match proxy::send_draft(
            &state.client,
            &state.config.graph_api_base,
            &state.config.graph_access_token_placeholder,
            send_id,
        )
        .await
        {
            Ok(upstream) => upstream,
            Err(error) => {
                eprintln!("[ERROR] {error}");
                return Ok(response(StatusCode::BAD_GATEWAY, "upstream request failed"));
            }
        };
        let status = upstream.status();
        let body = upstream.bytes().await.unwrap_or_default();
        return Ok(Response::builder()
            .status(status)
            .body(Full::new(body))
            .unwrap_or_else(|_| response(StatusCode::BAD_GATEWAY, "response build failed")));
    }
    if request
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .is_none_or(|value| !value.eq_ignore_ascii_case("application/json"))
    {
        return Ok(response(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "content-type must be application/json",
        ));
    }

    let raw_body = match read_limited(request.into_body()).await {
        Ok(body) => body,
        Err(message) => return Ok(response(StatusCode::PAYLOAD_TOO_LARGE, message)),
    };
    let validated_body = match Draft::parse_and_validate(&raw_body) {
        Ok(body) => body,
        Err(message) => return Ok(response(StatusCode::UNPROCESSABLE_ENTITY, message)),
    };
    let upstream = match proxy::create_draft(
        &state.client,
        &state.config.graph_api_base,
        &state.config.graph_access_token_placeholder,
        validated_body,
    )
    .await
    {
        Ok(upstream) => upstream,
        Err(error) => {
            eprintln!("[ERROR] {error}");
            return Ok(response(StatusCode::BAD_GATEWAY, "upstream request failed"));
        }
    };
    let status = upstream.status();
    let body = match upstream.bytes().await {
        Ok(body) => body,
        Err(_) => {
            return Ok(response(
                StatusCode::BAD_GATEWAY,
                "upstream body read failed",
            ));
        }
    };
    Ok(Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(Full::new(body))
        .unwrap_or_else(|_| response(StatusCode::BAD_GATEWAY, "response build failed")))
}
