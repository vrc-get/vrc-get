"use client";

import {
	queryOptions,
	useMutation,
	useQuery,
	useQueryClient,
} from "@tanstack/react-query";
import { createFileRoute } from "@tanstack/react-router";
import { ChevronDown, LayoutGrid, LayoutList, RefreshCw } from "lucide-react";
import { useRef, useState } from "react";
import { createProject } from "@/app/_main/projects/-create-project";
import { ProjectsGridCard } from "@/app/_main/projects/-projects-grid-card";
import Loading from "@/app/-loading";
import { HNavBar, VStack } from "@/components/layout";
import { SearchBox } from "@/components/SearchBox";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
	Tooltip,
	TooltipContent,
	TooltipTrigger,
} from "@/components/ui/tooltip";
import { assertNever } from "@/lib/assert-never";
import { commands } from "@/lib/bindings";
import { isFindKey, useDocumentEvent } from "@/lib/events";
import { useProjectUpdateInProgress } from "@/lib/global-events";
import { tc, tt } from "@/lib/i18n";
import { toastError, toastSuccess, toastThrownError } from "@/lib/toast";
import { ProjectsTableCard } from "./-projects-list-card";

export const Route = createFileRoute("/_main/projects/")({
	component: Page,
});

const environmentProjects = queryOptions({
	queryKey: ["environmentProjects"],
	queryFn: commands.environmentProjects,
});

function Page() {
	const result = useQuery(environmentProjects);
	const [search, setSearch] = useState("");

	const viewModeQuery = useQuery({
		initialData: "List",
		queryKey: ["environmentGetProjectViewMode"],
		queryFn: async () => {
			return await commands.environmentProjectViewMode();
		},
	});

	const queryClient = useQueryClient();

	const setViewModeMutation = useMutation({
		mutationFn: async (value: string) => {
			await commands.environmentSetProjectViewMode(value);
		},
		onMutate: async (value: string) => {
			await queryClient.setQueryData(["environmentGetProjectViewMode"], value);
		},
		onSuccess: async () => {
			await queryClient.invalidateQueries({
				queryKey: ["environmentGetProjectViewMode"],
			});
		},
	});

	const viewMode = viewModeQuery.data ?? true;

	const setViewMode = (value: string) => {
		setViewModeMutation.mutate(value);
	};

	const startCreateProject = () => void createProject();

	const loading = result.isFetching;

	return (
		<VStack>
			<ProjectViewHeader
				startCreateProject={startCreateProject}
				isLoading={loading}
				search={search}
				setSearch={setSearch}
				viewMode={viewMode}
				setViewMode={setViewMode}
			/>
			<main className="shrink overflow-hidden flex w-full h-full">
				{result.status === "pending" ? (
					<Card className="w-full shadow-none overflow-hidden p-4">
						<Loading loadingText={tc("general:loading...")} />
					</Card>
				) : result.status === "error" ? (
					<Card className="w-full shadow-none overflow-hidden p-4">
						{tc("projects:error:load error", { msg: result.error.message })}
					</Card>
				) : viewMode === "List" ? (
					<ProjectsTableCard
						projects={result.data}
						search={search}
						loading={loading}
					/>
				) : viewMode === "Grid" ? (
					<ProjectsGridCard
						projects={result.data}
						search={search}
						loading={loading}
					/>
				) : (
					<ProjectsTableCard
						projects={result.data}
						search={search}
						loading={loading}
					/>
				)}
			</main>
		</VStack>
	);
}

function ProjectViewHeader({
	startCreateProject,
	isLoading,
	search,
	setSearch,
	viewMode,
	setViewMode,
}: {
	startCreateProject?: () => void;
	isLoading?: boolean;
	search: string;
	setSearch: (search: string) => void;
	viewMode: string;
	setViewMode: (viewMode: string) => void;
}) {
	const queryClient = useQueryClient();
	const addProjectWithPicker = useMutation({
		mutationFn: async () => await commands.environmentAddProjectWithPicker(),
		onSuccess: (result) => {
			switch (result) {
				case "NoFolderSelected":
					// no-op
					break;
				case "InvalidSelection":
					toastError(tt("general:toast:invalid directory"));
					break;
				case "Successful":
					toastSuccess(tt("projects:toast:project added"));
					break;
				case "AlreadyAdded":
					toastError(tt("projects:toast:project already exists"));
					break;
				default:
					assertNever(result);
			}
		},
		onError: (e) => {
			console.error("Error adding project", e);
			toastThrownError(e);
		},
		onSettled: async () => {
			await queryClient.invalidateQueries(environmentProjects);
		},
	});

	const inProgress = useProjectUpdateInProgress();

	const searchRef = useRef<HTMLInputElement>(null);

	useDocumentEvent(
		"keydown",
		(e) => {
			if (isFindKey(e)) {
				searchRef.current?.focus();
			}
		},
		[],
	);

	isLoading = isLoading || inProgress;

	return (
		<HNavBar
			className={"shrink-0 compact:py-0"}
			leading={
				<>
					<p className="cursor-pointer font-bold grow-0 whitespace-pre mb-0 leading-tight">
						{tc("projects")}
					</p>
					<Tooltip>
						<TooltipTrigger asChild>
							<Button
								variant={"ghost"}
								size={"icon"}
								onClick={() =>
									queryClient.invalidateQueries(environmentProjects)
								}
								disabled={isLoading}
							>
								{isLoading ? (
									<RefreshCw className="w-5 h-5 animate-spin" />
								) : (
									<RefreshCw className={"w-5 h-5"} />
								)}
							</Button>
						</TooltipTrigger>
						<TooltipContent>{tc("projects:tooltip:refresh")}</TooltipContent>
					</Tooltip>

					<SearchBox
						className={"w-max grow"}
						value={search}
						onChange={(e) => setSearch(e.target.value)}
						ref={searchRef}
					/>

					<Button
						variant={"ghost"}
						onClick={() => {
							if (viewMode === "List") {
								setViewMode("Grid");
							} else {
								setViewMode("List");
							}
						}}
					>
						{viewMode === "List" ? (
							<>
								<LayoutList className={"w-5 h-5"} />
								<p className="ml-2">{tc("projects:list view")}</p>
							</>
						) : viewMode === "Grid" ? (
							<>
								<LayoutGrid className={"w-5 h-5"} />
								<p className="ml-2">{tc("projects:grid view")}</p>
							</>
						) : (
							<>
								<LayoutList className={"w-5 h-5"} />
								<p className="ml-2">{tc("projects:list view")}</p>
							</>
						)}
					</Button>
				</>
			}
			trailing={
				<DropdownMenu>
					<div className={"flex divide-x"}>
						<Button
							className={"rounded-r-none pl-4 pr-3"}
							onClick={startCreateProject}
						>
							{tc("projects:create new project")}
						</Button>
						<DropdownMenuTrigger asChild className={"rounded-l-none pl-2 pr-2"}>
							<Button>
								<ChevronDown className={"w-4 h-4"} />
							</Button>
						</DropdownMenuTrigger>
					</div>
					<DropdownMenuContent>
						<DropdownMenuItem onClick={() => addProjectWithPicker.mutate()}>
							{tc("projects:add existing project")}
						</DropdownMenuItem>
					</DropdownMenuContent>
				</DropdownMenu>
			}
		/>
	);
}
