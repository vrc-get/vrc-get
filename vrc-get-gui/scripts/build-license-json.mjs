/**
 * This file is used to generate a JSON file containing the licenses of all the dependencies.
 * This is based on the output of `cargo about generate --format=json` and `npx license-checker --production --json`.
 */
import {promisify} from "node:util";
import {execFile as execFileCallback} from "node:child_process";
import {mkdir, readFile, writeFile} from "node:fs/promises";

const execFile = promisify(execFileCallback);

/**
 * @interface CargoAbout
 * @property {CargoAboutLicense[]} licenses
 */

/**
 * @interface CargoAboutLicense
 * @property {string} name
 * @property {string} id
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
	const {stdout} = await execFile("cargo", ["about", "generate", "--format=json"], {
		maxBuffer: Number.MAX_SAFE_INTEGER,
		encoding: "utf8",
	});
	return JSON.parse(stdout);
}

/**
 * @typedef {Record<string, LicenseCheckerModule>} LicenseChecker
 */

/**
 * @interface LicenseCheckerModule
 * @property {string} licenses
 * @property {string|undefined} licenseFile
 */

/**
 * @return {Promise<LicenseChecker>}
 */
async function callLicenseChecker() {
	const {stdout} = await execFile("npx", ["license-checker", "--production", "--json"], {
		encoding: "utf8",
	});
	return JSON.parse(stdout);
}

/**
 * @typedef {Record<string, LicenseCheckerModuleWithFile>} LicenseCheckerWithFiles
 */

/**
 * @interface LicenseCheckerModuleWithFile
 * @property {string} licenses
 * @property {string|undefined} licenseFile
 * @property {string|undefined} licenseText
 */

/**
 * @return {Promise<LicenseCheckerWithFiles>}
 */
async function licenseCheckerWithLicenseText() {
	const result = await callLicenseChecker();
	await Promise.all(Object.values(result).map(async module => {
		if (!module.licenseFile) return;
		if (module.licenseFile.endsWith("README.md")) return; // ignore README.md since it's not a license file
		const file = await readFile(module.licenseFile, "utf8");
		module.licenseText = file;
	}));
	return result;
}

/**
 * @type {Promise<[CargoAbout, LicenseCheckerWithFiles]>}
 */
const promise = await Promise.all([callCargoAbout(), licenseCheckerWithLicenseText()]);
const [cargoAbout, licenseChecker] = promise;

/** @type {Map<string, string>} */
const licenseNames = new Map();

licenseNames.set("MIT", "MIT License");
licenseNames.set("ISC", "ISC License");
licenseNames.set("Apache-2.0", "Apache License 2.0");
licenseNames.set("MPL-2.0", "Mozilla Public License 2.0");

licenseNames.set("OpenSSL", "OpenSSL License");
licenseNames.set("CC-BY-4.0", "Creative Commons Attribution 4.0");
licenseNames.set("Unicode-DFS-2016", "Unicode License Agreement - Data Files and Software (2016)");

licenseNames.set("0BSD", "BSD Zero Clause License");
licenseNames.set("BSD-2-Clause", "BSD 2-Clause License");
licenseNames.set("BSD-3-Clause", "BSD 3-Clause License");

/** @type {Map<string, string>} */
const defaultLicenseTexts = new Map();

// lang=plaintext
defaultLicenseTexts.set("MIT", "MIT License\n" +
	"\n" +
	"Copyright (c) <year> <copyright holders>\n" +
	"\n" +
	"Permission is hereby granted, free of charge, to any person obtaining a copy of this software and " +
	"associated documentation files (the \"Software\"), to deal in the Software without restriction, " +
	"including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, " +
	"and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, " +
	"subject to the following conditions:\n" +
	"\n" +
	"The above copyright notice and this permission notice shall be " +
	"included in all copies or substantial portions of the Software.\n" +
	"\n" +
	"THE SOFTWARE IS PROVIDED \"AS IS\", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO " +
	"THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE " +
	"AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, " +
	"TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE " +
	"SOFTWARE.\n");

// ライセンスの種別、実テキストごとに分ける

/**
 * @interface PackageInfo
 * @property {string} name
 * @property {string} version
 * @property {string} url
 */

/** @type {Map<string, Map<string, PackageInfo[]>>} */
const licenses = new Map();

// add npm libraries
for (let [pkgNameAndVersion, module] of Object.entries(licenseChecker)) {
	const at = pkgNameAndVersion.lastIndexOf("@");
	const pkgName = pkgNameAndVersion.slice(0, at);
	if (pkgName === "vrc-get-gui") continue; // the package itself
	if (pkgName.startsWith("@tauri-apps/")) continue; // tauri apps should be added as rust
	const pkgVersion = pkgNameAndVersion.slice(at + 1);
	const license = module.licenses;
	const licenseByText = licenses.get(license) ?? new Map();
	licenses.set(license, licenseByText);
	const licenseText = module.licenseText ?? defaultLicenseTexts.get(license);
	if (!licenseText) throw new Error(`No license text for ${pkgNameAndVersion}: ${license}`);
	const packagesOfTheLicense = licenseByText.get(licenseText) ?? [];
	licenseByText.set(licenseText, packagesOfTheLicense);
	packagesOfTheLicense.push({
		name: pkgName,
		version: pkgVersion,
		url: `https://www.npmjs.com/package/${pkgName}/v/${pkgVersion}`
	});
}

// add rust libraries
for (let license of cargoAbout.licenses) {
	const licenseText = license.text;
	const licenseByText = licenses.get(license.id) ?? new Map();
	licenses.set(license.id, licenseByText);
	const packagesOfTheLicense = licenseByText.get(licenseText) ?? [];
	licenseByText.set(licenseText, packagesOfTheLicense);
	for (let usedBy of license.used_by) {
		packagesOfTheLicense.push({
			name: usedBy.crate.name,
			version: usedBy.crate.version,
			url: usedBy.crate.repository ?? `https://crates.io/crates/${usedBy.crate.name}`,
		});
	}
}

// finally, put to array
const result = [];

for (let [license, licenseByText] of licenses) {
	for (let [text, packages] of licenseByText) {
		const name = licenseNames.get(license);
		if (!name) throw new Error(`Unknown license: ${license}`);
		result.push({
			id: license,
			name,
			text,
			packages,
		});
	}
}

await mkdir("build", {recursive: true});
await writeFile("build/licenses.json", JSON.stringify(result));
