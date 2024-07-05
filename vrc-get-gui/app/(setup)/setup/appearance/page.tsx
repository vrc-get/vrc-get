"use client";

import {Card, CardDescription, CardFooter, CardHeader} from "@/components/ui/card";
import React from "react";
import {Button} from "@/components/ui/button";
import {LanguageSelector, ThemeSelector} from "@/components/common-setting-parts";
import {useRouter} from "next/navigation";

export default function Page() {
	const router = useRouter();

	const onNext = () => {
		// TODO: navigate to next page
	};

	return <div className={"w-full flex items-center justify-center"}>
		<Card className={"w-[500px] p-4"}>
			<CardHeader>
				<h1 className={"text-center"}>Welcome to ALCOM!</h1>
			</CardHeader>
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
			<CardFooter className="p-0 pt-3 items-end flex-row gap-2 justify-end">
				<Button onClick={onNext}>Next</Button>
			</CardFooter>
		</Card>
	</div>
}
