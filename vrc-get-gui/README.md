# vrc-get-gui (experimental)

This folder contains the experimental GUI version of vrc-get.

## Installation

The recommended way to install vrc-get-gui is download from [GitHub Releases][vrc-get-gui-releases].

[vrc-get-gui-releases]: https://github.com/anatawa12/vrc-get/releases?q=gui-v0

## Requirements (building)

To build vrc-get-gui, you need to have the following installed:

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

vrc-get-gui is currently based on tauri and next.js.

Run `npm run tauri dev` to start the development server and gui.
