#![allow(non_snake_case)]
use std::collections::HashSet;

#[macro_use]
mod error;
mod github;
mod profile;

use crate::error::Error;

#[tokio::main(flavor = "current_thread")]
pub async fn main() -> Result<(), Error>
{
    let mut p = profile::Profile::default();
    p.getData(&std::env::args().nth(1).unwrap()).await?;
    let svg = p.genSvg();

    let client = github::Client::withToken(&std::env::args().nth(1).unwrap())?;
    let hash = client.commitSingleFile("MetroWind", "test", "main", "profile.svg",
                                       &svg, "Update profile SVG").await?;
    println!("{}", hash);
    Ok(())
}
