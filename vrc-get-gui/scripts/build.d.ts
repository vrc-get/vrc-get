declare module "build:licenses.json" {
	const value: {
		id: string;
		name: string;
		text: string;
		packages: {
			name: string;
			version: string;
			url: string;
		}[];
	}[];
	export default value;
}
