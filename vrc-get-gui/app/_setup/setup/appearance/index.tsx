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
import {
	GuiAnimationSwitch,
	GuiCompactSwitch,
	LanguageSelector,
	ThemeSelector,
} from "@/components/common-setting-parts";
import { CardDescription } from "@/components/ui/card";
import { tc } from "@/lib/i18n";
import { SetupPageBase } from "../-setup-page-base";

export const Route = createFileRoute("/_setup/setup/appearance/")({
	component: Page,
});

function Page() {
	return (
		<SetupPageBase
			heading={tc("setup:appearance:heading")}
			Body={Body}
			pageId={"Appearance"}
		/>
	);
}

function Body() {
	return (
		<>
			<CardDescription className={"whitespace-normal"}>
				{tc("setup:appearance:description")}
			</CardDescription>
			<LanguageSelector />
			<ThemeSelector />
			<GuiAnimationSwitch />
			<GuiCompactSwitch />
		</>
	);
}
