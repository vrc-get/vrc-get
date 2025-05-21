import type { RustError } from "@/lib/bindings";

export function isHandleable(
	t: unknown,
): t is RustError & { type: "Handleable" } {
	return (
		typeof t === "object" &&
		t !== null &&
		// about type field
		"type" in t &&
		typeof t.type === "string" &&
		t.type === "Handleable" &&
		// message field
		"message" in t &&
		typeof t.message === "string" &&
		// body field
		"body" in t &&
		typeof t.body === "object" &&
		t.body !== null &&
		// body.type field
		"type" in t.body &&
		typeof t.body.type === "string"
	);
}
