use std::thread;
use std::io::Read;
use std::error::Error;
use iron::{Request,Response,IronError};
use iron::status;
use hyper::status::StatusCode;
use uuid::Uuid;
use regex::Regex;
use openssl::crypto::hmac;
use openssl::crypto::hash::Type;
// to make from_hex on strings work
use rustc_serialize::hex::FromHex;

use conf;
use routing;
use braid;
use github;
use message;

fn verify_hmac(mac: Vec<u8>, key: &[u8], data: &[u8]) -> bool {
    if let Some(mac) = String::from_utf8(mac).ok()
        .and_then(|mac_str| (&mac_str[..]).from_hex().ok()) {
            let generated: Vec<u8> = hmac::hmac(Type::SHA256, key, data).to_vec();
            mac == generated
        } else {
            false
        }
}

fn strip_leading_name(msg: &str) -> String {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"^/(\w+)\b").unwrap();
    }
    RE.replace(msg, "")
}

pub fn handle_braid_message(request: &mut Request, conf: conf::TomlConf) -> Result<Response,IronError> {
    // Verify MAC
    let mac = try!(request.headers.get_raw("X-Braid-Signature")
                   .and_then(|h| h.get(0))
                   .ok_or(IronError::new(routing::MissingMac,
                                         status::Unauthorized)));

    let braid_token = conf::get_conf_val(&conf, "braid", "token").unwrap();
    let braid_response_tag = conf::get_conf_val(&conf, "braid", "tag_id").expect("Couldn't load braid response tag id");
    let braid_response_tag_id = Uuid::parse_str(braid_response_tag.as_str()).expect("Couldn't parse tag uuid");

    let mut buf = Vec::new();
    request.body.read_to_end(&mut buf).unwrap();
    if !verify_hmac(mac.clone(), braid_token.as_bytes(), &buf[..]) {
        println!("Bad mac");
        return Err(IronError::new(routing::BadMac, status::Forbidden));
    }
    println!("Mac OK");
    match message::decode_transit_msgpack(buf) {
        Some(msg) => {
            let braid_conf = conf::get_conf_group(&conf, "braid")
                .expect("Missing braid config information");
            let github_conf = conf::get_conf_group(&conf, "github")
                .expect("Couldn't load github conf");
            thread::spawn(move || {
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
                    match braid::send_braid_request(&braid_conf, response_msg) {
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
                } else {
                    println!("Couldn't create issue");
                }
            });
        },
        None => println!("Couldn't parse message")
    }
    Ok(Response::with((status::Ok, "ok")))
}
