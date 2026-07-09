use cfg_if::cfg_if;
use rnex_core::common::with_setup;

#[tokio::main]
async fn main() {
    with_setup(async || {
        #[cfg(feature = "database-support")]
        {
            use rnex_core::executables::common::DB_POOL;
            use sqlx::PgPool;
            let database_url = std::env::var("RNEX_DATASTORE_DATABASE_URL")
                .expect("RNEX_DATASTORE_DATABASE_URL must be set");

            let pool = PgPool::connect(&database_url)
                .await
                .expect("Failed to create pool");

            DB_POOL.set(pool).expect("failed to set global DB_POOL");
        }

        cfg_if! {
            if #[cfg(feature = "friends")]{
                use rnex_core::executables::friends_backend::start_friends_backend;
                start_friends_backend().await;
            } else {
                use rnex_core::executables::regular_backend;
                regular_backend::start_regular_backend().await
            }
        }
    })
    .await;
}
