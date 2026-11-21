use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Deserialize, Clone, Debug)]
pub struct Owner {
    pub login: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Repo {
    pub name: String,
    pub full_name: String,
    pub description: Option<String>,
    pub html_url: String,
    pub language: Option<String>,
    pub stargazers_count: u32,
    pub forks_count: u32,
    pub open_issues_count: u32,
    pub updated_at: String,
    pub private: bool,
    pub fork: bool,
    pub archived: bool,
    #[serde(default)]
    pub topics: Vec<String>,
    pub owner: Owner,
}

#[derive(Deserialize, Clone, Debug)]
pub struct User {
    pub login: String,
    pub name: Option<String>,
}

fn get(token: &str, url: &str) -> Result<ureq::Response, String> {
    ureq::get(url)
        .set("Authorization", &format!("Bearer {token}"))
        .set("User-Agent", "vitrin-app")
        .set("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| match e {
            ureq::Error::Status(401, _) => "Token geçersiz veya süresi dolmuş (401)".into(),
            ureq::Error::Status(403, _) => "Erişim reddedildi / rate limit (403)".into(),
            other => other.to_string(),
        })
}

pub fn fetch_user(token: &str) -> Result<User, String> {
    get(token, "https://api.github.com/user")?
        .into_json()
        .map_err(|e| e.to_string())
}

/// Kişisel, iş birliği yapılan ve organizasyon depolarının tamamı (sayfalı).
pub fn fetch_repos(token: &str) -> Result<Vec<Repo>, String> {
    let mut all = Vec::new();
    for page in 1..=20 {
        let url = format!(
            "https://api.github.com/user/repos?per_page=100&page={page}&sort=updated&affiliation=owner,collaborator,organization_member"
        );
        let batch: Vec<Repo> = get(token, &url)?.into_json().map_err(|e| e.to_string())?;
        let n = batch.len();
        all.extend(batch);
        if n < 100 {
            break;
        }
    }
    Ok(all)
}

pub fn fetch_languages(token: &str, full_name: &str) -> Result<BTreeMap<String, u64>, String> {
    get(token, &format!("https://api.github.com/repos/{full_name}/languages"))?
        .into_json()
        .map_err(|e| e.to_string())
}
