"use client";

import { createFileRoute } from "@tanstack/react-router";
import {
	GuiAnimationSwitch,
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
			heading={tc("setup:entry:welcome")}
			Body={Body}
			nextPage={"/setup/unity-hub"}
			prevPage={null}
			pageId={"Appearance"}
			withoutSteps
		/>
	);
}

function Body() {
	return (
		<>
			<CardDescription className={"whitespace-normal text-lg text-center"}>
				{tc("setup:entry:welcome description")}
			</CardDescription>
			<div className={"pb-3"} />
			<CardDescription className={"whitespace-normal"}>
				{tc("setup:appearance:description")}
			</CardDescription>
			<LanguageSelector />
			<ThemeSelector />
			<GuiAnimationSwitch />
		</>
	);
}
