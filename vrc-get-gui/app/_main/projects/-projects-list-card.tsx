"use client";

import Loading from "@/app/-loading";
import { ScrollableCardTable } from "@/components/ScrollableCardTable";
import { Card } from "@/components/ui/card";
import { assertNever } from "@/lib/assert-never";
import type { TauriProject, TauriProjectType } from "@/lib/bindings";
import { commands } from "@/lib/bindings";
import { tc } from "@/lib/i18n";
import { toastThrownError } from "@/lib/toast";
import type { OpenUnityFunction, Result } from "@/lib/use-open-unity";
import { compareUnityVersionString } from "@/lib/version";
import { ChevronDown, ChevronUp, ChevronsUpDown, Star } from "lucide-react";
import type React from "react";
import { useEffect, useMemo, useState } from "react";
import { CreateProject } from "./-create-project";
import { ProjectRow } from "./-project-row";

const sortings = ["lastModified", "name", "unity", "type"] as const;

type SimpleSorting = (typeof sortings)[number];
type Sorting = SimpleSorting | `${SimpleSorting}Reversed`;

function isSorting(s: string): s is Sorting {
	return sortings.some(
		(sorting) => sorting === s || `${sorting}Reversed` === s,
	);
}

export default function ProjectsListCard({
	result,
	search,
	createProjectState,
	setCreateProjectState,
	openUnity,
	loading,
}: {
	// biome-ignore lint/suspicious/noExplicitAny: none
	result: any;
	search: string;
	createProjectState: "normal" | "creating";
	setCreateProjectState: React.Dispatch<
		React.SetStateAction<"normal" | "creating">
	>;
	openUnity: Result;
	loading: boolean;
}) {
	return (
		<>
			{result.status === "pending" ? (
				<Card className="w-full shadow-none overflow-hidden p-4">
					<Loading loadingText={tc("general:loading...")} />
				</Card>
			) : result.status === "error" ? (
				<Card className="w-full shadow-none overflow-hidden p-4">
					{tc("projects:error:load error", { msg: result.error.message })}
				</Card>
			) : (
				<ProjectsTableCard
					projects={result.data}
					search={search}
					loading={loading}
					openUnity={openUnity.openUnity}
					refresh={() => result.refetch()}
					onRemoved={() => result.refetch()}
				/>
			)}
			{createProjectState === "creating" && (
				<CreateProject
					close={() => setCreateProjectState("normal")}
					refetch={() => result.refetch()}
				/>
			)}
			{openUnity.dialog}
		</>
	);
}

function compareProjectType(
	a: TauriProjectType,
	b: TauriProjectType,
): 0 | -1 | 1 {
	if (a === b) return 0;

	// legacy unknown
	if (a === "LegacySdk2") return 1;
	if (b === "LegacySdk2") return -1;
	if (a === "UpmStarter") return 1;
	if (b === "UpmStarter") return -1;

	// legacy worlds
	if (a === "LegacyWorlds") return 1;
	if (b === "LegacyWorlds") return -1;
	if (a === "UpmWorlds") return 1;
	if (b === "UpmWorlds") return -1;

	// legacy avatars
	if (a === "LegacyAvatars") return 1;
	if (b === "LegacyAvatars") return -1;
	if (a === "UpmAvatars") return 1;
	if (b === "UpmAvatars") return -1;

	// unknown
	if (a === "Unknown") return 1;
	if (b === "Unknown") return -1;
	if (a === "VpmStarter") return 1;
	if (b === "VpmStarter") return -1;

	// worlds
	if (a === "Worlds") return 1;
	if (b === "Worlds") return -1;

	// avatars
	if (a === "Avatars") return 1;
	if (b === "Avatars") return -1;

	assertNever(a, "project type");
}

