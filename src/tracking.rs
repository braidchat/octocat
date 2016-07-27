use uuid::Uuid;
use rusqlite::Connection;

pub fn setup_tables() {
    let conn = Connection::open("threads_issues.sqlite")
        .expect("Couldn't open database!");
    conn.execute("CREATE TABLE IF NOT EXISTS watched_threads (
                    thread_id TEXT NOT NULL UNIQUE,
                    issue_number INTEGER NOT NULL UNIQUE
                 )", &[])
        .expect("Couldn't create the table");
}

pub fn add_watched_thread(thread_id: Uuid, issue_number: i64) {
    let conn = Connection::open("threads_issues.sqlite")
        .expect("Couldn't open database!");

    conn.execute("INSERT INTO watched_threads (thread_id, issue_number)
                  VALUES ($1, $2)",
                  &[&thread_id.simple().to_string(), &issue_number])
        .expect("Couldn't add watched thread");
}
