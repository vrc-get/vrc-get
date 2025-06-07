# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog].

[Keep a Changelog]: https://keepachangelog.com/en/1.1.0/

## [Unreleased]
### Added
- Support for Projects with Unity 2018 or older `#2106`
  - Projects with Unity 2018 cannot be added before, but since this version you can add to your project list.
  - Unity 2017 or older doesn't have Unity Package Manager, the base system for VPM,
    so you cannot manage packages for projects with older unity.
    You can only launch Unity to open the project.
  - Projects with Unity 4 or older are still not supported, but I hope no one want to use such a vintage Unity with ALCOM.
- New Project Template System `#2105` `#2114` `#2125` `#2129` `#2204` `#2259` `#2260` `#2261` `#2275`
  - You now can create Project Templates in ALCOM.
  - The new form of template can install multiple VPM packages at once, and you also can import unitypackages.
  - You now can create blank project along with this system change.
- Warning on upgrading major version or installing incompatible versions `#2159`
  - When you're upgrading package versions majorly, you'll see the warning message about breaking changes.
  - I hope this should reduce problems with unexpectedly upgrading packages majorly.
  - In addition, we added more messages when you're installing packages with some compatibility concerns.
  - The previous version only has those messages at the bottom of the window, so you may not notice the message.
  - Not only that, you now can see the package is upgraded, reinstalled, downgraded, or newly installed. 
- Menu option to copy a project `#2168` `#2219` `#2225`
  - Simple enough, you can copy a project.
- Remember recent project locations `#2182`
  - ALCOM now remembers a few multiple recent locations for project creation, and you can select from recent locations
- Support for flatpak installation of unity hub `#1586`
  - ALCOM now detects flatpak installation of unity hub automatically
- Projects page Grid View `#2245` `#2257`

### Changed
- Changed how we read VCC's project information `#1997` `#2036` `#2041`
  - Along with this, building this project no longer needs dotnet SDK to build.
- Migrated the project to Rust 2024 `#1956`
  - This is internal changes should not cause behavior changes
  - This would require Rust 1.85 for building this project
- Removed `cargo-about` from build-time dependency `#1961`
  - This is internal changes should not cause behavior changes
  - I listed here since this may need update on package metadata of some package managers
- The method to retrieve the list of Unity from Unity Hub `#1808` `#1971`
  - Since this version, ALCOM reads UnityHub's configuration files to get list of Unity installed to the machine.
  - Before this version, ALCOM called headless Unity Hub in the background.
  - New method might have some compatibility problem, especially with some sandbox system.
  - Please report us if you find some problem with the new system.
- Enhance os info for windows `#1968`
- You now can select multiple folders at once to adding project `#2018`
  - I didn't know official VCC had such a feature. Sorry for lack of feature!
- You now can toggle "Show Prerelease Packages" from Manage Project page `#2020`
  - You can toggle "Show Prerelease Packages" from Select Packages dropdown
- The requirements for unity project `#2106`
  - Since this version, `Projectsettings/ProjectVersion.txt` is required.
- Improves launching unity behavior `#2124`
  - On linux, ALCOM will now read exit code, therefore, Unity no longer remains as a defunct process.
  - On macOS, we now launch Unity as a distinct / individual process, therefore several macOS subsystems should treat Unity as Unity instead of Unity as a part of ALCOM.
- Downgraded glibc requirements for linux images `#2160`
  - This release will be built on ubuntu 22.04 so glibc 2.35 is new requirements
  - If you want to use on platforms with older glibc, build yourself or pull request to build on older environments.
- Loading projects / repositories is now asynchronously `#2169`
  - You should be able to open a project / install packages much quickly than before!
  - The reload button will keep rotating while loading asynchronously
- Open changelog, documentation, and reinstall single package from package list `#2184` `#2208`
  - You can open the changelog and documentation from `...` button at the right of package list
- Option to exclude VPM Packages from backups `#2185`
  - You can exclude VPM Packages from backups to reduce size of backup a little
  - However, if the package author ignored the recommendation from VRChat and us, and removed package from their repository, you may need to install another version when restoring the backup.
  - Since many of the repository maintainers have removed many packages in their repository and VPM Packages are relatively small, this feature is disabled by default. You can enable this on the settings page.
- Show the range of requested package in missing dependencies dialog `#2187`
- `LastSceneManagerSetup.txt` in `Library` directory will be included in backups or copying project `#2205`
  - With this file preserved, you can expect to open the last opened scene file when you opened projects restored from backups.
