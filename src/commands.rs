use regex::Regex;

use app_conf::{AppConf};
use message;
use braid;
use github;

fn strip_leading_name(msg: &str) -> String {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"^/(\w+)\b").unwrap();
    }
    RE.replace(msg, "")
}

pub fn parse_command(msg: message::Message, conf: AppConf) {
    let body = strip_leading_name(&msg.content[..]);
    if let Some(command) = body.split_whitespace().next() {
        match &command[..] {
            "list" => send_repos_list(msg, conf),
            "create" => create_github_issue(msg, conf),
            "help" | _ => send_help_response(msg, conf),
        }
    }
}

fn send_help_response(msg: message::Message, conf: AppConf) {
    let bot_name = conf.braid.name.clone();
    let mut help = String::new();
    help.push_str("I know the following commands:\n");
    help.push_str(
        format!("'/{} help' will make me respond with this message\n",
                bot_name).as_str());
    help.push_str(
        format!("'/{} list' will get you the connected github repos\n",
                bot_name).as_str());
    help.push_str(
        format!("'/{} create <repo> <text...>' and I'll create an issue",
                bot_name).as_str());
    help.push_str("in <repo> with the title 'text...'\n");

    braid::send_braid_request(message::response_to(msg, help), &conf.braid);

}

fn send_repos_list(msg: message::Message, conf: AppConf) {
    let mut reply = String::from("I know about the following repos\n");
    for r in conf.repos {
        reply.push_str(&r.org[..]);
        reply.push_str("/");
        reply.push_str(&r.repo[..]);
        reply.push_str("\n");
    }
    let msg = message::response_to(msg, reply);
    braid::send_braid_request(msg, &conf.braid);
}

fn create_github_issue(msg: message::Message, conf: AppConf) {
    let braid_conf = conf.braid.clone();

    let body = strip_leading_name(&msg.content[..]);
    let mut words = body.split_whitespace();
    let repo_conf = words.nth(1)
        .and_then(|s| github::find_repo_conf(s, &conf));
    let issue_title = words.collect::<Vec<_>>().join(" ");
    if let Some(repo_conf) = repo_conf {
        let sender = braid::get_user_nick(msg.user_id, &braid_conf)
            .unwrap_or("a braid user".to_owned());
        let content = format!(
            "Created by octocat bot on behalf of {} from [braid chat]({})",
            sender,
            braid::thread_url(&braid_conf, &msg));
        let gh_resp = github::create_issue(repo_conf, issue_title, content);
        if let Some(gh_issue) = gh_resp {
            // Opened webhook from github will open thread on braid
            println!("Issue opened: {:?}", gh_issue);
        } else {
            println!("Couldn't create issue");
            let err_resp = "Couldn't create issue, sorry".to_owned();
            braid::send_braid_request(message::response_to(msg, err_resp),
                                      &braid_conf);
        }
    } else {
        println!("Couldn't parse repo name");
        let err_resp = "Don't know which repo you mean, sorry".to_owned();
        braid::send_braid_request(message::response_to(msg, err_resp),
                                  &braid_conf);
    }
}
