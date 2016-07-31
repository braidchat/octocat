use uuid::Uuid;

use conf;

#[derive(Clone)]
pub struct GeneralConf {
    pub port: i64,
    pub db_name: String,
}

#[derive(Clone)]
pub struct BraidConf {
    pub name: String,
    pub api_url: String,
    pub site_url: String,
    pub app_id: String,
    pub token: String,
}

#[derive(Clone)]
pub struct GithubConf {
    pub webhook_secret: String,
}

#[derive(Clone)]
pub struct RepoConf {
    pub token: String,
    pub org: String,
    pub repo: String,
    pub tag_id: Uuid,
}

#[derive(Clone)]
pub struct AppConf {
    pub general: GeneralConf,
    pub braid: BraidConf,
    pub github: GithubConf,
    pub repos: Vec<RepoConf>,
}

pub fn load_conf(conf_filename: &str) -> AppConf {
    let conf = conf::load_conf(conf_filename)
        .expect("Couldn't load conf file!");
    conf::validate_conf_group(&conf, "general", &["port", "db_name"]);
    conf::validate_conf_group(&conf, "braid",
                              &["name", "api_url", "app_id", "token",
                                "site_url"]);
    conf::validate_conf_group(&conf, "github", &["webhook_secret"]);
    // Can unwrap below, since we've validated keys up here
    let general = GeneralConf {
        port: conf::get_conf_val_n(&conf, "general", "port").unwrap(),
        db_name: conf::get_conf_val(&conf, "general", "db_name").unwrap(),
    };
    let braid = BraidConf {
        name: conf::get_conf_val(&conf, "braid", "name")
            .unwrap().to_owned(),
        api_url: conf::get_conf_val(&conf, "braid", "api_url")
            .unwrap().to_owned(),
        site_url: conf::get_conf_val(&conf, "braid", "site_url")
            .unwrap().to_owned(),
        app_id: conf::get_conf_val(&conf, "braid", "app_id")
            .unwrap().to_owned(),
        token: conf::get_conf_val(&conf, "braid", "token")
            .unwrap().to_owned(),
    };
    let github = GithubConf {
        webhook_secret: conf::get_conf_val(&conf, "github", "webhook_secret")
            .unwrap().to_owned(),
    };
    let mut repos = vec![];
    for r in conf.get("repos").and_then(|r| r.as_slice())
        .expect("Missing conf for repos!") {
            let t = r.as_table()
                .expect("repos should be a list of tables");
            let rc = RepoConf {
                token: t.get("token").and_then(|t| t.as_str())
                    .expect("Repo missing token").to_owned(),
                    org: t.get("org").and_then(|t| t.as_str())
                        .expect("Repo missing org").to_owned(),
                        repo: t.get("repo").and_then(|t| t.as_str())
                            .expect("Repo missing repo name").to_owned(),
                            tag_id: t.get("tag_id")
                                .and_then(|t| t.as_str())
                                .and_then(|id| Uuid::parse_str(id).ok())
                                .expect("Repo missing braid tag_id").to_owned(),

            };
            repos.push(rc);
        }
    AppConf {
        general: general,
        braid: braid,
        github: github,
        repos: repos,
    }
}
