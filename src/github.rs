use conf::TomlConf;
use std::io::Read;
use std::collections::BTreeMap;
use hyper::Url;
use hyper::method::Method;
use hyper::client::{Client,Response};
use hyper::header::{Headers,ContentType,Authorization,Bearer,UserAgent};
use hyper::error::Result as HttpResult;
use serde_json;
use serde_json::value::{Value as JsonValue,Map};


static GITHUB_API_URL: &'static str = "https://api.github.com";

fn send_github_request(token: &str, endpoint: &str, data: JsonValue) -> HttpResult<Response> {
    let mut url_str = String::from(GITHUB_API_URL);
    url_str.push_str(endpoint);
    let mut headers = Headers::new();
    headers.set(ContentType::json());
    // XXX: Github seems to want the authorization to be "token ..." instead of Bearer
    headers.set(Authorization(Bearer { token: token.to_owned() }));
    headers.set(UserAgent("braidchat/octocat".to_owned()));
    let body = serde_json::to_string(&data).expect("Can't serialize data");
    let client = Client::new();
    client.post(url_str.as_str())
        .body(&body[..])
        .headers(headers)
        .send()
}

pub fn create_issue(github_conf: &TomlConf, title: String, content: String) -> Option<String> {
    let token = github_conf.get("token").and_then(|token| token.as_str()).expect("Missing GitHub token");

    let owner = github_conf.get("org").and_then(|org| org.as_str()).expect("Missing GitHub org");
    let repo = github_conf.get("repo").and_then(|repo| repo.as_str()).expect("Missing GitHub repo");
    let mut path = String::from("/repos/");
    path.push_str(owner);
    path.push_str("/");
    path.push_str(repo);
    path.push_str("/issues");

    let mut map = Map::new();
    map.insert(String::from("title"), JsonValue::String(title));
    map.insert(String::from("body"), JsonValue::String(content));
    let data = JsonValue::Object(map);

    match send_github_request(token, path.as_str(), data) {
        Err(e) => { println!("Error fetching from github"); None }
        Ok(mut resp) => {
            let mut buf = String::new();
            match resp.read_to_string(&mut buf) {
                Err(_) => { println!("Couldn't read response"); None },
                Ok(_) => {
                    match serde_json::from_str(&buf[..]) {
                        Ok(new_issue) => {
                            let new_issue: BTreeMap<String, JsonValue> = new_issue;
                            new_issue.get("html_url")
                                .and_then(|url| {
                                    match url {
                                        &JsonValue::String(ref s) => Some(s.to_owned()),
                                        _ => None
                                    }
                                })
                        }
                        Err(e) => { println!("Failed to parse json: {:?}", e); None }
                    }
                }
            }
        }
    }
}
