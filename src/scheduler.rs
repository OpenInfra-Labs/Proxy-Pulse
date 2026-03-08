use std::sync::Arc;
use tokio::time::{interval, Duration};
use tracing::{error, info};

use crate::config::AppConfig;
use crate::db::Database;
use crate::sources;
use crate::checker;

/// Start all background scheduler tasks
pub async fn start_schedulers(db: Database, config: Arc<AppConfig>) {
    let db_source = db.clone();
    let config_source = config.clone();

    // Source sync scheduler
    tokio::spawn(async move {
        let interval_secs = config_source.sources.sync_interval_secs;
        info!(interval_secs = interval_secs, "Starting source sync scheduler");

        // Run immediately on startup
        match sources::sync_sources(&db_source, &config_source.sources.providers).await {
            Ok(count) => info!(count = count, "Initial source sync complete"),
            Err(e) => error!(error = %e, "Initial source sync failed"),
        }
        // Also sync subscription sources from DB
        match sources::sync_subscription_sources(&db_source).await {
            Ok(count) => info!(count = count, "Initial subscription sync complete"),
            Err(e) => error!(error = %e, "Initial subscription sync failed"),
        }

        let mut ticker = interval(Duration::from_secs(interval_secs));
        ticker.tick().await; // Skip immediate tick (already ran)

        loop {
            ticker.tick().await;
            match sources::sync_sources(&db_source, &config_source.sources.providers).await {
                Ok(count) => info!(count = count, "Source sync complete"),
                Err(e) => error!(error = %e, "Source sync failed"),
            }
            // Sync subscription sources each cycle
            match sources::sync_subscription_sources(&db_source).await {
                Ok(count) => info!(count = count, "Subscription sync complete"),
                Err(e) => error!(error = %e, "Subscription sync failed"),
            }
        }
    });

    let db_checker = db.clone();
    let config_checker = config.clone();

    // Proxy checker scheduler
    tokio::spawn(async move {
        let interval_secs = config_checker.checker.interval_secs;
        info!(interval_secs = interval_secs, "Starting proxy checker scheduler");

        // Wait a bit for initial source sync to populate proxies
        tokio::time::sleep(Duration::from_secs(5)).await;

        let mut ticker = interval(Duration::from_secs(interval_secs));

        loop {
            ticker.tick().await;
            match checker::run_check_cycle(
                &db_checker,
                &config_checker.checker,
                &config_checker.scoring,
            )
            .await
            {
                Ok((s, f)) => info!(success = s, fail = f, "Check cycle complete"),
                Err(e) => error!(error = %e, "Check cycle failed"),
            }
        }
    });

    let db_cleanup = db.clone();

    // Log cleanup scheduler (daily)
    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(86400)); // 24 hours

        loop {
            ticker.tick().await;
            match db_cleanup.cleanup_old_logs(7).await {
                Ok(count) => info!(deleted = count, "Old check logs cleaned up"),
                Err(e) => error!(error = %e, "Log cleanup failed"),
            }
        }
    });
}
