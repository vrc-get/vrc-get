import json5 from "json5";

/** @type {import('next').NextConfig} */
const nextConfig = {
	output: 'export',
	webpack: config => {
		config.module.rules.push({
			test: /\.json5$/,
			type: 'json',
			parser: {
				parse: json5.parse,
			}
		});

		return config;
	}
};

export default nextConfig;
