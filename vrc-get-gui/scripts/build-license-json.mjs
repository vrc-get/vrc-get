/**
 * This file is used to generate a JSON file containing the licenses of all the dependencies.
 * This is based on the output of `cargo about generate --format=json`.
 */
import { exec as execCallback } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync } from "node:fs";
import { mkdir, readFile, readdir, writeFile } from "node:fs/promises";
import { promisify } from "node:util";

const exec = promisify(execCallback);

async function shouldRebuild() {
	async function readHashes() {
		try {
			return JSON.parse(await readFile("build/licenses.hashes.json", "utf8"));
		} catch (e) {
			return {};
		}
	}

	// compute hashes first
	let packageLockHash;
	let cargoLockHash;
	try {
		const packageLock = await readFile("package-lock.json", "utf8");
		packageLockHash = createHash("sha256").update(packageLock).digest("hex");
		const cargoLock = await readFile("../Cargo.lock", "utf8");
		cargoLockHash = createHash("sha256").update(cargoLock).digest("hex");
	} catch (e) {
		console.error("Error computing hash of lock file", e);
		return true;
	}

	try {
		let result;
		if (existsSync("build/licenses.json")) {
			const oldHashes = await readHashes();
			const oldPackageLockHash = oldHashes.packageLockHash;
			const oldCargoLockHash = oldHashes.cargoLockHash;
			console.log("Old package lock hash:", oldPackageLockHash);
			console.log("New package lock hash:", packageLockHash);
			console.log("Old cargo lock hash:", oldCargoLockHash);
			console.log("New cargo lock hash:", cargoLockHash);
			result =
				packageLockHash !== oldPackageLockHash ||
				cargoLockHash !== oldCargoLockHash;
		} else {
			console.log("build/licenses.json does not exist, rebuilding");
			result = true;
		}

		await mkdir("build", { recursive: true });
		await writeFile(
			"build/licenses.hashes.json",
			JSON.stringify({ packageLockHash, cargoLockHash }),
		);

		return result;
	} catch (e) {
		console.error(e);
		return true;
	}
}

/**
 * @interface CargoAbout
 * @property {CargoAboutLicense[]} licenses
 */

/**
 * @interface CargoAboutLicense
 * @property {string} name
 * @property {string | undefined} id
 * @property {string} short_id
 * @property {string} text
 * @property {CargoAboutUsedBy[]} used_by
 */

/**
 * @interface CargoAboutUsedBy
 * @property {CargoAboutCrate} crate
 */

/**
 * @interface CargoAboutCrate
 * @property {string} name
 * @property {string} version
 * @property {string} repository
 */

/**
 * @return {Promise<CargoAbout>}
 */
async function callCargoAbout() {
	const { stdout } = await exec("cargo about generate --format=json", {
		maxBuffer: Number.MAX_SAFE_INTEGER,
		encoding: "utf8",
	});
	return JSON.parse(stdout);
}

/**
 * @return {Promise<PackageLicenseInfo[]>}
 */
async function getLicencesFromPackageLockJson() {
	/**
	 * @type { {packages: {[p: string]: { dev?: boolean, license?: string, name?: string, version: string, optional?: boolean }}}}
	 */
	const data = JSON.parse(await readFile("package-lock.json", "utf8"));

	// some package doesn't have license key so listing here
	/** @type {Record<string, string>} */
	const knownLicenses = {
		streamsearch: "MIT",
		busboy: "MIT",
	};

	/**
	 * @type {PackageLicenseInfo[]}
	 */
	const result = [];

	for (const [packagePath, pkg] of Object.entries(data.packages)) {
		if (pkg.dev) continue; // we don't have to list-up dev packages
		if (packagePath === "") continue; // package itself
		const name =
			pkg.name ??
			packagePath.substring(
				packagePath.lastIndexOf("node_modules/") + "node_modules/".length,
			);
		const licenseId = pkg.license ?? knownLicenses[name];
		if (licenseId == null) {
			throw new Error(`no licenses for ${name}`);
		}

		let licenseText;
		if (!pkg.optional) {
			// find for LICENSE, LICENSE.txt, or license.md
			const licensesFile = (await readdir(packagePath)).find(
				(x) =>
					x.toLowerCase() === "license" ||
					x.toLowerCase() === "license.txt" ||
					x.toLowerCase() === "license.md",
			);
			if (licensesFile)
				licenseText = await readFile(`${packagePath}/${licensesFile}`, "utf-8");
		}

		result.push({
			name,
			version: pkg.version,
			url: `https://www.npmjs.com/package/${name}/v/${pkg.version}`,
			licenseId,
			licenseText,
		});
	}

	return result;
}

