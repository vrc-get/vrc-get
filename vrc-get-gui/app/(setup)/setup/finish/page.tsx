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
import {SetupPageBase} from "../setup-page-base";

export default function Page() {
	return <SetupPageBase
		heading={"Congratulations!"}
		Body={Body}
		nextPage={"/projects"}
		nextContent={"Start using ALCOM"}
	/>
}

function Body() {
	return (
		<>
			<CardDescription className={"whitespace-normal"}>
				This is the end of the setup process!<br/>
				You can now start using ALCOM.
			</CardDescription>
		</>
	)
}
