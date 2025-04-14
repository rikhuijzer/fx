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
            SELECT id, created_at, content FROM posts;
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

pub fn init(conn: &Connection) {
    Post::create_table(conn).expect("Failed to create table");
}
