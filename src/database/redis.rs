use darkredis::ConnectionPool;
use serde::{de::DeserializeOwned, Serialize};

use crate::utils::Error;

pub struct Redis {
    pool: ConnectionPool,
}

impl Redis {
    pub async fn new(conn_addr: &str) -> Result<Self, Error> {
        let pool = darkredis::ConnectionPool::create(conn_addr.to_owned(), None, 5).await?;

        Ok(Self { pool })
    }

    pub async fn get<D: DeserializeOwned>(&self, key: &str) -> Result<Option<D>, Error> {
        let mut conn = self.pool.get().await;

        if let Some(value) = conn.get(key).await? {
            let value = serde_json::from_slice(&value)?;
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }

    pub async fn set<T: Serialize>(&self, key: &str, value: &T, expiry: Option<u32>) -> Result<(), Error> {
        let mut conn = self.pool.get().await;

        let data = serde_json::to_string(value)?;

        match expiry {
            Some(ttl) => conn.set_and_expire_seconds(key, data, ttl).await?,
            None => conn.set(key, data).await?,
        }

        Ok(())
    }

    pub async fn delete(&self, key: &str) -> Result<(), Error> {
        let mut conn = self.pool.get().await;

        conn.del(key).await?;

        Ok(())
    }
}
