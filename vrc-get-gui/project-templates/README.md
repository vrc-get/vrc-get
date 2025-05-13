# Templates for vrc-get

This directory contains project templates used in vrc-get.

This template has several changes to improve user experience.

- There is no package.json in the root directory, which is metadata only for VCC.
- Several packages are upgraded in 2022 based on the migrated project. In VCC, many packages are outdated.
- The configuration file for com.vrchat.base with `"samplesHintCreated": true` is included since it's meaningless for new creators.
- new line for new scripts is OSNative instead of Windows since it is more common.
- Configurations for iOS platform are included since there's code for iOS in the VRCSDK.
  - Please note that iOS is not officially supported by VRChat, but I added this for future updates.
- `productGuid` is randomly initialized on creation with `00000000000000000000000000000000`.
- `productName` is created with same name as folder name with `{vrc-get-productName}`.
- For both unity versions, versions of some editor support packages are upgraded to a newer version.
