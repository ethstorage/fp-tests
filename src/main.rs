#![doc = include_str!("../README.md")]
#![warn(missing_debug_implementations, missing_docs, rustdoc::all)]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

use clap::Parser;
use color_eyre::Result;

mod cli;
mod fixture;
mod generator;
mod registry;
mod runner;
mod util;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    cli::Cli::parse().init_tracing_subscriber()?.run().await
}
