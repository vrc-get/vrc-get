# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog].

[Keep a Changelog]: https://keepachangelog.com/en/1.1.0/

## [Unreleased]
### Added
- Settings and Repositories page `#522`

### Changed
- The color of "Remove from the List" button is now default one. `#524`

### Deprecated

### Removed

### Fixed
- Unity launched by vrc-get is shown as a part of vrc-get-gui `#530`

### Security

## [0.1.0-beta.7] - 2024-03-04
### Added
- Remove Project [`#521`](https://github.com/anatawa12/vrc-get/pull/521)
- Migrate a Copy [`#522`](https://github.com/anatawa12/vrc-get/pull/522)

### Changed
- Show unity log while migration [`#519`](https://github.com/anatawa12/vrc-get/pull/519) 
- MacOS build is now a universal binary [`#520`](https://github.com/anatawa12/vrc-get/pull/520)
- Use local time for timestamp in log file [`#523`](https://github.com/anatawa12/vrc-get/pull/523)

### Fixed
- No user agent for http requests [`#513`](https://github.com/anatawa12/vrc-get/pull/513)

## [0.1.0-beta.6] - 2024-03-03
### Added
- Licenses page [`#504`](https://github.com/anatawa12/vrc-get/pull/504)
  - Currently under Settings page.
- Reinstall Packages [`#508`](https://github.com/anatawa12/vrc-get/pull/508)
  - Actually, this button is equivalent to `vrc-get resolve`.
  - To add this button, Upgrade All button is moved into the dropdown menu.

### Changed
- Improved logging [`#505`](https://github.com/anatawa12/vrc-get/pull/505)
- Improved indication for error [`#512`](https://github.com/anatawa12/vrc-get/pull/512)
- Show a project as not exists if the directory does not exist [`#512`](https://github.com/anatawa12/vrc-get/pull/512)

### Fixed
- We can click upgrade button while installing packages [`#507`](https://github.com/anatawa12/vrc-get/pull/507)
- Packages for Avatars are shown if the project is Worlds project and vice versa [`#510`](https://github.com/anatawa12/vrc-get/pull/510)

## [0.1.0-beta.5] - 2024-03-02
### Added
- Support for Show Prereleases [`#495`](https://github.com/anatawa12/vrc-get/pull/495)

### Changed
- The version name is shown on the menu instead of settings page [`#496`](https://github.com/anatawa12/vrc-get/pull/496)

### Fixed
- Fails to load package.json with invalid url in url field [`#492`](https://github.com/anatawa12/vrc-get/pull/492)
  - This makes `d4rkAvatarOptimizer` is recognized as not installed.
- Log file is not saved on windows [`#493`](https://github.com/anatawa12/vrc-get/pull/493)

## [0.1.0-beta.4] - 2024-03-01
### Added
- See and save logs of the vrc-get-gui [`#475`](https://github.com/anatawa12/vrc-get/pull/475) 
- Link to changelog when install [`#481`](https://github.com/anatawa12/vrc-get/pull/481)
  - This uses [`changelogUrl` of UPM Manifest][changelog-of-upm-manifest]. Please add `changelogUrl` to your vpm repository.
- Upgrade all button [`#483`](https://github.com/anatawa12/vrc-get/pull/483)
- Add version information to the settings page [`#484`](https://github.com/anatawa12/vrc-get/pull/484)

[changelog-of-upm-manifest]: https://docs.unity3d.com/2022.3/Documentation/Manual/upm-manifestPkg.html#changelogUrl
### Fixed
- Message protrudes from toasts or dialogs [`#469`](https://github.com/anatawa12/vrc-get/pull/469)
- Window name should be `vrc-get-gui` but was `vrc-get GUI` [`#474`](https://github.com/anatawa12/vrc-get/pull/474)

## [0.1.0-beta.3]
### Added
- Initial implementation [`#411`](https://github.com/anatawa12/vrc-get/pull/411)
    - This includes the following features
        - Load Project List from VCC's database
        - Adding Existing Project
        - List VPM Packages installed in the project
        - Add / Remove VPM package to / from the project
        - Open Unity
- Changelog [`#422`](https://github.com/anatawa12/vrc-get/pull/422)
- Apple code signing [`#422`](https://github.com/anatawa12/vrc-get/pull/422)
- Migrate vpm 2019 project to 2022 [`#435`](https://github.com/anatawa12/vrc-get/pull/435)

[Unreleased]: https://github.com/anatawa12/vrc-get/compare/gui-v0.1.0-beta.7...HEAD
[0.1.0-beta.7]: https://github.com/anatawa12/vrc-get/compare/gui-v0.1.0-beta.6...gui-v0.1.0-beta.7
[0.1.0-beta.6]: https://github.com/anatawa12/vrc-get/compare/gui-v0.1.0-beta.5...gui-v0.1.0-beta.6
[0.1.0-beta.5]: https://github.com/anatawa12/vrc-get/compare/gui-v0.1.0-beta.4...gui-v0.1.0-beta.5
[0.1.0-beta.4]: https://github.com/anatawa12/vrc-get/compare/gui-v0.1.0-beta.3...gui-v0.1.0-beta.4
[0.1.0-beta.3]: https://github.com/anatawa12/vrc-get/releases/tag/gui-v0.1.0-beta.3
