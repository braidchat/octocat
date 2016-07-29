use conf::TomlConf;
use std::io::Read;
use std::collections::BTreeMap;
use hyper::client::{Client,Response};
use hyper::header::{Headers,ContentType,Authorization,Bearer,UserAgent};
use hyper::error::Result as HttpResult;
use serde_json;
use serde_json::value::{Value as JsonValue,Map};
use uuid::Uuid;

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
    headers.set(Authorization(Bearer { token: token.to_owned() }));
    headers.set(UserAgent("braidchat/octocat".to_owned()));
    let body = serde_json::to_string(&data).expect("Can't serialize data");
    let client = Client::new();
    client.post(url_str.as_str())
        .body(&body[..])
        .headers(headers)
        .send()
}

pub fn find_repo_conf<'a>(name: &str, conf: &'a TomlConf) -> Option<&'a TomlConf> {
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

fn new_issue_from_webhook(issue_number: i64,
                          payload: BTreeMap<String, JsonValue>,
                          conf: TomlConf)
{
    let repo_name = match payload.get("repository")
        .and_then(|r| r.as_object())
        .and_then(|r| r.get("full_name"))
        .and_then(|n| n.as_string()) {
            Some(r) => r,
            None => {
                println!("Couldn't get repository from message");
                return
            }
        };
    if tracking::thread_for_issue(repo_name.to_owned(), issue_number).is_some() {
        println!("Already tracking this issue");
        return
    }
    let repo_conf = match find_repo_conf(repo_name, &conf) {
        Some(c) => c,
        None => {
            println!("Couldn't find conf for {}", repo_name);
            return
        }
    };

    let issue = match payload.get("issue") {
        Some(i) => i,
        None => { println!("No issue in payload!"); return }
    };

    let creator = match issue.find_path(&["user", "login"])
        .and_then(|u| u.as_string()) {
            Some(u) => u,
            None => { println!("Missing creator name"); return }
        };
    let issue_title = match issue.find("title")
        .and_then(|t| t.as_string()) {
            Some(t) => t,
            None => { println!("Missing issue title"); return }
        };
    let issue_url = match issue.find("html_url")
        .and_then(|u| u.as_string()) {
            Some(u) => u,
            None => { println!("Missing issue url"); return }
        };
    let content = format!("{} opened issue \"{}\"\n{}",
                          creator, issue_title, issue_url);

    let braid_response_tag = repo_conf.get("tag_id").and_then(|t| t.as_str())
        .expect("Couldn't load braid response tag id");
    let braid_response_tag_id = Uuid::parse_str(braid_response_tag)
        .expect("Couldn't parse tag uuid");
    let msg = message::new_thread_msg(braid_response_tag_id, content);
    let braid_conf = conf::get_conf_group(&conf, "braid")
        .expect("Missing braid config information");
    tracking::add_watched_thread(msg.thread_id, repo_name.to_owned(), issue_number);
    braid::send_braid_request(msg.clone(), &braid_conf);
    braid::start_watching_thread(msg.thread_id, &braid_conf);
}

fn comment_from_webhook(issue_number: i64, repo_name: &str,
                        update: BTreeMap<String, JsonValue>, conf: TomlConf) {
    println!("Update to issue {:?}", issue_number);
    let thread_id = match tracking::thread_for_issue(repo_name.to_owned(), issue_number) {
        Some(thread) => thread.thread_id,
        None => {
            return
        }
    };
    // TODO: check that this wasn't a comment we posted
    let comment = match update.get("comment") {
        Some(comment) => comment,
        None => { println!("No comment in issue!"); return }
    };
    let commenter = match comment.find_path(&["user", "login"])
        .and_then(|u| u.as_string()) {
            Some(c) => c,
            None => { println!("Missing commenter"); return }
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

pub fn update_from_github(msg_body: Vec<u8>, conf: TomlConf) {
    match serde_json::from_slice(&msg_body[..]) {
        Err(e) => println!("Couldn't parse update json: {:?}", e),
        Ok(update) => {
            // TODO: need to also check if it's issue closing/opening or whatever
            let update: BTreeMap<String, JsonValue> = update;
            let repo_name = match update.get("repository")
                .and_then(|r| r.as_object())
                .and_then(|r| r.get("full_name"))
                .and_then(|n| n.as_string()) {
                    Some(r) => r,
                    None => {
                        println!("Couldn't get repository from message");
                        return
                    }
                };
            let issue_number = match update.get("issue")
                .and_then(|i| i.as_object())
                .and_then(|i| i.get("number"))
                .and_then(|n| n.as_i64()) {
                    Some(i) => i,
                    None => { println!("Couldn't get issue #"); return }
                };
            let action = update.get("action").expect("Missing action").as_string().unwrap();
            match action {
                "opened" => new_issue_from_webhook(issue_number, update.clone(), conf),
                "created" => comment_from_webhook(issue_number, repo_name, update.clone(), conf),
                _ => println!("Unknown action from webhook {}", action),
            }
        }
    }
}

pub fn update_from_braid(thread: tracking::WatchedThread, msg: message::Message, conf: TomlConf)
{
    let braid_conf = conf::get_conf_group(&conf, "braid")
        .expect("Missing braid config information");
    let comment_user = braid::get_user_nick(msg.user_id, &braid_conf)
        .unwrap_or("some braid user".to_owned());

    let repo_name = thread.repository;
    let repo_conf = match find_repo_conf(&repo_name[..], &conf) {
        Some(conf) => conf,
        None => {
            println!("Couldn't find conf for repo {}", repo_name);
            return
        }
    };
    let token = match repo_conf.get("token").and_then(|token| token.as_str()) {
        Some(tok) => tok,
        None => { println!("Missing token for {}", repo_name); return }
    };

    let path = format!("/repos/{}/issues/{}/comments", repo_name,
                       thread.issue_number);

    let comment = format!("{} commented via [braid]({}):\n{}",
                          comment_user,
                          braid::thread_url(&braid_conf, &msg),
                          msg.content);
    let mut map = Map::new();
    map.insert(String::from("body"), JsonValue::String(comment));
    let data = JsonValue::Object(map);
    match send_github_request(token, &path[..], data) {
        Err(e) => {
            println!("Couldn't comment on issue: {:?}", e);
        }
        Ok(_) => {
            println!("Commented on issue");
        }
    }
}
