use std::error::Error;
use uuid::Uuid;
use regex::Regex;
use hyper::status::StatusCode;

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

fn send_to_braid(msg: message::Message, braid_conf: conf::TomlConf) {
    match braid::send_braid_request(&braid_conf, msg) {
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
            "help" => send_help_response(msg, conf),
            "list" => send_repos_list(msg, conf),
            "create" => create_github_issue(msg, conf),
            _ => send_help_response(msg, conf)
        }
    }
}

fn send_help_response(msg: message::Message, conf: conf::TomlConf) {
    let bot_name = conf::get_conf_val(&conf, "braid", "name")
        .expect("Bot needs a name");
    let mut help = String::new();
    help.push_str("I know the following commands:\n");
    help.push_str(format!("'/{} help' will make me respond with this message\n", bot_name).as_str());
    help.push_str(format!("'/{} list' will get you the connected github repos\n", bot_name).as_str());
    help.push_str(format!("'/{} create <repo> <text...>' and I'll create an issue in <repo> with the title 'text...'\n", bot_name).as_str());
    let msg = message::response_to(msg, help);

    let braid_conf = conf::get_conf_group(&conf, "braid")
        .expect("Missing braid config information");
    send_to_braid(msg, braid_conf);
}

fn send_repos_list(msg: message::Message, conf: conf::TomlConf) {
    let mut reply = String::new();
    let msg = message::response_to(msg, reply);
}

fn create_github_issue(msg: message::Message, conf: conf::TomlConf) {
    let braid_response_tag = conf::get_conf_val(&conf, "braid", "tag_id")
        .expect("Couldn't load braid response tag id");
    let braid_response_tag_id = Uuid::parse_str(braid_response_tag.as_str())
        .expect("Couldn't parse tag uuid");
    let braid_conf = conf::get_conf_group(&conf, "braid")
        .expect("Missing braid config information");
    let github_conf = conf::get_conf_group(&conf, "github")
        .expect("Couldn't load github conf");

    // TODO: get repo from message

    let content = format!("Created by octocat bot from [braid chat]({})",
    braid::thread_url(&braid_conf, &msg));
    let group_id = msg.group_id;
    let gh_resp = github::create_issue(
        &github_conf,
        strip_leading_name(&msg.content[..]),
        content);
    if let Some(url) = gh_resp {
        let braid_content = format!("New issue opened: {}", url);
        let response_msg = message::new_thread_msg(group_id,
                                                   braid_response_tag_id,
                                                   braid_content);
        send_to_braid(response_msg, braid_conf);
    } else {
        println!("Couldn't create issue");
    }
}