if (!(await shouldRebuild())) {
	console.log("Cache matched, skipping");
	process.exit(0);
}

/**
 * @type {Promise<[CargoAbout, PackageLicenseInfo[]]>}
 */
const promise = await Promise.all([
	callCargoAbout(),
	getLicencesFromPackageLockJson(),
]);
const [cargoAbout, packageLockJson] = promise;

/** @type {Map<string, string>} */
const licenseNames = new Map();

licenseNames.set("MIT", "MIT License");
licenseNames.set("ISC", "ISC License");
licenseNames.set("Apache-2.0", "Apache License 2.0");
licenseNames.set("MPL-2.0", "Mozilla Public License 2.0");
licenseNames.set("OFL-1.1", "SIL Open Font License 1.1");
licenseNames.set("BlueOak-1.0.0", "Blue Oak Model License 1.0.0");

licenseNames.set("OpenSSL", "OpenSSL License");
licenseNames.set("CC-BY-4.0", "Creative Commons Attribution 4.0");
licenseNames.set(
	"Unicode-DFS-2016",
	"Unicode License Agreement - Data Files and Software (2016)",
);
licenseNames.set("Unicode-3.0", "Unicode License v3");

licenseNames.set("0BSD", "BSD Zero Clause License");
licenseNames.set("BSD-2-Clause", "BSD 2-Clause License");
licenseNames.set("BSD-3-Clause", "BSD 3-Clause License");

/** @type {Map<string, string>} */
const defaultLicenseTexts = new Map();

// lang=plaintext
defaultLicenseTexts.set(
	"MIT",
	"MIT License\n" +
		"\n" +
		"Copyright (c) <year> <copyright holders>\n" +
		"\n" +
		"Permission is hereby granted, free of charge, to any person obtaining a copy of this software and " +
		'associated documentation files (the "Software"), to deal in the Software without restriction, ' +
		"including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, " +
		"and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, " +
		"subject to the following conditions:\n" +
		"\n" +
		"The above copyright notice and this permission notice shall be " +
		"included in all copies or substantial portions of the Software.\n" +
		"\n" +
		'THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO ' +
		"THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE " +
		"AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, " +
		"TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE " +
		"SOFTWARE.\n",
);

