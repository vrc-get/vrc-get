export type Licenses = {
	id: string;
	name: string;
	text: string;
	packages: {
		name: string;
		version: string;
		url: string;
	}[];
}[];

export async function loadLicenses(): Promise<Licenses | null> {
	try {
		return (await import("@/build/licenses.json")).default;
	} catch (e) {
		return null;
	}
}
