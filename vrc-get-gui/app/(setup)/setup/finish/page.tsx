"use client";

import {CardDescription} from "@/components/ui/card";
import React from "react";
import {SetupPageBase} from "../setup-page-base";
import {tc} from "@/lib/i18n";
import {useQuery} from "@tanstack/react-query";
import {loadOSApi} from "@/lib/os";

export default function Page() {
	const osType = useQuery({
		queryKey: ["osType"],
		queryFn: async () => loadOSApi().then(os => os.type()),
		initialData: "Windows_NT" as const
	}).data;

	const isMac = osType === "Darwin";

	return <SetupPageBase
		heading={"Setup Completed!"}
		Body={Body}
		nextPage={"/projects"}
		prevPage={isMac ? "/setup/backups" : "/setup/system-setting"}
		nextContent={tc("setup:finish:next")}
		pageId={null}
	/>
}

function Body() {
	return (
		<div className={"w-full h-full flex justify-center items-center"}>
			<CardDescription className={"whitespace-normal text-lg text-foreground text-center"}>
				{tc("setup:finish:description")}
			</CardDescription>
		</div>
	)
}