- Improved behavior when the project directory is not a valid project but the directory exists `#2225`
- Open Unity now will update `Last Modified` of a project. `#2228`

### Deprecated

### Removed

### Fixed
- Layout shift on select package `#2045`
- Unable to change the unity version from "unknown" if ProjectVersion.txt does not exists `#2183`
- Uninstall package is not reverted successfully if removing package is prevented by `ERROR_SHARING_VIOLATION` `#2209`
- Too Many Open Files on backing up some projects `#2262`

### Security

## [1.0.1] - 2025-02-05
### Added
- Add Korean localization [`#1822`](https://github.com/vrc-get/vrc-get/pull/1822)

### Fixed
- Fixed toast message when adding repositories [`#1815`](https://github.com/vrc-get/vrc-get/pull/1815)
- Improved several linux desktop support [`#1821`](https://github.com/vrc-get/vrc-get/pull/1821)
- Backup file used UTC time instead of Local time [`#1862`](https://github.com/vrc-get/vrc-get/pull/1862)
- Worlds templates doesn't have proper input axis settings [`#1902`](https://github.com/vrc-get/vrc-get/pull/1902)

## [1.0.0] - 2025-01-01
### Fixed
- Link to unity hub is hardcoded to Japanese [`#1810`](https://github.com/vrc-get/vrc-get/pull/1810)
  - Fixed link to respect currently configured language
- Fixed Logs page autoscroll not enable on start [`#1811`](https://github.com/vrc-get/vrc-get/pull/1811)
- Fixed failed to load project list with invalid unity version stored [`#1813`](https://github.com/vrc-get/vrc-get/pull/1813)

## [0.1.17] - 2024-12-22
### Changed
- Several GUI improvements [`#1672`](https://github.com/vrc-get/vrc-get/pull/1672) [`#1771`](https://github.com/vrc-get/vrc-get/pull/1771) [`#1775`](https://github.com/vrc-get/vrc-get/pull/1775) [`#1772`](https://github.com/vrc-get/vrc-get/pull/1772) [`#1779`](https://github.com/vrc-get/vrc-get/pull/1779)
- Removed `-debugCodeOptimization` from default unity arguments [`#1742`](https://github.com/vrc-get/vrc-get/pull/1742)
- Projects that failes to resolve will also be added to Project List now [`#1748`](https://github.com/vrc-get/vrc-get/pull/1748)
  - Previsously project dir is created but not added to list
- Dialog is shown when some installing packages are not found [`#1749`](https://github.com/vrc-get/vrc-get/pull/1749) [`#1776`](https://github.com/vrc-get/vrc-get/pull/1776)
  - The new dialog also suggest you to google & add repository for the package
  - Previously the first package we could not found are shown on the error toast but now collect and show missing packages as many as possible

### Fixed
- Prerelease version is choosen even if good stable version exists [`#1745`](https://github.com/vrc-get/vrc-get/pull/1745)

## [0.1.16] - 2024-11-12
### Added
- Support for China version of Unity releases like `2022.3.22f1c1` `#1558
- `rpm` `deb` packaging for Linux [`#1575`](https://github.com/vrc-get/vrc-get/pull/1575)
  - This is to test how good / bad `rpm` or `deb` distribution is.
  - We **may** create dnf / apt package repository in the future, but not planned for now.
- Skipping finding legacy assets when downgrading / upgrading / reinstalling package [`#1581`](https://github.com/vrc-get/vrc-get/pull/1581)
  - This will speed up the process of downgrading / upgrading / reinstalling package.

### Changed
- Separated quick open actions to own settings box. [`#1496`](https://github.com/vrc-get/vrc-get/pull/1496)
- Improved behavior with downloading package error [`#1557`](https://github.com/vrc-get/vrc-get/pull/1557)
- Installing unlocked package is now possible with warning [`#1557`](https://github.com/vrc-get/vrc-get/pull/1557)
- Added many logs for installing package [`#1557`](https://github.com/vrc-get/vrc-get/pull/1557)
- Migration feature is no longer marked as experimental [`#1559`](https://github.com/vrc-get/vrc-get/pull/1559)
- Several UX improvements [`#1561`](https://github.com/vrc-get/vrc-get/pull/1561) [`#1565`](https://github.com/vrc-get/vrc-get/pull/1565) [`#1569`](https://github.com/vrc-get/vrc-get/pull/1569) [`#1571`](https://github.com/vrc-get/vrc-get/pull/1571) [`#1573`](https://github.com/vrc-get/vrc-get/pull/1573)
- Added more error log [`#1652`](https://github.com/vrc-get/vrc-get/pull/1652)
- Improved error message when specified drive not found [`#1653`](https://github.com/vrc-get/vrc-get/pull/1653)

### Fixed
- Clicking VCC link while adding vpm repository would close previously opened add repository dialog [`#1570`](https://github.com/vrc-get/vrc-get/pull/1570)
- Opnening Templetes directory might fails [`#1641`](https://github.com/vrc-get/vrc-get/pull/1641)
- Backup file name is incorrect if project name contains '.' [`#1648`](https://github.com/vrc-get/vrc-get/pull/1648)
- Error creating project if the project path is "C:" [`#1651`](https://github.com/vrc-get/vrc-get/pull/1651)
- "missing field Verison" error if some unity version is missing [`#1654`](https://github.com/vrc-get/vrc-get/pull/1654)

## [0.1.15] - 2024-09-05
### Added
- System Information card to Settings Page [`#1406`](https://github.com/vrc-get/vrc-get/pull/1406)
- Traditional Chinese translation [`#1442`](https://github.com/vrc-get/vrc-get/pull/1442)
- Reinstall some selected packages [`#1450`](https://github.com/vrc-get/vrc-get/pull/1450)
- Install and Upgrade packages at once [`#1450`](https://github.com/vrc-get/vrc-get/pull/1450)
- Upgrade to the stable latest version even if some package has newer prerelease version [`#1450`](https://github.com/vrc-get/vrc-get/pull/1450)
- Buttons to open settings, logs, and templates location [`#1451`](https://github.com/vrc-get/vrc-get/pull/1451)
- Error page [`#1457`](https://github.com/vrc-get/vrc-get/pull/1457)
- Ctrl + F on Log, Projects List, and Project page will focus search box on the page [`#1485`](https://github.com/vrc-get/vrc-get/pull/1485)

### Changed
- GitHub Releases for ALCOM is no longer prereleases
- Moved log files to `<vpm-home>/vrc-get/gui-logs` [`#1446`](https://github.com/vrc-get/vrc-get/pull/1446)
- Logs pages overhaul [`#1456`](https://github.com/vrc-get/vrc-get/pull/1456)

### Fixed
- Fails to uninstall packages on macOS with filesystem that doesn't support resource fork [`#1402`](https://github.com/vrc-get/vrc-get/pull/1402)
  - This is typically seen on ExFAT or FAT32 filesystems, not on APFS or HFS+ filesystems.
  - macOS internally creates files starting with `._` for resource fork if the filesystem does not support resource fork.
  - vrc-get-vpm does not handle this file correctly and fails to uninstall the package.
- environment version mismatch error after resolving packages [`#1447`](https://github.com/vrc-get/vrc-get/pull/1447)
- Raw error for InstallAsUnlocked is shown on gui [`#1448`](https://github.com/vrc-get/vrc-get/pull/1448)
- Ctrl + F on Windows will show the search box by WebView2 [`#1485`](https://github.com/vrc-get/vrc-get/pull/1485)
- Project Path is shown instead of Project Name [`#1484`](https://github.com/vrc-get/vrc-get/pull/1484)

## [0.1.14] - 2024-08-13
### Added
- Check and better error for installing unlocked packages [`#1387`](https://github.com/vrc-get/vrc-get/pull/1387)
- Code signing for windows distribution [`#1391`](https://github.com/vrc-get/vrc-get/pull/1391)
  - I hope this would reduce problems with some protection system on windows

### Changed
- Save isMaximized instead of isFullScreen [`#1367`](https://github.com/vrc-get/vrc-get/pull/1367)
- Migrated to Tauri v2 Release Candidate [`#1350`](https://github.com/vrc-get/vrc-get/pull/1350) [`#1386`](https://github.com/vrc-get/vrc-get/pull/1386)
- Incognito mode of webview is now used [`#1388`](https://github.com/vrc-get/vrc-get/pull/1388)
  - This prevents webview from saving something to disk.
  - For macOS platform, `~/Library/WebKit/` will never be used.
  - For windows platform, due to WebView2 limitation, some cache will be saved.

### Fixed
- Unity Launched with ALCOM (AppImage) may crash on linux [`#1362`](https://github.com/vrc-get/vrc-get/pull/1362)
  - Some environment variables still referred ALCOM AppDir.
  - This PR fixes AppDir path for all environment variables.
- Tooltips on the Manage Packages table are not shown [`#1372`](https://github.com/vrc-get/vrc-get/pull/1372)
- Resolve needed button is shown if unlocked package in dependencies section [`#1387`](https://github.com/vrc-get/vrc-get/pull/1387)

## [0.1.13] - 2024-07-27
### Fixed
- Upgrading 0.1.11 to 0.1.12 will installs to wrong directory [`#1322`](https://github.com/vrc-get/vrc-get/pull/1322)

## [0.1.12] - 2024-07-25
### Added
- Way to customize default command-line arguments for unity [`#1304`](https://github.com/vrc-get/vrc-get/pull/1304)
  - You now can change default command-line arguments

### Changed
- Include commit hash for issue report [`#1288`](https://github.com/vrc-get/vrc-get/pull/1288)
- Use default config if `config.json` is corrupted [`#1307`](https://github.com/vrc-get/vrc-get/pull/1307)
  - Previously, ALCOM will crash if `config.json` is corrupted. Now, ALCOM will use default config if `config.json` is corrupted.

### Fixed
- Language Selector is very unstable [`#1287`](https://github.com/vrc-get/vrc-get/pull/1287)
- Typo in the bundle identifier [`#1291`](https://github.com/vrc-get/vrc-get/pull/1291)
  - The bundle identifier is changed to `com.anatawa12.vrc-get` from `com.anataw12.vrc-get`
  - This may leave the old bundle identifier in the registry. Please remove the old one manually.
- Not working as SPA on linux platform [`#1300`](https://github.com/vrc-get/vrc-get/pull/1300)
- Links are not working with KDE6 [`#1260`](https://github.com/vrc-get/vrc-get/pull/1260)
  - Since this version, system `xdg-open` will be used for opening links.
    Please make sure you have `xdg-open` in your environment.
    (I believe most modern desktop environment has that so I believe no opearation is needed)

## [0.1.11] - 2024-07-17
### Fixed
- Language Settings is not loaded on linux or macOS [`#1286`](https://github.com/vrc-get/vrc-get/pull/1286)

## [0.1.10] - 2024-07-16
### Added
- Change Unity version [`#1246`](https://github.com/vrc-get/vrc-get/pull/1246)

### Fixed
- Fails to open projects contains whitespace in project name on windows [`#1256`](https://github.com/vrc-get/vrc-get/pull/1256)

## [0.1.10-beta.4] - 2024-07-07
### Added
- Initial Setup Process [`#1214`](https://github.com/vrc-get/vrc-get/pull/1214)
  - There are few settings should be set up before using ALCOM.
  - Since this version, ALCOM will show the initial setup process if the settings are not set up.
- User Package Management [`#1222`](https://github.com/vrc-get/vrc-get/pull/1222)

### Fixed
- There is no way to remove ALCOM for `vcc:` link [`#1222`](https://github.com/vrc-get/vrc-get/pull/1222)
  - Since this version, ALCOM will remove ALCOM from the `vcc:` link registry on uninstallation.

## [0.1.10-beta.3] - 2024-07-04
### Added
- Importing / Exporting Repositories list [`#1209`](https://github.com/vrc-get/vrc-get/pull/1209)

### Fixed
- Maximized is not saved on windows [`#902`](https://github.com/vrc-get/vrc-get/pull/902)

## [0.1.10-beta.2] - 2024-07-03
Release pipeline fixes
## [0.1.10-beta.1] - 2024-07-03
### Added
- Button to clear package cache [`#1204`](https://github.com/vrc-get/vrc-get/pull/1204)

### Changed
- Error message will be shown if the SHA256 hash of the downloaded zip file does not match with the hash in the repository [`#1183`](https://github.com/vrc-get/vrc-get/pull/1183)
    - Currently, official VCC does not verify the hash of the downloaded zip file, but it's better to verify the hash.
    - For compatibility, even if the hash does not match, the file will be extracted with an error message.
    - In the future, we may make this a hard error.
- Localized upgrade notice dialog [`#1200`](https://github.com/vrc-get/vrc-get/pull/1200)
- Update notice can upgrade ALCOM to beta releases [`#1200`](https://github.com/vrc-get/vrc-get/pull/1200)

## [0.1.9] - 2024-06-26
### Fixed
- UI Freezes in many situations [`#1177`](https://github.com/vrc-get/vrc-get/pull/1177)

## [0.1.8] - 2024-06-25
### Added
- De-duplicating duplicated projects or Unity in VCC project list [`#1081`](https://github.com/vrc-get/vrc-get/pull/1081)
- Show package description on hovering package name / id [`#1118`](https://github.com/vrc-get/vrc-get/pull/1118)
- Customizing Command Line Arguments for Unity [`#1127`](https://github.com/vrc-get/vrc-get/pull/1127)
- Preserve Unity if multiple instance of the same unity version are installed [`#1127`](https://github.com/vrc-get/vrc-get/pull/1127)
- Open Project / Backup Location button on Settings Page [`#1140`](https://github.com/vrc-get/vrc-get/pull/1140)

### Changed
- Refine Dark Theme [`#1083`](https://github.com/vrc-get/vrc-get/pull/1083)
- Show package display name on uninstalled toast [`#1086`](https://github.com/vrc-get/vrc-get/pull/1086)
- Improved performance of package search [`#1131`](https://github.com/vrc-get/vrc-get/pull/1131)
- Swapped heroicons for lucide-react [`#1129`](https://github.com/vrc-get/vrc-get/pull/1129)

### Fixed
- Unity from Unity Hub will be registered as manually registered Unity [`#1081`](https://github.com/vrc-get/vrc-get/pull/1081)
- Unity selector radio button does not work well [`#1082`](https://github.com/vrc-get/vrc-get/pull/1082)
- `vcc:` link install button does not work well on linux [`#1117`](https://github.com/vrc-get/vrc-get/pull/1117)

## [0.1.7] - 2024-05-30
### Added
- Support for dark theme [`#1028`](https://github.com/vrc-get/vrc-get/pull/1028) [`#1044`](https://github.com/vrc-get/vrc-get/pull/1044)

### Changed
- Changed component library to shadcn [`#1028`](https://github.com/vrc-get/vrc-get/pull/1028)
- Informational message will be shown if you've installed fake latest because of the Unity version [`#1046`](https://github.com/vrc-get/vrc-get/pull/1046) [`#1061`](https://github.com/vrc-get/vrc-get/pull/1061) 
- Show newly installed packages and reinstalling packages separately [`#1052`](https://github.com/vrc-get/vrc-get/pull/1052)
- Prevents opening multiple unity instances [`#1055`](https://github.com/vrc-get/vrc-get/pull/1055) [`#1062`](https://github.com/vrc-get/vrc-get/pull/1062)
- Migration will be prevented if the project is opened in Unity [`#1055`](https://github.com/vrc-get/vrc-get/pull/1055) [`#1062`](https://github.com/vrc-get/vrc-get/pull/1062)

### Fixed
- Clicking '+' of the incompatible package will do nothing [`#1046`](https://github.com/vrc-get/vrc-get/pull/1046)
- Opening Manage Project page will cause VCC to crash [`#1048`](https://github.com/vrc-get/vrc-get/pull/1048)
- Fails to load installed unity versions if Unity 2018 is installed again [`#1051`](https://github.com/vrc-get/vrc-get/pull/1051)

## [0.1.6] - 2024-05-21
### Fixed
- Fails to load installed unity versions if Unity 2018 is installed [`#1024`](https://github.com/vrc-get/vrc-get/pull/1024)

## [0.1.5] - 2024-05-21
### Removed
- Support perMachine install [`#1021`](https://github.com/vrc-get/vrc-get/pull/1021)
  - Due to problems of tauri updater and installer, it's not good to support perMachine for now.

## [0.1.4] - 2024-05-20
### Added
- `vcc://` support [`#978`](https://github.com/vrc-get/vrc-get/pull/978)
  - This is enabled by default for macOS and you have to enable manually on Settings page for windows and linux.
- per-package `headers` field support [`#718`](https://github.com/vrc-get/vrc-get/pull/718)

### Changed
- Improved project Template selection [`#967`](https://github.com/vrc-get/vrc-get/pull/967)
- Ask installing unity for a project if not installed [`#988`](https://github.com/vrc-get/vrc-get/pull/988)
- Removed Visual Scripting from dependencies of template projects [`#991`](https://github.com/vrc-get/vrc-get/pull/991)
- Support more legacy browsers [`#994`](https://github.com/vrc-get/vrc-get/pull/994)
- Improved UI with narrow windows [`#1003`](https://github.com/vrc-get/vrc-get/pull/1003)
- Make package name gray-outed if not installed [`#1018`](https://github.com/vrc-get/vrc-get/pull/1018)

### Fixed
- Impossible to install for machine (for Windows) [`#976`](https://github.com/vrc-get/vrc-get/pull/976)
- Japanese variant of CJK Ideograph is used for Simplified Chinese [`#980`](https://github.com/vrc-get/vrc-get/pull/980)
  - Since this version, ALCOM will always use `system-ui` font for all languages.
- Last modified is not updated on opening unity again [`#995`](https://github.com/vrc-get/vrc-get/pull/995)

## [0.1.3] - 2024-05-13
### Added
- Template for unity 2022.3.6f1 [`#956`](https://github.com/vrc-get/vrc-get/pull/956)

### Changed
- Support repositories with bad packages [`#954`](https://github.com/vrc-get/vrc-get/pull/954)
- Use url in settings.json to load remote repository [`#955`](https://github.com/vrc-get/vrc-get/pull/955)

### Fixed
- Project page is not refreshed after unity patch migration [`#941`](https://github.com/vrc-get/vrc-get/pull/941)
- VCC Crashes after opening settings page [`#942`](https://github.com/vrc-get/vrc-get/pull/942)
- Resolve needed check ignores legacy packages [`#952`](https://github.com/vrc-get/vrc-get/pull/952)

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

[Unreleased]: https://github.com/vrc-get/vrc-get/compare/gui-v1.0.1...HEAD
[1.0.1]: https://github.com/vrc-get/vrc-get/compare/gui-v1.0.0...gui-v1.0.1
[1.0.0]: https://github.com/vrc-get/vrc-get/compare/gui-v0.1.17...gui-v1.0.0
[0.1.17]: https://github.com/vrc-get/vrc-get/compare/gui-v0.1.16...gui-v0.1.17
[0.1.16]: https://github.com/vrc-get/vrc-get/compare/gui-v0.1.15...gui-v0.1.16
[0.1.15]: https://github.com/vrc-get/vrc-get/compare/gui-v0.1.14...gui-v0.1.15
[0.1.14]: https://github.com/vrc-get/vrc-get/compare/gui-v0.1.13...gui-v0.1.14
[0.1.13]: https://github.com/vrc-get/vrc-get/compare/gui-v0.1.12...gui-v0.1.13
[0.1.12]: https://github.com/vrc-get/vrc-get/compare/gui-v0.1.11...gui-v0.1.12
[0.1.11]: https://github.com/vrc-get/vrc-get/compare/gui-v0.1.10...gui-v0.1.11
[0.1.10]: https://github.com/vrc-get/vrc-get/compare/gui-v0.1.10-beta.4...gui-v0.1.10
[0.1.10-beta.4]: https://github.com/vrc-get/vrc-get/compare/gui-v0.1.10-beta.3...gui-v0.1.10-beta.4
[0.1.10-beta.3]: https://github.com/vrc-get/vrc-get/compare/gui-v0.1.10-beta.2...gui-v0.1.10-beta.3
[0.1.10-beta.2]: https://github.com/vrc-get/vrc-get/compare/gui-v0.1.10-beta.1...gui-v0.1.10-beta.2
[0.1.10-beta.1]: https://github.com/vrc-get/vrc-get/compare/gui-v0.1.9...gui-v0.1.10-beta.1
[0.1.9]: https://github.com/vrc-get/vrc-get/compare/gui-v0.1.8...gui-v0.1.9
[0.1.8]: https://github.com/vrc-get/vrc-get/compare/gui-v0.1.7...gui-v0.1.8
[0.1.7]: https://github.com/vrc-get/vrc-get/compare/gui-v0.1.6...gui-v0.1.7
[0.1.6]: https://github.com/vrc-get/vrc-get/compare/gui-v0.1.5...gui-v0.1.6
[0.1.5]: https://github.com/vrc-get/vrc-get/compare/gui-v0.1.4...gui-v0.1.5
[0.1.4]: https://github.com/vrc-get/vrc-get/compare/gui-v0.1.3...gui-v0.1.4
[0.1.3]: https://github.com/vrc-get/vrc-get/compare/gui-v0.1.2...gui-v0.1.3
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
