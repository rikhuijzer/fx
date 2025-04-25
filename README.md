# fx &emsp; [![build status]][actions] [![docker svg]][docker]

[build status]: https://img.shields.io/github/actions/workflow/status/rikhuijzer/fx/ci.yml?branch=main
[actions]: https://github.com/rikhuijzer/fx/actions?query=branch%3Amain
[docker svg]: https://img.shields.io/badge/docker-%230db7ed.svg?logo=docker&logoColor=white
[docker]: https://hub.docker.com/repository/docker/rikhuijzer/fx

A (micro)blogging server that you can self-host.

## Features

- 🚀 Small footprint (requires only a few MB of memory).
- 📱 Mobile-friendly interface to allow publishing posts from mobile devices.
- 📝 Markdown support.
- 🔒 Backup your site as plain text files (to have useful diffs in Git).

## Background

What made sites like Twitter nice was that it was easy to quickly write something down and later be able to find it back.
For example, say you have just read a nice blog post and want to remember it for later, you could just tweet it.
However, X (formerly Twitter) and most other social media platforms have been locking this down.
Most posts can now only be viewed when you are logged in.
Furthermore, the X algorithm also discourages adding [links in posts](https://x.com/TheBubbleBubble/status/1849818873018610090) so as a user you are incentivized to not add links.
I think this is a sad development since links are an essential part of the internet.

## Installation

Via Docker Compose:

```yml
services:
  fx:
    image: 'rikhuijzer/fx:main'
    container_name: 'fx'
    environment:
      FX_PRODUCTION: 'true'
      FX_USERNAME: 'rik'
      FX_TITLE_SUFFIX: 'My Weblog'
      FX_FULL_NAME: 'John Doe'
      FX_ABOUT: 'Running a web server'
      FX_DOMAIN: 'example.com'
    env_file:
      # Put `FX_PASSWORD="<PASSWORD>"` in this file where `<PASSWORD>` is the
      # password for the admin user.
      - 'FX_PASSWORD.env'
    working_dir: '/app'
    ports:
      - '3000:3000'
    volumes:
      # Stores the SQLite database.
      - './data:/data:rw'
    restart: 'unless-stopped'
```

## Backup

You can backup your site to plain text files with

```bash
#!/usr/bin/env bash

set -eux pipefail

curl -L \
  -H "Authorization: Bearer $PASSWORD" \
  https://$DOMAIN/api/download/all.tar.xz > all.tar.xz

tar -xvf all.tar.xz
```

where `$PASSWORD` is the admin password (as set via the `FX_PASSWORD` environment variable) and `$DOMAIN` is the domain of your site (for example, `example.com`).
