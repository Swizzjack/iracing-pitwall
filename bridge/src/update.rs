//! GitHub Releases version check.
//!
//! Checks once at startup whether a newer `iracing-pitwall` release is available.
//! Any error (network down, 404 for private repo, parse failure) returns `None` silently.

use std::time::Duration;

use serde::Serialize;
use ts_rs::TS;

const REPO: &str = "Swizzjack/iracing-pitwall";

/// Returned when a newer release is found on GitHub.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../shared/")]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub latest_version: String,
    pub release_url: String,
}

/// Blocking HTTP call to the GitHub Releases API.
///
/// Returns `Some(UpdateInfo)` only when a *strictly newer* release is available.
/// All errors are silently swallowed so startup is never blocked.
///
/// For testing without modifying `Cargo.toml`, set the env var
/// `BRIDGE_VERSION_OVERRIDE` to a lower version (e.g. `0.1.0`).
pub fn check_for_update(current: &str) -> Option<UpdateInfo> {
    let url = format!(
        "https://api.github.com/repos/{}/releases/latest",
        REPO
    );

    log::info!("update check: querying GitHub (current: v{current})");

    let tls = match native_tls::TlsConnector::new() {
        Ok(t) => t,
        Err(e) => { log::warn!("update check: TLS init failed: {e}"); return None; }
    };

    let agent = ureq::AgentBuilder::new()
        .tls_connector(std::sync::Arc::new(tls))
        .timeout_connect(Duration::from_secs(5))
        .timeout_read(Duration::from_secs(10))
        .build();

    let body = match agent
        .get(&url)
        .set("User-Agent", "iracing-pitwall")
        .set("Accept", "application/vnd.github+json")
        .call()
    {
        Ok(resp) => match resp.into_string() {
            Ok(b) => b,
            Err(e) => { log::warn!("update check: failed to read response: {e}"); return None; }
        },
        Err(e) => { log::warn!("update check: HTTP request failed: {e}"); return None; }
    };

    let resp: serde_json::Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(e) => { log::warn!("update check: JSON parse error: {e}"); return None; }
    };

    let tag_name = match resp["tag_name"].as_str() {
        Some(t) => t,
        None => { log::warn!("update check: no tag_name in response"); return None; }
    };
    let release_url = match resp["html_url"].as_str() {
        Some(u) => u,
        None => { log::warn!("update check: no html_url in response"); return None; }
    };

    log::info!("update check: latest release is {tag_name}");

    if is_newer(tag_name, current) {
        Some(UpdateInfo {
            latest_version: tag_name.trim_start_matches('v').to_owned(),
            release_url: release_url.to_owned(),
        })
    } else {
        log::info!("update check: v{current} is already up to date");
        None
    }
}

/// Returns `true` when `remote` is strictly higher than `current`.
/// Both strings may carry an optional leading `v`.
fn is_newer(remote: &str, current: &str) -> bool {
    parse_ver(remote) > parse_ver(current)
}

fn parse_ver(v: &str) -> (u64, u64, u64) {
    let v = v.trim_start_matches('v');
    let mut parts = v.splitn(3, '.').map(|s| s.parse::<u64>().unwrap_or(0));
    let major = parts.next().unwrap_or(0);
    let minor = parts.next().unwrap_or(0);
    let patch = parts.next().unwrap_or(0);
    (major, minor, patch)
}

#[cfg(test)]
mod tests {
    use super::is_newer;

    #[test]
    fn same_version_is_not_newer() {
        assert!(!is_newer("v0.1.85", "0.1.85"));
        assert!(!is_newer("0.1.85", "v0.1.85"));
        assert!(!is_newer("0.1.85", "0.1.85"));
    }

    #[test]
    fn higher_patch_is_newer() {
        assert!(is_newer("v0.1.86", "v0.1.85"));
        assert!(is_newer("0.1.86", "0.1.85"));
    }

    #[test]
    fn lower_patch_is_not_newer() {
        assert!(!is_newer("v0.1.84", "v0.1.85"));
    }

    #[test]
    fn higher_minor_is_newer() {
        assert!(is_newer("v0.2.0", "v0.1.99"));
    }

    #[test]
    fn lower_minor_is_not_newer() {
        assert!(!is_newer("v0.1.0", "v0.2.0"));
    }

    #[test]
    fn higher_major_is_newer() {
        assert!(is_newer("v1.0.0", "v0.9.99"));
    }

    #[test]
    fn lower_major_is_not_newer() {
        assert!(!is_newer("v0.9.99", "v1.0.0"));
    }

    #[test]
    fn no_v_prefix_both() {
        assert!(is_newer("0.2.0", "0.1.99"));
    }
}
