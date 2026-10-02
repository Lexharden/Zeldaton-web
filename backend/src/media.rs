//! Item artwork: the pictures the catalog points at by file name.
//!
//! Two folders feed one namespace: `ART_DIR` (the images shipped with the site) and
//! `UPLOADS_DIR/items` (what organizers upload from the panel). An upload with the same name as a
//! shipped image shadows it; deleting the upload brings the original back. A catalog item stores
//! just the file name (`Hookshot-Art.png`) and the site loads `/api/media/items/<name>`.

use std::path::PathBuf;

use axum::body::Bytes;
use axum::extract::{Extension, Path, Query, State};
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE, HeaderName};
use axum::http::{HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::{Json, http::HeaderMap};
use chrono::Utc;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::accounts::Role;
use crate::admin::Principal;
use crate::error::ApiError;
use crate::hub::AppState;

/// Largest accepted upload. The panel shrinks pictures in the browser, so real files are far smaller.
pub const MAX_BYTES: usize = 2 * 1024 * 1024;

/// A plain file name with a picture extension: letters, digits and `_ - . ' ( )` or spaces. No
/// folders, no hidden files, no `..`.
pub fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.chars().count() <= 120
        && !name.starts_with('.')
        && !name.contains("..")
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '.' | '\'' | '(' | ')' | ' '))
        && content_type(name).is_some()
}

fn extension(name: &str) -> Option<String> {
    name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase())
}

fn content_type(name: &str) -> Option<&'static str> {
    match extension(name)?.as_str() {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "webp" => Some("image/webp"),
        _ => None,
    }
}

/// The bytes really are the picture the extension claims (never trust the name or the header).
fn matches_extension(name: &str, bytes: &[u8]) -> bool {
    match content_type(name) {
        Some("image/png") => bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]),
        Some("image/jpeg") => bytes.starts_with(&[0xFF, 0xD8, 0xFF]),
        Some("image/webp") => {
            bytes.len() > 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP"
        }
        _ => false,
    }
}

fn uploads_items(hub: &AppState) -> PathBuf {
    PathBuf::from(&hub.cfg.uploads_dir).join("items")
}

fn art_items(hub: &AppState) -> PathBuf {
    PathBuf::from(&hub.cfg.art_dir)
}

/// `GET /api/media/items/{file}`: public. The upload wins over the shipped image.
pub async fn serve(
    State(hub): State<AppState>,
    Path(file): Path<String>,
) -> Result<Response, ApiError> {
    if !valid_name(&file) {
        return Err(ApiError::NotFound);
    }
    let ctype = content_type(&file).ok_or(ApiError::NotFound)?;
    let bytes = match tokio::fs::read(uploads_items(&hub).join(&file)).await {
        Ok(b) => b,
        Err(_) => tokio::fs::read(art_items(&hub).join(&file))
            .await
            .map_err(|_| ApiError::NotFound)?,
    };
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static(ctype));
    headers.insert(
        CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=300"),
    );
    headers.insert(
        HeaderName::from_static("x-content-type-options"),
        HeaderValue::from_static("nosniff"),
    );
    Ok((headers, bytes).into_response())
}

async fn names_in(dir: PathBuf) -> Vec<(String, u64)> {
    let mut out = Vec::new();
    let Ok(mut rd) = tokio::fs::read_dir(dir).await else {
        return out;
    };
    while let Ok(Some(entry)) = rd.next_entry().await {
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        if !valid_name(&name) {
            continue;
        }
        if let Ok(meta) = entry.metadata().await
            && meta.is_file()
        {
            out.push((name, meta.len()));
        }
    }
    out
}

