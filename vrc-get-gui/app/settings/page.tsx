"use client"

import {Typography} from "@material-tailwind/react";
import Link from "next/link";

export default function Page() {
	return (
		<div className={"p-4 whitespace-normal"}>
			<Typography>Editing Settings is not supported yet. Please use <code>vrc-get</code> cli or official VCC instead for
				now.</Typography>
			<Link href={"/settings/licenses"}>View Licenses</Link>
		</div>
	);
}
