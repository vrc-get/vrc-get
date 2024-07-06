"use client";

import {CardDescription} from "@/components/ui/card";
import React from "react";
import {LanguageSelector, ThemeSelector} from "@/components/common-setting-parts";
import {SetupPageBase} from "../setup-page-base";

export default function Page() {
	return <SetupPageBase
		heading={"Welcome to ALCOM!"}
		Body={Body}
		nextPage={"/setup/unity-hub"}
		backContent={null}
	/>
}

function Body() {
	return <>
		<CardDescription className={"whitespace-normal text-lg text-center"}>
			ALCOM is an Open-Source Creator Companion for VRChat and other Unity Projects.
		</CardDescription>
		<div className={"pb-3"}/>
		<CardDescription className={"whitespace-normal"}>
			Please set up your appearance preferences at the beginning.<br/>
			You can change later on settings page.
		</CardDescription>
		<LanguageSelector/>
		<ThemeSelector/>
	</>
}
