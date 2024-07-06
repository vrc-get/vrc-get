"use client";

import {CardDescription} from "@/components/ui/card";
import React from "react";
import {SetupPageBase} from "../setup-page-base";
import {tc} from "@/lib/i18n";

export default function Page() {
	return <SetupPageBase
		heading={"Setup Completed!"}
		Body={Body}
		nextPage={"/projects"}
		nextContent={tc("setup:finish:next")}
		pageId={null}
	/>
}

function Body() {
	return (
		<>
			<CardDescription className={"whitespace-normal"}>
				{tc("setup:finish:description")}
			</CardDescription>
		</>
	)
}
