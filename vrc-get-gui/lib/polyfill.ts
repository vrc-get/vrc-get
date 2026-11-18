/*
 * Copyright (c) 2023 anatawa12 and other contributors
 * SPDX-License-Identifier: MPL-2.0
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org.
 */

// Very tiny polyfill for using statement

declare global {
	interface SymbolConstructor {
		readonly dispose: unique symbol;
		readonly asyncDispose: unique symbol;
	}
}

if (typeof Symbol.dispose !== "symbol")
	Object.defineProperty(Symbol, "dispose", { value: Symbol.for("dispose") });

if (typeof Symbol.asyncDispose !== "symbol")
	Object.defineProperty(Symbol, "asyncDispose", {
		value: Symbol.for("asyncDispose"),
	});

export {};
