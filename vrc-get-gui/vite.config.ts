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
});
