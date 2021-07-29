use std::collections::HashMap;

use serde_json::json;
use reqwest;

use crate::error::Error;

type VarMap<'a> = HashMap<&'a str, serde_json::Value>;
pub type LangUsage = HashMap<String, usize>;

macro_rules! varMap
{
    ( $( ( $k:literal : $v:expr ) ),* ) => {
        {
            let mut vars: VarMap = VarMap::new();
            $(
                vars.insert($k, serde_json::Value::from($v));
            )*
            vars
        }
    };
}

fn noVars() -> VarMap<'static>
{
    VarMap::new()
}

fn makePayload(query: &str, variables: &VarMap) -> Result<String, Error>
{
    // let vars = json!{"variables": variables};

    let vars_json = serde_json::to_value(variables).map_err(
        |_| rterr!("Failed to convert VarMap to JSON."))?;
    let data = json!({"variables": vars_json, "query": query});
    let r: String = serde_json::to_string_pretty(&data).map_err(
        |_| rterr!("Failed to serialize request"))?;
    println!("{}", r);
    Ok(r)
}

pub struct Client
{
    client: reqwest::Client,
}

impl Client
{
    pub fn withToken(token: &str) -> Result<Self, Error>
    {
        use reqwest::header::{HeaderMap, HeaderValue};
        let mut headers = HeaderMap::new();
        headers.insert("Authorization", HeaderValue::from_str(
            &format!("bearer {}", token)).map_err(
            |_| rterr!("Token is invalid header value"))?);
        headers.insert("User-Agent", HeaderValue::from_static("metrowind"));
        let client = reqwest::Client::builder().default_headers(headers)
            .build().map_err(
                |e| rterr!("Failed to create HTTP client: {}", e))?;
        Ok(Self { client: client })
    }

    /// Make a GraphQL query.
    async fn query(&self, q: &str, vars: &VarMap<'_>) ->
        Result<serde_json::Value, Error>
    {
        let res = self.client.post("https://api.github.com/graphql")
            .body(makePayload(q, vars)?).send().await.map_err(
                |e| rterr!("Failed to send request: {}", e))?
            .error_for_status().map_err(|e| rterr!("Query failed: {}", e))?;
        res.json().await.map_err(|_| rterr!("Failed to deserialize response"))
    }

    pub async fn getRepoCount(&self) -> Result<u64, Error>
    {
        let data = self.query(include_str!("../graphql/repo-count.graphql"),
                              &noVars()).await?;
        data["data"]["viewer"]["repositories"]["totalCount"].as_u64()
            .ok_or_else(|| rterr!("Invalid repo count"))
    }

    pub async fn getOverallLangs(&self, repo_count: u64) ->
        Result<LangUsage, Error>
    {
        let data = self.query(include_str!("../graphql/langs.graphql"),
                              &varMap!(("count": repo_count))).await?;
        println!("{}", serde_json::to_string_pretty(&data).unwrap());
        Ok(LangUsage::new())
    }
}