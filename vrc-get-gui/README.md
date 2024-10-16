# ALCOM (experimental)

This folder contains the experimental GUI version of vrc-get, ALCOM.

[Homepage (Help Wanted)](https://vrc-get.anatawa12.com/alcom/)

## Installation

The recommended way to install ALCOM is download from [GitHub Releases][alcom-releases].

If you want, you may download the HEAD build from [GitHub Actions][alcom-nightly]

[alcom-releases]: https://github.com/anatawa12/vrc-get/releases?q=gui-v0
[alcom-nightly]: https://github.com/vrc-get/vrc-get/actions/workflows/ci-gui.yml?query=branch%3Amaster

## Requirements (building)

To build ALCOM, you need to have the following installed:

- [Node.js] LTS — to build the web part of the project
- [npm] v10 — to install the dependencies of the web part (bundled with node.js so no extra attention needed in most case)
- [cargo] latest — to build the most part of the project
- [cargo-about] latest — to generate the licenses json (for development, not required but required for building release binary)
- [.NET SDK] v8 — to build vrc-get-litedb crate

Please note that ALCOM requires the latest version of cargo and cargo-about at that time. 
We update the required version of cargo and cargo-about without notice.
Therefore, you may need to update them before building the project.

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

For how to contribute localization to ALCOM (vrc-get-gui): [CONTRIBUTING.md](CONTRIBUTING.md) (**Please read [../CONTRIBUTING.md#configuration-requirements](../CONTRIBUTING.md#configuration-requirements) first before you read [CONTRIBUTING.md](CONTRIBUTING.md)!**)
