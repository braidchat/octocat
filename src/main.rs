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
// braid requests
extern crate hyper;
extern crate mime;
extern crate serde_json;
// configuration
extern crate toml;

use std::env;
use std::process;

use iron::{Iron,Request,IronError};
use iron::{method,status};

mod conf;
mod routing;
mod message;
mod github;
mod braid;
mod handler;


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
    // Start server
    println!("Bot {:?} starting", braid_conf.get("name").unwrap().as_str().unwrap());
    Iron::new(move |request : &mut Request| {
        let req_path = request.url.path().join("/");
        match request.method {
            method::Put => {
                if req_path == "message" {
                    handler::handle_braid_message(request, conf.clone())
                } else {
                    Err(IronError::new(routing::NoRoute, status::NotFound))
                }
            }
            method::Post => {
                if req_path == "issue" {
                    handler::handle_github_webhook(request, conf.clone())
                } else {
                    Err(IronError::new(routing::NoRoute, status::NotFound))
                }
            }
            _ => Err(IronError::new(routing::NoRoute, status::NotFound))
        }
    }).http(&bind_addr[..]).unwrap();
}
