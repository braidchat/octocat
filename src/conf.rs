use toml;
use std::collections::BTreeMap;
use std::io::Read;
use std::fs::File;

fn slurp(file_name: &str) -> Result<String, String> {
    let mut s = String::new();
    match File::open(file_name).and_then(|mut f| { f.read_to_string(&mut s) }) {
        Ok(_) => Ok(s),
        Err(_) => Err("Couldn't open file to read".to_owned())
    }
}

pub type TomlConf = BTreeMap<String, toml::Value>;

pub fn load_conf(file_name: &str) -> Result<TomlConf, String> {
    let contents = try!(slurp(file_name).map_err(|e| e.to_string()));
    toml::Parser::new(&contents).parse().ok_or("Couldn't parse TOML".to_owned())
}

pub fn get_conf_val(conf: &TomlConf, group: &str, key: &str) -> Option<String> {
    conf.get(group)
        .and_then(|v| v.as_table())
        .and_then(|tbl| tbl.get(key))
        .and_then(|key_v| key_v.as_str())
        .map(|s| s.to_owned())
}

pub fn get_conf_val_n(conf: &TomlConf, group: &str, key: &str) -> Option<i64> {
    conf.get(group)
        .and_then(|v| v.as_table())
        .and_then(|tbl| tbl.get(key))
        .and_then(|key_v| key_v.as_integer())
}

pub fn get_conf_group(conf: &TomlConf, group: &str) -> Option<toml::Table> {
    conf.get(group).and_then(|v| v.as_table()).cloned()
}

pub fn validate_conf_group(conf: &TomlConf, group: &str, keys: &[&str]) {
    let grp = get_conf_group(conf, group)
        .expect(&format!("Mssing configuration for {}", group)[..]);
    for k in keys {
        if !grp.contains_key(*k) {
            panic!("Missing {} configuration key '{}'", group, k);
        }
    }
}
