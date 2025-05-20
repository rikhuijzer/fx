use chrono::DateTime;
use chrono::Datelike;
use chrono::Utc;
use diligent_date_parser::parse_date;
use http::Uri;
use regex::Regex;
use rss::Channel;
use std::error::Error;
use std::io::BufReader;
use std::io::ErrorKind;

/// An RSS item after it has been extracted from a feed.
///
/// According to https://www.rssboard.org/rss-specification, all elements of an
/// item are optional, but "at least one of title or description must be
/// present."
#[derive(Clone, Debug)]
pub struct Item {
    /// Name of the feed from which the item was extracted.
    pub feed_name: String,
    /// Not all RSS items have a title. For example, Mastodon posts don't have
    /// one.
    pub title: Option<String>,
    /// Item synopsis.
    pub description: Option<String>,
    /// URL to the item.
    pub link: Option<String>,
    /// Date and time when the item was published.
    pub pub_date: Option<DateTime<Utc>>,
}

fn truncate(text: &str) -> String {
    let max_length = 60;
    let mut text = text.to_string();
    if text.len() > max_length {
        let mut pos = max_length;
        while pos > 0 && !text.is_char_boundary(pos) {
            pos -= 1;
        }
        text.truncate(pos);
        text.push_str("...");
    }
    text.to_string()
}

impl Item {
    pub fn new(
        feed_name: String,
        title: Option<String>,
        description: Option<String>,
        link: Option<String>,
        pub_date: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            feed_name,
            title,
            description,
            link,
            pub_date,
        }
    }
    pub fn to_html(&self) -> Option<String> {
        let text = if let Some(title) = &self.title {
            truncate(title.trim())
        } else if let Some(description) = &self.description {
            let re = Regex::new(r#"<[^>]*>"#).unwrap();
            let text = re.replace_all(description.trim(), " ").to_string();
            truncate(&text)
        } else {
            return None;
        };

        let link = if let Some(link) = &self.link {
            link
        } else {
            println!("No link for item: {:?}", self);
            return None;
        };

        Some(format!(
            r#"{}: <a href="{link}">{text}</a>"#,
            self.feed_name
        ))
    }
}

#[derive(Debug)]
pub struct Feed {
    pub items: Vec<Item>,
}

#[derive(Debug)]
struct Month {
    year: u32,
    month: u32,
}

fn months_range(start: DateTime<Utc>, n_months: u32) -> Vec<Month> {
    let mut months = Vec::new();
    for i in 0..n_months {
        let mut year = (start.year() as u32) - (i / 12);
        let month = if i % 12 >= start.month() {
            year -= 1;
            12 - (i % 12 - start.month())
        } else {
            start.month() - (i % 12)
        };
        let month = Month { year, month };
        months.push(month);
    }
    months
}

#[test]
fn test_months_range() {
    let start = Utc::now();
    let months = months_range(start, 24);
    assert_eq!(months.len(), 24);
    assert_eq!(months[0].year, start.year() as u32);
    assert_eq!(months.last().unwrap().year, start.year() as u32 - 2);
}

impl Feed {
    pub fn to_html(self, config: &RssConfig) -> String {
        let mut items = self.items;
        if items.is_empty() {
            return String::new();
        }
        items.sort_by(|a, b| b.pub_date.cmp(&a.pub_date));
        let most_recent = items.first().unwrap();
        let months = months_range(most_recent.pub_date.unwrap(), config.max_age_in_months);
        let mut out = String::new();

        for month in months {
            let current_items = items
                .iter()
                .filter(|item| {
                    if let Some(date) = item.pub_date {
                        date.year() == month.year as i32 && date.month() == month.month
                    } else {
                        println!("No pub_date for item: {:?}", item);
                        false
                    }
                })
                .collect::<Vec<_>>();
            if current_items.is_empty() {
                continue;
            }
            let joined = current_items
                .into_iter()
                .filter(|item| {
                    if item.to_html().is_none() {
                        println!("Failed to convert item to HTML: {:?}", item);
                        false
                    } else {
                        true
                    }
                })
                .map(|item| item.to_html().unwrap())
                .collect::<Vec<String>>()
                .join("</li>\n  <li>");
            let header = format!("<h2>{} - {}</h2>", month.year, month.month);
            let html = format!(r#"{header}<ul><li>{joined}</li></ul>"#);
            out.push_str(&html);
        }

        out
    }
}

#[test]
fn test_feed_to_html() {
    let config = RssConfig::new(vec![], 24);
    let items = vec![
        Item::new(
            "Test".to_string(),
            Some("Title 1".to_string()),
            None,
            None,
            Some(parse_pub_date("Thu, 02 Jan 2025 00:00:00 +0000").unwrap()),
        ),
        Item::new(
            "Test".to_string(),
            Some("Title 2".to_string()),
            None,
            None,
            Some(parse_pub_date("Thu, 07 Nov 2024 00:00:00 +0000").unwrap()),
        ),
    ];
    let feed = Feed { items };
    let html = feed.to_html(&config);
    println!("{}", html);
    assert!(!html.is_empty());
    assert!(html.contains("2025 - 1"));
    assert!(!html.contains("2024 - 12"));
    assert!(html.contains("2024 - 11"));
}

pub struct RssFeed {
    url: String,
}

impl RssFeed {
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
        }
    }
}

