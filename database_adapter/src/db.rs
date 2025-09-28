use serde::{Serialize, de::DeserializeOwned};
use sqlx::{Pool, Postgres, postgres::PgPoolOptions};
use std::fmt;
use tokio::runtime::Runtime;

#[derive(Debug)]
pub enum DbError {
    SqlxError(sqlx::Error),
    SerdeError(serde_json::Error),
    TokioError(std::io::Error),
}

impl fmt::Display for DbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DbError::SqlxError(e) => write!(f, "Database error: {e}"),
            DbError::SerdeError(e) => write!(f, "Serialization error: {e}"),
            DbError::TokioError(e) => write!(f, "Runtime error: {e}"),
        }
    }
}

impl std::error::Error for DbError {}

impl From<sqlx::Error> for DbError {
    fn from(error: sqlx::Error) -> Self {
        DbError::SqlxError(error)
    }
}

impl From<serde_json::Error> for DbError {
    fn from(error: serde_json::Error) -> Self {
        DbError::SerdeError(error)
    }
}

impl From<std::io::Error> for DbError {
    fn from(error: std::io::Error) -> Self {
        DbError::TokioError(error)
    }
}

pub trait Repository<T, Id> {
    /// Insert a new item with the given ID
    /// # Errors
    /// - Returns `DbError` if the operation fails
    fn insert(&self, id: Id, item: T) -> Result<(), DbError>;
    /// Update an existing item with the given ID
    /// # Errors
    /// - Returns `DbError` if the operation fails
    fn update(&self, id: Id, item: T) -> Result<(), DbError>;
    /// Remove an item with the given ID
    /// # Errors
    /// - Returns `DbError` if the operation fails
    fn remove(&self, id: Id) -> Result<(), DbError>;
    /// Get an item by ID
    /// # Errors
    /// - Returns `DbError` if the operation fails
    fn get(&self, id: &Id) -> Result<Option<T>, DbError>;
    /// Get the number of items in the repository
    /// # Errors
    /// - Returns `DbError` if the operation fails
    // TODO: remove
    fn len(&self) -> Result<usize, DbError>;
    /// Check if the repository is empty
    /// # Errors
    /// - Returns `DbError` if the operation fails
    fn is_empty(&self) -> Result<bool, DbError> {
        Ok(self.len()? == 0)
    }
    /// Find an item by a specific field and value
    /// # Errors
    /// - Returns `DbError` if the operation fails
    fn find_by_field(&self, field: &str, value: &str) -> Result<Option<T>, DbError>;
    /// Find all items by a specific field and value
    /// # Errors
    /// - Returns `DbError` if the operation fails
    fn find_all_by_field(&self, field: &str, value: &str) -> Result<Vec<(Id, T)>, DbError>;
}

/// Generic Postgres repository, stores T as JSON
#[derive(Clone)]
pub struct PostgresRepo<T, Id> {
    pool: Pool<Postgres>,
    table: String,
    _phantom: std::marker::PhantomData<(T, Id)>,
}

impl<T, Id> std::fmt::Debug for PostgresRepo<T, Id> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PostgresRepo")
            .field("table", &self.table)
            .field("_phantom", &self._phantom)
            .finish_non_exhaustive()
    }
}

impl<T, Id> PostgresRepo<T, Id>
where
    T: Serialize + DeserializeOwned + Send + Sync,
    Id: Serialize
        + for<'a> sqlx::Decode<'a, sqlx::Postgres>
        + sqlx::Type<sqlx::Postgres>
        + Send
        + Sync,
{
    /// Create a new Postgres repository
    /// # Errors
    /// - Returns `DbError` if the operation fails
    /// # Panics
    /// - Panics if `DATABASE_URL` is not set in the environment or .env file
    pub fn new(table: &str) -> Result<Self, DbError> {
        dotenvy::dotenv().ok();
        let db_url = std::env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set in .env file or environment");
        let table_name = table.to_string();

        // Use a separate thread to avoid runtime conflicts
        let (pool, ()) = std::thread::scope(|s| {
            let handle = s.spawn(|| {
                let rt = Runtime::new()?;
                let pool = rt.block_on(async {
                    PgPoolOptions::new()
                        .max_connections(5)
                        .connect(&db_url)
                        .await
                })?;

                // Ensure table exists
                let query = format!(
                    "CREATE TABLE IF NOT EXISTS {table_name} (
                        id   TEXT PRIMARY KEY,
                        data JSONB NOT NULL
                    )"
                );
                rt.block_on(async { sqlx::query(&query).execute(&pool).await })?;

                Ok::<_, DbError>((pool, ()))
            });
            handle
                .join()
                .map_err(|_| DbError::TokioError(std::io::Error::other("Thread panicked")))?
        })?;

        Ok(Self {
            pool,
            table: table.to_string(),
            _phantom: std::marker::PhantomData,
        })
    }
}

