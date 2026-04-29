//! Host-only Ed25519 signing sidecar — private key stays in this process (`verity-signer`).

use std::{
    net::{Ipv4Addr, SocketAddr},
    path::{Path, PathBuf},
    process::exit,
};

use axum::{
    body::Bytes,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use ed25519_dalek::{
    pkcs8::{DecodePrivateKey, EncodePublicKey},
    Signer, SigningKey,
};
use serde::Serialize;
use serde_json::{json, Value};

const DEFAULT_SIGNER_ADDR: &str = "127.0.0.1:8090";
const DEFAULT_KEY_FILE_NAME: &str = "arena_signing_key.hex";
const OPENSSL_GENPKEY_CMD: &str = "openssl genpkey -algorithm Ed25519 -outform DER | xxd -p | tr -d '\\n' > arena_signing_key.hex";

fn key_file_path_default() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(DEFAULT_KEY_FILE_NAME)
}

fn key_file_from_env() -> PathBuf {
    match std::env::var_os("VERITY_ARENA_SIGNING_KEY_FILE") {
        Some(p) => PathBuf::from(p),
        None => key_file_path_default(),
    }
}

fn load_key_or_exit(path: &Path) -> SigningKey {
    let raw = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("verity-signer: could not read signing key file `{}`: {e}", path.display());
            eprintln!("verity-signer: create the key once at this path using:");
            eprintln!("  {OPENSSL_GENPKEY_CMD}");
            exit(1);
        },
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        eprintln!(
            "verity-signer: signing key file `{}` is empty after trimming whitespace",
            path.display()
        );
        eprintln!(
            "verity-signer: expected PKCS#8 DER as a single lowercase hex line. Create it with:"
        );
        eprintln!("  {OPENSSL_GENPKEY_CMD}");
        exit(1);
    }

    let bytes = match hex::decode(trimmed) {
        Ok(b) => b,
        Err(e) => {
            eprintln!(
                "verity-signer: `{}` does not decode as hex ({e}); expected PKCS#8 DER encoded as lowercase hex.",
                path.display()
            );
            exit(1);
        },
    };

    match SigningKey::from_pkcs8_der(&bytes) {
        Ok(k) => k,
        Err(e) => {
            eprintln!("verity-signer: PKCS#8 parse failed for `{}`: {e}", path.display());
            eprintln!("verity-signer: ensure the file was produced with:");
            eprintln!("  {OPENSSL_GENPKEY_CMD}");
            exit(1);
        },
    }
}

#[derive(Serialize)]
struct PubKeyResponse {
    #[serde(rename = "publicKey")]
    public_key_hex: String,
}

async fn pubkey(State(signing_key): State<SigningKey>) -> Json<PubKeyResponse> {
    let der = match signing_key
        .verifying_key()
        .to_public_key_der()
    {
        Ok(d) => d,
        Err(e) => {
            eprintln!("verity-signer: internal error encoding public key DER: {e}");
            exit(1);
        },
    };
    Json(PubKeyResponse {
        public_key_hex: hex::encode(der.as_bytes()),
    })
}

async fn sign(State(signing_key): State<SigningKey>, body: Bytes) -> Response {
    let v: Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => {
            let err = serde_json::to_string(&json!({ "error": format!("invalid JSON body: {e}") }))
                .unwrap_or_else(|_| "{\"error\":\"invalid JSON body\"}".to_string());
            return (StatusCode::BAD_REQUEST, err).into_response();
        },
    };

    let message = match v
        .get("message")
        .and_then(Value::as_str)
    {
        Some(m) => m.to_string(),
        None => {
            return (
                StatusCode::BAD_REQUEST,
                serde_json::to_string(&json!({ "error": "missing or invalid 'message' field" }))
                    .unwrap_or_else(|_| "{\"error\":\"bad request\"}".to_string()),
            )
                .into_response();
        },
    };

    let signature = signing_key.sign(message.as_bytes());
    Json(serde_json::json!({ "signature": hex::encode(signature.to_bytes()) })).into_response()
}

#[tokio::main]
async fn main() {
    let key_path = key_file_from_env();
    let signing_key = load_key_or_exit(&key_path);

    let addr_str =
        std::env::var("VERITY_SIGNER_ADDR").unwrap_or_else(|_| DEFAULT_SIGNER_ADDR.to_string());
    let addr: SocketAddr = addr_str
        .parse()
        .unwrap_or_else(|e| {
            eprintln!("verity-signer: invalid VERITY_SIGNER_ADDR or default `{addr_str}`: {e}");
            exit(1);
        });
    match addr.ip() {
        std::net::IpAddr::V4(ip) if ip == Ipv4Addr::LOCALHOST => {},
        _ => {
            eprintln!(
                "verity-signer: listener must bind to 127.0.0.1 only (got {addr}); refusing to start."
            );
            exit(1);
        },
    }

    let app = Router::new()
        .route("/pubkey", get(pubkey))
        .route("/sign", post(sign))
        .with_state(signing_key);

    println!("verity-signer: listening on http://{addr} (key `{}`)", key_path.display());
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .unwrap_or_else(|e| {
            eprintln!("verity-signer: bind `{addr}` failed: {e}");
            exit(1);
        });

    axum::serve(listener, app)
        .await
        .unwrap_or_else(|e| {
            eprintln!("verity-signer: server failed: {e}");
            exit(1);
        });
}
