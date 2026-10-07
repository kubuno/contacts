use anyhow::{Context, Result};
use bytes::Bytes;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub struct AvatarService {
    pub local_path: String,
    pub temp_path:  String,
    pub max_bytes:  u64,
}

impl AvatarService {
    pub fn new(local_path: &str, temp_path: &str, max_mb: u64) -> Self {
        Self {
            local_path: local_path.to_string(),
            temp_path:  temp_path.to_string(),
            max_bytes:  max_mb * 1024 * 1024,
        }
    }

    pub fn avatar_path(&self, owner_id: Uuid, contact_id: Uuid) -> PathBuf {
        Path::new(&self.local_path)
            .join(owner_id.to_string())
            .join(format!("{contact_id}.webp"))
    }

    pub fn avatar_url(&self, owner_id: Uuid, contact_id: Uuid) -> String {
        format!("/contacts/contacts/{contact_id}/avatar?owner={owner_id}")
    }

    pub async fn save_avatar(&self, owner_id: Uuid, contact_id: Uuid, data: Bytes) -> Result<String> {
        if data.len() as u64 > self.max_bytes {
            anyhow::bail!("Avatar trop volumineux (max {} MB)", self.max_bytes / 1024 / 1024);
        }

        // Decode and re-encode as WebP for consistency
        let img = image::load_from_memory(&data)
            .context("Format d'image non reconnu")?;
        let img = img.resize(400, 400, image::imageops::FilterType::Lanczos3);

        let dest = self.avatar_path(owner_id, contact_id);
        if let Some(parent) = dest.parent() {
            tokio::fs::create_dir_all(parent).await.context("Création répertoire avatars")?;
        }

        let mut buf = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::WebP)
            .context("Encodage WebP")?;

        tokio::fs::write(&dest, &buf).await.context("Écriture avatar")?;

        Ok(dest.to_string_lossy().to_string())
    }

    pub async fn read_avatar(&self, path: &str) -> Result<Bytes> {
        let data = tokio::fs::read(path).await.context("Lecture avatar")?;
        Ok(Bytes::from(data))
    }

    pub async fn delete_avatar(&self, owner_id: Uuid, contact_id: Uuid) -> Result<()> {
        let path = self.avatar_path(owner_id, contact_id);
        if path.exists() {
            tokio::fs::remove_file(&path).await.context("Suppression avatar")?;
        }
        Ok(())
    }
}
