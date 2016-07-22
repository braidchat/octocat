#![feature(custom_derive, plugin)]
#![plugin(serde_macros)]

// main
#[macro_use] extern crate iron;
extern crate regex;
#[macro_use] extern crate lazy_static;
extern crate openssl;
extern crate rustc_serialize;
// Message parsing
extern crate rmp;
extern crate rmp_serde;
extern crate serde;
extern crate uuid;
extern crate byteorder;
// giphy/braid requests
extern crate hyper;
extern crate mime;
extern crate serde_json;
// configuration
extern crate toml;

use std::io::Read;
use std::thread;
use std::error::Error;
use std::env;
use std::process;

use iron::{Iron,Request,Response,IronError};
use iron::{method,status};
use hyper::status::StatusCode;
use regex::Regex;
use openssl::crypto::hmac;
use openssl::crypto::hash::Type;
use rustc_serialize::hex::FromHex;
use uuid::Uuid;

mod conf;
mod routing;
mod message;
mod github;
mod braid;

fn strip_leading_name(msg: &str) -> String {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"^/(\w+)\b").unwrap();
    }
    RE.replace(msg, "")
}

fn verify_hmac(mac: Vec<u8>, key: &[u8], data: &[u8]) -> bool {
    if let Some(mac) = String::from_utf8(mac).ok()
        .and_then(|mac_str| (&mac_str[..]).from_hex().ok()) {
            let generated: Vec<u8> = hmac::hmac(Type::SHA256, key, data).to_vec();
            mac == generated
        } else {
            false
        }
}

fn main() {
    let args: Vec<_> = env::args().collect();
    if args.len() <= 1 {
        println!("Usage: {} <configuration toml file>", args[0]);
        process::exit(1);
    }
    // Load configuration
    let ref conf_filename = args[1];
    let conf = conf::load_conf(&conf_filename[..]).expect("Couldn't load conf file!");
    let bind_port = conf::get_conf_val(&conf, "general", "port")
        .expect("Missing key port in section general");
    let bind_addr = format!("localhost:{}", bind_port);
    let braid_conf = conf::get_conf_group(&conf, "braid")
        .expect("Missing braid config information");
    let keys = ["name", "api_url", "app_id", "token"];
    for i in 0..keys.len() {
        let k = keys[i];
        if !braid_conf.contains_key(k) {
            panic!("Missing braid configuration key '{}'", k);
        }
    }
    let braid_token = conf::get_conf_val(&conf, "braid", "token").unwrap();
    let github_conf = conf::get_conf_group(&conf, "github").expect("Couldn't load github conf");
    let braid_response_tag = conf::get_conf_val(&conf, "braid", "tag_id").expect("Couldn't load braid response tag id");
    let braid_response_tag_id = Uuid::parse_str(braid_response_tag.as_str()).expect("Couldn't parse tag uuid");
    // Start server
    println!("Bot {:?} starting", braid_conf.get("name").unwrap().as_str().unwrap());
    Iron::new(move |request : &mut Request| {
        let req_path = request.url.path().join("/");
        match request.method {
            method::Put => {
                if req_path == "message" {
                    // Verify MAC
                    let mac = try!(request.headers.get_raw("X-Braid-Signature")
                                   .and_then(|h| h.get(0))
                                   .ok_or(IronError::new(routing::MissingMac,
                                                         status::Unauthorized)));
                    let mut buf = Vec::new();
                    request.body.read_to_end(&mut buf).unwrap();
                    if !verify_hmac(mac.clone(), braid_token.as_bytes(), &buf[..]) {
                        println!("Bad mac");
                        return Err(IronError::new(routing::BadMac, status::Forbidden));
                    }
                    println!("Mac OK");
                    match message::decode_transit_msgpack(buf) {
                        Some(msg) => {
                            let braid_conf = braid_conf.clone();
                            let github_conf = github_conf.clone();
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
                                    braid::send_braid_request(&braid_conf, response_msg);
                                } else {
                                    println!("Couldn't create issue");
                                }
                            });
                        },
                        None => println!("Couldn't parse message")
                    }
                    Ok(Response::with((status::Ok, "ok")))
                } else {
                    Err(IronError::new(routing::NoRoute, status::NotFound))
                }
            }
            _ => Err(IronError::new(routing::NoRoute, status::NotFound))
        }
    }).http(&bind_addr[..]).unwrap();
}
