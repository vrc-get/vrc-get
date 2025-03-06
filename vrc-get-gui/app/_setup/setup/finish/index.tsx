"use client";

import { CardDescription } from "@/components/ui/card";
import { useGlobalInfo } from "@/lib/global-info";
import { tc } from "@/lib/i18n";
import { createFileRoute } from "@tanstack/react-router";
import { SetupPageBase } from "../-setup-page-base";

export const Route = createFileRoute("/_setup/setup/finish/")({
	component: Page,
});

function Page() {
	const shouldInstallDeepLink = useGlobalInfo().shouldInstallDeepLink;

	return (
		<SetupPageBase
			heading={tc("setup:finish:heading")}
			Body={Body}
			nextPage={"/projects"}
			prevPage={
				shouldInstallDeepLink ? "/setup/system-setting" : "/setup/backups"
			}
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
