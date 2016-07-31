use uuid::Uuid;
use rusqlite::Connection;

use app_conf::AppConf;

fn get_conn(conf: &AppConf) -> Connection {
    Connection::open(&conf.general.db_name[..])
        .expect("Couldn't open database!")
}

pub struct WatchedThread {
    pub thread_id: Uuid,
    pub issue_number: i64,
    pub repository: String,
}

pub fn setup_tables(conf: &AppConf) {
    let conn = get_conn(conf);
    conn.execute_batch("BEGIN;
                        CREATE TABLE IF NOT EXISTS watched_threads (
                         thread_id TEXT NOT NULL UNIQUE,
                         issue_number INTEGER NOT NULL,
                         repository TEXT NOT NULL
                        );

                        CREATE TABLE IF NOT EXISTS posted_comments (
                         thread_id TEXT NOT NULL,
                         comment_id INTEGER NOT NULL
                        );

                        CREATE UNIQUE INDEX IF NOT EXISTS repo_idx
                         ON watched_threads (repository, issue_number);
                        COMMIT;")
        .expect("Couldn't create the table");
}

pub fn add_watched_thread(thread_id: Uuid,
                          repo: String,
                          issue_number: i64,
                          conf: &AppConf)
{
    let conn = get_conn(conf);

    match conn.execute("INSERT INTO watched_threads (thread_id, issue_number, repository)
                  VALUES ($1, $2, $3)",
                  &[&thread_id.simple().to_string(), &issue_number, &repo]) {
        Ok(_) => { println!("Watching thread {}, {}, {}", thread_id, repo, issue_number); }
        Err(e) => { println!("Couldn't save watched thread {} {} {}: {:?}",
                             thread_id, repo, issue_number, e);
        }
    }

}

pub fn thread_for_issue(repo: String, issue_number: i64, conf: &AppConf) -> Option<WatchedThread>
{
    let conn = get_conn(conf);

    match conn.query_row("SELECT thread_id FROM watched_threads
                    WHERE repository = $0 AND issue_number = $1",
                    &[&repo, &issue_number],
                    |row| row.get::<_, String>(0)) {
        Ok(thread_id) => Uuid::parse_str(&thread_id[..])
            .ok()
            .map(|t_id| WatchedThread {
                thread_id: t_id,
                repository: repo,
                issue_number: issue_number,
            }),
        Err(e) => {
            println!("Couldn't find thread for issue: {:?}", e);
            None
        }
    }
}


pub fn issue_for_thread(thread_id: Uuid, conf: &AppConf) -> Option<WatchedThread> {
    let conn = get_conn(conf);

    match conn.query_row(
        "SELECT issue_number, repository FROM watched_threads
         WHERE thread_id = $0",
         &[&thread_id.simple().to_string()],
         |row| WatchedThread {
             thread_id: thread_id,
             issue_number: row.get::<_, i64>(0),
             repository: row.get::<_, String>(1),
         })
    {
        Ok(issue) => Some(issue),
        Err(e) => {
            println!("Couldn't find issue for thread: {:?}", e);
            None
        }
    }
}

pub fn track_comment(thread_id: Uuid, comment_id: i64, conf: &AppConf) {
    let conn = get_conn(conf);

    match conn.execute("INSERT INTO posted_comments (thread_id, comment_id)
                        VALUES ($1, $2)",
                        &[&thread_id.simple().to_string(), &comment_id]) {
        Ok(_) => { println!("Tracking posted comment {} from {}", comment_id, thread_id); },
        Err(e) => { println!("Couldn't track comment: {:?}", e); }
    }
}

pub fn did_we_post_comment(thread_id: Uuid, comment_id: i64, conf: &AppConf) -> bool
{
    let conn = get_conn(conf);
    match conn.query_row("SELECT count(*) FROM posted_comments
                           WHERE thread_id = $0 AND comment_id = $1",
                          &[&thread_id.simple().to_string(), &comment_id],
                          |row| row.get::<_, i64>(0) != 0) {
        Ok(c) => c,
        Err(_) => false,
    }
}
