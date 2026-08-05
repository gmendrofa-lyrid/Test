use std::ops::{Deref, DerefMut};

use async_stream::try_stream;
use deadpool::managed::{Manager, Metrics, RecycleResult};
use futures::TryStreamExt;
use futures::future::BoxFuture;
use futures::stream::BoxStream;
use sqlx::mysql::MySqlConnectOptions;
use sqlx::{
        ConnectOptions, Connection, Database, Describe, Either, Error as SqlxError, Execute,
        Executor, MySql, MySqlConnection,
};

#[derive(Clone, Debug)]
pub struct DbPool {
        options: MySqlConnectOptions,
}

impl DbPool {
        pub fn new(options: MySqlConnectOptions) -> Self {
                Self { options }
        }
}

impl Manager for DbPool {
        type Type = MySqlConnection;
        type Error = SqlxError;

        async fn create(&self) -> Result<MySqlConnection, SqlxError> {
                self.options.connect().await
        }

        async fn recycle(&self, obj: &mut Self::Type, _: &Metrics) -> RecycleResult<SqlxError> {
                Ok(obj.ping().await?)
        }
}

pub type DeadPool = deadpool::managed::Pool<DbPool>;

#[derive(Clone, Debug)]
pub struct Pool(pub DeadPool);

impl Pool {
        pub fn new(options: MySqlConnectOptions, max_size: usize) -> anyhow::Result<Self> {
                Ok(Self(DeadPool::builder(DbPool { options })
                        .max_size(max_size)
                        .build()?))
        }

        pub fn init() -> anyhow::Result<Self> {
                let options = MySqlConnectOptions::new()
                        .host(&std::env::var("DB_HOST").expect("DB_HOST must be set"))
                        .port(std::env::var("DB_PORT")
                                .expect("DB_PORT must be set")
                                .parse()
                                .unwrap())
                        .username(&std::env::var("DB_USER").expect("DB_USER must be set"))
                        .password(&std::env::var("DB_PASSWORD").expect("DB_PASSWORD must be set"))
                        .database(&std::env::var("DB_NAME").expect("DB_NAME must be set"));
                Self::new(options, 4)
        }
}

impl Deref for Pool {
        type Target = DeadPool;

        fn deref(&self) -> &Self::Target {
                &self.0
        }
}

impl DerefMut for Pool {
        fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
        }
}

impl Executor<'_> for &'_ Pool {
        type Database = MySql;

        fn fetch_many<'e, 'qr: 'e, E>(
                self,
                query: E,
        ) -> BoxStream<
                'e,
                Result<
                        Either<<MySql as Database>::QueryResult, <MySql as Database>::Row>,
                        sqlx::Error,
                >,
        >
        where
                E: 'qr + Execute<'qr, Self::Database>,
        {
                let pool = self.clone();

                Box::pin(try_stream! {
                    let mut conn = pool.get().await.map_err(|_| sqlx::Error::PoolTimedOut)?;
                    let conn = conn.deref_mut();
                    let mut s = conn.fetch_many(query);

                    while let Some(v) = s.try_next().await? {
                        yield v;
                    }
                })
        }

        fn fetch_optional<'e, 'qr: 'e, E>(
                self,
                query: E,
        ) -> BoxFuture<'e, Result<Option<<MySql as Database>::Row>, sqlx::Error>>
        where
                E: 'qr + Execute<'qr, Self::Database>,
        {
                let pool = self.clone();

                Box::pin(async move {
                        pool.get()
                                .await
                                .map_err(|_| sqlx::Error::PoolTimedOut)?
                                .deref_mut()
                                .fetch_optional(query)
                                .await
                })
        }

        fn prepare_with<'e, 'qr: 'e>(
                self,
                sql: &'qr str,
                parameters: &'e [<Self::Database as Database>::TypeInfo],
        ) -> BoxFuture<'e, Result<<Self::Database as Database>::Statement<'qr>, sqlx::Error>>
        {
                let pool = self.clone();

                Box::pin(async move {
                        pool.get()
                                .await
                                .map_err(|_| sqlx::Error::PoolTimedOut)?
                                .deref_mut()
                                .prepare_with(sql, parameters)
                                .await
                })
        }

        #[doc(hidden)]
        fn describe<'e, 'qr: 'e>(
                self,
                sql: &'qr str,
        ) -> BoxFuture<'e, Result<Describe<Self::Database>, sqlx::Error>> {
                let pool = self.clone();

                Box::pin(async move {
                        pool.get()
                                .await
                                .map_err(|_| sqlx::Error::PoolTimedOut)?
                                .deref_mut()
                                .describe(sql)
                                .await
                })
        }
}
