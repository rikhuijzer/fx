CREATE TABLE posts (
    -- don't add title since posts can add titles with # <TITLE>

    id INTEGER NOT NULL PRIMARY KEY,
    created_at DATETIME NOT NULL,
    content TEXT NOT NULL
);
