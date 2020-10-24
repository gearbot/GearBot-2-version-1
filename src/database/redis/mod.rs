use darkredis::ConnectionPool;
use serde::{de::DeserializeOwned, Serialize};

use crate::core::BotContext;
use crate::database::redis::api_structs::{ApiRequest, Reply, Request};
use crate::gearbot_error;
use crate::utils::{ApiCommunicaionError, DatabaseError};
use futures_util::StreamExt;
use std::sync::Arc;
use team_info::get_team_info;

pub mod api_structs;
mod team_info;

pub struct Redis {
    pool: ConnectionPool,
}

impl Redis {
    pub async fn new(conn_addr: &str) -> Result<Self, darkredis::Error> {
        let pool = darkredis::ConnectionPool::create(conn_addr.to_owned(), None, 5).await?;
        Ok(Self { pool })
    }

    pub async fn get<D: DeserializeOwned>(&self, key: &str) -> Result<Option<D>, DatabaseError> {
        let mut conn = self.pool.get().await;

        if let Some(value) = conn.get(key).await? {
            let value = serde_json::from_slice(&value).map_err(|e| DatabaseError::Deserializing(e))?;
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }

    pub async fn set<T: Serialize>(&self, key: &str, value: &T, expiry: Option<u32>) -> Result<(), DatabaseError> {
        let mut conn = self.pool.get().await;

        let data = serde_json::to_string(value).map_err(|e| DatabaseError::Serializing(e))?;

        match expiry {
            Some(ttl) => conn.set_and_expire_seconds(key, data, ttl).await?,
            None => conn.set(key, data).await?,
        }

        Ok(())
    }

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
                serde_json::to_string(&reply).map_err(|e| ApiCommunicaionError::Serializing(e))?,
            )
            .await?;
        Ok(())
    }
}
