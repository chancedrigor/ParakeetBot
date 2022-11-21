# parakeet-bot
A Discord bot that play sounds in a voice channel.

This bot reads the following enviromental variables:

`TOKEN`
: the discord api token for your bot

`RUST_LOG`
: controls logging see [this](https://docs.rs/env_logger/latest/env_logger/#enabling-logging) for details

Additionally, the bot will read from a `.env` in the working directory for env vars.