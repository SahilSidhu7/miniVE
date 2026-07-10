use bollard::image::{ListImagesOptions, RemoveImageOptions};
use serde::Serialize;

use crate::state::AppState;

// camelCase so `ManageImages.svelte` can read `img.repoTag`/`sizeBytes`/`createdUnix`
// directly — plain serde field names would serialize as snake_case instead.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CachedImage {
    pub id: String,
    pub repo_tag: String,
    pub size_bytes: i64,
    pub created_unix: i64,
}

#[tauri::command]
pub async fn list_cached_images(state: tauri::State<'_, AppState>) -> Result<Vec<CachedImage>, String> {
    let images = state
        .docker
        .list_images(None::<ListImagesOptions<String>>)
        .await
        .map_err(|e| e.to_string())?;
    Ok(images
        .into_iter()
        .map(|img| CachedImage {
            id: img.id.clone(),
            repo_tag: img.repo_tags.first().cloned().unwrap_or(img.id),
            size_bytes: img.size,
            created_unix: img.created,
        })
        .collect())
}

#[tauri::command]
pub async fn remove_cached_image(state: tauri::State<'_, AppState>, id: String) -> Result<(), String> {
    state
        .docker
        .remove_image(&id, None::<RemoveImageOptions>, None)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}
