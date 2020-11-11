create table message
(
    id                   bigint        not null primary key,
    encrypted_content    bytea         not null,
    author_id            bigint        not null,
    channel_id           bigint        not null,
    guild_id             bigint        not null,
    kind       int2          not null,
    pinned     bool default false
);
create index message_ag_index on message (author_id, guild_id);
create index message_channel_index on message (channel_id);
create index message_guild_index on message (guild_id);
create index message_pinned_index on message (channel_id, pinned) where pinned = true;


create table attachment
(
    id         bigint       not null primary key,
    name       varchar(255) not null,
    image      bool         not null,
    message_id bigint       not null
);

create type historyType as enum
    (
        'note',
        'warning',
        'censor',
        'mute',
        'kick',
        'cleankick',
        'tempban',
        'ban',
        'forceban',
        'unban'
        );

create table history
(
    id       serial primary key not null,
    guild_id bigint             not null,
    user_id  bigint             not null,
    mod_id   bigint             not null,
    type     historyType        not null,
    start    timestamptz        not null default now(),
    "end"    timestamptz        null
);

create index history_guild_index on history (guild_id);
create index history_guild_user_index on history (guild_id, user_id);
create index history_guild_mod_index on history (guild_id, mod_id);

create type timedActionType as enum (
    'mute',
    'tempban'
    );

create table timedAction
(
    id         serial not null primary key,
    history_id int    not null references history (id) unique
);

create view pendingActions as
select h.*
from timedAction
         inner join history h on timedAction.history_id = h.id
order by "end" desc;


create table customCommand
(
    id       serial        not null primary key,
    guild_id bigint        not null,
    trigger  varchar(30)   not null,
    response varchar(2000) not null
);

create unique index custom_command_guild_trigger_unique on customCommand (guild_id, trigger);


create table guildConfig
(
    id bigint primary key not null,
    config jsonb not null,
    encryption_key bytea not null
);

create table webhook
(
    channel_id bigint primary key not null,
    id bigint not null,
    token varchar(255) not null
)