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
// tracking braid thread <-> github issues
extern crate rusqlite;

use std::env;
use std::process;

use iron::{Iron,Request,IronError};
use iron::{method,status};

mod app_conf;
mod conf;
mod routing;
mod message;
mod github;
mod braid;
mod handler;
mod commands;
mod tracking;


fn main() {
    let args: Vec<_> = env::args().collect();
    if args.len() <= 1 {
        println!("Usage: {} <configuration toml file>", args[0]);
        process::exit(1);
    }
    // Load configuration
    let conf_filename = &args[1];
    let conf = app_conf::load_conf(&conf_filename[..]);
    tracking::setup_tables(&conf);
    // Start server
    let bind_addr = format!("localhost:{}", conf.general.port);
    println!("Bot {:?} starting", conf.braid.name);
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
