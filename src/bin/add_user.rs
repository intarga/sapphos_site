fn main() {
    let args: Vec<String> = std::env::args().collect();
    let username = args[1].clone();
    let password_hash = password_auth::generate_hash(args[2].clone());

    // FIXME: Sqlite by doesn't handle multiple processes inserting very well.
    // For now we can run this when the site is down safely, or take the risk of collision when
    // it's up (our site is very quiet so odds of collision are extremely low)
    // ultimately we should probably set up a busy lock timeout or wal mode to mitigate this
    let conn = rusqlite::Connection::open("db/db.sqlite3").expect("failed to open db...");
    conn.execute(
        "INSERT INTO users (username, password_hash) VALUES ($1, $2)",
        (username, password_hash),
    )
    .expect("failed to insert");
}
