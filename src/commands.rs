use regex::Regex;

use conf;
use message;
use braid;
use github;

fn strip_leading_name(msg: &str) -> String {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"^/(\w+)\b").unwrap();
    }
    RE.replace(msg, "")
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

    braid::send_braid_request(message::response_to(msg, help), &braid_conf);

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
            braid::send_braid_request(msg, &braid_conf);
        }
        Err(e) => {
            println!("Error loading repos: {}", e);
        }
    }
}

fn create_github_issue(msg: message::Message, conf: conf::TomlConf) {
    let braid_conf = conf::get_conf_group(&conf, "braid")
        .expect("Missing braid config information");

    let body = strip_leading_name(&msg.content[..]);
    let mut words = body.split_whitespace();
    let repo_conf = words.nth(1)
        .and_then(|s| github::find_repo_conf(s, &conf));
    let issue_title = words.collect::<Vec<_>>().join(" ");
    if let Some(repo_conf) = repo_conf {
        let content = format!(
            "Created by octocat bot on behalf of {} from [braid chat]({})",
            braid::get_user_nick(msg.user_id, &braid_conf).unwrap_or("a braid user".to_owned()),
            braid::thread_url(&braid_conf, &msg));
        let gh_resp = github::create_issue(repo_conf, issue_title, content);
        if let Some(gh_issue) = gh_resp {
            // Opened webhook from github will open thread on braid
            println!("Issue opened: {:?}", gh_issue);
        } else {
            println!("Couldn't create issue");
            let err_resp = "Couldn't create issue, sorry".to_owned();
            braid::send_braid_request(message::response_to(msg, err_resp), &braid_conf);
        }
    } else {
        println!("Couldn't parse repo name");
        let err_resp = "Don't know which repo you mean, sorry".to_owned();
        braid::send_braid_request(message::response_to(msg, err_resp), &braid_conf);
    }
}
