#![deny(
    clippy::expect_used,
    clippy::future_not_send,
    clippy::pedantic,
    clippy::as_conversions,
    clippy::unwrap_used,
    unsafe_code
)]
#![allow(
    clippy::module_name_repetitions,
    clippy::missing_errors_doc,
    clippy::manual_non_exhaustive,
    clippy::multiple_crate_versions
)]

use std::io;

use clap::Parser;
use evaltor::{EvaltorArgs, server};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), io::Error> {
    dotenvy::dotenv().map_err(io::Error::other)?;

    let args = EvaltorArgs::parse();

    let listener = TcpListener::bind(format!("127.0.0.1:{}", args.port))
        .await
        .map_err(io::Error::other)?;

    let app = server(args).await?;

    axum::serve(listener, app).await
}
