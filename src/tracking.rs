use uuid::Uuid;
use rusqlite::Connection;

#[derive(Debug)]
pub struct WatchedThread {
    pub thread_id: Uuid,
    pub group_id: Uuid,
    pub issue_number: i64,
}

pub fn setup_tables() {
    let conn = Connection::open("threads_issues.sqlite")
        .expect("Couldn't open database!");
    conn.execute("CREATE TABLE IF NOT EXISTS watched_threads (
                    thread_id TEXT NOT NULL UNIQUE,
                    group_id TEXT NOT NULL,
                    issue_number INTEGER NOT NULL UNIQUE
                 )", &[])
        .expect("Couldn't create the table");
}

pub fn add_watched_thread(thread_id: Uuid, group_id: Uuid, issue_number: i64) {
    let conn = Connection::open("threads_issues.sqlite")
        .expect("Couldn't open database!");

    conn.execute("INSERT INTO watched_threads (thread_id, group_id, issue_number)
                  VALUES ($1, $2, $3)",
                  &[&thread_id.simple().to_string(),
                    &group_id.simple().to_string(),
                    &issue_number])
        .expect("Couldn't add watched thread");
}

pub fn thread_for_issue(issue_number: i64) -> Option<WatchedThread> {
    let conn = Connection::open("threads_issues.sqlite")
        .expect("Couldn't open database!");

    match conn.query_row("SELECT thread_id, group_id FROM watched_threads
                    WHERE issue_number = $0", &[&issue_number],
                    |row| {
                        let th = row.get::<_, String>(0);
                        let gr = row.get::<_, String>(1);
                        WatchedThread {
                            issue_number: issue_number,
                            thread_id: Uuid::parse_str(&th[..])
                                .expect("Couldn't parse thread id!"),
                            group_id: Uuid::parse_str(&gr[..])
                                .expect("Couldn't parse group id!"),
                        }
                    })
    {
        Ok(wt) => Some(wt),
        Err(e) => {
            println!("Error looking up thread with issue {}: {:?}",
                     issue_number, e);
            None
        }
    }
}
