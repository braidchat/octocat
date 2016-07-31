use std::io::Read;
use std::collections::BTreeMap;
use hyper::client::{Client,Response};
use hyper::header::{Headers,ContentType,Authorization,Bearer,UserAgent};
use hyper::error::Result as HttpResult;
use serde_json;
use serde_json::value::{Value as JsonValue,Map};

use app_conf::{AppConf,RepoConf};
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

pub fn find_repo_conf<'a>(name: &str, conf: &'a AppConf) -> Option<&'a RepoConf> {
    if name.contains('/') {
        let mut split = name.splitn(2, '/');
        let org = split.next().unwrap();
        let repo = split.next().unwrap();
        for r in &conf.repos {
            if r.repo == repo && r.org == org {
                return Some(r)
            }
        }
        None
    } else {
        for r in &conf.repos {
            if r.repo == name {
                return Some(r)
            }
        }
        None
    }
}

#[derive(Debug)]
pub struct GithubIssue {
    pub url: String,
    pub number: i64,
}

pub fn create_issue(github_conf: &RepoConf, title: String, content: String)
    -> Option<GithubIssue>
{
    let token = github_conf.token.clone();
    let owner = github_conf.org.clone();
    let repo = github_conf.repo.clone();
    let mut path = String::from("/repos/");
    path.push_str(&owner[..]);
    path.push_str("/");
    path.push_str(&repo[..]);
    path.push_str("/issues");

    let mut map = Map::new();
    map.insert(String::from("title"), JsonValue::String(title));
    map.insert(String::from("body"), JsonValue::String(content));
    let data = JsonValue::Object(map);

    match send_github_request(&token[..], path.as_str(), data) {
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

fn new_issue_from_webhook(issue_number: i64, payload: JsonValue, conf: AppConf)
{
    let repo_name = match payload.find_path(&["repository", "full_name"])
        .and_then(|n| n.as_string()) {
            Some(r) => r,
            None => {
                println!("Couldn't get repository from message");
                return
            }
        };
    if tracking::thread_for_issue(repo_name.to_owned(), issue_number, &conf)
        .is_some()
        {
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

    let issue = match payload.find("issue") {
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

    let braid_response_tag_id = repo_conf.tag_id;
    let msg = message::new_thread_msg(braid_response_tag_id, content);
    let braid_conf = conf.braid.clone();
    tracking::add_watched_thread(msg.thread_id, repo_name.to_owned(),
                                 issue_number, &conf);
    braid::send_braid_request(msg.clone(), &braid_conf);
    braid::start_watching_thread(msg.thread_id, &braid_conf);
}

fn comment_from_webhook(issue_number: i64, repo_name: &str, update: JsonValue, conf: AppConf) {
    println!("Update to issue {:?}", issue_number);
    let thread_id = match tracking::thread_for_issue(repo_name.to_owned(),
                                                     issue_number,
                                                     &conf)
    {
        Some(thread) => thread.thread_id,
        None => {
            println!("Not tracking this issue though");
            return
        }
    };
    let comment = match update.find("comment") {
        Some(comment) => comment,
        None => { println!("No comment in issue!"); return }
    };
    let comment_id = match comment.find("id")
        .and_then(|i| i.as_i64()) {
        Some(i) => i,
        None => { println!("Missing comment id!"); return }
    };
    if tracking::did_we_post_comment(thread_id, comment_id, &conf) {
        println!("webhook for our own comment");
        return
    }
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
    braid::send_braid_request(msg, &conf.braid);
}

fn closed_issue_from_webhook(issue_number: i64, repo_name: &str, update: JsonValue, conf: AppConf) {
    println!("Issue {} in {} closed", issue_number, repo_name);
    let thread_id = match tracking::thread_for_issue(repo_name.to_owned(),
                                                     issue_number, &conf)
    {
        Some(thread) => thread.thread_id,
        None => {
            println!("Not tracking this issue though");
            return
        }
    };
    let closer = update.find_path(&["sender", "login"])
        .and_then(|u| u.as_string())
        .unwrap_or("an unknown user");
    let msg_body = format!("issue has been closed by {}", closer);
    let msg = message::reply_to_thread(thread_id, msg_body);
    braid::send_braid_request(msg, &conf.braid);
}

pub fn update_from_github(msg_body: Vec<u8>, conf: AppConf) {
    match serde_json::from_slice(&msg_body[..]) {
        Err(e) => println!("Couldn't parse update json: {:?}", e),
        Ok(update) => {
            let update: JsonValue = update;
            let repo_name = match update.find_path(&["repository", "full_name"])
                .and_then(|n| n.as_string()) {
                    Some(r) => r,
                    None => {
                        println!("Couldn't get repository from message");
                        return
                    }
                };
            let issue_number = match update.find_path(&["issue", "number"])
                .and_then(|n| n.as_i64()) {
                    Some(i) => i,
                    None => { println!("Couldn't get issue #"); return }
                };
            let action = match update.find("action")
                .and_then(|a| a.as_string()) {
                    Some(a) => a,
                    None => { println!("Couldn't get issue action!"); return }
                };
            match action {
                "opened" => new_issue_from_webhook(issue_number, update.clone(), conf),
                "created" => comment_from_webhook(issue_number, repo_name, update.clone(), conf),
                "closed" => closed_issue_from_webhook(issue_number, repo_name, update.clone(), conf),
                _ => println!("Unknown action from webhook {}", action),
            }
        }
    }
}

pub fn update_from_braid(thread: tracking::WatchedThread, msg: message::Message, conf: AppConf)
{
    let comment_user = braid::get_user_nick(msg.user_id, &conf.braid)
        .unwrap_or("some braid user".to_owned());

    let repo_name = thread.repository;
    let repo_conf = match find_repo_conf(&repo_name[..], &conf) {
        Some(conf) => conf,
        None => {
            println!("Couldn't find conf for repo {}", repo_name);
            return
        }
    };
    let token = repo_conf.token.clone();
    let path = format!("/repos/{}/issues/{}/comments", repo_name,
                       thread.issue_number);

    let comment = format!("{} commented via [braid]({}):\n{}",
    comment_user,
    braid::thread_url(&conf.braid, &msg),
    msg.content);
    let mut map = Map::new();
    map.insert(String::from("body"), JsonValue::String(comment));
    let data = JsonValue::Object(map);
    match send_github_request(&token[..], &path[..], data) {
        Err(e) => println!("Error sending github request: {:?}", e),
        Ok(mut resp) => {
            let mut buf = String::new();
            match resp.read_to_string(&mut buf) {
                Err(e) => println!("Error reading github response: {:?}", e),
                Ok(_) => {
                    match serde_json::from_str(&buf[..]) {
                        Err(e) => println!("Couldn't parse json from github: {:?}", e),
                        Ok(new_comment) => {
                            let new_comment: JsonValue = new_comment;
                            if let Some(id) = new_comment.find("id").and_then(|i| i.as_i64()) {
                                tracking::track_comment(msg.thread_id,
                                                        id,
                                                        &conf);
                            } else {
                                println!("Couldn't get comment id");
                            }
                        }
                    }
                }
            }
        }
    }
}
