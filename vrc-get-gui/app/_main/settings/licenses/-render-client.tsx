"use client";

import licenses from "build:licenses.json";
import { VStack } from "@/components/layout";
import { ScrollableCard } from "@/components/ScrollableCard";
import { ScrollPageContainer } from "@/components/ScrollPageContainer";
import { Card } from "@/components/ui/card";
import { commands } from "@/lib/bindings";

export default function RenderPage() {
	return (
		<ScrollPageContainer>
			<VStack>
				<Card className={"p-4"}>
					<p>
						This project is built on top of many open-source projects.
						<br />
						Here are the licenses of the projects used in this project:
					</p>
					<ul />
				</Card>

				{licenses.map((license) => (
					<Card className={"p-4"} key={license.text}>
						<h3>{license.name}</h3>
						<h4>Used by:</h4>
						<ul className={"ml-2"}>
							{license.packages.map((pkg) => (
								<li key={`${pkg.name}@${pkg.version}`}>
									<button
										type="button"
										onClick={() => commands.utilOpenUrl(pkg.url)}
									>
										{pkg.name} ({pkg.version})
									</button>
								</li>
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
