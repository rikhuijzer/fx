# fx &emsp; [![build status]][actions] [![docker svg]][docker]

[build status]: https://img.shields.io/github/actions/workflow/status/rikhuijzer/fx/ci.yml?branch=main
[actions]: https://github.com/rikhuijzer/fx/actions?query=branch%3Amain
[docker svg]: https://img.shields.io/badge/docker-%230db7ed.svg?logo=docker&logoColor=white
[docker]: https://hub.docker.com/repository/docker/rikhuijzer/fx

A (micro)blogging server that you can self-host.

## Features

- 🚀 Low costs due to small footprint (only a few MB of memory are required).
- 📝 Write posts in Markdown.
- 📱 Publish and edit from mobile device.
- 📁 Upload files and images to embed them in posts.
- 🔒 Automatically backup to plain text files, see [Backup](#backup).

## Background

This site is aimed at people who want to write down something and later be able to find it back.
For example, say you have just read a nice blog post and want to remember it for later, you can describe the post in your own words and add a link to it.
Or if you want to remember a cooking recipe or some code snippet, you can write it down for later.
This way, you and other people can benefit from the things you and others have written down.

## Installation

Via Docker Compose:

```yml
services:
  fx:
    image: 'rikhuijzer/fx:main'
    container_name: 'fx'
    environment:
      FX_USERNAME: 'john'
      FX_DOMAIN: 'example.com'
    env_file:
      # Contains `FX_PASSWORD="<PASSWORD>"`.
      - 'FX_PASSWORD.env'
    ports:
      - '3000:3000'
    volumes:
      # Stores the SQLite database.
      - './data:/data:rw'
    restart: 'unless-stopped'
```

## API

### Backup

You can backup your site to plain text files with the following shell script:

```bash
#!/usr/bin/env bash
set -euxo pipefail

ARCHIVE_PATH="all.tar.xz"

curl \
  -H "Authorization: Bearer $FX_PASSWORD" \
  https://$DOMAIN/api/download/all.tar.xz > "$ARCHIVE_PATH"

tar --verbose -xf "$ARCHIVE_PATH"
rm "$ARCHIVE_PATH"
```

where `$FX_PASSWORD` is the admin password (as set via the `FX_PASSWORD` environment variable) and `$DOMAIN` is the domain of your site (for example, `example.com`).

Assuming this file is named `backup.sh` and executable (`chmod +x backup.sh`), you can run a backup in a GitHub Actions workflow with the following YAML:

```yml
name: ci
on:
  schedule:
    - cron: '24 0,6,12,18 * * *'
  push:
    branches:
      - main
  pull_request:
  workflow_dispatch:
jobs:
  backup:
    permissions:
      contents: write
    runs-on: ubuntu-24.04
    steps:
      - uses: actions/checkout@v4
      - run: ./backup.sh
        env:
          FX_PASSWORD: ${{ secrets.FX_PASSWORD }}
      - if: github.event_name != 'pull_request'
        run: |
          if [ -n "$(git status --porcelain)" ]; then
            git config --global user.email "$GITHUB_ACTOR@users.noreply.github.com"
            git config --global user.name "$GITHUB_ACTOR"

            git add .
            git commit -m '[bot] backup'
            git push
          fi

```

This will backup your site every 6 hours.
An example backup repository is [here](https://github.com/rikhuijzer/fx-backup).

To trigger a backup for each change to the website, you can set the following environment variables:

```yml
FX_TRIGGER_TOKEN: 'github_pat_...'
FX_TRIGGER_OWNER_REPO: 'johndoe/fx-backup'
FX_TRIGGER_BRANCH: 'main' # Optional
FX_TRIGGER_WORKFLOW_ID: 'backup.yml' # Optional
```

To obtain the token, you can use the following steps:

1. Go to <https://github.com/settings/personal-access-tokens/new>.
1. Set name: `fx-backup-trigger`.
1. Set description: `Used by fx to trigger a backup`.
1. Set repository access: `Only select repositories: <OWNER>/<REPO>`.
1. Set permissions: `Actions` (Read and write).
1. Copy the token.

See the [GitHub documentation](https://docs.github.com/en/rest/actions/workflows?apiVersion=2022-11-28#create-a-workflow-dispatch-event) for more information.

### Update

You can update the `about` text via:

```bash
curl \
  -X PUT \
  -H "Authorization: Bearer $FX_PASSWORD" \
  https://$DOMAIN/api/settings/about \
  -d "Some text"
```
