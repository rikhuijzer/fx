# fx

[build status]: https://img.shields.io/github/actions/workflow/status/rikhuijzer/fx/ci.yml?branch=main
[actions]: https://github.com/rikhuijzer/fx/actions?query=branch%3Amain
[docker svg]: https://img.shields.io/badge/docker-%230db7ed.svg?logo=docker&logoColor=white
[docker]: https://hub.docker.com/repository/docker/rikhuijzer/fx

A Twitter/Bluesky-like (micro)blogging service that you can self-host.

## Features

- 🚀 Low costs due to small footprint (only a few MB of memory are required).
- 📝 Write posts in Markdown.
- 🖥 Built-in syntax highlighting.
- ∑ Built-in display for math expressions (LaTeX syntax, e.g. `$E=mc^2$`).
- 📱 Publish and edit from desktop or mobile device.
- 📁 Upload files and images to embed them in posts.
- 🔒 Automatically backup to plain text files, see [Backup](#backup).
- 🔭 Follow RSS feeds.

## Demo

There is a demo site where you can log in and create posts at <https://fx-demo.huijzer.xyz>.
The demo site resets every hour.

Other sites that are running fx:

- <https://huijzer.xyz/>
- <https://hoekmanmilieuadvies.nl/>
- <https://fadlaoui.fr/>
- <https://rozumem.xyz/>

Feel free to submit a PR if you want your site to be listed here.

## Background

This site is aimed at people who want to write down something and later be able to find it back.
Think of it as a public notebook, where you can write down anything you want.
For example, say you have just read a nice blog post, cooking recipe, or code snippet, and want to remember it for later, you can quickly write a short description and post it on your site.
Later, you can use Google or the built-in search to find it again.

Another use-case could be if you are a teacher who often gets the same questions.
Instead of copy-pasting the same answer each time, you can write a post and share the link with your students.

Compared to social media, having your own site mitigates the risk of being (shadow) banned.
If you host your posts on your own site, you have more control over your content.
Furthermore, domains in most countries are protected by law, so nobody can just take your content down.

Compared to static site generators, this server is meant to make it easier to write and edit posts.
With a static site generator, the publishing workflow often means that you have to add a file, commit it, and then wait for the build to complete.
Or you have to be on your desktop to run the server locally.
With this server, you can write your posts inside the web interface.
In my experience, this lowers the barrier to write posts since it is now possible to see the result of a change within seconds instead of minutes.
Like Twitter/Bluesky or other social media platforms, with this site you can now just quickly write something down whenever it comes up.

## Installation

Via Docker Compose (if you're not used to managing Linux servers, [here is a howto](https://huijzer.xyz/posts/150/how-to-run-services-on-a-linux-server)):

```yml
services:
  fx:
    image: 'rikhuijzer/fx:1'
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
    healthcheck:
      test: ['CMD', '/fx', 'check-health']
    restart: 'unless-stopped'
```

For the full list of `FX_` environment variables, see [fx/src/lib.rs](https://github.com/rikhuijzer/fx/blob/main/fx/src/lib.rs).
Regarding the health check, Docker Compose does not restart containers when it fails.
To make that happen, you can write your own CRON job script to check for failures, or use [autoheal](https://github.com/willfarrell/docker-autoheal).

## Syndication

To share a post, you can either get the URL from the navigation bar or you can copy the longer link that is available below each post.
The longer link includes a so called slug, which makes the URL more descriptive (for example, `/posts/1` versus `/posts/1/hello-world`).

Next, Publish (on your) Own Site, Syndicate Everywhere (POSSE) can be used to make the posts seen by more people.
For example, you can share the link to your article on Reddit, X, BlueSky, Discord, Facebook, Hacker News, LinkedIn, or Mastodon.
As long as you politely ask and try to add value, most sites are usually accepting links to blog posts.
Then comments to the article can be made there and people can decide to share the post with other people.
For example, Simon Willison uses this over at his [fedi instance](https://fedi.simonwillison.net/@simon).
Another idea could be to politely ask another writer for a guest post or a shoutout.

## Blogroll

The blogroll can be used to follow RSS feeds.
Unlike social media, with RSS feeds all articles will show up on your feed.
Conversely, the social media algorithm may hide certain articles without you realizing it.

## API

### Backup

You can backup your site to plain text files with the following shell script:

```bash
#!/usr/bin/env bash
set -euxo pipefail

DOMAIN="example.com"

cleanup() {
  rm -rf files/ posts/ settings/
}

download() {
  ARCHIVE_PATH="all.tar.xz"
  curl --proto "=https" --tlsv1.2 -sSf \
    -H "Authorization: Bearer $FX_PASSWORD" \
    https://$DOMAIN/api/download/all.tar.xz > "$ARCHIVE_PATH"

  tar --verbose -xf "$ARCHIVE_PATH"
  rm "$ARCHIVE_PATH"
}

commit() {
  if [ -n "$(git status --porcelain)" ]; then
    git config --global user.email "$GITHUB_ACTOR@users.noreply.github.com"
    git config --global user.name "$GITHUB_ACTOR"

    git add .
    git commit -m '[bot] backup'
    git push
  fi
}

if [[ "$1" == "cleanup" ]]; then
  cleanup
elif [[ "$1" == "download" ]]; then
  download
elif [[ "$1" == "commit" ]]; then
  commit
fi
```

where `$FX_PASSWORD` is the admin password (as set via the `FX_PASSWORD` environment variable) and `$DOMAIN` is the domain of your site.

Assuming this file is named `backup.sh` and executable (`chmod +x backup.sh`), you can run a backup in a GitHub Actions workflow with the following YAML:

```yml
name: backup
concurrency:
  group: ${{ github.workflow }}
on:
  schedule:
    - cron: '24 0 * * *'
  push:
    branches:
      - main
  pull_request:
  workflow_dispatch:
jobs:
  run:
    permissions:
      contents: write
    runs-on: ubuntu-24.04
    timeout-minutes: 10
    steps:
        # Avoiding `actions/checkout` since it runs concurrently even when
        # concurrency group is set, see
        # https://github.com/actions/checkout/discussions/1125.
      - run: >
          git clone --depth=1
          https://x-access-token:${{ secrets.GITHUB_TOKEN }}@github.com/${{ github.repository }}.git
          .
      - run: ./backup.sh cleanup
      - run: ./backup.sh download
        env:
          FX_PASSWORD: ${{ secrets.FX_PASSWORD }}
      - if: github.event_name != 'pull_request'
        run: ./backup.sh commit
```

This will backup your site at least once per day.
An example backup repository is [here](https://github.com/rikhuijzer/fx-backup).

To trigger a backup for each change to the website, you can set the following environment variables:

```yml
FX_TRIGGER_TOKEN: 'github_pat_...'
FX_TRIGGER_OWNER_REPO: 'johndoe/fx-backup'
FX_TRIGGER_HOST: 'https://github.com' # Optional.
FX_TRIGGER_BRANCH: 'main' # Optional.
FX_TRIGGER_WORKFLOW_ID: 'backup.yml' # Optional.
```

To obtain the GitHub token, you can use the following steps:

1. Go to <https://github.com/settings/personal-access-tokens/new>.
1. Set name: `fx-backup-trigger`.
1. Set description: `Used by fx to trigger a backup`.
1. Set repository access: `Only select repositories: <OWNER>/<REPO>`.
1. Set permissions: `Actions` (Read and write).
1. Copy the token.

See the [GitHub documentation](https://docs.github.com/en/rest/actions/workflows?apiVersion=2022-11-28#create-a-workflow-dispatch-event) for more information.

It also supports triggering a workflow in a Forgejo instance.
For that use set `FX_TRIGGER_HOST` to the URL of the Forgejo instance.
To obtain the Forgejo token, use:

1. Go to `https://<DOMAIN>/user/settings/applications`.
1. Set name: `fx-backup-trigger`.
1. Set repository access to `Public only` or `All (public, private, and limited)` depending on your needs.
1. Set permissions: `repository` (Read and write).
1. Copy the token.

### Update

You can update the `about` text via the API:

```bash
curl \
  -X PUT \
  -H "Authorization: Bearer $FX_PASSWORD" \
  https://$DOMAIN/api/settings/about \
  -d "Some text"
```
