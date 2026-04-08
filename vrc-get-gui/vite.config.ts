import path from "node:path";
import tailwindcss from "@tailwindcss/vite";
import { TanStackRouterVite } from "@tanstack/router-plugin/vite";
import react from "@vitejs/plugin-react-swc";
import { defineConfig } from "vite";
import json5Plugin from "vite-plugin-json5";
import viteBuildLicenseJson from "./scripts/vite-build-license-json";

// https://vitejs.dev/config/
export default defineConfig({
	plugins: [
		tailwindcss(),
		json5Plugin(),
		viteBuildLicenseJson({
			rootDir: __dirname,
		}),
		TanStackRouterVite({
			target: "react",
			autoCodeSplitting: true,
			routesDirectory: "app",
			generatedRouteTree: "lib/routeTree.gen.ts",
		}),
		react(),
	],
	resolve: {
		alias: {
			"@/app": path.join(__dirname, "./app"),
			"@/components": path.join(__dirname, "./components"),
			"@/lib": path.join(__dirname, "./lib"),
			"@/locales": path.join(__dirname, "./locales"),
			"@/build": path.join(__dirname, "./build"),
		},
	},
	build: {
		outDir: "out",
		chunkSizeWarningLimit: Number.POSITIVE_INFINITY,
	},
	server: {
		port: 3030,
		strictPort: true,
		watch: {
			ignored: ["**/*.rs", "project-templates/"],
		},
	},
	clearScreen: false,
	define: {
		ALCOM_UPDATE_UPDATER_DISABLED_MESSAGE: JSON.stringify(
			makeEnvMessageTable("ALCOM_UPDATE_UPDATER_DISABLED"),
		),
	},
});

function makeEnvMessageTable(envName: string): Record<string, string> | null {
	const env = process.env;
	const english = env[`${envName}_MESSAGE`];
	// first check for en message
	if (!english) return null;

	// there is english message. We'll add other languages as well
	const result: Record<string, string> = {};

	const regex = new RegExp(`^${envName}_(?<locale>[A-Z_]+)_MESSAGE$`);

	for (const [envName, envValue] of Object.entries(process.env)) {
		if (!envValue) continue;
		const matchResult = regex.exec(envValue);
		if (matchResult == null) continue;
		// biome-ignore lint/style/noNonNullAssertion: we have defined match group.
		const localeName = matchResult.groups!.locale.toLowerCase();
		if (localeName.length === 0) continue;
		result[localeName] = envValue;
	}

	result.en = english;

	return result;
}
