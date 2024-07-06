"use client";

import {CardDescription} from "@/components/ui/card";
import React from "react";
import {LanguageSelector, ThemeSelector} from "@/components/common-setting-parts";
import {SetupPageBase} from "../setup-page-base";
import {tc} from "@/lib/i18n";

export default function Page() {
	return <SetupPageBase
		heading={tc("setup:entry:welcome")}
		Body={Body}
		nextPage={"/setup/unity-hub"}
		backContent={null}
		pageId={"Appearance"}
		withoutSteps
	/>
}

function Body() {
	return <>
		<CardDescription className={"whitespace-normal text-lg text-center"}>
			{tc("setup:entry:welcome description")}
		</CardDescription>
		<div className={"pb-3"}/>
		<CardDescription className={"whitespace-normal"}>
			{tc("setup:appearance:description")}
		</CardDescription>
		<LanguageSelector/>
		<ThemeSelector/>
	</>
}
