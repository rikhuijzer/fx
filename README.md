# fx &emsp; [![build status]][actions] [![docker svg]][docker]

[build status]: https://img.shields.io/github/actions/workflow/status/rikhuijzer/fx/ci.yml?branch=main
[actions]: https://github.com/rikhuijzer/fx/actions?query=branch%3Amain
[docker svg]: https://img.shields.io/badge/docker-%230db7ed.svg?logo=docker&logoColor=white
[docker]: https://hub.docker.com/repository/docker/rikhuijzer/fx

A simple (micro)blogging server that you can host yourself.

What made sites like Twitter nice was that it was easy to quickly write something down and later be able to find it back.
For example, say you have just read a nice blog post and want to remember it for later, you could just tweet it.
However, X (formerly Twitter) and most other social media platforms have been locking this down.
Most posts can now only be viewed when you are logged in.
Furthermore, the X algorithm also discourages adding [links in posts](https://x.com/TheBubbleBubble/status/1849818873018610090) so as a user you are incentivized to not add links.
I think this is a sad development since links are an essential part of the internet.

## Features

- Small footprint (requires less than 100 MB of RAM)
- Mobile-friendly interface for writing and publishing posts from your phone
- Markdown support
