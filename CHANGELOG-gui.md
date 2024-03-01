# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog].

[Keep a Changelog]: https://keepachangelog.com/en/1.1.0/

## [Unreleased]
### Added
- See and save logs of the vrc-get-gui `#475` 
- Link to changelog when install `#481`
  - This uses [`changelogUrl` of UPM Manifest][changelog-of-upm-manifest]. Please add `changelogUrl` to your vpm repository.
- Upgrade all button `#483`

[changelog-of-upm-manifest]: https://docs.unity3d.com/2022.3/Documentation/Manual/upm-manifestPkg.html#changelogUrl
### Changed

### Deprecated

### Removed

### Fixed
- Message protrudes from toasts or dialogs `#469`
- Window name should be `vrc-get-gui` but was `vrc-get GUI` `#474`

### Security

## [0.1.0-beta.3]
### Added
- Initial implementation `#411`
    - This includes the following features
        - Load Project List from VCC's database
        - Adding Existing Project
        - List VPM Packages installed in the project
        - Add / Remove VPM package to / from the project
        - Open Unity
- Changelog `#422`
- Apple code signing `#422`
- Migrate vpm 2019 project to 2022 `#435`

[Unreleased]: https://github.com/anatawa12/vrc-get/compare/gui-v0.1.0-beta.3...HEAD
[0.1.0-beta.3]: https://github.com/anatawa12/vrc-get/releases/tag/gui-v0.1.0-beta.3
