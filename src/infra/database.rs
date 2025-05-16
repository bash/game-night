use crate::impl_resolve_for_state;
use crate::services::{Resolve, ResolveContext};
use anyhow::Result;
use diesel::SqliteConnection;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::sync_connection_wrapper::SyncConnectionWrapper;
use nameof::name_of;
use rocket::fairing::{self, Fairing};
use rocket::{error, Build, Rocket};
use std::fmt;

type Connection = SyncConnectionWrapper<SqliteConnection>;
type Pool = diesel_async::pooled_connection::deadpool::Pool<Connection>;
pub(crate) type DieselPoolConnection =
    diesel_async::pooled_connection::deadpool::Object<Connection>;

#[derive(Clone)]
pub(crate) struct DieselConnectionPool(Pool);

impl fmt::Debug for DieselConnectionPool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple(name_of!(DieselConnectionPool))
            .finish_non_exhaustive()
    }
}

impl_resolve_for_state!(DieselConnectionPool: "Diesel Connection Pool");

impl DieselConnectionPool {
    pub async fn get(&self) -> Result<DieselPoolConnection> {
        Ok(self.0.get().await?)
    }

    pub(crate) fn fairing() -> impl Fairing {
        fairing::AdHoc::try_on_ignite("Diesel Connection Pool", |rocket| async {
            match connect_database(&rocket).await {
                Ok(database) => Ok(rocket.manage(DieselConnectionPool(database))),
                Err(e) => {
                    error!("Failed to connect to database: {e:?}");
                    Err(rocket)
                }
            }
        })
    }
}

async fn connect_database(rocket: &Rocket<Build>) -> Result<Pool> {
    let sqlite_url: String = rocket
        .figment()
        .focus("databases.sqlite")
        .extract_inner("url")?;
    let config = AsyncDieselConnectionManager::new(sqlite_url);
    Ok(Pool::builder(config).build()?)
}

impl Resolve for DieselPoolConnection {
    async fn resolve(ctx: &ResolveContext<'_>) -> Result<Self> {
        Ok(ctx.resolve::<DieselConnectionPool>().await?.0.get().await?)
    }
}
