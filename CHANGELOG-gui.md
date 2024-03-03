# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog].

[Keep a Changelog]: https://keepachangelog.com/en/1.1.0/

## [Unreleased]
### Added

### Changed
- Improved logging `#505`

### Deprecated

### Removed

### Fixed

### Security

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

[Unreleased]: https://github.com/anatawa12/vrc-get/compare/gui-v0.1.0-beta.5...HEAD
[0.1.0-beta.5]: https://github.com/anatawa12/vrc-get/compare/gui-v0.1.0-beta.4...gui-v0.1.0-beta.5
[0.1.0-beta.4]: https://github.com/anatawa12/vrc-get/compare/gui-v0.1.0-beta.3...gui-v0.1.0-beta.4
[0.1.0-beta.3]: https://github.com/anatawa12/vrc-get/releases/tag/gui-v0.1.0-beta.3
