// #![deny(warnings)]
#![doc = include_str!("../README.md")]
use std::env;

use anyhow::{Context, Result};
use clap::{crate_version, FromArgMatches, IntoApp};

mod raen;
mod ext;
use crate::raen::Raen;

fn main() -> Result<()> {
    let args = env::args_os();
    let matches = Raen::command()
        .version(crate_version!())
        .bin_name("raen")
        .get_matches_from(args);

    Raen::from_arg_matches(&matches)
        .context("Command not found")?
        .run()
}