defaultLicenseTexts.set(
	"Apache-2.0",
	"                                 Apache License\n" +
		"                           Version 2.0, January 2004\n" +
		"                        http://www.apache.org/licenses/\n" +
		"\n" +
		"   TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION\n" +
		"\n" +
		"   1. Definitions.\n" +
		"\n" +
		'      "License" shall mean the terms and conditions for use, reproduction,\n' +
		"      and distribution as defined by Sections 1 through 9 of this document.\n" +
		"\n" +
		'      "Licensor" shall mean the copyright owner or entity authorized by\n' +
		"      the copyright owner that is granting the License.\n" +
		"\n" +
		'      "Legal Entity" shall mean the union of the acting entity and all\n' +
		"      other entities that control, are controlled by, or are under common\n" +
		"      control with that entity. For the purposes of this definition,\n" +
		'      "control" means (i) the power, direct or indirect, to cause the\n' +
		"      direction or management of such entity, whether by contract or\n" +
		"      otherwise, or (ii) ownership of fifty percent (50%) or more of the\n" +
		"      outstanding shares, or (iii) beneficial ownership of such entity.\n" +
		"\n" +
		'      "You" (or "Your") shall mean an individual or Legal Entity\n' +
		"      exercising permissions granted by this License.\n" +
		"\n" +
		'      "Source" form shall mean the preferred form for making modifications,\n' +
		"      including but not limited to software source code, documentation\n" +
		"      source, and configuration files.\n" +
		"\n" +
		'      "Object" form shall mean any form resulting from mechanical\n' +
		"      transformation or translation of a Source form, including but\n" +
		"      not limited to compiled object code, generated documentation,\n" +
		"      and conversions to other media types.\n" +
		"\n" +
		'      "Work" shall mean the work of authorship, whether in Source or\n' +
		"      Object form, made available under the License, as indicated by a\n" +
		"      copyright notice that is included in or attached to the work\n" +
		"      (an example is provided in the Appendix below).\n" +
		"\n" +
		'      "Derivative Works" shall mean any work, whether in Source or Object\n' +
		"      form, that is based on (or derived from) the Work and for which the\n" +
		"      editorial revisions, annotations, elaborations, or other modifications\n" +
		"      represent, as a whole, an original work of authorship. For the purposes\n" +
		"      of this License, Derivative Works shall not include works that remain\n" +
		"      separable from, or merely link (or bind by name) to the interfaces of,\n" +
		"      the Work and Derivative Works thereof.\n" +
		"\n" +
		'      "Contribution" shall mean any work of authorship, including\n' +
		"      the original version of the Work and any modifications or additions\n" +
		"      to that Work or Derivative Works thereof, that is intentionally\n" +
		"      submitted to Licensor for inclusion in the Work by the copyright owner\n" +
		"      or by an individual or Legal Entity authorized to submit on behalf of\n" +
		'      the copyright owner. For the purposes of this definition, "submitted"\n' +
		"      means any form of electronic, verbal, or written communication sent\n" +
		"      to the Licensor or its representatives, including but not limited to\n" +
		"      communication on electronic mailing lists, source code control systems,\n" +
		"      and issue tracking systems that are managed by, or on behalf of, the\n" +
		"      Licensor for the purpose of discussing and improving the Work, but\n" +
		"      excluding communication that is conspicuously marked or otherwise\n" +
		'      designated in writing by the copyright owner as "Not a Contribution."\n' +
		"\n" +
		'      "Contributor" shall mean Licensor and any individual or Legal Entity\n' +
		"      on behalf of whom a Contribution has been received by Licensor and\n" +
		"      subsequently incorporated within the Work.\n" +
		"\n" +
		"   2. Grant of Copyright License. Subject to the terms and conditions of\n" +
		"      this License, each Contributor hereby grants to You a perpetual,\n" +
		"      worldwide, non-exclusive, no-charge, royalty-free, irrevocable\n" +
		"      copyright license to reproduce, prepare Derivative Works of,\n" +
		"      publicly display, publicly perform, sublicense, and distribute the\n" +
		"      Work and such Derivative Works in Source or Object form.\n" +
		"\n" +
		"   3. Grant of Patent License. Subject to the terms and conditions of\n" +
		"      this License, each Contributor hereby grants to You a perpetual,\n" +
		"      worldwide, non-exclusive, no-charge, royalty-free, irrevocable\n" +
		"      (except as stated in this section) patent license to make, have made,\n" +
		"      use, offer to sell, sell, import, and otherwise transfer the Work,\n" +
		"      where such license applies only to those patent claims licensable\n" +
		"      by such Contributor that are necessarily infringed by their\n" +
		"      Contribution(s) alone or by combination of their Contribution(s)\n" +
		"      with the Work to which such Contribution(s) was submitted. If You\n" +
		"      institute patent litigation against any entity (including a\n" +
		"      cross-claim or counterclaim in a lawsuit) alleging that the Work\n" +
		"      or a Contribution incorporated within the Work constitutes direct\n" +
		"      or contributory patent infringement, then any patent licenses\n" +
		"      granted to You under this License for that Work shall terminate\n" +
		"      as of the date such litigation is filed.\n" +
		"\n" +
		"   4. Redistribution. You may reproduce and distribute copies of the\n" +
		"      Work or Derivative Works thereof in any medium, with or without\n" +
		"      modifications, and in Source or Object form, provided that You\n" +
		"      meet the following conditions:\n" +
		"\n" +
		"      (a) You must give any other recipients of the Work or\n" +
		"          Derivative Works a copy of this License; and\n" +
		"\n" +
		"      (b) You must cause any modified files to carry prominent notices\n" +
		"          stating that You changed the files; and\n" +
		"\n" +
		"      (c) You must retain, in the Source form of any Derivative Works\n" +
		"          that You distribute, all copyright, patent, trademark, and\n" +
		"          attribution notices from the Source form of the Work,\n" +
		"          excluding those notices that do not pertain to any part of\n" +
		"          the Derivative Works; and\n" +
		"\n" +
		'      (d) If the Work includes a "NOTICE" text file as part of its\n' +
		"          distribution, then any Derivative Works that You distribute must\n" +
		"          include a readable copy of the attribution notices contained\n" +
		"          within such NOTICE file, excluding those notices that do not\n" +
		"          pertain to any part of the Derivative Works, in at least one\n" +
		"          of the following places: within a NOTICE text file distributed\n" +
		"          as part of the Derivative Works; within the Source form or\n" +
		"          documentation, if provided along with the Derivative Works; or,\n" +
		"          within a display generated by the Derivative Works, if and\n" +
		"          wherever such third-party notices normally appear. The contents\n" +
		"          of the NOTICE file are for informational purposes only and\n" +
		"          do not modify the License. You may add Your own attribution\n" +
		"          notices within Derivative Works that You distribute, alongside\n" +
		"          or as an addendum to the NOTICE text from the Work, provided\n" +
		"          that such additional attribution notices cannot be construed\n" +
		"          as modifying the License.\n" +
		"\n" +
		"      You may add Your own copyright statement to Your modifications and\n" +
		"      may provide additional or different license terms and conditions\n" +
		"      for use, reproduction, or distribution of Your modifications, or\n" +
		"      for any such Derivative Works as a whole, provided Your use,\n" +
		"      reproduction, and distribution of the Work otherwise complies with\n" +
		"      the conditions stated in this License.\n" +
		"\n" +
		"   5. Submission of Contributions. Unless You explicitly state otherwise,\n" +
		"      any Contribution intentionally submitted for inclusion in the Work\n" +
		"      by You to the Licensor shall be under the terms and conditions of\n" +
		"      this License, without any additional terms or conditions.\n" +
		"      Notwithstanding the above, nothing herein shall supersede or modify\n" +
		"      the terms of any separate license agreement you may have executed\n" +
		"      with Licensor regarding such Contributions.\n" +
		"\n" +
		"   6. Trademarks. This License does not grant permission to use the trade\n" +
		"      names, trademarks, service marks, or product names of the Licensor,\n" +
		"      except as required for reasonable and customary use in describing the\n" +
		"      origin of the Work and reproducing the content of the NOTICE file.\n" +
		"\n" +
		"   7. Disclaimer of Warranty. Unless required by applicable law or\n" +
		"      agreed to in writing, Licensor provides the Work (and each\n" +
		'      Contributor provides its Contributions) on an "AS IS" BASIS,\n' +
		"      WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or\n" +
		"      implied, including, without limitation, any warranties or conditions\n" +
		"      of TITLE, NON-INFRINGEMENT, MERCHANTABILITY, or FITNESS FOR A\n" +
		"      PARTICULAR PURPOSE. You are solely responsible for determining the\n" +
		"      appropriateness of using or redistributing the Work and assume any\n" +
		"      risks associated with Your exercise of permissions under this License.\n" +
		"\n" +
		"   8. Limitation of Liability. In no event and under no legal theory,\n" +
		"      whether in tort (including negligence), contract, or otherwise,\n" +
		"      unless required by applicable law (such as deliberate and grossly\n" +
		"      negligent acts) or agreed to in writing, shall any Contributor be\n" +
		"      liable to You for damages, including any direct, indirect, special,\n" +
		"      incidental, or consequential damages of any character arising as a\n" +
		"      result of this License or out of the use or inability to use the\n" +
		"      Work (including but not limited to damages for loss of goodwill,\n" +
		"      work stoppage, computer failure or malfunction, or any and all\n" +
		"      other commercial damages or losses), even if such Contributor\n" +
		"      has been advised of the possibility of such damages.\n" +
		"\n" +
		"   9. Accepting Warranty or Additional Liability. While redistributing\n" +
		"      the Work or Derivative Works thereof, You may choose to offer,\n" +
		"      and charge a fee for, acceptance of support, warranty, indemnity,\n" +
		"      or other liability obligations and/or rights consistent with this\n" +
		"      License. However, in accepting such obligations, You may act only\n" +
		"      on Your own behalf and on Your sole responsibility, not on behalf\n" +
		"      of any other Contributor, and only if You agree to indemnify,\n" +
		"      defend, and hold each Contributor harmless for any liability\n" +
		"      incurred by, or claims asserted against, such Contributor by reason\n" +
		"      of your accepting any such warranty or additional liability.\n" +
		"\n" +
		"   END OF TERMS AND CONDITIONS\n" +
		"\n" +
		"   APPENDIX: How to apply the Apache License to your work.\n" +
		"\n" +
		"      To apply the Apache License to your work, attach the following\n" +
		'      boilerplate notice, with the fields enclosed by brackets "[]"\n' +
		"      replaced with your own identifying information. (Don't include\n" +
		"      the brackets!)  The text should be enclosed in the appropriate\n" +
		"      comment syntax for the file format. We also recommend that a\n" +
		"      file or class name and description of purpose be included on the\n" +
		'      same "printed page" as the copyright notice for easier\n' +
		"      identification within third-party archives.\n" +
		"\n" +
		"   Copyright [yyyy] [name of copyright owner]\n" +
		"\n" +
		'   Licensed under the Apache License, Version 2.0 (the "License");\n' +
		"   you may not use this file except in compliance with the License.\n" +
		"   You may obtain a copy of the License at\n" +
		"\n" +
		"       http://www.apache.org/licenses/LICENSE-2.0\n" +
		"\n" +
		"   Unless required by applicable law or agreed to in writing, software\n" +
		'   distributed under the License is distributed on an "AS IS" BASIS,\n' +
		"   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.\n" +
		"   See the License for the specific language governing permissions and\n" +
		"   limitations under the License.\n",
);

