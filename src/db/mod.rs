mod auth;
mod proxy;
mod stats;
mod subscription;

use anyhow::Result;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::str::FromStr;
use tracing::info;

#[derive(Clone)]
pub struct Database {
    pub pool: SqlitePool,
}

impl Database {
    pub async fn new(url: &str) -> Result<Self> {
        let opts = SqliteConnectOptions::from_str(url)?
            .create_if_missing(true)
            .pragma("journal_mode", "WAL")
            .pragma("synchronous", "NORMAL")
            .pragma("cache_size", "-2000")
            .pragma("mmap_size", "0")
            .pragma("journal_size_limit", "67108864");

        let pool = SqlitePoolOptions::new()
            .max_connections(8)
            .min_connections(1)
            .idle_timeout(std::time::Duration::from_secs(60))
            .connect_with(opts)
            .await?;

        let db = Self { pool };
        db.run_migrations().await?;
        Ok(db)
    }

    async fn run_migrations(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS proxies (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                ip TEXT NOT NULL,
                port INTEGER NOT NULL,
                protocol TEXT NOT NULL DEFAULT 'http',
                anonymity TEXT NOT NULL DEFAULT 'unknown',
                country TEXT NOT NULL DEFAULT 'unknown',
                score REAL NOT NULL DEFAULT 0.0,
                is_alive INTEGER NOT NULL DEFAULT 0,
                success_count INTEGER NOT NULL DEFAULT 0,
                fail_count INTEGER NOT NULL DEFAULT 0,
                consecutive_fails INTEGER NOT NULL DEFAULT 0,
                avg_latency_ms REAL NOT NULL DEFAULT 0.0,
                last_check_at TEXT,
                last_success_at TEXT,
                next_check_at TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                source TEXT NOT NULL DEFAULT 'unknown',
                UNIQUE(ip, port, protocol)
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS check_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                proxy_id INTEGER NOT NULL,
                target TEXT NOT NULL,
                success INTEGER NOT NULL DEFAULT 0,
                latency_ms REAL,
                error TEXT,
                checked_at TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (proxy_id) REFERENCES proxies(id) ON DELETE CASCADE
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_proxies_alive ON proxies(is_alive);")
            .execute(&self.pool)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_proxies_score ON proxies(score DESC);")
            .execute(&self.pool)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_proxies_country ON proxies(country);")
            .execute(&self.pool)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_proxies_next_check ON proxies(next_check_at);")
            .execute(&self.pool)
            .await?;
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_check_logs_proxy ON check_logs(proxy_id, checked_at DESC);",
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_check_logs_checked_at ON check_logs(checked_at);",
        )
        .execute(&self.pool)
        .await?;

        // Subscription sources table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS subscription_sources (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                source_type TEXT NOT NULL DEFAULT 'url',
                url TEXT,
                content TEXT,
                protocol_hint TEXT NOT NULL DEFAULT 'auto',
                is_enabled INTEGER NOT NULL DEFAULT 1,
                sync_interval_secs INTEGER NOT NULL DEFAULT 21600,
                proxy_count INTEGER NOT NULL DEFAULT 0,
                last_sync_at TEXT,
                last_error TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Migration: add sync_interval_secs column if it doesn't exist
        let _ = sqlx::query(
            "ALTER TABLE subscription_sources ADD COLUMN sync_interval_secs INTEGER NOT NULL DEFAULT 21600",
        )
        .execute(&self.pool)
        .await;

        let _ = sqlx::query(
            "UPDATE subscription_sources SET sync_interval_secs = 21600 WHERE sync_interval_secs IS NULL OR sync_interval_secs = 0",
        )
        .execute(&self.pool)
        .await;

        // Users table (multi-user auth with roles)
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                username TEXT NOT NULL UNIQUE,
                password_hash TEXT NOT NULL,
                role TEXT NOT NULL DEFAULT 'user',
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Sessions table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS sessions (
                token TEXT PRIMARY KEY,
                user_id INTEGER NOT NULL,
                expires_at TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        // API keys table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS api_keys (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                key_hash TEXT NOT NULL UNIQUE,
                preview TEXT NOT NULL,
                expires_at TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        let _ = sqlx::query("ALTER TABLE api_keys ADD COLUMN expires_at TEXT")
            .execute(&self.pool)
            .await;

        // Migration: add role column to users if missing
        let _ = sqlx::query("ALTER TABLE users ADD COLUMN role TEXT NOT NULL DEFAULT 'user'")
            .execute(&self.pool)
            .await;
        let _ = sqlx::query("UPDATE users SET role = 'admin' WHERE role = 'user' AND id IN (SELECT id FROM users ORDER BY id LIMIT 1)")
            .execute(&self.pool)
            .await;

        // User preferences table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS user_preferences (
                user_id INTEGER PRIMARY KEY,
                theme TEXT NOT NULL DEFAULT 'system',
                language TEXT NOT NULL DEFAULT 'en',
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Migration: add timezone column to user_preferences if missing
        let _ = sqlx::query("ALTER TABLE user_preferences ADD COLUMN timezone TEXT NOT NULL DEFAULT 'auto'")
            .execute(&self.pool)
            .await;

        // Repair broken check_logs FK from previous migrations (SQLite 3.26+ bug)
        self.repair_check_logs_fk().await?;

        // Migration: change UNIQUE(ip, port) → UNIQUE(ip, port, protocol)
        self.migrate_proxy_unique_constraint().await?;

        Ok(())
    }

    /// Repair check_logs FK reference broken by SQLite 3.26+ auto-updating
    /// FK definitions during ALTER TABLE RENAME.
    async fn repair_check_logs_fk(&self) -> Result<()> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT sql FROM sqlite_master WHERE type='table' AND name='check_logs'",
        )
        .fetch_optional(&self.pool)
        .await?;

