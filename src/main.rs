#![allow(non_snake_case)]
use std::collections::HashSet;

#[macro_use]
mod error;
mod github;

use crate::error::Error;

#[tokio::main(flavor = "current_thread")]
pub async fn main() -> Result<(), Error>
{
    let client = github::Client::withToken(&std::env::args().nth(1).unwrap())?;
    let repo_count = client.getRepoCount().await?;
    let mut ignored_langs: HashSet<String> = HashSet::new();
    ignored_langs.insert(String::from("HTML"));
    let usage = github::topLanguages(
        client.getOverallLangs(repo_count).await?, 5, ignored_langs);

    for (lang, size) in usage
    {
        println!("{}: {}B", lang, size);
    }
    Ok(())
}
