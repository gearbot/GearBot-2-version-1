use twilight::model::{
    channel::{embed::Embed, Message},
    id::{ChannelId, MessageId},
};

use crate::Error;

use super::CommandContext;
use crate::translation::GearBotString;
use fluent_bundle::{FluentArgs, FluentValue};

impl CommandContext {
    pub async fn send_message(
        &self,
        channel_id: ChannelId,
        key: GearBotString,
        args: FluentArgs<'_>,
    ) -> Result<Message, Error> {
        let translated = self.translate_with_args(key, &args);
        let sent_msg_handle = self
            .bot_context
            .http
            .create_message(channel_id)
            .content(translated)?
            .await?;

        Ok(sent_msg_handle)
    }

    pub async fn send_message_raw(&self, message: impl Into<String>, channel_id: ChannelId) -> Result<Message, Error> {
        let sent_msg_handle = self
            .bot_context
            .http
            .create_message(channel_id)
            .content(message)?
            .await?;

        Ok(sent_msg_handle)
    }

    pub async fn send_embed(&self, embed: Embed, channel_id: ChannelId) -> Result<Message, Error> {
        let sent_embed_handle = self.bot_context.http.create_message(channel_id).embed(embed)?.await?;

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
            .content(msg)?
            .embed(embed)?
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
            .content(updated_content.into())?
            .await?;

        Ok(updated_message_handle)
    }

    pub async fn reply(&self, key: GearBotString, args: FluentArgs<'_>) -> Result<Message, Error> {
        let translated = self.translate_with_args(key, &args);
        let sent_msg_handle = self
            .bot_context
            .http
            .create_message(self.message.channel.get_id())
            .content(translated)?
            .await?;

        Ok(sent_msg_handle)
    }

    pub async fn reply_raw(&self, message: impl Into<String>) -> Result<Message, Error> {
        let sent_msg_handle = self
            .bot_context
            .http
            .create_message(self.message.channel.get_id())
            .content(message)?
            .await?;

        Ok(sent_msg_handle)
    }

    pub async fn reply_embed(&self, embed: Embed) -> Result<Message, Error> {
        let sent_embed_handle = self
            .bot_context
            .http
            .create_message(self.message.channel.get_id())
            .embed(embed)?
            .await?;

        Ok(sent_embed_handle)
    }

    pub async fn reply_with_embed(
        &self,
        key: GearBotString,
        args: FluentArgs<'_>,
        embed: Embed,
    ) -> Result<Message, Error> {
        let translated = self.translate_with_args(key, &args);
        let sent_handle = self
            .bot_context
            .http
            .create_message(self.message.channel.get_id())
            .content(translated)?
            .embed(embed)?
            .await?;

        Ok(sent_handle)
    }

    pub async fn reply_raw_with_embed(&self, message: impl Into<String>, embed: Embed) -> Result<Message, Error> {
        let sent_handle = self
            .bot_context
            .http
            .create_message(self.message.channel.get_id())
            .content(message)?
            .embed(embed)?
            .await?;

        Ok(sent_handle)
    }
}
