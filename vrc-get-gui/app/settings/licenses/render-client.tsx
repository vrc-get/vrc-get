"use client";

import {Card, Typography} from "@material-tailwind/react";
import Link from "next/link";
import {Licenses} from "@/lib/licenses";
import {shellOpen} from "@/lib/shellOpen";

export default function RenderPage({licenses}: { licenses: Licenses | null }) {
	if (licenses === null) {
		return (
			<div className={"p-4 whitespace-normal"}>
				<Typography>Failed to load licenses.</Typography>
			</div>
		);
	}

	return (
		<div className={"overflow-y-scroll"}>
			<Card className={"m-4 p-4"}>
				<Typography>
					This project is built on top of many open-source projects.<br/>
					Here are the licenses of the projects used in this project:
				</Typography>
				<ul>
				</ul>
			</Card>

			{licenses.map((license, idx) => (
				<Card className={"m-4 p-4"} key={idx}>
					<Typography as={'h3'}>{license.name}</Typography>
					<Typography as={'h4'}>Used by:</Typography>
					<ul className={"ml-2"}>
						{license.packages.map(pkg => (
							<li key={`${pkg.name}@${pkg.version}`}><a
								onClick={() => shellOpen(pkg.url)}>{pkg.name} ({pkg.version})</a></li>
						))}
					</ul>
					<Card className={"p-3 max-h-52 overflow-y-scroll"}>
						<pre className={"whitespace-pre-wrap"}>{license.text}</pre>
					</Card>
				</Card>
			))}
		</div>
	);
}
