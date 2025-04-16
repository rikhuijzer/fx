use crate::ServeArgs;
use chrono::DateTime;
use chrono::NaiveDateTime;
use chrono::TimeZone;
use chrono::Utc;
use rusqlite::Connection;
use rusqlite::Result;

pub trait SqliteDateTime {
    const FORMAT: &str = "%Y-%m-%d %H:%M:%S";
    #[allow(dead_code)]
    fn from_sqlite(s: &str) -> Self;
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
        let stmt = "CREATE TABLE IF NOT EXISTS kv (key TEXT PRIMARY KEY, value BLOB)";
        conn.execute(stmt, [])
    }
    pub fn insert(conn: &Connection, key: &str, value: &[u8]) -> Result<usize> {
        let stmt = &format!("INSERT INTO kv (key, value) VALUES ('{key}', ?)");
        conn.execute(stmt, [value])
    }
    pub fn get(conn: &Connection, key: &str) -> Result<Kv> {
        let stmt = "SELECT key, value FROM kv WHERE key = ?";
        let kv: Kv = conn.prepare(stmt)?.query_row([key], |row| {
            Ok(Kv {
                key: row.get("key")?,
                value: row.get("value")?,
            })
        })?;
        Ok(kv)
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
    assert_eq!(kv.value, value);
}

#[derive(Clone, Debug)]
pub struct Post {
    #[allow(dead_code)]
    pub id: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub content: String,
}

impl Post {
    fn create_table(conn: &Connection) -> Result<usize> {
        let stmt = "
            CREATE TABLE IF NOT EXISTS posts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                created_at DATETIME NOT NULL,
                content TEXT NOT NULL
            );
        ";
        conn.execute(stmt, [])
    }
    pub fn insert(conn: &Connection, created_at: DateTime<Utc>, content: &str) -> Result<usize> {
        let stmt = "
            INSERT INTO posts (created_at, content)
            VALUES (?, ?);
        ";
        let created_at = created_at.to_sqlite();
        let content = content.to_string();
        conn.execute(stmt, [created_at, content])
    }
    pub fn list(conn: &Connection) -> Result<Vec<Post>> {
        let stmt = "
            SELECT id, created_at, content
            FROM posts
            ORDER BY id DESC;
        ";
        let posts = conn
            .prepare(stmt)?
            .query_map([], |row| {
                let date_str: String = row.get("created_at")?;
                let created_at = DateTime::from_sqlite(&date_str);
                Ok(Post {
                    id: row.get("id")?,
                    created_at,
                    content: row.get("content")?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(posts)
    }
    pub fn get(conn: &Connection, id: i64) -> Result<Post> {
        let stmt = "
            SELECT id, created_at, content FROM posts WHERE id = ?;
        ";
        conn.prepare(stmt)?.query_row([id], |row| {
            let date_str: String = row.get("created_at")?;
            let created_at = DateTime::from_sqlite(&date_str);
            Ok(Post {
                id: row.get("id")?,
                created_at,
                content: row.get("content")?,
            })
        })
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

pub fn init(args: &ServeArgs, conn: &Connection) {
    init_tables(conn);

    if !args.production {
        let now = chrono::Utc::now();
        Post::insert(conn, now, "lorem ipsum").unwrap();
        let now = chrono::Utc::now();
        let content = indoc::indoc! {"
            # Code

            Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do
            eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim
            ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut
            aliquip ex ea commodo consequat.
            ```rust
            x = 1
            ```
        "};
        Post::insert(conn, now, &content).unwrap();
    }
}
