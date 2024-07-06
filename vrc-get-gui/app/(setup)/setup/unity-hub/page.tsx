"use client";

import {Card, CardDescription, CardFooter, CardHeader} from "@/components/ui/card";
import React from "react";
import {Button} from "@/components/ui/button";
import {FilePathRow} from "@/components/common-setting-parts";
import {useRouter} from "next/navigation";
import {Accordion, AccordionContent, AccordionItem, AccordionTrigger} from "@/components/ui/accordion";
import {environmentGetSettings, environmentPickUnityHub} from "@/lib/bindings";
import {tc} from "@/lib/i18n";
import {useQuery} from "@tanstack/react-query";
import {shellOpen} from "@/lib/shellOpen";
import {BodyProps, SetupPageBase} from "../setup-page-base";

export default function Page() {
	return <SetupPageBase
		heading={"Install Unity Hub"}
		Body={Body}
		nextPage={"/setup/project-path"}
		pageId={"UnityHub"}
	/>
}

function Body({environment, refetch}: BodyProps) {
	const hubInstalled = !!environment.unity_hub;

	return <>
		<CardDescription className={"whitespace-normal"}>
			To get started, you need to install Unity Hub.
			Unity Hub is the preferred way to install and manage Unity versions.<br/>
			ALCOM will suggest you installing Unity with Unity Hub if you don't have and necessary for the project or
			operation.
		</CardDescription>
		{hubInstalled
			? <>
				<div className={"pb-4"}/>
				<p className={"whitespace-normal text-muted-foreground"}>
					Using this Unity Hub:
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
						Download Unity Hub from unity.com
					</Button>
					<Button onClick={refetch}>
						Recheck Installation
					</Button>
				</div>
				<Accordion type="single" collapsible>
					<AccordionItem value={"you-have"} className={"border-none"}>
						<AccordionTrigger className={"text-sm"}>Have you installed Unity Hub?</AccordionTrigger>
						<AccordionContent>
							<p className={"whitespace-normal"}>
								In this case, ALCOM fails to detect your Unity Hub installation.
								Please manually set path to Unity Hub executable.
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
					<span className={"text-destructive"}>{"Unity Hub is not found."}</span>
				</div>
			</>
		}
	</>
}
