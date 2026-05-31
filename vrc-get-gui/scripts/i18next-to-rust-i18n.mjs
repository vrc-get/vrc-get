#!/usr/bin/env node

import { readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import JSON5 from "json5";

function setNested(root, dottedKey, value) {
	const parts = dottedKey.split(":");
	let current = root;

	for (let i = 0; i < parts.length - 1; i++) {
		const key = parts[i];
		if (!(key in current)) current[key] = {};
		if (
			typeof current[key] !== "object" ||
			current[key] === null ||
			Array.isArray(current[key])
		) {
			throw new Error(`Key collision at '${parts.slice(0, i + 1).join(":")}'`);
		}
		current = current[key];
	}

	current[parts.at(-1)] = value;
}

function toYaml(value, indent = 0) {
	const pad = "  ".repeat(indent);
	if (typeof value !== "object" || value === null || Array.isArray(value)) {
		return `${JSON.stringify(String(value))}`;
	}

	return Object.entries(value)
		.map(([key, child]) => {
			const escapedKey = `'${key.replaceAll("'", "''")}'`;
			if (typeof child === "object" && child !== null && !Array.isArray(child)) {
				const nested = toYaml(child, indent + 1);
				return `${pad}${escapedKey}:\n${nested}`;
			}
			return `${pad}${escapedKey}: ${JSON.stringify(String(child))}`;
		})
		.join("\n");
}

async function main() {
	const inputPath = process.argv[2] ?? "locales/en.json5";
	const outputPath =
		process.argv[3] ??
		path.join(
			path.dirname(inputPath),
			`${path.basename(inputPath, path.extname(inputPath))}.yml`,
		);

	const source = await readFile(inputPath, "utf8");
	const parsed = JSON5.parse(source);
	const translationRoot = parsed.translation ?? parsed;
	if (typeof translationRoot !== "object" || translationRoot === null) {
		throw new Error("Expected object at root or translation");
	}

	const rustI18n = {};
	for (const [key, value] of Object.entries(translationRoot)) {
		setNested(rustI18n, key, value);
	}

	const yaml = `${toYaml(rustI18n)}\n`;
	await writeFile(outputPath, yaml, "utf8");
	console.log(`Converted ${inputPath} -> ${outputPath}`);
}

main().catch((error) => {
	console.error(error);
	process.exitCode = 1;
});
