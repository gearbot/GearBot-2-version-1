use super::GuildContext;
use crate::Error;

use twilight::model::{
    channel::{embed::Embed, Message},
    id::{ChannelId, MessageId},
};

impl GuildContext {
    pub async fn send_message(
        &self,
        message: impl Into<String>,
        channel_id: ChannelId,
    ) -> Result<Message, Error> {
        let sent_msg_handle = self
            .bot_context
            .http
            .create_message(channel_id)
            .content(message)
            .await?;

        Ok(sent_msg_handle)
    }

    pub async fn send_embed(&self, embed: Embed, channel_id: ChannelId) -> Result<Message, Error> {
        let sent_embed_handle = self
            .bot_context
            .http
            .create_message(channel_id)
            .embed(embed)
            .await?;

        Ok(sent_embed_handle)
    }

    pub async fn send_message_with_embed(
        &self,
        msg: impl Into<String>,
        embed: Embed,
        channel_id: ChannelId,
    ) -> Result<Message, Error> {
        let sent_handle = self
            .bot_context
            .http
            .create_message(channel_id)
            .content(msg)
            .embed(embed)
            .await?;

        Ok(sent_handle)
    }

    pub async fn update_message(
        &self,
        updated_content: impl Into<String>,
        channel_id: ChannelId,
        msg_id: MessageId,
    ) -> Result<Message, Error> {
        let updated_message_handle = self
            .bot_context
            .http
            .update_message(channel_id, msg_id)
            .content(updated_content.into())
            .await?;

        Ok(updated_message_handle)
    }
}