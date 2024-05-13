# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog].

[Keep a Changelog]: https://keepachangelog.com/en/1.1.0/

## [Unreleased]
### Added
- Template for unity 2022.3.6f1 `#956`

### Changed
- Support repositories with bad packages `#954`
- Use url in settings.json to load remote repository `#955`

### Deprecated

### Removed

### Fixed
- Project page is not refreshed after unity patch migration `#941`
- VCC Crashes after opening settings page `#942`
- Resolve needed check ignores legacy packages `#952`

### Security

## [0.1.2] - 2024-05-10
### Fixed
- Unity version used in template is not updated [`#933`](https://github.com/vrc-get/vrc-get/pull/933)
  - We upgraded Unity to 2022.3.22f1

## [0.1.1] - 2024-05-10
### Added
- `/opt/unityhub/unityhub` to the unity hub search path [`#812`](https://github.com/vrc-get/vrc-get/pull/812)
  - The path is the default path for official apt distribution
- Issue Report button feature in Settings [`#821`](https://github.com/vrc-get/vrc-get/pull/821)
- German translation [`#824`](https://github.com/vrc-get/vrc-get/pull/824)
- SDK2 Project is now shown as type "SDK2" [`#869`](https://github.com/vrc-get/vrc-get/pull/869)
- Select Unity Path if there are two or more Unity of the same version installed [`#863`](https://github.com/vrc-get/vrc-get/pull/863)
  - Currently, ALCOM will ask every time you open Unity. We will implement saving the selection in the future.
- Unity 2022 patch version migration [`#863`](https://github.com/vrc-get/vrc-get/pull/863)
- Legacy Assets are remove even if the specified GUID does not match with the actual GUID [`#901`](https://github.com/vrc-get/vrc-get/pull/901)
  - This follows VCC 2.3.0 beta 3 behavior.
- Added a French language translation [`#904`](https://github.com/vrc-get/vrc-get/pull/904)
- Updated the recommended Unity 2022 version to 2022.3.22f1 [`#928`](https://github.com/vrc-get/vrc-get/pull/928)
- Resolve suggestion [`#930`](https://github.com/vrc-get/vrc-get/pull/930)

### Fixed
- Impossible to control some portion if the window is narrow [`#805`](https://github.com/vrc-get/vrc-get/pull/805)
- Reorder Sidebar menu for clearer organization [`#820`](https://github.com/vrc-get/vrc-get/pull/820)
- Background is black if dark mode [`#811`](https://github.com/vrc-get/vrc-get/pull/811)
  - Plaease wait a while for the dark mode support
- Added dedicated messages for bulk actions in manage packages page [`#819`](https://github.com/vrc-get/vrc-get/pull/819)
- Panics are ignored [`#846`](https://github.com/vrc-get/vrc-get/pull/846)
  - From this version, panics will be logged to error logs instead of stderr.
- We cannot see packages from newly added repository just after adding repository [`#903`](https://github.com/vrc-get/vrc-get/pull/903)

## [0.1.0] - 2024-04-18
## [0.1.0-rc.0] - 2024-04-18
### Changed
- Reduced network load by reducing fetching remote repository [`#800`](https://github.com/vrc-get/vrc-get/pull/800)
  - Remote repositories will not be fetched for 5 minutes after the last fetch.
  - Please click the refresh button on the package page if you want to fetch the remote repository immediately.
- Preserve if fullscreen [`#801`](https://github.com/vrc-get/vrc-get/pull/801)

### Fixed
- Bad behaviors with minimizing the window [`#798`](https://github.com/vrc-get/vrc-get/pull/798)
- Error if backup folder does not exist [`#799`](https://github.com/vrc-get/vrc-get/pull/799)
- Unable to control if error occurs while backup is in progress [`#799`](https://github.com/vrc-get/vrc-get/pull/799)

## [0.1.0-beta.21] - 2024-04-16
### Added
- Simplified Chinese localization [`#765`](https://github.com/vrc-get/vrc-get/pull/765)
  - Thank you [@lonelyicer](https://github.com/lonelyicer)!
- Improved handling for unlocked packages [`#790`](https://github.com/vrc-get/vrc-get/pull/790)
- locale detection [`#771`](https://github.com/vrc-get/vrc-get/pull/771)

### Fixed
- Window size is not preserved when the app is closed with command + Q in macOS [`#769`](https://github.com/vrc-get/vrc-get/pull/769)
- Panic with relative paths [`#770`](https://github.com/vrc-get/vrc-get/pull/770)
- Update last modified on open Unity not working [`#775`](https://github.com/vrc-get/vrc-get/pull/775)
- Multiple instances can be launched [`#791`](https://github.com/vrc-get/vrc-get/pull/791)

## [0.1.0-beta.20] - 2024-04-13
### Added
- Check for update button on the settings page [`#762`](https://github.com/vrc-get/vrc-get/pull/762)
- Click version name to copy version name [`#761`](https://github.com/vrc-get/vrc-get/pull/761)
- Bulk upgrade, install, and remove packages [`#752`](https://github.com/vrc-get/vrc-get/pull/752)

### Changed
- Relax validation for `package.json` for local user packages [`#750`](https://github.com/vrc-get/vrc-get/pull/750)
- Use star instead of check on the favorite row in the project list [`#755`](https://github.com/vrc-get/vrc-get/pull/755)
- Moved the `Upgrade All` button to front [`#757`](https://github.com/vrc-get/vrc-get/pull/757)
- Renamed the application to ALCOM [`#760`](https://github.com/vrc-get/vrc-get/pull/760)

## [0.1.0-beta.19] - 2024-04-07
### Added
- Remove old log files [`#721`](https://github.com/vrc-get/vrc-get/pull/721) [`#729`](https://github.com/vrc-get/vrc-get/pull/729)
- Add repository with headers [`#725`](https://github.com/vrc-get/vrc-get/pull/725)

### Changed
- GUI Style improvement [`#722`](https://github.com/vrc-get/vrc-get/pull/722) [`#721`](https://github.com/vrc-get/vrc-get/pull/721) [`#720`](https://github.com/vrc-get/vrc-get/pull/720) [`#730`](https://github.com/vrc-get/vrc-get/pull/730) [`#731`](https://github.com/vrc-get/vrc-get/pull/731) [`#739`](https://github.com/vrc-get/vrc-get/pull/739)
- Confirm when removing repository [`#725`](https://github.com/vrc-get/vrc-get/pull/725)

### Fixed
- Last Modified row is not localized [`#723`](https://github.com/vrc-get/vrc-get/pull/723)

## [0.1.0-beta.18] - 2024-04-05
### Added
- Backup Project [`#714`](https://github.com/vrc-get/vrc-get/pull/714)
- Favorite project and sort by name [`#717`](https://github.com/vrc-get/vrc-get/pull/717)

### Fixed
- Unity 2022 Migration can fail with Compilation Error [`#712`](https://github.com/vrc-get/vrc-get/pull/712)

## [0.1.0-beta.17] - 2024-04-01
### Changed
- Update last modified on open Unity [`#697`](https://github.com/vrc-get/vrc-get/pull/697)

### Fixed
- Shown language name is not changed [`#694`](https://github.com/vrc-get/vrc-get/pull/694) 
- Installing package while Unity can be failed [`#696`](https://github.com/vrc-get/vrc-get/pull/696)

## [0.1.0-beta.16] - 2024-03-29
### Added
- Japanese Localization [`#674`](https://github.com/vrc-get/vrc-get/pull/674)

### Changed
- Package names on the Apply Changes dialog and a few other texts are now bold [`#676`](https://github.com/vrc-get/vrc-get/pull/676)
- Deleting a project now moves its folder to trash instead of hard deleting [`#676`](https://github.com/vrc-get/vrc-get/pull/676)

### Fixed
- World icon does not appear in the project list [`#625`](https://github.com/vrc-get/vrc-get/pull/625)
- Remove project button on the manage project page is not working [`#676`](https://github.com/vrc-get/vrc-get/pull/676)
- Project name warning is too bright [`#676`](https://github.com/vrc-get/vrc-get/pull/676)
- Unable to touch any button if Apply Changes dialog is long [`#676`](https://github.com/vrc-get/vrc-get/pull/676)
- The package order is not deterministic [`#676`](https://github.com/vrc-get/vrc-get/pull/676)
- Path separator is not correct on Windows [`#676`](https://github.com/vrc-get/vrc-get/pull/676)
- Same project can be added multiple times [`#676`](https://github.com/vrc-get/vrc-get/pull/676)
- Create button on the creating project dialog is not disabled [`#676`](https://github.com/vrc-get/vrc-get/pull/676)
- We can control the web ui while file picker is opened [`#676`](https://github.com/vrc-get/vrc-get/pull/676)
- Unrecoverable error when trying to add a non-project folder [`#676`](https://github.com/vrc-get/vrc-get/pull/676)
- Prerelease packages can be installed if version range has prerelease while the option is off [`#676`](https://github.com/vrc-get/vrc-get/pull/676)
- Handling whitespaces in the path of the project is not correct [`#676`](https://github.com/vrc-get/vrc-get/pull/676)
- We could not add a Unity manually [`#676`](https://github.com/vrc-get/vrc-get/pull/676)
- Legacy packages of installed packages are shown [`#676`](https://github.com/vrc-get/vrc-get/pull/676)
- False positive conflicts with legacy packages [`#676`](https://github.com/vrc-get/vrc-get/pull/676)
- Package order on the Apply Changes dialog is not deterministic [`#676`](https://github.com/vrc-get/vrc-get/pull/676)

## [0.1.0-beta.15] - 2024-03-16

- Not working on Windows [`#615`](https://github.com/vrc-get/vrc-get/pull/615)

## [0.1.0-beta.14] - 2024-03-16

- Create Project from Template [`#594`](https://github.com/vrc-get/vrc-get/pull/594)
    - Please note that vrc-get uses our own templates for project creation.
- Preserve window size [`#607`](https://github.com/vrc-get/vrc-get/pull/607)
- Toggle show prerelease packages [`#610`](https://github.com/vrc-get/vrc-get/pull/610)
- i18n support [`#614`](https://github.com/vrc-get/vrc-get/pull/614)

- vrc-get specific configuration is moved
  to `vrc-get/settings.json` [`#607`](https://github.com/vrc-get/vrc-get/pull/607)
    - This is done because we separated file for vrc-get-gui config file, and we may have more in the future os
      to not dirty the folder, I moved the config file to `vrc-get` folder.

- Bad behaviors with Unity 2018 [`#605`](https://github.com/vrc-get/vrc-get/pull/605)
- Bad behavior when trying installing the installed version [`#608`](https://github.com/vrc-get/vrc-get/pull/608)
- Some embedded / installed vpm package is not recognized by
  vrc-get [`#609`](https://github.com/vrc-get/vrc-get/pull/609)
- Http page can be opened in the browser [`#611`](https://github.com/vrc-get/vrc-get/pull/611)

## [0.1.0-beta.13] - 2024-03-12

- Migrate legacy VRCSDK3 project to VPM VRCSDK [`#580`](https://github.com/vrc-get/vrc-get/pull/580)

- Show "Not Selected" instead of "none" if the corresponding repositories are not
  selected [`#568`](https://github.com/vrc-get/vrc-get/pull/568)
- vrc-get now finds unity hub from registry key [`#590`](https://github.com/vrc-get/vrc-get/pull/590)

- Legacy Projects cannot be added to VCC project list [`#586`](https://github.com/vrc-get/vrc-get/pull/586)
- Removing repository doesn't remove package from list [`#587`](https://github.com/vrc-get/vrc-get/pull/587)

## [0.1.0-beta.12] - 2024-03-09

## [0.1.0-beta.11] - 2024-03-09

## [0.1.0-beta.10] - 2024-03-09

## [0.1.0-beta.9] - 2024-03-09

## [0.1.0-beta.8] - 2024-03-09

- Settings and Repositories page [`#522`](https://github.com/vrc-get/vrc-get/pull/522)
- Auto Update [`#557`](https://github.com/vrc-get/vrc-get/pull/557)

- The color of "Remove from the List" button is now default one. [`#524`](https://github.com/vrc-get/vrc-get/pull/524)

- Unity launched by vrc-get is shown as a part of vrc-get-gui [`#530`](https://github.com/vrc-get/vrc-get/pull/530)
- Fails to load all repositories if fails to load one repository [`#551`](https://github.com/vrc-get/vrc-get/pull/551)

## [0.1.0-beta.7] - 2024-03-04

- Remove Project [`#521`](https://github.com/anatawa12/vrc-get/pull/521)
- Migrate a Copy [`#522`](https://github.com/anatawa12/vrc-get/pull/522)

- Show unity log while migration [`#519`](https://github.com/anatawa12/vrc-get/pull/519)
- MacOS build is now a universal binary [`#520`](https://github.com/anatawa12/vrc-get/pull/520)
- Use local time for timestamp in log file [`#523`](https://github.com/anatawa12/vrc-get/pull/523)

- No user agent for http requests [`#513`](https://github.com/anatawa12/vrc-get/pull/513)

## [0.1.0-beta.6] - 2024-03-03

- Licenses page [`#504`](https://github.com/anatawa12/vrc-get/pull/504)
    - Currently under Settings page.
- Reinstall Packages [`#508`](https://github.com/anatawa12/vrc-get/pull/508)
    - Actually, this button is equivalent to `vrc-get resolve`.
    - To add this button, Upgrade All button is moved into the dropdown menu.

- Improved logging [`#505`](https://github.com/anatawa12/vrc-get/pull/505)
- Improved indication for error [`#512`](https://github.com/anatawa12/vrc-get/pull/512)
- Show a project as not exists if the directory does not exist [`#512`](https://github.com/anatawa12/vrc-get/pull/512)

- We can click upgrade button while installing packages [`#507`](https://github.com/anatawa12/vrc-get/pull/507)
- Packages for Avatars are shown if the project is Worlds project and vice
  versa [`#510`](https://github.com/anatawa12/vrc-get/pull/510)

## [0.1.0-beta.5] - 2024-03-02

- Support for Show Prereleases [`#495`](https://github.com/anatawa12/vrc-get/pull/495)

- The version name is shown on the menu instead of settings page [`#496`](https://github.com/anatawa12/vrc-get/pull/496)

- Fails to load package.json with invalid url in url field [`#492`](https://github.com/anatawa12/vrc-get/pull/492)
    - This makes `d4rkAvatarOptimizer` is recognized as not installed.
- Log file is not saved on windows [`#493`](https://github.com/anatawa12/vrc-get/pull/493)

## [0.1.0-beta.4] - 2024-03-01

- See and save logs of the vrc-get-gui [`#475`](https://github.com/anatawa12/vrc-get/pull/475)
- Link to changelog when install [`#481`](https://github.com/anatawa12/vrc-get/pull/481)
    - This uses [`changelogUrl` of UPM Manifest][changelog-of-upm-manifest]. Please add `changelogUrl` to your vpm
      repository.
- Upgrade all button [`#483`](https://github.com/anatawa12/vrc-get/pull/483)
- Add version information to the settings page [`#484`](https://github.com/anatawa12/vrc-get/pull/484)

[changelog-of-upm-manifest]: https://docs.unity3d.com/2022.3/Documentation/Manual/upm-manifestPkg.html#changelogUrl

- Message protrudes from toasts or dialogs [`#469`](https://github.com/anatawa12/vrc-get/pull/469)
- Window name should be `vrc-get-gui` but was `vrc-get GUI` [`#474`](https://github.com/anatawa12/vrc-get/pull/474)

## [0.1.0-beta.3]

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

[Unreleased]: https://github.com/vrc-get/vrc-get/compare/gui-v0.1.2...HEAD
[0.1.2]: https://github.com/vrc-get/vrc-get/compare/gui-v0.1.1...gui-v0.1.2
[0.1.1]: https://github.com/vrc-get/vrc-get/compare/gui-v0.1.0...gui-v0.1.1
[0.1.0]: https://github.com/vrc-get/vrc-get/compare/gui-v0.1.0-rc.0...gui-v0.1.0
[0.1.0-rc.0]: https://github.com/vrc-get/vrc-get/compare/gui-v0.1.0-beta.21...gui-v0.1.0-rc.0
[0.1.0-beta.21]: https://github.com/vrc-get/vrc-get/compare/gui-v0.1.0-beta.20...gui-v0.1.0-beta.21
[0.1.0-beta.20]: https://github.com/vrc-get/vrc-get/compare/gui-v0.1.0-beta.19...gui-v0.1.0-beta.20
[0.1.0-beta.19]: https://github.com/vrc-get/vrc-get/compare/gui-v0.1.0-beta.18...gui-v0.1.0-beta.19
[0.1.0-beta.18]: https://github.com/vrc-get/vrc-get/compare/gui-v0.1.0-beta.17...gui-v0.1.0-beta.18
[0.1.0-beta.17]: https://github.com/vrc-get/vrc-get/compare/gui-v0.1.0-beta.16...gui-v0.1.0-beta.17
[0.1.0-beta.16]: https://github.com/vrc-get/vrc-get/compare/gui-v0.1.0-beta.15...gui-v0.1.0-beta.16
[0.1.0-beta.15]: https://github.com/vrc-get/vrc-get/compare/gui-v...gui-v0.1.0-beta.15

[0.1.0-beta.14]: https://github.com/vrc-get/vrc-get/compare/gui-v0.1.0-beta.13...gui-v0.1.0-beta.14

[0.1.0-beta.13]: https://github.com/vrc-get/vrc-get/compare/gui-v0.1.0-beta.12...gui-v0.1.0-beta.13

[0.1.0-beta.12]: https://github.com/vrc-get/vrc-get/compare/gui-v0.1.0-beta.11...gui-v0.1.0-beta.12

[0.1.0-beta.11]: https://github.com/vrc-get/vrc-get/compare/gui-v0.1.0-beta.10...gui-v0.1.0-beta.11

[0.1.0-beta.10]: https://github.com/vrc-get/vrc-get/compare/gui-v0.1.0-beta.9...gui-v0.1.0-beta.10

[0.1.0-beta.9]: https://github.com/vrc-get/vrc-get/compare/gui-v0.1.0-beta.8...gui-v0.1.0-beta.9

[0.1.0-beta.8]: https://github.com/vrc-get/vrc-get/compare/gui-v0.1.0-beta.7...gui-v0.1.0-beta.8

[0.1.0-beta.7]: https://github.com/anatawa12/vrc-get/compare/gui-v0.1.0-beta.6...gui-v0.1.0-beta.7

[0.1.0-beta.6]: https://github.com/anatawa12/vrc-get/compare/gui-v0.1.0-beta.5...gui-v0.1.0-beta.6

[0.1.0-beta.5]: https://github.com/anatawa12/vrc-get/compare/gui-v0.1.0-beta.4...gui-v0.1.0-beta.5

[0.1.0-beta.4]: https://github.com/anatawa12/vrc-get/compare/gui-v0.1.0-beta.3...gui-v0.1.0-beta.4

[0.1.0-beta.3]: https://github.com/anatawa12/vrc-get/releases/tag/gui-v0.1.0-beta.3
