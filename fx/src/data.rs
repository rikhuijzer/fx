use crate::ServeArgs;
use chrono::DateTime;
use chrono::NaiveDateTime;
use chrono::TimeZone;
use chrono::Utc;
#[cfg(feature = "clib")]
use rusqlite::Connection;
#[cfg(feature = "clib")]
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
    #[cfg(feature = "clib")]
    pub fn create_table(conn: &Connection) -> Result<usize> {
        let stmt = "CREATE TABLE IF NOT EXISTS kv (key TEXT PRIMARY KEY, value BLOB)";
        conn.execute(stmt, [])
    }
    #[cfg(feature = "clib")]
    pub fn insert(conn: &Connection, key: &str, value: &[u8]) -> Result<usize> {
        let stmt = &format!("INSERT INTO kv (key, value) VALUES ('{key}', ?)");
        conn.execute(stmt, [value])
    }
    #[cfg(feature = "clib")]
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
#[cfg(feature = "clib")]
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
    /// The id of the post.
    #[allow(dead_code)]
    pub id: i64,
    /// The date and time the post was created.
    pub created: chrono::DateTime<chrono::Utc>,
    /// The date and time the post was last updated.
    pub updated: chrono::DateTime<chrono::Utc>,
    /// The content of the post.
    pub content: String,
}

impl Post {
    #[cfg(feature = "clib")]
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
    #[cfg(feature = "clib")]
    pub fn insert(
        conn: &Connection,
        created: DateTime<Utc>,
        updated: DateTime<Utc>,
        content: &str,
    ) -> Result<usize> {
        let stmt = "
            INSERT INTO posts (created, updated, content)
            VALUES (?, ?, ?);
        ";
        let created = created.to_sqlite();
        let updated = updated.to_sqlite();
        let content = content.to_string();
        conn.execute(stmt, [created, updated, content])
    }
    #[cfg(feature = "clib")]
    pub fn list(conn: &Connection) -> Result<Vec<Post>> {
        let stmt = "
            SELECT id, created, updated, content
            FROM posts
            ORDER BY id DESC;
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
    #[cfg(not(feature = "clib"))]
    pub fn list(db: &DbConn) -> Result<Vec<Post>> {
        let conn = db.conn.lock().unwrap();
        let stmt = "
            SELECT id, created, updated, content
            FROM posts
            ORDER BY id DESC;
        ";
        let stmt = conn.prepare(stmt)?;
        let query = stmt.bind(&[]);
        let posts = query.all::<Post>().await?;
    }
    #[cfg(feature = "clib")]
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
    #[cfg(feature = "clib")]
    pub fn update(self: &Self, conn: &Connection) -> Result<usize> {
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
    #[cfg(feature = "clib")]
    pub fn delete(conn: &Connection, id: i64) -> Result<usize> {
        let stmt = "DELETE FROM posts WHERE id = ?";
        conn.execute(stmt, [id])
    }
}

#[cfg(feature = "clib")]
pub fn connect(args: &ServeArgs) -> Result<Connection> {
    let conn = if args.production {
        let path = &args.database_path;
        Connection::open(path)?
    } else {
        Connection::open_in_memory()?
    };
    Ok(conn)
}

#[cfg(feature = "clib")]
fn init_tables(conn: &Connection) {
    Post::create_table(conn).expect("Failed to create posts table");
    Kv::create_table(conn).expect("Failed to create kv table");
}

#[cfg(feature = "clib")]
pub fn init(args: &ServeArgs, conn: &Connection) {
    init_tables(conn);

    if !args.production {
        let now = chrono::Utc::now();
        let content = indoc::indoc! {"
            [Lorem](https://example.com/lorem) ipsum
        "};
        Post::insert(conn, now, now, content).unwrap();
        let now = chrono::Utc::now();
        let content = indoc::indoc! {"
            # Code

            Dolor sit amet, consectetur adipiscing elit, sed do
            eiusmod tempor incididunt ut `labore` et dolore magna aliqua. Ut enim
            ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut
            aliquip ex ea commodo consequat.
            ```rust
            x = 1
            ```
        "};
        Post::insert(conn, now, now, content).unwrap();
    }
}
