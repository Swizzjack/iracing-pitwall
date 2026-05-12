//! HTTP client for the iRacing Data API.
//!
//! Handles:
//! - Bearer token auth (`Authorization: Bearer …`)
//! - Two-step S3 pre-signed URL resolution (`{link}` → payload)
//! - Lazy token refresh on 401
//! - Rate-limit header respect (429 / `Retry-After`)

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use reqwest::{Client, Response, StatusCode};
use tokio::sync::Mutex;

use super::models::{DataApiLink, MemberInfo, MemberRecentRaces, SubsessionResult};
use super::token_store::{self, StoredTokens};

const BASE_URL: &str = "https://members-ng.iracing.com";

#[derive(Clone)]
pub struct ApiClient {
    http: Client,
    tokens: Arc<Mutex<Option<StoredTokens>>>,
    token_path: PathBuf,
}

impl ApiClient {
    pub fn new(token_path: PathBuf) -> Result<Self> {
        let http = Client::builder()
            .use_rustls_tls()
            .gzip(true)
            .timeout(Duration::from_secs(30))
            .build()
            .context("build reqwest client")?;
        let stored = token_store::load(&token_path);
        Ok(Self {
            http,
            tokens: Arc::new(Mutex::new(stored)),
            token_path,
        })
    }

    pub async fn is_linked(&self) -> bool {
        self.tokens.lock().await.is_some()
    }

    pub async fn get_linked_info(&self) -> (bool, Option<String>, Option<i64>) {
        let guard = self.tokens.lock().await;
        match &*guard {
            None => (false, None, None),
            Some(t) => (true, t.member_name.clone(), t.cust_id),
        }
    }

    pub async fn store_tokens(&self, mut tokens: StoredTokens) -> Result<()> {
        // Fetch cust_id + display_name on first link.
        if tokens.cust_id.is_none() {
            match self.fetch_member_info_with(&tokens.access_token).await {
                Ok((cust_id, name)) => {
                    tokens.cust_id = Some(cust_id);
                    tokens.member_name = Some(name);
                }
                Err(e) => log::warn!("could not fetch member info: {e}"),
            }
        }
        token_store::save(&self.token_path, &tokens)?;
        *self.tokens.lock().await = Some(tokens);
        Ok(())
    }

    /// Fetch the subsession result for the given `sub_session_id`.
    pub async fn get_subsession_result(&self, sub_session_id: i64) -> Result<SubsessionResult> {
        let url = format!("{BASE_URL}/data/results/get?subsession_id={sub_session_id}&include_licenses=false");
        let json = self.get_data(&url).await?;
        serde_json::from_value(json).context("parse SubsessionResult")
    }

    /// Fetch the list of recent races for a cust_id.
    pub async fn get_recent_races(&self, cust_id: i64) -> Result<MemberRecentRaces> {
        let url = format!("{BASE_URL}/data/stats/member_recent_races?cust_id={cust_id}");
        let json = self.get_data(&url).await?;
        serde_json::from_value(json).context("parse MemberRecentRaces")
    }

    // ─── internal ─────────────────────────────────────────────────────────

    /// GET a Data API endpoint — returns the payload from the pre-signed S3 link.
    async fn get_data(&self, url: &str) -> Result<serde_json::Value> {
        let resp = self.authed_get(url).await?;
        let link_resp: DataApiLink = resp.json().await.context("parse link response")?;
        // S3 pre-signed URL — must NOT include an Authorization header.
        let payload = self
            .http
            .get(&link_resp.link)
            .send()
            .await
            .context("s3 fetch")?;
        self.check_rate_limit(&payload).await?;
        let json = payload.json::<serde_json::Value>().await.context("parse s3 payload")?;
        Ok(json)
    }

    /// Authenticated GET with automatic refresh-on-401.
    async fn authed_get(&self, url: &str) -> Result<Response> {
        let token = self.get_valid_token().await?;
        let resp = self
            .http
            .get(url)
            .bearer_auth(&token)
            .send()
            .await
            .context("http get")?;

        if resp.status() == StatusCode::UNAUTHORIZED {
            // Token may have just expired — refresh and retry once.
            log::info!("api: 401 received, refreshing token");
            let new_token = self.force_refresh().await?;
            let resp2 = self
                .http
                .get(url)
                .bearer_auth(&new_token)
                .send()
                .await
                .context("http get (retry)")?;
            self.check_rate_limit(&resp2).await?;
            if !resp2.status().is_success() {
                return Err(anyhow!("api error {} for {url}", resp2.status()));
            }
            return Ok(resp2);
        }

        self.check_rate_limit(&resp).await?;
        if !resp.status().is_success() {
            return Err(anyhow!("api error {} for {url}", resp.status()));
        }
        Ok(resp)
    }

    async fn get_valid_token(&self) -> Result<String> {
        let guard = self.tokens.lock().await;
        let tokens = guard.as_ref().ok_or_else(|| anyhow!("not linked to iRacing"))?;
        if !tokens.is_access_expired() {
            return Ok(tokens.access_token.clone());
        }
        drop(guard);
        self.force_refresh().await
    }

    async fn force_refresh(&self) -> Result<String> {
        let refresh_token = {
            let guard = self.tokens.lock().await;
            guard
                .as_ref()
                .and_then(|t| t.refresh_token.clone())
                .ok_or_else(|| anyhow!("no refresh token available"))?
        };

        let mut new_tokens = super::auth::refresh_access_token(&refresh_token).await?;

        // preserve cust_id / member_name from existing stored tokens
        {
            let guard = self.tokens.lock().await;
            if let Some(existing) = &*guard {
                if new_tokens.cust_id.is_none() {
                    new_tokens.cust_id = existing.cust_id;
                    new_tokens.member_name = existing.member_name.clone();
                }
            }
        }

        let access = new_tokens.access_token.clone();
        token_store::save(&self.token_path, &new_tokens)?;
        *self.tokens.lock().await = Some(new_tokens);
        log::info!("api: token refreshed");
        Ok(access)
    }

    async fn check_rate_limit(&self, resp: &Response) -> Result<()> {
        if resp.status() == StatusCode::TOO_MANY_REQUESTS {
            let wait = resp
                .headers()
                .get("Retry-After")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(60);
            log::warn!("api: rate limited, waiting {wait}s");
            tokio::time::sleep(Duration::from_secs(wait)).await;
            return Err(anyhow!("rate limited (429)"));
        }
        Ok(())
    }

    async fn fetch_member_info_with(&self, token: &str) -> Result<(i64, String)> {
        let url = format!("{BASE_URL}/data/member/get");
        let resp = self.http.get(&url).bearer_auth(token).send().await?;
        let link: DataApiLink = resp.json().await?;
        let payload = self.http.get(&link.link).send().await?;
        let info: MemberInfo = payload.json().await?;
        let member = info
            .members
            .and_then(|v| v.into_iter().next())
            .ok_or_else(|| anyhow!("empty member list"))?;
        Ok((member.cust_id, member.display_name))
    }
}
