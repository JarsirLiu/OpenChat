mod postgres;

use anyhow::Context;
use sqlx::{postgres::PgPoolOptions, PgPool};

#[derive(Clone)]
pub enum DatabasePool {
    #[allow(dead_code)]
    Compat(PgPool),
    Postgres(PgPool),
}

#[derive(Clone)]
pub struct Database {
    pool: DatabasePool,
}

impl Database {
    pub async fn connect(database_url: &str) -> anyhow::Result<Self> {
        if !is_postgres_url(database_url) {
            anyhow::bail!("OPENCHAT_DATABASE_URL must be a postgres URL");
        }

        let pool = PgPoolOptions::new()
            .max_connections(20)
            .connect(database_url)
            .await
            .context("failed to connect to postgres")?;

        let database = Self {
            pool: DatabasePool::Postgres(pool),
        };
        database.migrate().await?;
        Ok(database)
    }

    pub fn pool(&self) -> &DatabasePool {
        &self.pool
    }

    async fn migrate(&self) -> anyhow::Result<()> {
        match &self.pool {
            DatabasePool::Compat(_) => {
                anyhow::bail!("legacy non-postgres database support has been removed")
            }
            DatabasePool::Postgres(pool) => postgres::migrate(pool).await?,
        }

        Ok(())
    }
}

fn is_postgres_url(database_url: &str) -> bool {
    database_url.starts_with("postgres://") || database_url.starts_with("postgresql://")
}