function ProjectsTableCard({
	projects,
	search,
	onRemoved,
	loading,
	refresh,
	openUnity,
}: {
	projects: TauriProject[];
	openUnity: OpenUnityFunction;
	search?: string;
	loading?: boolean;
	onRemoved?: () => void;
	refresh?: () => void;
}) {
	const [sorting, setSortingState] = useState<Sorting>("lastModified");

	useEffect(() => {
		(async () => {
			let newSorting = await commands.environmentGetProjectSorting();
			if (newSorting === null) newSorting = "lastModified";
			if (!isSorting(newSorting)) {
				setSortingState("lastModified");
			} else {
				setSortingState(newSorting);
			}
		})();
	}, []);

	const projectsShown = useMemo(() => {
		const searched = projects.filter((project) =>
			project.name.toLowerCase().includes(search?.toLowerCase() ?? ""),
		);
		searched.sort((a, b) => b.last_modified - a.last_modified);
		switch (sorting) {
			case "lastModified":
				// already sorted
				break;
			case "lastModifiedReversed":
				searched.sort((a, b) => a.last_modified - b.last_modified);
				break;
			case "name":
				searched.sort((a, b) => a.name.localeCompare(b.name));
				break;
			case "nameReversed":
				searched.sort((a, b) => b.name.localeCompare(a.name));
				break;
			case "type":
				searched.sort((a, b) =>
					compareProjectType(a.project_type, b.project_type),
				);
				break;
			case "typeReversed":
				searched.sort((a, b) =>
					compareProjectType(b.project_type, a.project_type),
				);
				break;
			case "unity":
				searched.sort((a, b) => compareUnityVersionString(a.unity, b.unity));
				break;
			case "unityReversed":
				searched.sort((a, b) => compareUnityVersionString(b.unity, a.unity));
				break;
			default:
				assertNever(sorting);
		}
		searched.sort((a, b) => {
			if (a.favorite && !b.favorite) return -1;
			if (!a.favorite && b.favorite) return 1;
			return 0;
		});
		return searched;
	}, [projects, sorting, search]);

	const thClass = "sticky top-0 z-10 border-b border-primary p-2.5";
	const iconClass = "size-3 invisible project-table-header-chevron-up-down";

	const setSorting = async (simpleSorting: SimpleSorting) => {
		let newSorting: Sorting;
		if (sorting === simpleSorting) {
			newSorting = `${simpleSorting}Reversed`;
		} else if (sorting === `${simpleSorting}Reversed`) {
			newSorting = simpleSorting;
		} else {
			newSorting = simpleSorting;
		}
		setSortingState(newSorting);

		try {
			await commands.environmentSetProjectSorting(newSorting);
		} catch (e) {
			console.error("Error setting project sorting", e);
			toastThrownError(e);
		}
	};

	const headerBg = (target: SimpleSorting) =>
		sorting === target || sorting === `${target}Reversed`
			? "bg-primary text-primary-foreground"
			: "bg-secondary text-secondary-foreground";
	const icon = (target: SimpleSorting) =>
		sorting === target ? (
			<ChevronDown className={"size-3"} />
		) : sorting === `${target}Reversed` ? (
			<ChevronUp className={"size-3"} />
		) : (
			<ChevronsUpDown className={iconClass} />
		);

	return (
		<ScrollableCardTable className={"h-full w-full"}>
			<thead>
				<tr>
					<th className={`${thClass} bg-secondary text-secondary-foreground`}>
						<Star className={"size-4"} />
					</th>
					<th className={`${thClass} ${headerBg("name")}`}>
						<button
							type="button"
							className={"flex w-full project-table-button"}
							onClick={() => setSorting("name")}
						>
							{icon("name")}
							<small className="font-normal leading-none">
								{tc("general:name")}
							</small>
						</button>
					</th>
					<th className={`${thClass} ${headerBg("type")}`}>
						<button
							type="button"
							className={"flex w-full project-table-button"}
							onClick={() => setSorting("type")}
						>
							{icon("type")}
							<small className="font-normal leading-none">
								{tc("projects:type")}
							</small>
						</button>
					</th>
					<th className={`${thClass} ${headerBg("unity")}`}>
						<button
							type="button"
							className={"flex w-full project-table-button"}
							onClick={() => setSorting("unity")}
						>
							{icon("unity")}
							<small className="font-normal leading-none">
								{tc("projects:unity")}
							</small>
						</button>
					</th>
					<th className={`${thClass} ${headerBg("lastModified")}`}>
						<button
							type="button"
							className={"flex w-full project-table-button"}
							onClick={() => setSorting("lastModified")}
						>
							{icon("lastModified")}
							<small className="font-normal leading-none">
								{tc("projects:last modified")}
							</small>
						</button>
					</th>
					<th className={`${thClass} bg-secondary text-secondary-foreground`} />
				</tr>
			</thead>
			<tbody>
				{projectsShown.map((project) => (
					<ProjectRow
						key={project.index}
						project={project}
						loading={loading}
						refresh={refresh}
						onRemoved={onRemoved}
						openUnity={openUnity}
					/>
				))}
			</tbody>
		</ScrollableCardTable>
	);
}
