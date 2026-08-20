use std::collections::HashMap;
use std::{env, fs};

#[derive(Debug, Clone)]
pub struct Config {
    pub inter_vm_bearer_sha256: [u8; 32],
    pub graph_access_token_placeholder: String,
    pub listen_addr: String,
    pub graph_api_base: String,
}

#[derive(Debug)]
pub struct ConfigError(pub String);

impl std::fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

impl std::error::Error for ConfigError {}

fn load_dotenv(path: &str) -> HashMap<String, String> {
    let mut variables = HashMap::new();
    let Ok(contents) = fs::read_to_string(path) else {
        return variables;
    };
    for line in contents.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            variables.insert(key.trim().to_string(), value.trim().to_string());
        }
    }
    variables
}

fn get_var(dotenv: &HashMap<String, String>, key: &str) -> Option<String> {
    env::var(key).ok().or_else(|| dotenv.get(key).cloned())
}

fn decode_sha256(value: &str) -> Result<[u8; 32], ConfigError> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(ConfigError(
            "INTER_VM_BEARER_SHA256 must be exactly 64 hexadecimal characters".into(),
        ));
    }
    let mut digest = [0_u8; 32];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        let pair = std::str::from_utf8(pair).expect("validated hexadecimal is UTF-8");
        digest[index] = u8::from_str_radix(pair, 16).expect("validated hexadecimal");
    }
    Ok(digest)
}

impl Config {
    pub fn load() -> Result<Self, ConfigError> {
        let dotenv = load_dotenv(".env");
        Ok(Self {
            inter_vm_bearer_sha256: get_var(&dotenv, "INTER_VM_BEARER_SHA256")
                .ok_or_else(|| ConfigError("INTER_VM_BEARER_SHA256 not set".into()))
                .and_then(|value| decode_sha256(&value))?,
            graph_access_token_placeholder: get_var(&dotenv, "CLIMICROSOFT365_ACCESS_TOKEN")
                .ok_or_else(|| ConfigError("CLIMICROSOFT365_ACCESS_TOKEN not set".into()))?,
            listen_addr: get_var(&dotenv, "LISTEN_ADDR")
                .unwrap_or_else(|| "127.0.0.1:18081".into()),
            graph_api_base: get_var(&dotenv, "GRAPH_API_BASE")
                .unwrap_or_else(|| "https://graph.microsoft.com".into()),
        })
    }
}
