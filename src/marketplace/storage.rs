/// Plugin Marketplace Storage Layer (S3/GCS)

use crate::error::{Error, Result};
use base64::{Engine as _, engine::general_purpose};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;

static STORAGE: OnceCell<Arc<dyn StorageBackend>> = OnceCell::new();

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub backend: StorageType,
    pub bucket: String,
    pub region: Option<String>,
    pub endpoint: Option<String>,
    pub access_key: Option<String>,
    pub secret_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StorageType {
    S3,
    GCS,
    Local,
}

/// Storage backend trait
#[async_trait::async_trait]
pub trait StorageBackend: Send + Sync {
    /// Upload a plugin tarball
    async fn upload(&self, key: &str, data: &[u8]) -> Result<String>;

    /// Download a plugin tarball
    async fn download(&self, key: &str) -> Result<Vec<u8>>;

    /// Delete a plugin tarball
    async fn delete(&self, key: &str) -> Result<()>;

    /// Get download URL
    fn get_url(&self, key: &str) -> String;
}

/// Initialize storage backend
pub async fn init_storage(config: StorageConfig) -> Result<()> {
    let backend: Arc<dyn StorageBackend> = match config.backend {
        StorageType::S3 => Arc::new(S3Storage::new(config)?),
        StorageType::GCS => Arc::new(GCSStorage::new(config)?),
        StorageType::Local => Arc::new(LocalStorage::new(config)?),
    };

    STORAGE
        .set(backend)
        .map_err(|_| Error::Config("Storage backend already initialized".to_string()))?;

    tracing::info!("Storage backend initialized successfully");
    Ok(())
}

/// Get storage backend
fn get_storage() -> Result<&'static Arc<dyn StorageBackend>> {
    STORAGE
        .get()
        .ok_or_else(|| Error::Config("Storage backend not initialized".to_string()))
}

/// Upload plugin tarball and return URL + checksum
pub async fn upload_plugin(name: &str, version: &str, tarball_base64: &str) -> Result<(String, String)> {
    let storage = get_storage()?;

    // Decode base64
    let tarball_data = general_purpose::STANDARD
        .decode(tarball_base64)
        .map_err(|e| Error::Config(format!("Invalid base64 tarball: {}", e)))?;

    // Calculate SHA-256 checksum
    let mut hasher = Sha256::new();
    hasher.update(&tarball_data);
    let checksum = format!("{:x}", hasher.finalize());

    // Generate storage key: plugins/{name}/{version}/{name}-{version}.tar.gz
    let key = format!("plugins/{}/{}/{}-{}.tar.gz", name, version, name, version);

    // Upload to storage
    let url = storage.upload(&key, &tarball_data).await?;

    tracing::info!("Uploaded plugin {} v{} to {}", name, version, url);

    Ok((url, checksum))
}

/// Download plugin tarball
pub async fn download_plugin(name: &str, version: &str) -> Result<Vec<u8>> {
    let storage = get_storage()?;

    let key = format!("plugins/{}/{}/{}-{}.tar.gz", name, version, name, version);
    storage.download(&key).await
}

// ============================================================================
// S3 Storage Implementation
// ============================================================================

struct S3Storage {
    bucket: String,
    region: String,
    endpoint: Option<String>,
}

impl S3Storage {
    fn new(config: StorageConfig) -> Result<Self> {
        Ok(Self {
            bucket: config.bucket,
            region: config.region.unwrap_or_else(|| "us-east-1".to_string()),
            endpoint: config.endpoint,
        })
    }
}

