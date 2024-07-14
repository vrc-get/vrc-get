"use client";

import { CardDescription } from "@/components/ui/card";
import { useGlobalInfo } from "@/lib/global-info";
import { tc } from "@/lib/i18n";
import React from "react";
import { SetupPageBase } from "../setup-page-base";

export default function Page() {
	const isMac = useGlobalInfo().osType === "Darwin";

	return (
		<SetupPageBase
			heading={tc("setup:finish:heading")}
			Body={Body}
			nextPage={"/projects"}
			prevPage={isMac ? "/setup/backups" : "/setup/system-setting"}
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
