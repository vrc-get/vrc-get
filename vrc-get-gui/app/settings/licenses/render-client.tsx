"use client";

import {Card} from "@/components/ui/card";
import Link from "next/link";
import {Licenses} from "@/lib/licenses";
import {shellOpen} from "@/lib/shellOpen";

export default function RenderPage({licenses}: { licenses: Licenses | null }) {
	if (licenses === null) {
		return (
			<div className={"p-4 whitespace-normal"}>
				<p>Failed to load licenses.</p>
			</div>
		);
	}

	return (
		<div className={"overflow-y-scroll"}>
			<Card className={"m-4 p-4"}>
				<p>
					This project is built on top of many open-source projects.<br/>
					Here are the licenses of the projects used in this project:
				</p>
				<ul>
				</ul>
			</Card>

			{licenses.map((license, idx) => (
				<Card className={"m-4 p-4"} key={idx}>
					<h3>{license.name}</h3>
					<h4>Used by:</h4>
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
