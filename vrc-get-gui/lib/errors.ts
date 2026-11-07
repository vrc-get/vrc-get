/*
 * Copyright (c) 2023 anatawa12 and other contributors
 * SPDX-License-Identifier: MPL-2.0
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org.
 */

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
