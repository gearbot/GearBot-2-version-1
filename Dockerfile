FROM ubuntu:latest
RUN apt-get update && apt-get install  ca-certificates libssl-dev -y
WORKDIR /GearBot
COPY ./target/x86_64-unknown-linux-gnu/release/gearbot /GearBot/gearbot
COPY ./lang /GearBot/lang
ENTRYPOINT /GearBot/gearbot