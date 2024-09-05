import json5 from "json5";

/** @type {import('next').NextConfig} */
const nextConfig = {
	output: "export",
	eslint: {
		ignoreDuringBuilds: true,
	},
	transpilePackages: [
		// using class fields
		"@tanstack",
		// using '??'
		"react-i18next",
		// using '?.'
		"@radix-ui",
		"tailwind-merge",
	],
	webpack: (config) => {
		config.module.rules.push({
			test: /\.json5$/,
			type: "json",
			parser: {
				parse: json5.parse,
			},
		});

		return config;
	},
};

export default nextConfig;
