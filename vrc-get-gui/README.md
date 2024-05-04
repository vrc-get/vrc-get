# ALCOM (experimental)

This folder contains the experimental GUI version of vrc-get, ALCOM.

## Installation

The recommended way to install ALCOM is download from [GitHub Releases][alcom-releases].

[alcom-releases]: https://github.com/anatawa12/vrc-get/releases?q=gui-v0

## Requirements (building)

To build ALCOM, you need to have the following installed:

- [Node.js] v20 (to build the web part of the project)
- [npm] (to install the dependencies of the web part)
- [cargo] (to build the Rust part of the project)
- [cargo-about] (to generate the licenses json)
- [.NET SDK] v8 (to build vrc-get-litedb crate)

[Node.js]: https://nodejs.org/en
[npm]: https://www.npmjs.com
[cargo]: https://doc.rust-lang.org/cargo/
[cargo-about]: https://github.com/EmbarkStudios/cargo-about
[.NET SDK]: https://dotnet.microsoft.com/download

## Building

To build the project, run the following command:

```bash
npm run tauri build
```

## Development

ALCOM is currently based on tauri and next.js.

Run `npm run tauri dev` to start the development server and gui.

## Contribution

For how to contribute localization to ALCOM (vrc-get-gui): [CONTRIBUTING.md](CONTRIBUTING.md) (**Please read [/CONTRIBUTING.md#configuration-requirements](.../CONTRIBUTING.md#configuration-requirements) first before you read [CONTRIBUTING.md](CONTRIBUTING.md)!**)