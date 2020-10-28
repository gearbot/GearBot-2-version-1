use darkredis::ConnectionPool;
use serde::{de::DeserializeOwned, Serialize};

use crate::core::BotContext;
use crate::database::redis::api_structs::{ApiRequest, Reply, Request};
use crate::error::{ApiCommunicaionError, DatabaseError};
use crate::gearbot_error;
use futures_util::StreamExt;
use std::sync::Arc;
use team_info::get_team_info;

pub mod api_structs;
mod team_info;

/// An abstraction layer around a connection to Redis.
///
/// All interactions with Redis should go through this.
pub struct Redis {
    pool: ConnectionPool,
}

impl Redis {
    /// Creates a new connection pool using the specified addresss.
    ///
    /// 5 connections are opened by default.
    pub async fn new(conn_addr: &str) -> Result<Self, darkredis::Error> {
        let pool = ConnectionPool::create(conn_addr.to_owned(), None, 5).await?;
        Ok(Self { pool })
    }

    /// Retrieves a value from Redis.
    ///
    /// Returns `None` if the key didn't exist.
    pub async fn get<D: DeserializeOwned>(&self, key: &str) -> Result<Option<D>, DatabaseError> {
        let mut conn = self.pool.get().await;

        if let Some(value) = conn.get(key).await? {
            let value = serde_json::from_slice(&value).map_err(DatabaseError::Deserializing)?;
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }

    /// Inserts a value into Redis.
    ///
    /// The value will automatically expire at the optionally provided time.
    pub async fn set<T: Serialize>(&self, key: &str, value: &T, expiry: Option<u32>) -> Result<(), DatabaseError> {
        let mut conn = self.pool.get().await;

        let data = serde_json::to_string(value).map_err(DatabaseError::Serializing)?;

        match expiry {
            Some(ttl) => conn.set_and_expire_seconds(key, data, ttl).await?,
            None => conn.set(key, data).await?,
        }

        Ok(())
    }

    /// Deletes a value from Redis.
    pub async fn delete(&self, key: &str) -> Result<(), darkredis::Error> {
        let mut conn = self.pool.get().await;

        conn.del(key).await?;

        Ok(())
    }

    pub async fn establish_api_link(&self, ctx: Arc<BotContext>) {
        let con = match self.pool.spawn("api_connection").await {
            Ok(con) => con,
            Err(e) => {
                log::error!("ERROR: {}", e);
                panic!("error");
            }
        };

        log::debug!("establishing api connection");

        con.subscribe(&["api-out"])
            .await
            .unwrap()
            .for_each(|message| async {
                let content = message.message;
                //TODO: handle errors
                let message: ApiRequest = serde_json::from_slice(&content).unwrap();
                log::debug!("Received {} request from the api", message.request.get_type());

                let result = match message.request {
                    Request::TeamInfo => get_team_info(ctx.clone()).await,
                };

                match result {
                    Ok(data) => {
                        match self
                            .send_to_api(Reply {
                                uuid: message.uuid,
                                data,
                            })
                            .await
                        {
                            Ok(_) => {}
                            Err(e) => gearbot_error!("Failed to send message to the api: {}", e),
                        }
                    }
                    Err(e) => gearbot_error!(
                        "Failed to handle a message from the api ({}): {}",
                        message.request.get_type(),
                        e
                    ),
                }
            })
            .await;
    }

    pub async fn send_to_api(&self, reply: Reply) -> Result<(), ApiCommunicaionError> {
        self.pool
            .get()
            .await
            .publish(
                "gearbot-out",
                serde_json::to_string(&reply).map_err(ApiCommunicaionError::Serializing)?,
            )
            .await?;
        Ok(())
    }
}
