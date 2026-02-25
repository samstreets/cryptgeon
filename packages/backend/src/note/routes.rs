use axum::{
    extract::{ConnectInfo, Path},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc, time::SystemTime};
use tokio::sync::Mutex;
use tracing::info;

use crate::note::{generate_id, Note, NoteInfo};
use crate::store;
use crate::{config, lock::SharedState};

use super::NotePublic;

pub fn now() -> u32 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs() as u32
}

/// Extracts the real client IP. Prefers the CF-Connecting-IP header set by
/// Cloudflare tunnels, falling back to the direct socket address.
fn client_ip(headers: &HeaderMap, addr: &SocketAddr) -> String {
    headers
        .get("CF-Connecting-IP")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| addr.ip().to_string())
}

#[derive(Deserialize)]
pub struct OneNoteParams {
    id: String,
}

pub async fn preview(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(OneNoteParams { id }): Path<OneNoteParams>,
) -> Response {
    let ip = client_ip(&headers, &addr);
    let note = store::get(&id);

    match note {
        Ok(Some(n)) => {
            info!(action = "preview", note_id = %id, client_ip = %ip, "note preview requested");
            (StatusCode::OK, Json(NoteInfo { meta: n.meta })).into_response()
        }
        Ok(None) => {
            info!(action = "preview_not_found", note_id = %id, client_ip = %ip, "note preview — not found or already expired");
            (StatusCode::NOT_FOUND).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[derive(Serialize, Deserialize)]
struct CreateResponse {
    id: String,
}

pub async fn create(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(mut n): Json<Note>,
) -> Response {
    let ip = client_ip(&headers, &addr);
    let id = generate_id();

    if n.views == None && n.expiration == None {
        return (
            StatusCode::BAD_REQUEST,
            "At least views or expiration must be set",
        )
            .into_response();
    }
    if !*config::ALLOW_ADVANCED {
        n.views = Some(1);
        n.expiration = None;
    }
    match n.views {
        Some(v) => {
            if v > *config::MAX_VIEWS || v < 1 {
                return (StatusCode::BAD_REQUEST, "Invalid views").into_response();
            }
            n.expiration = None; // views overrides expiration
        }
        _ => {}
    }
    match n.expiration {
        Some(e) => {
            if e > *config::MAX_EXPIRATION || e < 1 {
                return (StatusCode::BAD_REQUEST, "Invalid expiration").into_response();
            }
            let expiration = now() + (e * 60);
            n.expiration = Some(expiration);
        }
        _ => {}
    }

    let note_type = n.meta.parse::<serde_json::Value>()
        .ok()
        .and_then(|v| v.get("type").and_then(|t| t.as_str()).map(String::from))
        .unwrap_or_else(|| "unknown".to_string());

    match store::set(&id.clone(), &n.clone()) {
        Ok(_) => {
            info!(
                action = "create",
                note_id = %id,
                client_ip = %ip,
                note_type = %note_type,
                views = ?n.views,
                expiration = ?n.expiration,
                "note created"
            );
            (StatusCode::OK, Json(CreateResponse { id })).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn delete(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(OneNoteParams { id }): Path<OneNoteParams>,
    state: axum::extract::State<SharedState>,
) -> Response {
    let ip = client_ip(&headers, &addr);

    let mut locks_map = state.locks.lock().await;
    let lock = locks_map
        .entry(id.clone())
        .or_insert_with(|| Arc::new(Mutex::new(())))
        .clone();
    drop(locks_map);
    let _guard = lock.lock().await;

    let note = store::get(&id);
    match note {
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        Ok(None) => {
            info!(action = "read_not_found", note_id = %id, client_ip = %ip, "note read — not found or already deleted");
            (StatusCode::NOT_FOUND).into_response()
        }
        Ok(Some(note)) => {
            let mut changed = note.clone();
            if changed.views == None && changed.expiration == None {
                return (StatusCode::BAD_REQUEST).into_response();
            }
            match changed.views {
                Some(v) => {
                    changed.views = Some(v - 1);
                    let id = id.clone();
                    if v <= 1 {
                        match store::del(&id) {
                            Err(e) => {
                                return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
                                    .into_response();
                            }
                            _ => {
                                info!(action = "read_and_destroy", note_id = %id, client_ip = %ip, remaining_views = 0, "note read and destroyed (view limit reached)");
                            }
                        }
                    } else {
                        match store::set(&id, &changed.clone()) {
                            Err(e) => {
                                return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
                                    .into_response();
                            }
                            _ => {
                                info!(action = "read", note_id = %id, client_ip = %ip, remaining_views = v - 1, "note read");
                            }
                        }
                    }
                }
                _ => {}
            }

            let n = now();
            match changed.expiration {
                Some(e) => {
                    if e < n {
                        match store::del(&id.clone()) {
                            Ok(_) => {
                                info!(action = "read_expired", note_id = %id, client_ip = %ip, "note read — expired and destroyed");
                                return (StatusCode::BAD_REQUEST).into_response();
                            }
                            Err(e) => {
                                return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
                                    .into_response()
                            }
                        }
                    }
                }
                _ => {}
            }

            return (
                StatusCode::OK,
                Json(NotePublic {
                    contents: changed.contents,
                    meta: changed.meta,
                }),
            )
                .into_response();
        }
    }
}
