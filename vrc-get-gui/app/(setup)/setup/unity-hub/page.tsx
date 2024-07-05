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

export default function Page() {
	const router = useRouter();

	const result = useQuery({
		queryKey: ["environmentGetSettings"],
		queryFn: environmentGetSettings
	})

	const onBack = () => {
		router.back()
	};

	const onNext = () => {
		// TODO: fetch next page from backend
	};

	const hubInstalled = !!result.data?.unity_hub;

	return <div className={"w-full flex items-center justify-center"}>
		<Card className={"w-[500px] p-4"}>
			<CardHeader>
				<h1 className={"text-center"}>Install Unity Hub</h1>
			</CardHeader>
			<CardDescription className={"whitespace-normal"}>
				To get started, you need to install Unity Hub.
				Unity Hub is the preferred way to install and manage Unity versions.<br/>
				ALCOM will suggest you installing Unity with Unity Hub if you don't have and necessary for the project or
				operation.
			</CardDescription>
			{
				result.isLoading
					? <p>Loading...</p>
					: hubInstalled
						? <>
							<div className={"pb-4"}/>
							<p className={"whitespace-normal text-muted-foreground"}>
								Using this Unity Hub:
							</p>
							<FilePathRow
								withoutSelect
								path={result.data?.unity_hub ?? ""}
								pick={environmentPickUnityHub}
								refetch={() => result.refetch()}
								notFoundMessage={"Unity Hub Not Found"}
								successMessage={tc("settings:toast:unity hub path updated")}
							/>
						</>
						: <>
							<div className={"p-2"}/>
							<div className={"flex flex-row gap-1"}>
								<Button onClick={() => shellOpen("https://unity.com/ja/download")} className={"px-2 py-1 text-xs h-7"}>
									Download Unity Hub from unity.com
								</Button>
								<Button onClick={() => result.refetch()} className={"px-2 py-1 text-xs h-7"}>
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
											path={result.data?.unity_hub ?? ""}
											pick={environmentPickUnityHub}
											refetch={() => result.refetch()}
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
			<CardFooter className="p-0 pt-3 items-end flex-row gap-2 justify-end">
				<Button onClick={onBack}>Back</Button>
				<Button disabled={!hubInstalled} onClick={onNext}>Next</Button>
			</CardFooter>
		</Card>
	</div>
}
