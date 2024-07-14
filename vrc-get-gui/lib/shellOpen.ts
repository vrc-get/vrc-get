import { open } from "@tauri-apps/api/shell";

export async function shellOpen(url: string) {
	await open(url);
}
