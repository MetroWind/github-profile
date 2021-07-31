use std::collections::{HashMap, HashSet};

use serde_json::json;
use serde::Serialize;
use reqwest;

use crate::error::Error;

type VarMap<'a> = HashMap<&'a str, serde_json::Value>;
pub type LangUsage = HashMap<String, u64>;

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

fn makePayload(query: &str, variables: &VarMap) ->
    Result<serde_json::Value, Error>
{
    let vars_json = serde_json::to_value(variables).map_err(
        |_| rterr!("Failed to convert VarMap to JSON."))?;
    Ok(json!({"variables": vars_json, "query": query}))
}

struct CommitHash
{
    commit_hash: String,
    tree_hash: String,
}

impl CommitHash
{
    pub fn new(commit_hash: &str, tree_hash: &str) -> Self
    {
        Self {
            commit_hash: commit_hash.to_owned(),
            tree_hash: tree_hash.to_owned(),
        }
    }
}

struct FileHash
{
    path: String,
    hash: String,
}

impl FileHash
{
    fn new(path: &str, hash: &str) -> Self
    {
        Self {
            path: path.to_owned(),
            hash: hash.to_owned(),
        }
    }
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


    async fn get(&self, uri: &str) -> Result<serde_json::Value, Error>
    {
        let res = self.client.get(uri).send().await.map_err(
                |e| rterr!("Failed to send request: {}", e))?
            .error_for_status().map_err(|e| rterr!("Query failed: {}", e))?;
        res.json().await.map_err(|_| rterr!("Failed to deserialize response"))
    }

    async fn post<T: Serialize + ?Sized>(&self, uri: &str, data: &T) ->
        Result<serde_json::Value, Error>
    {
        let res = self.client.post(uri).json(&data).send().await.map_err(
            |e| rterr!("Failed to send request: {}", e))?;
        let uri: String = res.url().as_str().to_owned();
        let status = res.status();
        let payload: serde_json::Value = res.json().await.map_err(
            |_| rterr!("Failed to deserialize response"))?;
        if !status.is_success()
        {
            Err(rterr!("Request failed at URI {} with code {}. Payload:\n{}",
                       uri, status.as_u16(),
                       serde_json::to_string_pretty(&payload).unwrap()))
        }
        else
        {
            Ok(payload)
        }
    }

    /// Make a GraphQL query.
    async fn query(&self, q: &str, vars: &VarMap<'_>) ->
        Result<serde_json::Value, Error>
    {
        self.post("https://api.github.com/graphql", &makePayload(q, vars)?)
            .await
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
        let mut usage = LangUsage::new();
        for repo in data["data"]["viewer"]["repositories"]["edges"].as_array()
            .ok_or_else(|| rterr!("Invalid repositories"))?
        {
            for lang_edge in repo["node"]["languages"]["edges"].as_array()
                .ok_or_else(|| rterr!("Invalid languages"))?
            {
                let size = lang_edge["size"].as_u64().ok_or_else(
                    || rterr!("Invalid language size"))?;
                let lang = lang_edge["node"]["name"].as_str().ok_or_else(
                    || rterr!("Invalid language name"))?;
                if let Some(s) = usage.get_mut(lang)
                {
                    *s += size;
                }
                else
                {
                    usage.insert(lang.to_owned(), size);
                }
            }
        }
        Ok(usage)
    }

    pub async fn getLogin(&self) -> Result<String, Error>
    {
        let data = self.query(include_str!("../graphql/viewer-login.graphql"),
                              &noVars()).await?;
        Ok(data["data"]["viewer"]["login"].as_str().ok_or_else(
            || rterr!("Failed to extract user login"))?.to_owned())
    }

    /// Get the HEAD commit of a repo.
    async fn getHead(&self, owner: &str, name: &str) ->
        Result<CommitHash, Error>
    {
        let data = self.query(include_str!("../graphql/head.graphql"),
                              &varMap!(("name": name),
                                       ("owner": owner))).await?;
        Ok(CommitHash::new(
            data["data"]["repository"]["object"]["oid"].as_str()
                .ok_or_else(|| rterr!("Failed to extract commit hash"))?,
            data["data"]["repository"]["object"]["tree"]["oid"].as_str()
                 .ok_or_else(|| rterr!("Failed to extract tree hash"))?))
    }

    /// Return the hash of the new tree.
    async fn createTree(&self, owner: &str, repo: &str, path: &str,
                        base_tree: &str, content: &str) -> Result<String, Error>
    {
        let uri = format!("https://api.github.com/repos/{}/{}/git/trees",
                          owner, repo);
        let payload = json!({
            "base_tree": base_tree,
            "tree": [{
                "path": path,
                "mode": "100644",
                "type": "blob",
                "content": content,
            }]});
        let data = self.post(&uri, &payload)
            .await?;
        Ok(data["sha"].as_str().ok_or_else(
            || rterr!("Failed to extract hash from new tree"))?.to_owned())
    }

    /// Return hash of the new commit.
    pub async fn commitSingleFile(&self, owner: &str, repo: &str, branch: &str,
                                  path: &str, content: &str, msg: &str) ->
        Result<String, Error>
    {
        // Create tree
        let head = self.getHead(owner, repo).await?;
        let new_tree = self.createTree(owner, repo, path, &head.tree_hash,
                                       content).await?;
        // Create commit
        let uri = format!("https://api.github.com/repos/{}/{}/git/commits",
                          owner, repo);
        let payload = json!({
            "message": msg,
            "tree": new_tree,
            "parents": [head.commit_hash],
            "author": {
                "name": "Profile Bot",
                "email": "metrowind@github.com"
            }});

        let data = self.post(&uri, &payload).await?;
        let new_commit = data["sha"].as_str().ok_or_else(
            || rterr!("Failed to extract hash from new commit"))?;

        // Update reference
        let uri =
            format!("https://api.github.com/repos/{}/{}/git/refs/heads/{}",
                    owner, repo, branch);
        let payload = json!({ "sha": new_commit });
        let _ = self.post(&uri, &payload).await?;
        Ok(new_commit.to_owned())
    }
}

pub fn topLanguages(mut usage: LangUsage, top_n: usize,
                    ignores: &HashSet<String>) -> Vec<(String, u64)>
{
    let mut langs: Vec<(String, u64)> = Vec::new();
    for (lang, size) in usage.drain()
    {
        langs.push((lang, size));
    }
    langs.sort_by(
        |pair1, pair2| pair1.1.partial_cmp(&pair2.1).unwrap().reverse());
    langs.drain(..).filter(|pair| ignores.get(&pair.0).is_none()).take(top_n)
        .collect()
}
