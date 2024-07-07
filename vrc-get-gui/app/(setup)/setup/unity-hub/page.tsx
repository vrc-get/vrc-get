"use client";

import {CardDescription} from "@/components/ui/card";
import React from "react";
import {Button} from "@/components/ui/button";
import {FilePathRow} from "@/components/common-setting-parts";
import {Accordion, AccordionContent, AccordionItem, AccordionTrigger} from "@/components/ui/accordion";
import {environmentPickUnityHub} from "@/lib/bindings";
import {tc} from "@/lib/i18n";
import {shellOpen} from "@/lib/shellOpen";
import {BodyProps, SetupPageBase} from "../setup-page-base";

export default function Page() {
	return <SetupPageBase
		heading={tc("setup:unity-hub:heading")}
		Body={Body}
		nextPage={"/setup/project-path"}
		prevPage={"/setup/appearance"}
		pageId={"UnityHub"}
	/>
}

function Body({environment, refetch}: BodyProps) {
	const hubInstalled = !!environment.unity_hub;

	return <>
		<CardDescription className={"whitespace-normal"}>
			{tc("setup:unity-hub:description")}
		</CardDescription>
		{hubInstalled
			? <>
				<div className={"pb-4"}/>
				<p className={"whitespace-normal text-muted-foreground"}>
					{tc("setup:unity-hub:using this unity hub")}:
				</p>
				<FilePathRow
					withoutSelect
					path={environment.unity_hub ?? ""}
					pick={environmentPickUnityHub}
					refetch={refetch}
					notFoundMessage={"Unity Hub Not Found"}
					successMessage={tc("settings:toast:unity hub path updated")}
				/>
			</>
			: <>
				<div className={"p-2"}/>
				<div className={"flex flex-row flex-wrap gap-2"}>
					<Button onClick={() => shellOpen("https://unity.com/ja/download")}>
						{tc("Download Unity Hub from unity.com")}
					</Button>
					<Button onClick={refetch}>
						{tc("setup:unity-hub:recheck installation")}
					</Button>
				</div>
				<Accordion type="single" collapsible>
					<AccordionItem value={"you-have"} className={"border-none"}>
						<AccordionTrigger className={"text-sm"}>{tc("setup:unity-hub:detection failed collapse")}</AccordionTrigger>
						<AccordionContent>
							<p className={"whitespace-normal"}>
								{tc("setup:unity-hub:detection failed description")}
							</p>
							<FilePathRow
								withoutSelect
								path={environment.unity_hub}
								pick={environmentPickUnityHub}
								refetch={refetch}
								notFoundMessage={"Unity Hub Not Found"}
								successMessage={tc("settings:toast:unity hub path updated")}
							/>
						</AccordionContent>
					</AccordionItem>
				</Accordion>
				<div className={"flex w-full"}>
					<span className={"text-destructive"}>{tc("setup:unity-hub:not found")}</span>
				</div>
			</>
		}
	</>
}
