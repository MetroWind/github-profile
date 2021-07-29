#![allow(non_snake_case)]

#[macro_use]
mod error;
mod github;

use crate::error::Error;

#[tokio::main(flavor = "current_thread")]
pub async fn main() -> Result<(), Error>
{
    let client = github::Client::withToken(&std::env::args().nth(1).unwrap())?;
    let repo_count = client.getRepoCount().await?;
    client.getOverallLangs(repo_count).await?;
    Ok(())
}
