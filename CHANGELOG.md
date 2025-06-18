# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.1.1] - 2025-06-02

### Fixed

- Fix horizontal rule (`---`) in content.

## [1.1.0] - 2025-05-28

### Added

- Pagination for posts on the homepage.

### Fixed

- Allow seeing full feed items on blogroll even when long.

## [1.0.4] - 2025-05-24

### Fixed

- Fix math display in preview when only inline math is present.

## [1.0.3] - 2025-05-24

### Fixed

- Fix code block highlighting in preview.
- Show inline math in preview.
- Remove short URL from below posts.
- Fix login/logout link on some pages.

## [1.0.2] - 2025-05-22

### Added

- Add copy slug button below posts ([#68](https://github.com/rikhuijzer/fx/pull/68))
- Allow slug behind URL (for example, `/posts/1/my-post`)

### Fixed

- Open Graph description for homepage.
- Open Graph title.
- Open Graph type.
- Filter old items from blogroll.
- Improve `domain_from_url`.

## [1.0.1] - 2025-05-17

### Fixed

- Init blogroll setting to avoid crash on startup

## [1.0.0] - 2025-05-17

### Added

- Blogroll ([#63](https://github.com/rikhuijzer/fx/pull/63))

### Fixed

- Fix `\n` instead of newline in rss ([#65](https://github.com/rikhuijzer/fx/pull/65))

[1.1.1]: https://github.com/rikhuijzer/fx/compare/v1.1.0...v1.1.1
[1.1.0]: https://github.com/rikhuijzer/fx/compare/v1.0.4...v1.1.0
[1.0.4]: https://github.com/rikhuijzer/fx/compare/v1.0.3...v1.0.4
[1.0.3]: https://github.com/rikhuijzer/fx/compare/v1.0.2...v1.0.3
[1.0.2]: https://github.com/rikhuijzer/fx/compare/v1.0.1...v1.0.2
[1.0.1]: https://github.com/rikhuijzer/fx/compare/v1.0.0...v1.0.1
[1.0.0]: https://github.com/rikhuijzer/fx/releases/tag/v1.0.0
