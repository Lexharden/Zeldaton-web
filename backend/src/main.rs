use std::time::Duration;

use chrono::Utc;
use tokio::sync::mpsc::unbounded_channel;
use zeldathon_server::accounts::{self, MIN_PASSWORD_LEN, Role};
use zeldathon_server::auth::random_token;
use zeldathon_server::config::Config;
use zeldathon_server::hub::Hub;
use zeldathon_server::{app, db, seed};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,sqlx=warn".into()),
        )
        .init();

    let cfg = Config::from_env();
    if cfg.admin_token_generated {
        tracing::warn!(
            "ADMIN_TOKEN is not set (or shorter than 16 chars); generated one for this run: {}",
            cfg.admin_token
        );
    }

    let pool = db::connect(&cfg.database_url).await?;
    db::migrate(&pool).await?;

    if !db::has_event(&pool).await? {
        let seeded = seed::build(Utc::now());
        for op in seeded.ops {
            db::apply(&pool, op).await?;
        }
        println!("\n=== First run: racer ingest tokens (shown ONCE, store them safely) ===");
        for (id, token) in &seeded.tokens {
            println!("  {id:<12} {token}");
        }
        println!("=======================================================================\n");
        if let Some(path) = &cfg.dev_tokens_file {
            let map: std::collections::BTreeMap<_, _> = seeded.tokens.iter().cloned().collect();
            std::fs::write(path, serde_json::to_string_pretty(&map)?)?;
            println!("(dev) tokens also written to {path} -- never commit or deploy this file\n");
        }
    }

    // First organizer account (the panel's login). ADMIN_TOKEN keeps working as an emergency key.
    if accounts::count_users(&pool).await? == 0 {
        let (password, generated) = match cfg.admin_password.clone() {
            Some(p) if p.chars().count() >= MIN_PASSWORD_LEN => (p, false),
            other => {
                if other.is_some() {
                    tracing::warn!(
                        "ADMIN_PASSWORD is shorter than {MIN_PASSWORD_LEN} characters; ignoring it"
                    );
                }
                (random_token()[..20].to_string(), true)
            }
        };
        accounts::create_user(&pool, &cfg.admin_user, &password, Role::Admin).await?;
        println!("\n=== First run: organizer account for /admin ===");
        println!("  user:     {}", cfg.admin_user.trim().to_lowercase());
        if generated {
            println!("  password: {password}   (shown ONCE: change it in the panel)");
        } else {
            println!("  password: (the ADMIN_PASSWORD you set)");
        }
        println!("===============================================\n");
    }

    // Databases created before the catalog existed get the factory one (never overwrites edits).
    if db::ensure_catalog(&pool).await? {
        tracing::info!("catalog seeded with the factory items and objectives");
    }

    let mut state = db::load(&pool).await?.expect("event exists after seeding");
    state.recover(Utc::now());
    tracing::info!(racers = state.racers.len(), "state loaded");

    let (persist_tx, persist_rx) = unbounded_channel();
    tokio::spawn(db::writer(pool.clone(), persist_rx));
    let hub = Hub::new(state, persist_tx, cfg.clone(), pool);

    // Expired browser sessions are useless: sweep them once an hour.
    let sweep_pool = hub.pool.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(3600));
        loop {
            interval.tick().await;
            if let Err(e) = accounts::purge_expired(&sweep_pool, Utc::now()).await {
                tracing::warn!(error = %e, "could not purge expired sessions");
            }
        }
    });

    // Scheduler: exhaustion, daily resets and lost connections.
    let ticker = hub.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(500));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            ticker.tick(Utc::now());
        }
    });

    let listener = tokio::net::TcpListener::bind(&cfg.bind).await?;
    tracing::info!("listening on http://{}", cfg.bind);
    axum::serve(listener, app::build(hub))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
            tracing::info!("shutting down");
        })
        .await?;
    Ok(())
}
