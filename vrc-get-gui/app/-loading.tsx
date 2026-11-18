/*
 * Copyright (c) 2023 anatawa12 and other contributors
 * SPDX-License-Identifier: MPL-2.0
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org.
 */

import { LoaderCircle } from "lucide-react";

export default function Loading({
	loadingText = "Loading...",
}: {
	loadingText?: React.ReactNode;
}) {
	return (
		<div className="flex flex-col items-center justify-center h-full w-full space-y-4">
			<LoaderCircle className="h-10 w-10 animate-spin" />
			<p className="text-xl font-semibold text-gray-700">{loadingText}</p>
		</div>
	);
}
