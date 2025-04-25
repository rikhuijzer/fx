use crate::ServeArgs;
use chrono::DateTime;
use chrono::NaiveDateTime;
use chrono::TimeZone;
use chrono::Utc;
use rusqlite::Connection;
use rusqlite::Result;

pub trait SqliteDateTime {
    const FORMAT: &str = "%Y-%m-%d %H:%M:%S";
    fn from_sqlite(text: &str) -> Self;
    fn to_sqlite(&self) -> String;
}

impl SqliteDateTime for DateTime<Utc> {
    fn from_sqlite(text: &str) -> Self {
        let s = text.trim().to_string();
        let naive = NaiveDateTime::parse_from_str(&s, Self::FORMAT).unwrap();
        Utc::from_utc_datetime(&Utc, &naive)
    }
    fn to_sqlite(&self) -> String {
        self.format(Self::FORMAT).to_string()
    }
}

#[test]
fn test_sqlite_datetime() {
    let dt = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
    println!("before: {}", dt);
    let text = dt.to_sqlite();
    println!("s: {}", text);
    let dt2 = chrono::DateTime::from_sqlite(&text);
    println!("after: {}", dt2);
    assert_eq!(dt, dt2);
}

/// Key-value entry.
#[derive(Clone, Debug)]
pub struct Kv {
    pub key: String,
    pub value: Vec<u8>,
}

impl Kv {
    pub fn create_table(conn: &Connection) -> Result<usize> {
        let stmt = "
            CREATE TABLE IF NOT EXISTS kv (key TEXT PRIMARY KEY, value BLOB)
        ";
        conn.execute(stmt, [])
    }
    pub fn insert(conn: &Connection, key: &str, value: &[u8]) -> Result<usize> {
        let stmt = &format!(
            "
            INSERT OR REPLACE INTO kv (key, value) VALUES ('{key}', ?)
        "
        );
        conn.execute(stmt, [value])
    }
    pub fn get(conn: &Connection, key: &str) -> Result<Vec<u8>> {
        let stmt = "SELECT key, value FROM kv WHERE key = ?";
        let value: Vec<u8> = conn
            .prepare(stmt)?
            .query_row([key], |row| row.get("value"))?;
        Ok(value)
    }
}

#[test]
fn test_kv() {
    let conn = Connection::open_in_memory().unwrap();
    Kv::create_table(&conn).unwrap();
    let key = "key";
    let value = b"value";
    Kv::insert(&conn, key, value).unwrap();
    let kv = Kv::get(&conn, key).unwrap();
    assert_eq!(kv, value);
}

#[derive(Clone, Debug)]
pub struct Post {
    /// The id of the post.
    pub id: i64,
    /// The date and time the post was created.
    pub created: chrono::DateTime<chrono::Utc>,
    /// The date and time the post was last updated.
    pub updated: chrono::DateTime<chrono::Utc>,
    /// The content of the post.
    pub content: String,
}

impl Post {
    fn create_table(conn: &Connection) -> Result<usize> {
        let stmt = "
            CREATE TABLE IF NOT EXISTS posts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                created DATETIME NOT NULL,
                updated DATETIME NOT NULL,
                content TEXT NOT NULL
            );
        ";
        conn.execute(stmt, [])
    }
    pub fn insert(
        conn: &Connection,
        created: DateTime<Utc>,
        updated: DateTime<Utc>,
        content: &str,
    ) -> Result<i64> {
        let stmt = "
            INSERT INTO posts (created, updated, content)
            VALUES (?, ?, ?);
        ";
        let created = created.to_sqlite();
        let updated = updated.to_sqlite();
        let content = content.to_string();
        conn.execute(stmt, [created, updated, content])?;
        let id = conn.last_insert_rowid();
        Ok(id)
    }
    pub fn list(conn: &Connection) -> Result<Vec<Post>> {
        let stmt = "
            SELECT id, created, updated, content
            FROM posts
            ORDER BY created DESC;
        ";
        let posts = conn
            .prepare(stmt)?
            .query_map([], |row| {
                let created_str: String = row.get("created")?;
                let created = DateTime::from_sqlite(&created_str);
                let updated_str: String = row.get("updated")?;
                let updated = DateTime::from_sqlite(&updated_str);
                let content = row.get("content")?;
                Ok(Post {
                    id: row.get("id")?,
                    created,
                    updated,
                    content,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(posts)
    }
    pub fn get(conn: &Connection, id: i64) -> Result<Post> {
        let stmt = "
            SELECT id, created, updated, content FROM posts WHERE id = ?;
        ";
        conn.prepare(stmt)?.query_row([id], |row| {
            let created_str: String = row.get("created")?;
            let created = DateTime::from_sqlite(&created_str);
            let updated_str: String = row.get("updated")?;
            let updated = DateTime::from_sqlite(&updated_str);
            let content = row.get("content")?;
            Ok(Post {
                id: row.get("id")?,
                created,
                updated,
                content,
            })
        })
    }
    pub fn update(&self, conn: &Connection) -> Result<usize> {
        let stmt = "
            UPDATE posts SET created = ?, updated = ?, content = ?
            WHERE id = ?;
        ";
        let created = self.created.to_sqlite();
        let updated = self.updated.to_sqlite();
        let content = self.content.to_string();
        let id = self.id.to_string();
        conn.execute(stmt, [created, updated, content, id])
    }
    pub fn delete(conn: &Connection, id: i64) -> Result<usize> {
        let stmt = "DELETE FROM posts WHERE id = ?";
        conn.execute(stmt, [id])
    }
}

pub fn connect(args: &ServeArgs) -> Result<Connection> {
    let conn = if args.production {
        let path = &args.database_path;
        Connection::open(path)?
    } else {
        Connection::open_in_memory()?
    };
    Ok(conn)
}

fn init_tables(conn: &Connection) {
    Post::create_table(conn).expect("Failed to create posts table");
    Kv::create_table(conn).expect("Failed to create kv table");
}

fn init_kv(conn: &Connection, key: &str, value: &[u8]) {
    if Kv::get(conn, key).is_err() {
        Kv::insert(conn, key, value).unwrap();
    }
}

fn init_data(args: &ServeArgs, conn: &Connection) {
    init_kv(conn, "site_name", b"My Weblog");
    let about = if args.production { "" } else { "About" };
    init_kv(conn, "about", about.as_bytes());

    if !args.production {
        let now = chrono::Utc::now();
        let content = "[Lorem](https://example.com/lorem) ipsum ut enim ad \
        minim veniam sit amet ipsum lorem consectetur adipiscing elit sed do eiusmod";
        Post::insert(conn, now, now, content).unwrap();
        let now = chrono::Utc::now();
        let content = indoc::indoc! {r#"
            # Code

            Dolor sit amet, consectetur adipiscing elit, sed do
            eiusmod tempor incididunt ut `labore` et dolore magna aliqua.

            Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris
            nisi ut aliquip ex ea commodo consequat.

            ```rust
            x = 1;

            println!("{x}");
            ```
        "#}
        .trim();
        Post::insert(conn, now, now, content).unwrap();
    }
}

pub fn init(args: &ServeArgs, conn: &Connection) {
    init_tables(conn);
    init_data(args, conn);
}
