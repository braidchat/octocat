use std::thread;
use std::io::Read;
use iron::{Request,Response,IronError};
use iron::status;
use openssl::crypto::hmac;
use openssl::crypto::hash::Type;
// to make from_hex on strings work
use rustc_serialize::hex::FromHex;

use conf;
use routing;
use message;
use commands;

fn verify_hmac(mac: Vec<u8>, key: &[u8], data: &[u8]) -> bool {
    if let Some(mac) = String::from_utf8(mac).ok()
        .and_then(|mac_str| (&mac_str[..]).from_hex().ok()) {
            let generated: Vec<u8> = hmac::hmac(Type::SHA256, key, data).to_vec();
            mac == generated
        } else {
            false
        }
}

pub fn handle_braid_message(request: &mut Request, conf: conf::TomlConf) -> Result<Response,IronError> {
    // Verify MAC
    let mac = try!(request.headers.get_raw("X-Braid-Signature")
                   .and_then(|h| h.get(0))
                   .ok_or(IronError::new(routing::MissingMac,
                                         status::Unauthorized)));

    let braid_token = conf::get_conf_val(&conf, "braid", "token").unwrap();

    let mut buf = Vec::new();
    request.body.read_to_end(&mut buf).unwrap();
    if !verify_hmac(mac.clone(), braid_token.as_bytes(), &buf[..]) {
        println!("Bad mac");
        return Err(IronError::new(routing::BadMac, status::Forbidden));
    }
    println!("Mac OK");
    match message::decode_transit_msgpack(buf) {
        Some(msg) => {
            thread::spawn(move || { commands::parse_command(msg, conf); });
        },
        None => println!("Couldn't parse message")
    }
    Ok(Response::with((status::Ok, "ok")))
}

pub fn handle_github_webhook(request: &mut Request, conf: conf::TomlConf) -> Result<Response,IronError> {
    Ok(Response::with((status::Ok, "ok")))
}