pub struct RssConfig {
    /// URLs of RSS feeds.
    pub feeds: Vec<RssFeed>,
    /// Only show items that are less than this age in months.
    max_age_in_months: u32,
}

fn parse_pub_date(pub_date: &str) -> Option<DateTime<Utc>> {
    parse_date(pub_date).map(|date| date.with_timezone(&Utc))
}

#[test]
fn test_parse_pub_date() {
    let pub_date = "2023-12-12 00:00:00 UTC";
    let date = parse_pub_date(pub_date).unwrap();
    assert_eq!(date.year(), 2023);
    assert_eq!(date.month(), 12);
    assert_eq!(date.day(), 12);
}

fn sanitize(text: &str) -> String {
    String::from_utf8_lossy(text.as_bytes()).to_string()
}

fn items_from_rss(content: &str) -> Option<Vec<Item>> {
    let reader = BufReader::new(content.as_bytes());
    let channel = Channel::read_from(reader);
    if let Ok(channel) = channel {
        let mut items = Vec::new();
        for item in channel.items {
            let pub_date = if let Some(pub_date) = item.pub_date {
                parse_pub_date(&pub_date)
            } else {
                None
            };
            let feed_name = &channel.title;
            let title = item.title.map(|title| sanitize(&title));
            let description = item.description.map(|description| sanitize(&description));
            let item = Item::new(feed_name.clone(), title, description, item.link, pub_date);
            items.push(item);
        }
        Some(items)
    } else {
        None
    }
}

fn domain_from_url(url: &str) -> String {
    let url = if !url.starts_with("http") {
        format!("https://{}", url)
    } else {
        url.to_string()
    };
    match url.parse::<Uri>() {
        Ok(uri) => uri.host().unwrap_or(&url).to_string(),
        Err(_) => url.to_string(),
    }
}

#[test]
fn test_domain_from_url() {
    assert_eq!(domain_from_url("https://example.com/"), "example.com");
    assert_eq!(domain_from_url("example.com"), "example.com");
    assert_eq!(domain_from_url("example.com/foo"), "example.com");
}

fn items_from_atom(content: &str) -> Option<Vec<Item>> {
    let reader = BufReader::new(content.as_bytes());
    let feed = atom_syndication::Feed::read_from(reader);
    if let Ok(feed) = feed {
        let mut items = Vec::new();
        let author = feed.authors().first().map(|author| author.name.clone());
        let name = match author {
            Some(author) => author,
            None => domain_from_url(&feed.id),
        };
        for entry in feed.entries {
            let pub_date = if let Some(pub_date) = entry.published {
                let date = Some(pub_date);
                date.map(|date| date.with_timezone(&Utc))
            } else {
                println!("No pub_date for entry from {}", &name);
                None
            };
            let title = sanitize(&entry.title);
            let description = entry.summary.map(|summary| sanitize(&summary));
            let link = entry.links.first().map(|link| link.href.to_string());
            let item = Item::new(name.clone(), Some(title), description, link, pub_date);
            items.push(item);
        }
        Some(items)
    } else {
        None
    }
}

async fn items_from_feed(feed: &RssFeed) -> Result<Vec<Item>, Box<dyn Error + Send>> {
    let url = feed.url.clone();
    let client = reqwest::Client::builder().build().unwrap();
    let response = match client.get(url).send().await {
        Ok(response) => response,
        Err(e) => {
            println!("Failed to fetch feed {}: {:?}", feed.url, e);
            return Err(Box::new(e));
        }
    };
    let content = match response.text().await {
        Ok(content) => content,
        Err(e) => {
            println!("Failed to get text from feed {}: {:?}", feed.url, e);
            return Err(Box::new(e));
        }
    };
    // Not trying to determine the feed format here since in the end all that
    // matters whether the parser can parse the feed or not.
    if let Some(items) = items_from_atom(&content) {
        return Ok(items);
    }
    if let Some(items) = items_from_rss(&content) {
        return Ok(items);
    }
    let msg = format!("Failed to parse feed {}", feed.url);
    Err(Box::new(std::io::Error::new(ErrorKind::InvalidInput, msg)))
}

impl RssConfig {
    pub fn new(feeds: Vec<RssFeed>, max_age_in_months: u32) -> Self {
        Self {
            feeds,
            max_age_in_months,
        }
    }
    pub async fn download_items(&self) -> Vec<Item> {
        let futures: Vec<_> = self.feeds.iter().map(items_from_feed).collect();
        let results = futures::future::join_all(futures).await;
        results
            .into_iter()
            .filter_map(Result::ok)
            .flatten()
            .collect()
    }
}
