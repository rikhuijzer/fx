use chrono::NaiveDateTime;
use chrono::TimeZone;
use chrono::Utc;
use rusqlite::Connection;
use rusqlite::Result;

pub struct Database {
    pub conn: Connection,
}

pub struct Post {
    pub id: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub content: String,
}

pub trait SqliteDateTime {
    const FORMAT: &str = "%Y-%m-%d %H:%M:%S";
    fn from_sqlite(s: &str) -> Self;
    fn to_sqlite(&self) -> String;
}

impl SqliteDateTime for chrono::DateTime<chrono::Utc> {
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
    pub fn insert(conn: &Connection, post: &Post) -> Result<usize> {
        let stmt = "
            INSERT INTO posts (created_at, content)
            VALUES (?, ?);
        ";
        let created_at = post.created_at.to_sqlite();
        conn.execute(stmt, [created_at, post.content.clone()])
    }
}

pub fn init(production: bool) -> Result<Database> {
    let conn = if production {
        let path = "/data/db.sqlite";
        Connection::open(path)?
    } else {
        Connection::open_in_memory()?
    };
    Post::create_table(&conn)?;
    Ok(Database { conn })
}
