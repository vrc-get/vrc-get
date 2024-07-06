"use client";

import {CardDescription} from "@/components/ui/card";
import React from "react";
import {SetupPageBase} from "../setup-page-base";

export default function Page() {
	return <SetupPageBase
		heading={"Congratulations!"}
		Body={Body}
		nextPage={"/projects"}
		nextContent={"Start using ALCOM"}
		pageId={null}
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
