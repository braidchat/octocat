use std::error::Error;
use uuid::Uuid;
use regex::Regex;
use hyper::status::StatusCode;

use conf;
use message;
use braid;
use github;
use tracking;

fn strip_leading_name(msg: &str) -> String {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"^/(\w+)\b").unwrap();
    }
    RE.replace(msg, "")
}

fn send_to_braid(msg: message::Message, braid_conf: &conf::TomlConf) {
    match braid::send_braid_request(braid_conf, msg) {
        Ok(r) => {
            println!("Sent message to braid");
            if r.status == StatusCode::Created {
                println!("Message created!");
            } else {
                println!("Something went wrong: {:?}", r);
            }
        }
        Err(e) =>
            println!("Failed to send to braid: {:?}",
                     e.description()),
    }
}

pub fn parse_command(msg: message::Message, conf: conf::TomlConf) {
    let body = strip_leading_name(&msg.content[..]);
    if let Some(command) = body.split_whitespace().next() {
        match &command[..] {
            "list" => send_repos_list(msg, conf),
            "create" => create_github_issue(msg, conf),
            "help" | _ => send_help_response(msg, conf),
        }
    }
}

fn send_help_response(msg: message::Message, conf: conf::TomlConf) {
    let bot_name = conf::get_conf_val(&conf, "braid", "name")
        .expect("Bot needs a name");

    let braid_conf = conf::get_conf_group(&conf, "braid")
        .expect("Missing braid config information");

    let mut help = String::new();
    help.push_str("I know the following commands:\n");
    help.push_str(format!("'/{} help' will make me respond with this message\n", bot_name).as_str());
    help.push_str(format!("'/{} list' will get you the connected github repos\n", bot_name).as_str());
    help.push_str(format!("'/{} create <repo> <text...>' and I'll create an issue in <repo> with the title 'text...'\n", bot_name).as_str());

    send_to_braid(message::response_to(msg, help), &braid_conf);

}

fn get_repos(conf: &conf::TomlConf) -> Result<Vec<String>, String> {
    let mut repo_list = vec![];
    let repos = try!(conf.get("repos")
                     .and_then(|r| r.as_slice())
                     .ok_or("Repos should be a list of tables"));
    for repo in repos {
        let repo = try!(repo.as_table().ok_or("repo isn't a table?"));
        let org = try!(repo.get("org")
                       .and_then(|o| o.as_str())
                       .ok_or("Repo is missing org"));
        let repo = try!(repo.get("repo")
                        .and_then(|r| r.as_str())
                        .ok_or("Repo is missing repo"));
        repo_list.push(format!("{}/{}", org, repo));
    }
    Ok(repo_list)
}

fn send_repos_list(msg: message::Message, conf: conf::TomlConf) {
    let braid_conf = conf::get_conf_group(&conf, "braid")
        .expect("Missing braid config information");
    let mut reply = String::from("I know about the following repos\n");
    match get_repos(&conf) {
        Ok(repos) => {
            for r in repos {
                reply.push_str(r.as_str());
                reply.push_str("\n");
            }
            let msg = message::response_to(msg, reply);
            send_to_braid(msg, &braid_conf);
        }
        Err(e) => {
            println!("Error loading repos: {}", e);
        }
    }
}

fn find_repo_conf(name: String, conf: &conf::TomlConf) -> Option<&conf::TomlConf> {
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

fn create_github_issue(msg: message::Message, conf: conf::TomlConf) {
    let braid_conf = conf::get_conf_group(&conf, "braid")
        .expect("Missing braid config information");

    let body = strip_leading_name(&msg.content[..]);
    let mut words = body.split_whitespace();
    let repo_conf = words.nth(1)
        .and_then(|s| find_repo_conf(s.to_owned(), &conf));
    let issue_title = words.collect::<Vec<_>>().join(" ");
    if let Some(repo_conf) = repo_conf {
        let content = format!("Created by octocat bot from [braid chat]({})",
        braid::thread_url(&braid_conf, &msg));
        let group_id = msg.group_id;
        let gh_resp = github::create_issue(repo_conf, issue_title, content);
        if let Some(gh_issue) = gh_resp {
            let braid_content = format!("New issue opened: {}", gh_issue.url);
            let braid_response_tag = repo_conf.get("tag_id").and_then(|t| t.as_str())
                .expect("Couldn't load braid response tag id");
            let braid_response_tag_id = Uuid::parse_str(braid_response_tag)
                .expect("Couldn't parse tag uuid");
            let response_msg = message::new_thread_msg(group_id,
                                                       braid_response_tag_id,
                                                       braid_content);
            tracking::add_watched_thread(response_msg.thread_id, gh_issue.number);
            send_to_braid(response_msg, &braid_conf);
        } else {
            println!("Couldn't create issue");
            let err_resp = "Couldn't create issue, sorry".to_owned();
            send_to_braid(message::response_to(msg, err_resp), &braid_conf);
        }
    } else {
        println!("Couldn't parse repo name");
        let err_resp = "Don't know which repo you mean, sorry".to_owned();
        send_to_braid(message::response_to(msg, err_resp), &braid_conf);
    }
}