impl<T, Id> Repository<T, Id> for PostgresRepo<T, Id>
where
    T: Serialize + DeserializeOwned + Send + Sync,
    Id: ToString
        + std::str::FromStr
        + for<'a> sqlx::Decode<'a, sqlx::Postgres>
        + sqlx::Type<sqlx::Postgres>
        + Send
        + Sync,
{
    fn insert(&self, id: Id, item: T) -> Result<(), DbError> {
        let data = serde_json::to_value(item)?;
        let query = format!("INSERT INTO {} (id, data) VALUES ($1, $2)", self.table);
        let pool = self.pool.clone();
        let id_str = id.to_string();

        // Always use a separate thread with its own runtime to avoid nested runtime issues
        std::thread::scope(|s| {
            let handle = s.spawn(|| {
                let rt = Runtime::new()?;
                rt.block_on(async {
                    sqlx::query(&query)
                        .bind(id_str)
                        .bind(data)
                        .execute(&pool)
                        .await
                })
            });
            handle
                .join()
                .map_err(|_| DbError::TokioError(std::io::Error::other("Thread panicked")))?
                .map_err(DbError::from)
        })?;
        Ok(())
    }

    fn update(&self, id: Id, item: T) -> Result<(), DbError> {
        let data = serde_json::to_value(item)?;
        let query = format!("UPDATE {} SET data = $2 WHERE id = $1", self.table);
        let pool = self.pool.clone();
        let id_str = id.to_string();

        std::thread::scope(|s| {
            let handle = s.spawn(|| {
                let rt = Runtime::new()?;
                rt.block_on(async {
                    sqlx::query(&query)
                        .bind(id_str)
                        .bind(data)
                        .execute(&pool)
                        .await
                })
            });
            handle
                .join()
                .map_err(|_| DbError::TokioError(std::io::Error::other("Thread panicked")))?
                .map_err(DbError::from)
        })?;
        Ok(())
    }

    fn remove(&self, id: Id) -> Result<(), DbError> {
        let query = format!("DELETE FROM {} WHERE id = $1", self.table);
        let pool = self.pool.clone();
        let id_str = id.to_string();

        std::thread::scope(|s| {
            let handle = s.spawn(|| {
                let rt = Runtime::new()?;
                rt.block_on(async { sqlx::query(&query).bind(id_str).execute(&pool).await })
            });
            handle
                .join()
                .map_err(|_| DbError::TokioError(std::io::Error::other("Thread panicked")))?
                .map_err(DbError::from)
        })?;
        Ok(())
    }

    fn get(&self, id: &Id) -> Result<Option<T>, DbError> {
        let query = format!("SELECT data FROM {} WHERE id = $1", self.table);
        let pool = self.pool.clone();
        let id_str = id.to_string();

        let row: Option<serde_json::Value> = std::thread::scope(|s| {
            let handle = s.spawn(|| {
                let rt = Runtime::new()?;
                rt.block_on(async {
                    sqlx::query_scalar(&query)
                        .bind(id_str)
                        .fetch_optional(&pool)
                        .await
                })
            });
            handle
                .join()
                .map_err(|_| DbError::TokioError(std::io::Error::other("Thread panicked")))?
                .map_err(DbError::from)
        })?;
        Ok(row.map(|val| serde_json::from_value(val).unwrap()))
    }

    fn len(&self) -> Result<usize, DbError> {
        let query = format!("SELECT COUNT(*) FROM {}", self.table);
        let pool = self.pool.clone();

        let (count,): (i64,) = std::thread::scope(|s| {
            let handle = s.spawn(|| {
                let rt = Runtime::new()?;
                rt.block_on(async { sqlx::query_as(&query).fetch_one(&pool).await })
            });
            handle
                .join()
                .map_err(|_| DbError::TokioError(std::io::Error::other("Thread panicked")))?
                .map_err(DbError::from)
        })?;
        Ok(count.saturating_abs() as usize)
    }
    fn find_by_field(&self, field: &str, value: &str) -> Result<Option<T>, DbError> {
        let query = format!(
            "SELECT data FROM {} WHERE data->>$1 = $2 LIMIT 1",
            self.table
        );
        let pool = self.pool.clone();
        let field = field.to_string();
        let value = value.to_string();

        let row: Option<serde_json::Value> = std::thread::scope(|s| {
            let handle = s.spawn(|| {
                let rt = Runtime::new()?;
                rt.block_on(async {
                    sqlx::query_scalar(&query)
                        .bind(field)
                        .bind(value)
                        .fetch_optional(&pool)
                        .await
                })
            });
            handle
                .join()
                .map_err(|_| DbError::TokioError(std::io::Error::other("Thread panicked")))?
                .map_err(DbError::from)
        })?;

        Ok(row.map(|val| serde_json::from_value(val).unwrap()))
    }

    fn find_all_by_field(&self, field: &str, value: &str) -> Result<Vec<(Id, T)>, DbError> {
        let query = format!(
            "SELECT id, data FROM {} WHERE data->>$1 = $2 ORDER BY data->>'date' DESC",
            self.table
        );
        let pool = self.pool.clone();
        let field = field.to_string();
        let value = value.to_string();

        let rows: Vec<(String, serde_json::Value)> = std::thread::scope(|s| {
            let handle = s.spawn(|| {
                let rt = Runtime::new()?;
                rt.block_on(async {
                    sqlx::query_as(&query)
                        .bind(field)
                        .bind(value)
                        .fetch_all(&pool)
                        .await
                })
            });
            handle
                .join()
                .map_err(|_| DbError::TokioError(std::io::Error::other("Thread panicked")))?
                .map_err(DbError::from)
        })?;

        let result = rows
            .into_iter()
            .filter_map(|(id_str, val)| {
                // Parse the string ID back to the proper type
                let id = id_str.parse().ok()?;
                let item: T = serde_json::from_value(val).ok()?;
                Some((id, item))
            })
            .collect();

        Ok(result)
    }
}
