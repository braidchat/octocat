use std::io::Read;
use std::collections::BTreeMap;
use hyper::Url;
use hyper::method::Method;
use hyper::client::{Client,Response};
use hyper::header::{Headers,ContentType};
use hyper::error::Result as HttpResult;
use mime::{Mime,TopLevel,SubLevel};
use serde_json;
use serde_json::value::Value as JsonValue;


static GITHUB_API_URL: &'static str = "https://api.github.com";

fn send_github_request(token: &str, endpoint: &str, data: BTreeMap<String, String>) -> HttpResult<Response> {
    let mut url_str = String::from(GITHUB_API_URL);
    url_str.push_str(endpoint);
    let mut headers = Headers::new();
    headers.set(ContentType::json());
    let client = Client::new();
    client.post(url_str.as_str())
        .body("")
        .headers(headers)
        .send()
}

pub fn create_issue(content: String) -> Option<String> {
    None
}
