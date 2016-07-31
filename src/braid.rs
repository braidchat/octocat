use std::io::Read;
use std::error::Error;
use hyper::header::{Headers,ContentType,Authorization,Basic};
use hyper::client::Client;
use hyper::status::StatusCode;
use mime::{Mime,TopLevel,SubLevel};
use uuid::Uuid;

use app_conf::BraidConf;
use message;

pub fn send_braid_request(message: message::Message, braid_conf: &BraidConf)
{
    let api_url = format!("{}/bots/message", braid_conf.api_url);
    let body = message::encode_transit_msgpack(message)
        .expect("Couldn't encode body to send!");
    let client = Client::new();
    let mut headers = Headers::new();
    headers.set(ContentType(Mime(TopLevel::Application,
                              SubLevel::Ext("transit+msgpack".to_owned()),
                              vec![])));
    headers.set(Authorization(Basic{
        username: braid_conf.app_id.clone(),
        password: Some(braid_conf.token.clone())}));
    match client.put(&api_url[..]).body(&body[..]).headers(headers).send() {
        Ok(r) => {
            println!("Sent message to braid");
            if r.status == StatusCode::Created {
                println!("Message created!");
            } else {
                println!("Something went wrong: {:?}", r);
            }
        }
        Err(e) =>
            println!("Failed to send to braid: {:?}", e.description()),

    }
}

pub fn get_user_nick(user_id: Uuid, braid_conf: &BraidConf) -> Option<String> {
    let api_url = format!("{}/bots/names/{}", braid_conf.api_url,
                          user_id.hyphenated().to_string());
    let mut headers = Headers::new();
    headers.set(Authorization(Basic {
        username: braid_conf.app_id.clone(),
        password: Some(braid_conf.token.clone())}));
    let client = Client::new();
    match client.get(&api_url[..]).headers(headers).send() {
        Ok(mut r) => {
            if r.status == StatusCode::Ok {
                let mut buf = String::new();
                r.read_to_string(&mut buf).ok().and(Some(buf))
            } else {
                println!("Something went wrong: {:?}", r);
                None
            }
        }
        Err(e) => {
            println!("Failed to get from braid: {:?}", e.description());
            None
        }

    }
}

pub fn thread_url(braid_conf: &BraidConf, msg: &message::Message) -> String {
    format!("{}/{}/thread/{}", braid_conf.site_url, msg.group_id, msg.thread_id)
}

pub fn start_watching_thread(thread_id: Uuid, braid_conf: &BraidConf) {
    let api_url = format!("{}/bots/subscribe/{}", braid_conf.api_url,
                          thread_id.hyphenated().to_string());
    let mut headers = Headers::new();
    headers.set(Authorization(Basic {
        username: braid_conf.app_id.clone(),
        password: Some(braid_conf.token.clone())}));
    let client = Client::new();
    match client.put(&api_url[..]).headers(headers).send() {
        Ok(r) => {
            println!("Sent message to braid");
            if r.status == StatusCode::Created {
                println!("Getting notifications from braid for thread");
            } else {
                println!("Something went wrong: {:?}", r);
            }
        }
        Err(e) =>
            println!("Failed to send to braid: {:?}", e.description()),

    }
}
