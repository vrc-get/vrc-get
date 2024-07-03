"use client";

import {Card} from "@/components/ui/card";
import {Licenses} from "@/lib/licenses";
import {shellOpen} from "@/lib/shellOpen";
import {ScrollableCard} from "@/components/ScrollableCard";
import {ScrollPageContainer} from "@/components/ScrollPageContainer";
import {VStack} from "@/components/layout";

export default function RenderPage({licenses}: { licenses: Licenses | null }) {
	if (licenses === null) {
		return (
			<div className={"whitespace-normal"}>
				<p>Failed to load licenses.</p>
			</div>
		);
	}

	return (
		<ScrollPageContainer>
			<VStack>
				<Card className={"p-4"}>
					<p>
						This project is built on top of many open-source projects.<br/>
						Here are the licenses of the projects used in this project:
					</p>
					<ul>
					</ul>
				</Card>

				{licenses.map((license, idx) => (
					<Card className={"p-4"} key={idx}>
						<h3>{license.name}</h3>
						<h4>Used by:</h4>
						<ul className={"ml-2"}>
							{license.packages.map(pkg => (
								<li key={`${pkg.name}@${pkg.version}`}><a
									onClick={() => shellOpen(pkg.url)}>{pkg.name} ({pkg.version})</a></li>
							))}
						</ul>
						<ScrollableCard className="max-h-52">
							<pre className={"whitespace-pre-wrap"}>{license.text}</pre>
						</ScrollableCard>
					</Card>
				))}
			</VStack>
		</ScrollPageContainer>
	);
}
