FROM rust:latest as builder
USER root
RUN apt-get update && apt-get install cmake -y
WORKDIR /compile
# Exist to (ab)use caching Docker layers of dependencies
RUN mkdir ./src
RUN echo "fn main() {}" > ./src/main.rs
COPY ./Cargo.toml ./Cargo.toml
COPY ./Cargo.lock ./Cargo.lock
COPY ./.cargo ./.cargo
RUN cargo build --release
RUN rm -f ./target/release/deps/gearbot*
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