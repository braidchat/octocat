use std::error::Error;
use hyper::header::{Headers,ContentType,Authorization,Basic};
use hyper::client::Client;
use hyper::status::StatusCode;
use mime::{Mime,TopLevel,SubLevel};

use conf;
use message;

pub fn send_braid_request(message: message::Message, braid_conf: &conf::TomlConf)
{
    let api_url = braid_conf.get("api_url").unwrap().as_str().unwrap();
    let bot_id = braid_conf.get("app_id").unwrap().as_str().unwrap().to_owned();
    let token = braid_conf.get("token").unwrap().as_str().unwrap().to_owned();
    let body = message::encode_transit_msgpack(message)
        .expect("Couldn't encode body to send!");
    let client = Client::new();
    let mut headers = Headers::new();
    headers.set(ContentType(Mime(TopLevel::Application,
                              SubLevel::Ext("transit+msgpack".to_owned()),
                              vec![])));
    headers.set(Authorization(Basic{username: bot_id, password: Some(token)}));
    match client.put(api_url).body(&body[..]).headers(headers).send() {
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

pub fn thread_url(braid_conf: &conf::TomlConf, msg: &message::Message) -> String {
    let site_url = braid_conf.get("site_url").unwrap().as_str().unwrap();
    format!("{}/{}/thread/{}", site_url, msg.group_id, msg.thread_id)
}
