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
		router.push("/projects")
	};

	const hubInstalled = !!result.data?.unity_hub;

	return <div className={"w-full flex items-center justify-center"}>
		<Card className={"w-[500px] p-4"}>
			<CardHeader>
				<h1 className={"text-center"}>Configuration</h1>
			</CardHeader>
			<div className={"pb-4"}/>
			<CardDescription className={"whitespace-normal"}>
				This is the end of the setup process!<br/>
				You can now start using ALCOM.
			</CardDescription>
			<CardFooter className="p-0 pt-3 items-end flex-row gap-2 justify-end">
				<Button onClick={onBack}>Back</Button>
				<Button disabled={!hubInstalled} onClick={onNext}>Start using ALCOM</Button>
			</CardFooter>
		</Card>
	</div>
}
