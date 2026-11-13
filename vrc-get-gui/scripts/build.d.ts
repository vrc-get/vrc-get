/*
 * Copyright (c) 2023 anatawa12 and other contributors
 * SPDX-License-Identifier: MPL-2.0
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org.
 */

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
