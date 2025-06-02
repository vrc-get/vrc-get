"use client";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select";
import type { TauriProject } from "@/lib/bindings";
import { commands } from "@/lib/bindings";
import { tc } from "@/lib/i18n";
import { toastThrownError } from "@/lib/toast";
import { compareUnityVersionString } from "@/lib/version";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ArrowDown, ArrowUp } from "lucide-react";
import { useMemo } from "react";
import { ProjectGridItem } from "./-project-grid-item";
import {
	compareProjectType,
	isSorting,
	type sortings,
} from "./-projects-list-card";

type SimpleSorting = (typeof sortings)[number];
type Sorting = SimpleSorting | `${SimpleSorting}Reversed`;

const sortingOptions: { key: SimpleSorting; label: string }[] = [
	{ key: "name", label: "general:name" },
	{ key: "type", label: "projects:type" },
	{ key: "unity", label: "projects:unity" },
	{ key: "lastModified", label: "projects:last modified" },
];

export function ProjectsGridCard({
	projects,
	search,
	loading,
}: {
	projects: TauriProject[];
	search?: string;
	loading?: boolean;
}) {
	const queryClient = useQueryClient();

	const sortingQuery = useQuery({
		initialData: "lastModified" as Sorting,
		queryKey: ["environmentGetProjectSorting"],
		queryFn: async () => {
			const newSorting = await commands.environmentGetProjectSorting();
			return !isSorting(newSorting) ? "lastModified" : newSorting;
		},
	});

	const setSortingStateMutation = useMutation({
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

	const currentKey = sortingQuery.data.replace(
		/Reversed$/,
		"",
	) as SimpleSorting;
	const isReversed = sortingQuery.data.endsWith("Reversed");

	const handleChangeSortingKey = (key: SimpleSorting) => {
		const newSorting = isReversed ? `${key}Reversed` : key;
		setSortingStateMutation.mutate({ sorting: newSorting as Sorting });
	};

	const toggleOrder = () => {
		const newSorting: Sorting = isReversed
			? currentKey
			: `${currentKey}Reversed`;
		setSortingStateMutation.mutate({ sorting: newSorting });
	};

	const projectsShown = useMemo(() => {
		const filtered = projects.filter((p) =>
			p.name.toLowerCase().includes(search?.toLowerCase() ?? ""),
		);

		const sorters: Record<
			SimpleSorting,
			(a: TauriProject, b: TauriProject) => number
		> = {
			lastModified: (a, b) => b.last_modified - a.last_modified,
			name: (a, b) => a.name.localeCompare(b.name),
			type: (a, b) => compareProjectType(a.project_type, b.project_type),
			unity: (a, b) => compareUnityVersionString(a.unity, b.unity),
		};

		const sorter = sorters[currentKey];
		filtered.sort((a, b) => (isReversed ? -1 : 1) * sorter(a, b));

		filtered.sort((a, b) => {
			if (a.favorite && !b.favorite) return -1;
			if (!a.favorite && b.favorite) return 1;
			return 0;
		});

		return filtered;
	}, [projects, search, currentKey, isReversed]);

	return (
		<div className="flex flex-col h-full w-full overflow-hidden">
			<Card className="flex items-center mb-3 flex-wrap">
				<div className="flex items-center gap-2 m-2 ml-4">
					<p className="grow-0 whitespace-pre mb-0 leading-tight">
						{tc("projects:sort by")}
					</p>
					<Select
						value={currentKey}
						onValueChange={(value) =>
							handleChangeSortingKey(value as SimpleSorting)
						}
					>
						<SelectTrigger className="w-40">
							<SelectValue />
						</SelectTrigger>
						<SelectContent>
							{sortingOptions.map((option) => (
								<SelectItem key={option.key} value={option.key}>
									{tc(option.label)}
								</SelectItem>
							))}
						</SelectContent>
					</Select>
				</div>

				<Button variant="ghost" size="icon" onClick={toggleOrder}>
					{isReversed ? (
						<ArrowUp className="size-4" />
					) : (
						<ArrowDown className="size-4" />
					)}
				</Button>
			</Card>
			<ScrollArea
				type="auto"
				className="h-full w-full vrc-get-scrollable-card rounded-l-xl"
				scrollBarClassName="bg-background rounded-full border-l-0 p-[1.5px]"
			>
				<div className="grid grid-cols-1 sm:grid-cols-1 md:grid-cols-2 lg:grid-cols-2 gap-3 overflow-x-hidden mr-4">
					{projectsShown.map((project) => (
						<ProjectGridItem
							key={project.path}
							project={project}
							loading={loading}
						/>
					))}
				</div>
			</ScrollArea>
		</div>
	);
}
