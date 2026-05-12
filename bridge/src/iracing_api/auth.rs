//! OAuth2 Authorization Code + PKCE flow for the iRacing Data API.
//!
//! Implemented directly with reqwest (no oauth2 crate) to avoid version-matching issues.
//!
//! Flow:
//! 1. `start_flow()` binds an ephemeral loopback server, builds the authorize URL.
//! 2. Caller opens the URL in the browser.
//! 3. iRacing redirects to `http://127.0.0.1:<port>/callback?code=…`.
//! 4. Loopback handler returns the code; caller exchanges it for tokens.

use std::collections::HashMap;
use std::future::IntoFuture;
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use axum::{extract::Query, response::Html, routing::get, Router};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::RngCore;
use reqwest::Client;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tokio::net::TcpListener;
use tokio::sync::{oneshot, Mutex};

use super::token_store::StoredTokens;

const AUTH_URL: &str = "https://oauth.iracing.com/oauth2/authorize";
const TOKEN_URL: &str = "https://oauth.iracing.com/oauth2/token";

// Public native-app client_id — replace with your registered client_id if needed.
const CLIENT_ID: &str = "iracing-pitwall";

fn generate_pkce() -> (String, String) {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    let verifier = URL_SAFE_NO_PAD.encode(&bytes);
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    (verifier, challenge)
}

fn generate_state() -> String {
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(&bytes)
}

/// Starts the OAuth2 PKCE flow. Returns the authorization URL to open in the browser
/// and a `JoinHandle` that resolves once the callback is received with the tokens.
pub async fn start_flow() -> Result<(String, tokio::task::JoinHandle<Result<StoredTokens>>)> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .context("bind oauth loopback")?;
    let port = listener.local_addr()?.port();
    let redirect_uri = format!("http://127.0.0.1:{port}/callback");

    let (verifier, challenge) = generate_pkce();
    let state_token = generate_state();

    let auth_url = format!(
        "{AUTH_URL}?response_type=code&client_id={CLIENT_ID}\
         &redirect_uri={redirect}&code_challenge={challenge}\
         &code_challenge_method=S256&state={state}",
        redirect = urlencoding::encode(&redirect_uri),
        challenge = challenge,
        state = state_token,
    );

    let (code_tx, code_rx) = oneshot::channel::<(String, String)>();
    let code_tx = Arc::new(Mutex::new(Some(code_tx)));

    #[derive(Deserialize)]
    struct CallbackParams {
        code: String,
        state: String,
    }

    let tx_clone = code_tx.clone();
    let callback_handler = move |Query(params): Query<CallbackParams>| {
        let tx = tx_clone.clone();
        async move {
            if let Some(sender) = tx.lock().await.take() {
                let _ = sender.send((params.code, params.state));
            }
            Html("<html><body><h2>iRacing Pitwall</h2><p>Authorization complete — you can close this tab.</p></body></html>")
        }
    };

    let app = Router::new().route("/callback", get(callback_handler));
    let csrf = state_token.clone();

    let handle = tokio::spawn(async move {
        let server = axum::serve(listener, app);
        let (code, state) = tokio::select! {
            result = code_rx => result.context("callback channel closed")?,
            _ = tokio::time::sleep(std::time::Duration::from_secs(120)) => {
                return Err(anyhow!("OAuth callback timed out"));
            }
            r = server.into_future() => {
                r.context("loopback server error")?;
                return Err(anyhow!("loopback server stopped before callback"));
            }
        };

        if state != csrf {
            return Err(anyhow!("OAuth CSRF mismatch"));
        }

        exchange_code(&code, &verifier, &redirect_uri).await
    });

    Ok((auth_url, handle))
}

async fn exchange_code(code: &str, verifier: &str, redirect_uri: &str) -> Result<StoredTokens> {
    let client = Client::builder().use_rustls_tls().build()?;
    let mut params = HashMap::new();
    params.insert("grant_type", "authorization_code");
    params.insert("code", code);
    params.insert("client_id", CLIENT_ID);
    params.insert("code_verifier", verifier);
    params.insert("redirect_uri", redirect_uri);

    exchange_tokens(&client, &params).await
}

/// Refresh an existing access token using the stored refresh token.
pub async fn refresh_access_token(refresh_token: &str) -> Result<StoredTokens> {
    let client = Client::builder().use_rustls_tls().build()?;
    let mut params = HashMap::new();
    params.insert("grant_type", "refresh_token");
    params.insert("refresh_token", refresh_token);
    params.insert("client_id", CLIENT_ID);

    exchange_tokens(&client, &params).await
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<u64>,
}

async fn exchange_tokens(client: &Client, params: &HashMap<&str, &str>) -> Result<StoredTokens> {
    let resp = client
        .post(TOKEN_URL)
        .form(params)
        .send()
        .await
        .context("token endpoint")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(anyhow!("token exchange failed {status}: {body}"));
    }

    let tr: TokenResponse = resp.json().await.context("parse token response")?;
    let expires_at = tr
        .expires_in
        .map(|secs| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|now| now.as_secs() as i64 + secs as i64)
                .unwrap_or(0)
        })
        .unwrap_or(0);

    Ok(StoredTokens {
        access_token: tr.access_token,
        refresh_token: tr.refresh_token,
        expires_at,
        cust_id: None,
        member_name: None,
    })
}