/// `GET /api/admin/media/items`: every picture a catalog item can use.
pub async fn list(State(hub): State<AppState>) -> Json<Vec<Value>> {
    let uploaded = names_in(uploads_items(&hub)).await;
    let mut all: Vec<Value> = uploaded
        .iter()
        .map(|(n, b)| json!({ "name": n, "bytes": b, "uploaded": true }))
        .collect();
    for (name, bytes) in names_in(art_items(&hub)).await {
        if !uploaded.iter().any(|(n, _)| n == &name) {
            all.push(json!({ "name": name, "bytes": bytes, "uploaded": false }));
        }
    }
    all.sort_by_key(|v| v["name"].as_str().unwrap_or("").to_lowercase());
    Json(all)
}

#[derive(Deserialize)]
pub struct UploadQuery {
    name: String,
}

/// `POST /api/admin/media/items?name=<file>`: the body is the picture itself.
pub async fn upload(
    State(hub): State<AppState>,
    Extension(p): Extension<Principal>,
    Query(q): Query<UploadQuery>,
    body: Bytes,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    p.require(Role::Admin)?;
    if !valid_name(&q.name) {
        return Err(ApiError::BadRequest(
            "file name: letters, digits, spaces and _ - . ' ( ), ending in .png, .jpg or .webp"
                .into(),
        ));
    }
    if body.is_empty() || body.len() > MAX_BYTES {
        return Err(ApiError::BadRequest(format!(
            "the picture must be between 1 byte and {} MB",
            MAX_BYTES / 1024 / 1024
        )));
    }
    if !matches_extension(&q.name, &body) {
        return Err(ApiError::BadRequest(
            "the file is not a real PNG, JPEG or WebP picture".into(),
        ));
    }
    let dir = uploads_items(&hub);
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(ApiError::internal)?;
    // Write beside the target and rename, so a reader never sees half a file.
    let tmp = dir.join(format!(".{}.tmp", Utc::now().timestamp_millis()));
    tokio::fs::write(&tmp, &body)
        .await
        .map_err(ApiError::internal)?;
    tokio::fs::rename(&tmp, dir.join(&q.name))
        .await
        .map_err(ApiError::internal)?;
    hub.audit_as(
        &p.actor,
        "media.item.upload",
        None,
        json!({ "name": q.name, "bytes": body.len() }),
    );
    Ok((
        StatusCode::CREATED,
        Json(json!({ "name": q.name, "bytes": body.len(), "uploaded": true })),
    ))
}

/// `DELETE /api/admin/media/items/{file}`: removes an upload (never a shipped image).
pub async fn delete(
    State(hub): State<AppState>,
    Extension(p): Extension<Principal>,
    Path(file): Path<String>,
) -> Result<StatusCode, ApiError> {
    p.require(Role::Admin)?;
    if !valid_name(&file) {
        return Err(ApiError::NotFound);
    }
    tokio::fs::remove_file(uploads_items(&hub).join(&file))
        .await
        .map_err(|_| ApiError::NotFound)?;
    hub.audit_as(&p.actor, "media.item.delete", None, json!({ "name": file }));
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_plain_picture_files() {
        for ok in [
            "Hookshot-Art.png",
            "Goron's_Ruby_-_NP_OOT_Player's_Guide.png",
            "a b (1).WEBP",
        ] {
            assert!(valid_name(ok), "{ok}");
        }
        for bad in [
            "",
            ".hidden.png",
            "../x.png",
            "a/b.png",
            "a\\b.png",
            "x.svg",
            "x.png.exe",
            "noext",
            "<script>.png",
        ] {
            assert!(!valid_name(bad), "{bad}");
        }
    }

    #[test]
    fn content_must_match_the_extension() {
        let png = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0];
        assert!(matches_extension("a.png", &png));
        assert!(!matches_extension("a.jpg", &png));
        assert!(!matches_extension("a.png", b"<svg></svg>"));
        assert!(matches_extension("a.jpeg", &[0xFF, 0xD8, 0xFF, 0xE0]));
        assert!(matches_extension("a.webp", b"RIFF\0\0\0\0WEBPVP8 "));
    }
}
