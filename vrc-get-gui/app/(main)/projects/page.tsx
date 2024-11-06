"use client";

import { HNavBar, VStack } from "@/components/layout";
import { tc } from "@/lib/i18n";
import ProjectsListCard from "./projects-list-card";

export default function Page() {
	return (
		<VStack>
			<HNavBar className={"flex-shrink-0"}>
				<p className="cursor-pointer py-1.5 font-bold flex-grow-0">
					{tc("projects")}
				</p>
				<div className={"flex-grow"} />
			</HNavBar>
			<main className="flex-shrink overflow-hidden flex w-full h-full">
				<ProjectsListCard />
			</main>
		</VStack>
	);
}
