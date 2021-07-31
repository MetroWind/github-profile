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
    let matches = clap::App::new("GitHub Profile")
        .version("0.1")
        .author("MetroWind <chris.corsair@gmail.com>")
        .about("Generate and commit GitHub profile SVG")
        .arg(clap::Arg::new("TOKEN")
            .about("The personal token to authenticate with")
            .required(true)
            .index(1))
        .arg(clap::Arg::new("branch")
            .short('b')
            .long("branch")
            .value_name("BRANCH")
            .about("Push the generated SVG to BRANCH. Default: master")
            .takes_value(true))
        .get_matches();

    let mut p = profile::Profile::default();
    let client = github::Client::withToken(matches.value_of("TOKEN").unwrap())?;
    p.getData(&client).await?;
    let svg = p.genSvg();

    let branch =
        if let Some(b) = matches.value_of("branch") {b} else {"master"};
    let username = client.getLogin().await?;
    let _ = client.commitSingleFile(&username, &username, branch, "profile.svg",
                                    &svg, "Update profile SVG").await?;

    Ok(())
}