        let Some((sql,)) = row else { return Ok(()) };

        if !sql.contains("proxies_old") {
            return Ok(());
        }

        info!("Repairing check_logs table: fixing broken foreign key reference");

        let mut tx = self.pool.begin().await?;

        sqlx::query("PRAGMA legacy_alter_table=ON")
            .execute(&mut *tx)
            .await?;

        sqlx::query("ALTER TABLE check_logs RENAME TO _check_logs_repair")
            .execute(&mut *tx)
            .await?;

        sqlx::query(
            r#"
            CREATE TABLE check_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                proxy_id INTEGER NOT NULL,
                target TEXT NOT NULL,
                success INTEGER NOT NULL DEFAULT 0,
                latency_ms REAL,
                error TEXT,
                checked_at TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (proxy_id) REFERENCES proxies(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query("INSERT INTO check_logs SELECT * FROM _check_logs_repair")
            .execute(&mut *tx)
            .await?;

        sqlx::query("DROP TABLE _check_logs_repair")
            .execute(&mut *tx)
            .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_check_logs_proxy ON check_logs(proxy_id, checked_at DESC)",
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query("PRAGMA legacy_alter_table=OFF")
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        info!("check_logs table repaired");

        Ok(())
    }

    /// Migrate proxies table: UNIQUE(ip, port) → UNIQUE(ip, port, protocol).
    async fn migrate_proxy_unique_constraint(&self) -> Result<()> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT sql FROM sqlite_master WHERE type='table' AND name='proxies'",
        )
        .fetch_optional(&self.pool)
        .await?;

        let Some((sql,)) = row else { return Ok(()) };

        if sql.contains("UNIQUE(ip, port, protocol)") {
            return Ok(());
        }

        info!("Migrating proxies table: UNIQUE(ip, port) → UNIQUE(ip, port, protocol)");

        let mut tx = self.pool.begin().await?;

        // Prevent SQLite 3.26+ from updating FK references in other tables
        sqlx::query("PRAGMA legacy_alter_table=ON")
            .execute(&mut *tx)
            .await?;

        sqlx::query("ALTER TABLE proxies RENAME TO proxies_old")
            .execute(&mut *tx)
            .await?;

        sqlx::query(
            r#"
            CREATE TABLE proxies (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                ip TEXT NOT NULL,
                port INTEGER NOT NULL,
                protocol TEXT NOT NULL DEFAULT 'http',
                anonymity TEXT NOT NULL DEFAULT 'unknown',
                country TEXT NOT NULL DEFAULT 'unknown',
                score REAL NOT NULL DEFAULT 0.0,
                is_alive INTEGER NOT NULL DEFAULT 0,
                success_count INTEGER NOT NULL DEFAULT 0,
                fail_count INTEGER NOT NULL DEFAULT 0,
                consecutive_fails INTEGER NOT NULL DEFAULT 0,
                avg_latency_ms REAL NOT NULL DEFAULT 0.0,
                last_check_at TEXT,
                last_success_at TEXT,
                next_check_at TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                source TEXT NOT NULL DEFAULT 'unknown',
                UNIQUE(ip, port, protocol)
            )
            "#,
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO proxies (id, ip, port, protocol, anonymity, country, score,
                is_alive, success_count, fail_count, consecutive_fails, avg_latency_ms,
                last_check_at, last_success_at, next_check_at, created_at, updated_at, source)
            SELECT id, ip, port, protocol, anonymity, country, score,
                is_alive, success_count, fail_count, consecutive_fails, avg_latency_ms,
                last_check_at, last_success_at, next_check_at, created_at, updated_at, source
            FROM proxies_old
            "#,
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query("DROP TABLE proxies_old")
            .execute(&mut *tx)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_proxies_alive ON proxies(is_alive)")
            .execute(&mut *tx)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_proxies_score ON proxies(score DESC)")
            .execute(&mut *tx)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_proxies_country ON proxies(country)")
            .execute(&mut *tx)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_proxies_next_check ON proxies(next_check_at)")
            .execute(&mut *tx)
            .await?;

        sqlx::query("PRAGMA legacy_alter_table=OFF")
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        info!("Migration complete: proxies table now supports same IP:port with different protocols");

        Ok(())
    }
}
