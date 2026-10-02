//! Racer photos, uploaded from the organizer panel.
//!
//! Each racer has at most one photo, stored as `UPLOADS_DIR/racers/<racer-id>-<millis>.<ext>` and
//! served publicly at `/api/media/racers/<file>`. The name changes with every upload, so browsers
//! and proxies can cache a photo for a long time and still show the new one right away. The racer's
//! `avatarUrl` points at it. (Catalog item pictures are not uploaded: they are plain files in the
//! site's `public/art/items` folder, and an item names the file.)

use std::path::PathBuf;

use axum::Json;
use axum::body::Bytes;
use axum::extract::{Extension, Path, State};
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE, HeaderName};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use chrono::Utc;
use serde_json::json;

use crate::accounts::Role;
use crate::admin::Principal;
use crate::domain::Racer;
use crate::engine::RacerPatch;
use crate::error::ApiError;
use crate::hub::AppState;

/// Largest accepted upload. The panel shrinks photos in the browser, so real files are far smaller.
pub const MAX_BYTES: usize = 2 * 1024 * 1024;

/// The picture format the bytes really are (never trust a file name or a header).
fn sniff(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        Some("png")
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("jpg")
    } else if bytes.len() > 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some("webp")
    } else {
        None
    }
}

fn content_type(ext: &str) -> Option<&'static str> {
    match ext {
        "png" => Some("image/png"),
        "jpg" => Some("image/jpeg"),
        "webp" => Some("image/webp"),
        _ => None,
    }
}

/// `<racer-id>-<digits>.<png|jpg|webp>` and nothing else (no folders, no hidden files).
/// Returns the racer id and the extension.
fn parse_file(file: &str) -> Option<(&str, &str)> {
    let (stem, ext) = file.rsplit_once('.')?;
    content_type(ext)?;
    let (id, stamp) = stem.rsplit_once('-')?;
    let id_ok = !id.is_empty()
        && id.len() <= 40
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    (id_ok && !stamp.is_empty() && stamp.chars().all(|c| c.is_ascii_digit())).then_some((id, ext))
}

fn racers_dir(hub: &AppState) -> PathBuf {
    PathBuf::from(&hub.cfg.uploads_dir).join("racers")
}

/// `GET /api/media/racers/{file}`: public.
pub async fn serve(
    State(hub): State<AppState>,
    Path(file): Path<String>,
) -> Result<Response, ApiError> {
    let (_, ext) = parse_file(&file).ok_or(ApiError::NotFound)?;
    let ctype = content_type(ext).ok_or(ApiError::NotFound)?;
    let bytes = tokio::fs::read(racers_dir(&hub).join(&file))
        .await
        .map_err(|_| ApiError::NotFound)?;
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static(ctype));
    // The name changes with every upload, so a long cache is safe.
    headers.insert(
        CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=86400"),
    );
    headers.insert(
        HeaderName::from_static("x-content-type-options"),
        HeaderValue::from_static("nosniff"),
    );
    Ok((headers, bytes).into_response())
}

fn exists(hub: &AppState, id: &str) -> bool {
    hub.read(|s| s.idx(id).is_some())
}

/// Deletes every stored photo of `id`, except `keep`.
async fn remove_photos(hub: &AppState, id: &str, keep: Option<&str>) {
    let Ok(mut rd) = tokio::fs::read_dir(racers_dir(hub)).await else {
        return;
    };
    while let Ok(Some(entry)) = rd.next_entry().await {
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        if keep == Some(name.as_str()) {
            continue;
        }
        if parse_file(&name).is_some_and(|(owner, _)| owner == id) {
            let _ = tokio::fs::remove_file(entry.path()).await;
        }
    }
}

/// `POST /api/admin/racers/{id}/photo`: the body is the picture itself. Replaces any previous photo.
pub async fn upload_photo(
    State(hub): State<AppState>,
    Extension(p): Extension<Principal>,
    Path(id): Path<String>,
    body: Bytes,
) -> Result<(StatusCode, Json<Racer>), ApiError> {
    p.require(Role::Admin)?;
    if !exists(&hub, &id) {
        return Err(ApiError::NotFound);
    }
    if body.is_empty() || body.len() > MAX_BYTES {
        return Err(ApiError::BadRequest(format!(
            "the photo must be between 1 byte and {} MB",
            MAX_BYTES / 1024 / 1024
        )));
    }
    let ext = sniff(&body).ok_or_else(|| {
        ApiError::BadRequest("the file is not a real PNG, JPEG or WebP picture".into())
    })?;
    let dir = racers_dir(&hub);
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(ApiError::internal)?;
    let file = format!("{id}-{}.{ext}", Utc::now().timestamp_millis());
    // Write beside the target and rename, so a reader never sees half a file.
    let tmp = dir.join(format!(".{file}.tmp"));
    tokio::fs::write(&tmp, &body)
        .await
        .map_err(ApiError::internal)?;
    tokio::fs::rename(&tmp, dir.join(&file))
        .await
        .map_err(ApiError::internal)?;
    let patch = RacerPatch {
        avatar_url: Some(format!("/api/media/racers/{file}")),
        ..Default::default()
    };
    let racer = hub
        .update_racer(&id, patch, Utc::now())
        .map_err(ApiError::BadRequest)?;
    remove_photos(&hub, &id, Some(&file)).await;
    hub.audit_as(
        &p.actor,
        "racer.photo.upload",
        Some(&id),
        json!({ "bytes": body.len() }),
    );
    Ok((StatusCode::CREATED, Json(racer)))
}

/// `DELETE /api/admin/racers/{id}/photo`: back to the generated placeholder.
pub async fn delete_photo(
    State(hub): State<AppState>,
    Extension(p): Extension<Principal>,
    Path(id): Path<String>,
) -> Result<Json<Racer>, ApiError> {
    p.require(Role::Admin)?;
    if !exists(&hub, &id) {
        return Err(ApiError::NotFound);
    }
    let patch = RacerPatch {
        avatar_url: Some(String::new()),
        ..Default::default()
    };
    let racer = hub
        .update_racer(&id, patch, Utc::now())
        .map_err(ApiError::BadRequest)?;
    remove_photos(&hub, &id, None).await;
    hub.audit_as(&p.actor, "racer.photo.delete", Some(&id), json!({}));
    Ok(Json(racer))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_names_are_racer_photos_and_nothing_else() {
        assert_eq!(
            parse_file("ralbat-1760000000000.png"),
            Some(("ralbat", "png"))
        );
        assert_eq!(parse_file("ana-b-5.webp"), Some(("ana-b", "webp")));
        for bad in [
            "",
            ".x-1.png",
            "../x-1.png",
            "a/b-1.png",
            "ralbat.png",
            "ralbat-x.png",
            "ralbat-1.svg",
            "Ralbat-1.png",
            "ralbat-1.png.exe",
        ] {
            assert_eq!(parse_file(bad), None, "{bad}");
        }
    }

    #[test]
    fn the_content_decides_the_format() {
        assert_eq!(
            sniff(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0]),
            Some("png")
        );
        assert_eq!(sniff(&[0xFF, 0xD8, 0xFF, 0xE0]), Some("jpg"));
        assert_eq!(sniff(b"RIFF\0\0\0\0WEBPVP8 "), Some("webp"));
        assert_eq!(sniff(b"<svg></svg>"), None);
    }
}
