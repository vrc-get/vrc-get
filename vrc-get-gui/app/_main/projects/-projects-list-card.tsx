"use client";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ChevronDown, ChevronsUpDown, ChevronUp, Star } from "lucide-react";
import { useMemo } from "react";
import { ScrollableCardTable } from "@/components/ScrollableCardTable";
import { assertNever } from "@/lib/assert-never";
import type { TauriProject, TauriProjectType } from "@/lib/bindings";
import { commands } from "@/lib/bindings";
import { tc } from "@/lib/i18n";
import { toastThrownError } from "@/lib/toast";
import { compareUnityVersionString } from "@/lib/version";
import { ProjectRow } from "./-project-row";

export const sortings = ["lastModified", "name", "unity", "type"] as const;

type SimpleSorting = (typeof sortings)[number];
type Sorting = SimpleSorting | `${SimpleSorting}Reversed`;

export function isSorting(s: string | unknown): s is Sorting {
	return sortings.some(
		(sorting) => sorting === s || `${sorting}Reversed` === s,
	);
}

export function compareProjectType(
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

export function ProjectsTableCard({
	projects,
	search,
	loading,
	compact,
}: {
	projects: TauriProject[];
	search?: string;
	loading?: boolean;
	compact?: boolean;
}) {
	const sortingQuery = useQuery({
		initialData: "lastModified" as Sorting,
		queryKey: ["environmentGetProjectSorting"],
		queryFn: async () => {
			const newSorting = await commands.environmentGetProjectSorting();
			return !isSorting(newSorting) ? "lastModified" : newSorting;
		},
	});

	const setSortingStateMutation = useSetProjectSortingMutation();

	const projectsShown = useMemo(() => {
		return sortSearchProjects(projects, search ?? "", sortingQuery.data);
	}, [projects, search, sortingQuery.data]);

	const thClass = "sticky top-0 z-10 border-b border-primary p-2.5";
	const iconClass = "size-3 invisible project-table-header-chevron-up-down";

	const setSorting = async (simpleSorting: SimpleSorting) => {
		let newSorting: Sorting;
		if (sortingQuery.data === simpleSorting) {
			newSorting = `${simpleSorting}Reversed`;
		} else if (sortingQuery.data === `${simpleSorting}Reversed`) {
			newSorting = simpleSorting;
		} else {
			newSorting = simpleSorting;
		}
		setSortingStateMutation.mutate({ sorting: newSorting });
	};

	const headerBg = (target: SimpleSorting) =>
		sortingQuery.data === target || sortingQuery.data === `${target}Reversed`
			? "bg-primary text-primary-foreground"
			: "bg-secondary text-secondary-foreground";
	const icon = (target: SimpleSorting) =>
		sortingQuery.data === target ? (
			<ChevronDown className={"size-3"} />
		) : sortingQuery.data === `${target}Reversed` ? (
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
								{tc("general:last modified")}
							</small>
						</button>
					</th>
					<th className={`${thClass} bg-secondary text-secondary-foreground`} />
				</tr>
			</thead>
			<tbody>
				{projectsShown.map((project) => (
					<ProjectRow key={project.path} project={project} loading={loading} compact={compact} />
				))}
			</tbody>
		</ScrollableCardTable>
	);
}

export function sortSearchProjects(
	projects: TauriProject[],
	search: string,
	sorting: Sorting,
): TauriProject[] {
	const searched = projects.filter((project) =>
		project.name.toLowerCase().includes(search?.toLowerCase() ?? ""),
	);

	searched.sort((a, b) => b.last_modified - a.last_modified);

	switch (sorting) {
		case "lastModified":
			searched.sort((a, b) => b.last_modified - a.last_modified);
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
}

export function useSetProjectSortingMutation() {
	const queryClient = useQueryClient();

	return useMutation({
		mutationFn: async ({ sorting }: { sorting: Sorting }) => {
			await commands.environmentSetProjectSorting(sorting);
		},
		onMutate: async ({ sorting }) => {
			await queryClient.cancelQueries({
				queryKey: ["environmentGetProjectSorting"],
			});
			queryClient.setQueryData(["environmentGetProjectSorting"], () => sorting);
		},
		onError: (error) => {
			console.error("Error setting project sorting", error);
			toastThrownError(error);
		},
	});
}