#[async_trait::async_trait]
impl StorageBackend for S3Storage {
    async fn upload(&self, key: &str, data: &[u8]) -> Result<String> {
        // In a real implementation, use aws_sdk_s3 to upload
        // For now, return a placeholder URL
        tracing::warn!("S3 upload not fully implemented, returning placeholder URL");

        let url = if let Some(ref endpoint) = self.endpoint {
            format!("{}/{}/{}", endpoint, self.bucket, key)
        } else {
            format!(
                "https://{}.s3.{}.amazonaws.com/{}",
                self.bucket, self.region, key
            )
        };

        // TODO: Implement actual S3 upload using aws_sdk_s3:
        // let client = aws_sdk_s3::Client::new(&config);
        // client.put_object()
        //     .bucket(&self.bucket)
        //     .key(key)
        //     .body(data.into())
        //     .send()
        //     .await?;

        Ok(url)
    }

    async fn download(&self, key: &str) -> Result<Vec<u8>> {
        // TODO: Implement actual S3 download using aws_sdk_s3
        Err(Error::Config("S3 download not yet implemented".to_string()))
    }

    async fn delete(&self, key: &str) -> Result<()> {
        // TODO: Implement actual S3 delete using aws_sdk_s3
        Ok(())
    }

    fn get_url(&self, key: &str) -> String {
        format!(
            "https://{}.s3.{}.amazonaws.com/{}",
            self.bucket, self.region, key
        )
    }
}

// ============================================================================
// GCS Storage Implementation
// ============================================================================

struct GCSStorage {
    bucket: String,
}

impl GCSStorage {
    fn new(config: StorageConfig) -> Result<Self> {
        Ok(Self {
            bucket: config.bucket,
        })
    }
}

#[async_trait::async_trait]
impl StorageBackend for GCSStorage {
    async fn upload(&self, key: &str, data: &[u8]) -> Result<String> {
        // TODO: Implement GCS upload using google-cloud-storage crate
        tracing::warn!("GCS upload not fully implemented, returning placeholder URL");

        let url = format!("https://storage.googleapis.com/{}/{}", self.bucket, key);
        Ok(url)
    }

    async fn download(&self, key: &str) -> Result<Vec<u8>> {
        Err(Error::Config("GCS download not yet implemented".to_string()))
    }

    async fn delete(&self, key: &str) -> Result<()> {
        Ok(())
    }

    fn get_url(&self, key: &str) -> String {
        format!("https://storage.googleapis.com/{}/{}", self.bucket, key)
    }
}

// ============================================================================
// Local Storage Implementation (for development)
// ============================================================================

struct LocalStorage {
    base_path: std::path::PathBuf,
    base_url: String,
}

impl LocalStorage {
    fn new(config: StorageConfig) -> Result<Self> {
        let base_path = std::path::PathBuf::from(config.bucket);
        std::fs::create_dir_all(&base_path)
            .map_err(|e| Error::Config(format!("Failed to create storage directory: {}", e)))?;

        let base_url = config
            .endpoint
            .unwrap_or_else(|| "http://localhost:8080/storage".to_string());

        Ok(Self { base_path, base_url })
    }
}

#[async_trait::async_trait]
impl StorageBackend for LocalStorage {
    async fn upload(&self, key: &str, data: &[u8]) -> Result<String> {
        let file_path = self.base_path.join(key);

        // Create parent directories
        if let Some(parent) = file_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| Error::Config(format!("Failed to create directories: {}", e)))?;
        }

        // Write file
        tokio::fs::write(&file_path, data)
            .await
            .map_err(|e| Error::Config(format!("Failed to write file: {}", e)))?;

        let url = format!("{}/{}", self.base_url, key);
        tracing::info!("Uploaded plugin to local storage: {}", file_path.display());

        Ok(url)
    }

    async fn download(&self, key: &str) -> Result<Vec<u8>> {
        let file_path = self.base_path.join(key);

        tokio::fs::read(&file_path)
            .await
            .map_err(|e| Error::Config(format!("Failed to read file: {}", e)))
    }

    async fn delete(&self, key: &str) -> Result<()> {
        let file_path = self.base_path.join(key);

        tokio::fs::remove_file(&file_path)
            .await
            .map_err(|e| Error::Config(format!("Failed to delete file: {}", e)))
    }

    fn get_url(&self, key: &str) -> String {
        format!("{}/{}", self.base_url, key)
    }
}
