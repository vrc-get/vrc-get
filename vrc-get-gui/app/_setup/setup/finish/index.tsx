/*
 * Copyright (c) 2023 anatawa12 and other contributors
 * SPDX-License-Identifier: MPL-2.0
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org.
 */

"use client";

import { createFileRoute } from "@tanstack/react-router";
import { CardDescription } from "@/components/ui/card";
import { tc } from "@/lib/i18n";
import { SetupPageBase } from "../-setup-page-base";

export const Route = createFileRoute("/_setup/setup/finish/")({
	component: Page,
});

function Page() {
	return (
		<SetupPageBase
			heading={tc("setup:finish:heading")}
			Body={Body}
			nextContent={tc("setup:finish:next")}
			pageId={null}
		/>
	);
}

function Body() {
	return (
		<div className={"w-full h-full flex justify-center items-center"}>
			<CardDescription
				className={"whitespace-normal text-lg text-foreground text-center"}
			>
				{tc("setup:finish:description")}
			</CardDescription>
		</div>
	);
}
