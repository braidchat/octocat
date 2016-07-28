use conf::TomlConf;
use std::io::Read;
use std::collections::BTreeMap;
use hyper::client::{Client,Response};
use hyper::header::{Headers,ContentType,Authorization,Bearer,UserAgent};
use hyper::error::Result as HttpResult;
use serde_json;
use serde_json::value::{Value as JsonValue,Map};

use conf;
use tracking;
use braid;
use message;

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

pub fn find_repo_conf(name: String, conf: &TomlConf) -> Option<&TomlConf> {
    if name.contains('/') {
        let mut split = name.splitn(2, '/');
        let org = split.next().unwrap();
        let repo = split.next().unwrap();
        conf.get("repos")
            .and_then(|r| r.as_slice())
            .and_then(|rs| {
                let mut it = rs.iter();
                it.find(|r| {
                    let t = r.as_table();
                    let o = t.and_then(|r| r.get("org"))
                        .and_then(|n| n.as_str())
                        .map_or(false, |r_org| r_org == org);
                    let r = t.and_then(|r| r.get("repo"))
                        .and_then(|n| n.as_str())
                        .map_or(false, |r_repo| r_repo == repo);
                    o && r
                }).and_then(|found| found.as_table())
            })
    } else {
        conf.get("repos")
            .and_then(|r| r.as_slice())
            .and_then(|rs| {
                let mut it = rs.iter();
                it.find(|r| {
                    r.as_table()
                        .and_then(|r| r.get("repo"))
                        .and_then(|n| n.as_str())
                        .map_or(false, |r_name| r_name == name)
                }).and_then(|found| found.as_table())
            })
    }
}

pub struct GithubIssue {
    pub url: String,
    pub number: i64,
}

pub fn create_issue(github_conf: &TomlConf, title: String, content: String)
    -> Option<GithubIssue>
{
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
        Err(e) => { println!("Error fetching from github: {:?}", e); None }
        Ok(mut resp) => {
            let mut buf = String::new();
            match resp.read_to_string(&mut buf) {
                Err(_) => { println!("Couldn't read response"); None },
                Ok(_) => {
                    match serde_json::from_str(&buf[..]) {
                        Ok(new_issue) => {
                            let new_issue: BTreeMap<String, JsonValue> = new_issue;
                            let url = new_issue.get("html_url")
                                .and_then(|url| url.as_string() );
                            let number: Option<i64> = new_issue.get("number")
                                .and_then(|n| { n.as_i64() });
                            if let (Some(u), Some(n)) = (url, number) {
                                Some(GithubIssue { url: u.to_owned(), number: n })
                            } else {
                                None
                            }
                        }
                        Err(e) => { println!("Failed to parse json: {:?}", e); None }
                    }
                }
            }
        }
    }
}

pub fn update_from_github(msg_body: Vec<u8>, conf: TomlConf) {
    match serde_json::from_slice(&msg_body[..]) {
        Err(e) => println!("Couldn't parse update json: {:?}", e),
        Ok(update) => {
            let update: BTreeMap<String, JsonValue> = update;
            let repo = update.get("repository")
                .and_then(|r| r.as_object())
                .and_then(|r| r.get("full_name"))
                .and_then(|n| n.as_string() );
            let issue_number = update.get("issue")
                .and_then(|i| i.as_object())
                .and_then(|i| i.get("number"))
                .and_then(|n| n.as_i64());
            if repo.is_none() { println!("Couldn't get repo"); return }
            if issue_number.is_none() { println!("Couldn't get issue #"); return }
            let repo = repo.unwrap();
            let issue_number = issue_number.unwrap();
            println!("Update to issue {:?} in {:?}", issue_number, repo);
            let repo_conf = match find_repo_conf(repo.to_owned(), &conf) {
                Some(conf) => conf,
                None => { println!("No conf for repo {}", repo); return }
            };
            let thread_id = match tracking::thread_for_issue(issue_number) {
                Some(id) => id,
                None => {
                    println!("No thread for issue {} in {}", issue_number, repo);
                    return
                }
            };
            println!("conf {:?} thread id {:?}", repo_conf, thread_id);
            let comment = match update.get("comment") {
                Some(comment) => comment,
                None => { println!("No comment in issue!"); return }
            };
            let commenter = match comment.find_path(&["user", "login"])
                .and_then(|u| u.as_string()) {
                    Some(c) => c,
                    None => { println!("Missing commenter)"); return }
                };
            let comment_body = match comment.find("body")
                .and_then(|b| b.as_string()) {
                    Some(b) => b,
                    None => { println!("Missing comment body"); return }
                };
            let msg_body = format!("{} commented:\n{}", commenter, comment_body);
            let msg = message::reply_to_thread(thread_id, msg_body);
            let braid_conf = conf::get_conf_group(&conf, "braid")
                .expect("Missing braid config information");
            braid::send_braid_request(msg, &braid_conf);
        }
    }
}