// ライセンスの種別、実テキストごとに分ける

/**
 * @interface PackageLicenseInfo
 * @property {string} name
 * @property {string} version
 * @property {string} url
 * @property {string} licenseId
 * @property {string|undefined} licenseText
 */

/**
 * @interface PackageInfo
 * @property {string} name
 * @property {string} version
 * @property {string} url
 */

/** @type {Map<string, Map<string, PackageInfo[]>>} */
const licenses = new Map();

/**
 * @param packageInfo {PackageLicenseInfo}
 */
function addPackageToLicenses(packageInfo) {
	const licenseId = packageInfo.licenseId;
	let licenseText = packageInfo.licenseText;

	if (licenseText == null) {
		licenseText = defaultLicenseTexts.get(licenseId);
		if (!licenseText) {
			throw new Error(
				`No license text for ${packageInfo.name}@${packageInfo.version}: ${licenseId}`,
			);
		}
	}

	const licenseByText = licenses.get(licenseId) ?? new Map();
	licenses.set(licenseId, licenseByText);
	const packagesOfTheLicense = licenseByText.get(licenseText) ?? [];
	licenseByText.set(licenseText, packagesOfTheLicense);
	packagesOfTheLicense.push(packageInfo);
}

// add npm libraries
for (/** @type {PackageLicenseInfo} */ const packageInfo of packageLockJson) {
	if (packageInfo.name.startsWith("@tauri-apps/")) continue; // tauri apps should be added as rust

	addPackageToLicenses(packageInfo);
}

