/// Plugin Marketplace Database Layer

use crate::error::{Error, Result};
use crate::marketplace::types::*;
use once_cell::sync::OnceCell;
use sqlx::{PgPool, Row};
use std::sync::Arc;

static DB_POOL: OnceCell<Arc<PgPool>> = OnceCell::new();

/// Initialize database connection pool
pub async fn init_db_pool(database_url: &str) -> Result<()> {
    let pool = PgPool::connect(database_url)
        .await
        .map_err(|e| Error::Config(format!("Failed to connect to database: {}", e)))?;

    DB_POOL
        .set(Arc::new(pool))
        .map_err(|_| Error::Config("Database pool already initialized".to_string()))?;

    tracing::info!("Database connection pool initialized");
    Ok(())
}

/// Get database pool
fn get_pool() -> Result<&'static Arc<PgPool>> {
    DB_POOL
        .get()
        .ok_or_else(|| Error::Config("Database pool not initialized".to_string()))
}

/// Database schema migrations
pub const SCHEMA_SQL: &str = r#"
-- Plugins table
CREATE TABLE IF NOT EXISTS plugins (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(255) UNIQUE NOT NULL,
    latest_version VARCHAR(50) NOT NULL,
    author VARCHAR(255) NOT NULL,
    description TEXT NOT NULL,
    homepage VARCHAR(512),
    repository VARCHAR(512),
    license VARCHAR(100) NOT NULL,
    downloads BIGINT DEFAULT 0,
    rating REAL,
    rating_count INT DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Plugin versions table
CREATE TABLE IF NOT EXISTS plugin_versions (
    id BIGSERIAL PRIMARY KEY,
    plugin_id BIGINT NOT NULL REFERENCES plugins(id) ON DELETE CASCADE,
    version VARCHAR(50) NOT NULL,
    tarball_url VARCHAR(512) NOT NULL,
    checksum_sha256 CHAR(64) NOT NULL,
    manifest JSONB NOT NULL,
    published_at TIMESTAMPTZ DEFAULT NOW(),
    yanked BOOLEAN DEFAULT FALSE,
    UNIQUE(plugin_id, version)
);

-- Plugin ratings table
CREATE TABLE IF NOT EXISTS plugin_ratings (
    id BIGSERIAL PRIMARY KEY,
    plugin_id BIGINT NOT NULL REFERENCES plugins(id) ON DELETE CASCADE,
    user_id VARCHAR(255) NOT NULL,
    rating INT CHECK (rating >= 1 AND rating <= 5),
    review TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(plugin_id, user_id)
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_plugins_name ON plugins(name);
CREATE INDEX IF NOT EXISTS idx_plugins_downloads ON plugins(downloads DESC);
CREATE INDEX IF NOT EXISTS idx_plugins_rating ON plugins(rating DESC NULLS LAST);
CREATE INDEX IF NOT EXISTS idx_plugin_versions_plugin_id ON plugin_versions(plugin_id);
CREATE INDEX IF NOT EXISTS idx_plugin_ratings_plugin_id ON plugin_ratings(plugin_id);

-- Full-text search index
CREATE INDEX IF NOT EXISTS idx_plugins_search ON plugins
    USING GIN(to_tsvector('english', name || ' ' || description));
"#;

/// Run database migrations
pub async fn run_migrations() -> Result<()> {
    let pool = get_pool()?;

    sqlx::query(SCHEMA_SQL)
        .execute(pool.as_ref())
        .await
        .map_err(|e| Error::Config(format!("Failed to run migrations: {}", e)))?;

    tracing::info!("Database migrations completed successfully");
    Ok(())
}

/// Insert a new plugin
pub async fn insert_plugin(plugin: &PluginManifest) -> Result<i64> {
    let pool = get_pool()?;

    let row = sqlx::query(
        r#"
        INSERT INTO plugins (name, latest_version, author, description, homepage, repository, license)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING id
        "#,
    )
    .bind(&plugin.name)
    .bind(&plugin.version)
    .bind(&plugin.author)
    .bind(&plugin.description)
    .bind(&plugin.homepage)
    .bind(&plugin.repository)
    .bind(&plugin.license)
    .fetch_one(pool.as_ref())
    .await
    .map_err(|e| Error::Config(format!("Failed to insert plugin: {}", e)))?;

    let id: i64 = row.get("id");
    Ok(id)
}

/// Insert a new plugin version
pub async fn insert_plugin_version(
    plugin_id: i64,
    version: &str,
    tarball_url: &str,
    checksum: &str,
    manifest: &PluginManifest,
) -> Result<i64> {
    let pool = get_pool()?;

    let manifest_json = serde_json::to_value(manifest)
        .map_err(|e| Error::Config(format!("Failed to serialize manifest: {}", e)))?;

    let row = sqlx::query(
        r#"
        INSERT INTO plugin_versions (plugin_id, version, tarball_url, checksum_sha256, manifest)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id
        "#,
    )
    .bind(plugin_id)
    .bind(version)
    .bind(tarball_url)
    .bind(checksum)
    .bind(manifest_json)
    .fetch_one(pool.as_ref())
    .await
    .map_err(|e| Error::Config(format!("Failed to insert plugin version: {}", e)))?;

    let id: i64 = row.get("id");
    Ok(id)
}

/// Get plugin by name
pub async fn get_plugin_by_name(name: &str) -> Result<Option<Plugin>> {
    let pool = get_pool()?;

    let row = sqlx::query_as::<_, Plugin>(
        r#"
        SELECT id, name, latest_version, author, description, homepage, repository,
               license, downloads, rating, created_at, updated_at
        FROM plugins
        WHERE name = $1
        "#,
    )
    .bind(name)
    .fetch_optional(pool.as_ref())
    .await
    .map_err(|e| Error::Config(format!("Failed to fetch plugin: {}", e)))?;

    Ok(row)
}

/// Search plugins with full-text search
pub async fn search_plugins(query: &PluginSearchQuery) -> Result<PluginListResponse> {
    let pool = get_pool()?;

    let page = query.page.unwrap_or(1);
    let per_page = query.per_page.unwrap_or(20).min(100); // Max 100 per page
    let offset = (page - 1) * per_page;

    let mut sql = String::from(
        r#"
        SELECT id, name, latest_version, author, description, downloads, rating
        FROM plugins
        WHERE 1=1
        "#,
    );

    // Add search filter
    if let Some(ref search_query) = query.query {
        sql.push_str(&format!(
            " AND to_tsvector('english', name || ' ' || description) @@ plainto_tsquery('english', '{}')",
            search_query
        ));
    }

    // Add sorting
    let sort_clause = match query.sort_by {
        Some(SortBy::Downloads) => "downloads DESC",
        Some(SortBy::Rating) => "rating DESC NULLS LAST",
        Some(SortBy::RecentlyUpdated) => "updated_at DESC",
        Some(SortBy::Name) | None => "name ASC",
    };

    sql.push_str(&format!(" ORDER BY {} LIMIT {} OFFSET {}", sort_clause, per_page, offset));

    let rows = sqlx::query(&sql)
        .fetch_all(pool.as_ref())
        .await
        .map_err(|e| Error::Config(format!("Failed to search plugins: {}", e)))?;

    let plugins: Vec<PluginMetadata> = rows
        .iter()
        .map(|row| PluginMetadata {
            name: row.get("name"),
            version: row.get("latest_version"),
            author: row.get("author"),
            description: row.get("description"),
            downloads: row.get("downloads"),
            rating: row.get("rating"),
            capabilities: vec![], // Would need to parse from manifest
        })
        .collect();

    // Get total count
    let count_row = sqlx::query("SELECT COUNT(*) as count FROM plugins")
        .fetch_one(pool.as_ref())
        .await
        .map_err(|e| Error::Config(format!("Failed to count plugins: {}", e)))?;

    let total: i64 = count_row.get("count");

    Ok(PluginListResponse {
        plugins,
        total,
        page,
        per_page,
    })
}

/// Increment download count
pub async fn increment_downloads(plugin_id: i64) -> Result<()> {
    let pool = get_pool()?;

    sqlx::query("UPDATE plugins SET downloads = downloads + 1 WHERE id = $1")
        .bind(plugin_id)
        .execute(pool.as_ref())
        .await
        .map_err(|e| Error::Config(format!("Failed to increment downloads: {}", e)))?;

    Ok(())
}

/// Add or update plugin rating
pub async fn upsert_rating(plugin_id: i64, user_id: &str, rating: i32, review: Option<&str>) -> Result<()> {
    let pool = get_pool()?;

    // Upsert rating
    sqlx::query(
        r#"
        INSERT INTO plugin_ratings (plugin_id, user_id, rating, review)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (plugin_id, user_id)
        DO UPDATE SET rating = $3, review = $4, created_at = NOW()
        "#,
    )
    .bind(plugin_id)
    .bind(user_id)
    .bind(rating)
    .bind(review)
    .execute(pool.as_ref())
    .await
    .map_err(|e| Error::Config(format!("Failed to upsert rating: {}", e)))?;

    // Recalculate average rating
    let avg_row = sqlx::query(
        "SELECT AVG(rating) as avg_rating, COUNT(*) as count FROM plugin_ratings WHERE plugin_id = $1",
    )
    .bind(plugin_id)
    .fetch_one(pool.as_ref())
    .await
    .map_err(|e| Error::Config(format!("Failed to calculate average rating: {}", e)))?;

    let avg_rating: Option<f32> = avg_row.get("avg_rating");
    let rating_count: i64 = avg_row.get("count");

    sqlx::query("UPDATE plugins SET rating = $1, rating_count = $2 WHERE id = $3")
        .bind(avg_rating)
        .bind(rating_count as i32)
        .bind(plugin_id)
        .execute(pool.as_ref())
        .await
        .map_err(|e| Error::Config(format!("Failed to update plugin rating: {}", e)))?;

    Ok(())
}
