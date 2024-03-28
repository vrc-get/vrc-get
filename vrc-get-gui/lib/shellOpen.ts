import {open} from "@tauri-apps/api/shell";

export async function shellOpen(url: string) {
	// since @tauri-apps/api uses navigator while importing, we need to import it in a function to avoid unexpected SSR
	await open(url);
}
