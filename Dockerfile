FROM rust:latest as builder
USER root
RUN apt-get update && apt-get install cmake -y
WORKDIR /compile
COPY ./Cargo.toml ./Cargo.toml
COPY ./Cargo.lock ./Cargo.lock
COPY ./.cargo ./.cargo
COPY ./assets ./assets
COPY ./migrations ./migrations
COPY ./team.toml ./team.toml
COPY ./src ./src
COPY ./.git ./.git
RUN cargo build --release

FROM debian:buster-slim
WORKDIR /GearBot
COPY --from=builder ./compile/target/release/gearbot /GearBot/gearbot
COPY ./lang /GearBot/lang
ENTRYPOINT /GearBot/gearbot