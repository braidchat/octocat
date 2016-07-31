use std::thread;
use std::io::Read;
use iron::{Request,Response,IronError};
use iron::status;
use iron::error::HttpError;
use openssl::crypto::hmac;
use openssl::crypto::hash::Type;
// to make from_hex on strings work
use rustc_serialize::hex::FromHex;

use app_conf::AppConf;
use routing;
use message;
use commands;
use github;
use tracking;

fn verify_braid_hmac(mac: Vec<u8>, key: &[u8], data: &[u8]) -> bool {
    if let Some(mac) = String::from_utf8(mac).ok()
        .and_then(|mac_str| (&mac_str[..]).from_hex().ok()) {
            let generated: Vec<u8> = hmac::hmac(Type::SHA256, key, data).to_vec();
            mac == generated
        } else {
            false
        }
}

fn verify_github_hmac(mac: Vec<u8>, key: &[u8], data: &[u8]) -> bool {
    let sig_str = String::from_utf8(mac).ok().unwrap_or_default();
    if let Some(mac) = sig_str
        .splitn(2, '=').last()
        .and_then(|mac_str| (&mac_str[..]).from_hex().ok()) {
        let generated: Vec<u8> = hmac::hmac(Type::SHA1, key, data).to_vec();
        mac == generated
    } else {
        false
    }
}


pub fn handle_braid_message(request: &mut Request, conf: AppConf) -> Result<Response,IronError> {
    // Verify MAC
    let mac = try!(request.headers.get_raw("X-Braid-Signature")
                   .and_then(|h| h.get(0))
                   .ok_or(IronError::new(routing::MissingMac,
                                         status::Unauthorized)));

    let braid_token = conf.braid.token.clone();
    let mut buf = Vec::new();
    request.body.read_to_end(&mut buf).unwrap(); // TODO: check
    if !verify_braid_hmac(mac.clone(), braid_token.as_bytes(), &buf[..]) {
        println!("Bad mac");
        return Err(IronError::new(routing::BadMac, status::Forbidden));
    }
    println!("Mac OK");
    match message::decode_transit_msgpack(buf) {
        Some(msg) => {
            thread::spawn(move || {
                if let Some(thread) = tracking::issue_for_thread(msg.thread_id,
                                                                 &conf)
                {
                    github::update_from_braid(thread, msg, conf);
                } else {
                    commands::parse_command(msg, conf);
                }
            });
        },
        None => println!("Couldn't parse message")
    }
    Ok(Response::with((status::Ok, "ok")))
}

pub fn handle_github_webhook(request: &mut Request, conf: AppConf) -> Result<Response,IronError> {
    let mac = try!(request.headers.get_raw("X-Hub-Signature")
                   .and_then(|h| h.get(0))
                   .ok_or(IronError::new(routing::MissingMac, status::Unauthorized)));

    let github_token = conf.github.webhook_secret.clone();
    let mut buf = Vec::new();
    match request.body.read_to_end(&mut buf) {
        Err(e) => {
            println!("Couldn't read github body: {:?}", e);
            Err(IronError::new(HttpError::Io(e), status::BadRequest))
        }
        Ok(_) => {
            if !verify_github_hmac(mac.clone(), github_token.as_bytes(), &buf[..]) {
                println!("Bad mac");
                return Err(IronError::new(routing::BadMac, status::Forbidden));
            }
            println!("Mac OK");

            thread::spawn(move || { github::update_from_github(buf, conf) });
            Ok(Response::with((status::Ok, "ok")))
        }
    }
}