// add rust libraries
for (const license of cargoAbout.licenses) {
	const licneseId = license.id ?? license.short_id;
	if (licneseId == null)
		throw new Error(`No license for ${JSON.stringify(license)}`);
	for (const usedBy of license.used_by) {
		addPackageToLicenses({
			name: usedBy.crate.name,
			version: usedBy.crate.version,
			licenseId: license.id ?? license.short_id,
			licenseText: license.text,
		});
	}
}

// other third-party things

// Anton font
addPackageToLicenses({
	name: "Anton font",
	version: "1.0.0",
	url: "https://fonts.google.com/specimen/Anton",
	licenseId: "OFL-1.1",
	licenseText: await readFile("third-party/Anton-Regular-OFL.txt", "utf-8"),
});

// The logo
addPackageToLicenses({
	name: "ALCOM Icon",
	version: "1.0.0",
	url: "https://github.com/vrc-get/vrc-get",
	licenseId: "CC-BY-4.0",
	licenseText: await readFile("icon-LICENSE", "utf-8"),
});

// finally, put to array
const result = [];

for (const [license, licenseByText] of licenses) {
	const name = licenseNames.get(license);
	if (!name) throw new Error(`Unknown license: ${license}`);
	for (const [text, packages] of licenseByText) {
		result.push({
			id: license,
			name,
			text,
			packages,
		});
	}
}

await mkdir("build", { recursive: true });
await writeFile("build/licenses.json", JSON.stringify(result));
