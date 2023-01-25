use anyhow::*;
use core::result::Result::Ok;
use std::env;
use log::{debug, error};
use pod_core::event::{AppEvent, EventSender, NotificationEvent, SenderExt};

async fn get_latest_release(platform: &str) -> Result<String> {
    let releases = reqwest::get("https://arteme.github.io/pod-ui/static/latest.txt").await?
        .text().await?;

    let lines = releases.split("\n");
    let tag = lines.flat_map(|line| {
        let cols = line.split(" ").collect::<Vec<_>>();
        if cols.get(0) == Some(&platform) {
            cols.get(1).map(|t| t.to_string())
        } else {
            None
        }
    }).next();

    if let Some(tag) = tag {
        Ok(tag)
    } else {
        bail!("Platform {:?} not found from latest releases list", platform)
    }
}

fn tag_to_ver(tag: &str) -> Option<semver::Version> {
    let v = if tag.starts_with("v") {
        tag.chars().skip(1).collect::<String>()
    } else {
        tag.to_string()
    };

    semver::Version::parse(v.as_str()).ok()
}

fn current_tag() -> String {
    env::var("DEBUG_GIT_TAG").ok()
        .or(option_env!("GIT_TAG").map(|s| s.into()))
        .unwrap_or("".into())
}

fn current_platform() -> String {
    env::var("DEBUG_RELEASE_PLATFORM").ok()
        .or(option_env!("RELEASE_PLATFORM").map(|s| s.into()))
        .unwrap_or("".into())
}

pub fn new_release_check(app_event_tx: &EventSender) {
    if option_env!("RELEASE_CHECK").is_none() {
        debug!("Release check skipped");
        return;
    }

    let tag = current_tag();
    debug!("Release check: current tag {:?}", tag);

    let app_event_tx = app_event_tx.clone();

    tokio::spawn(async move {
        let rel = match get_latest_release(current_platform().as_str()).await {
            Ok(v) => { v }
            Err(e) => {
                error!("Error in latest release check: {}", e);
                return;
            }
        };
        let latest = tag_to_ver(rel.as_str());
        let current = tag_to_ver(tag.as_str());
        let new_version_available = match (&current, &latest) {
            (Some(c), Some(l)) => { l > c }
            _ => { false }
        };

        if new_version_available {
            let msg = format!("New release <b>{}</b> is available!", rel);
            let e = NotificationEvent { msg, id: None };
            app_event_tx.send_or_warn(AppEvent::Notification(e));
        }
    });
}
