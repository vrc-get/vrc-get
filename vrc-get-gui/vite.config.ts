import path from "node:path";
import { dataToEsm } from "@rollup/pluginutils";
import tailwindcss from "@tailwindcss/vite";
import { tanstackRouter } from "@tanstack/router-plugin/vite";
import react from "@vitejs/plugin-react";
import JSON5 from "json5";
import { defineConfig, type Plugin } from "vite";
import viteBuildLicenseJson from "./scripts/vite-build-license-json";

export function json5Plugin(): Plugin {
	const json5ExtRE = /\.json5$/;

	return {
		name: "vite:json5",
		transform(json, id) {
			if (!json5ExtRE.test(id)) return null;
			try {
				// Parse the JSON5
				const parsed = JSON5.parse(json);
				// Convert the parsed JSON5 data to an ES module export
				return {
					code: dataToEsm(parsed, {}),
					map: { mappings: "" },
				};
			} catch (e) {
				const error = e instanceof Error ? e : new Error(String(e));
				this.error(error.message);
			}
		},
	};
}

// https://vitejs.dev/config/
export default defineConfig({
	plugins: [
		tailwindcss(),
		json5Plugin(),
		viteBuildLicenseJson({
			rootDir: __dirname,
		}),
		tanstackRouter({
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
});
