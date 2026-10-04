"use client";

import {
	queryOptions,
	useMutation,
	useQuery,
	useQueryClient,
} from "@tanstack/react-query";
import { createFileRoute } from "@tanstack/react-router";
import {
	FilePathRow,
	ProjectPathWarnings,
} from "@/components/common-setting-parts";
import { CardDescription } from "@/components/ui/card";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select";
import { assertNever } from "@/lib/assert-never";
import { commands, type TauriSyncWithLitedbMode } from "@/lib/bindings";
import { tc } from "@/lib/i18n";
import { toastError, toastSuccess, toastThrownError } from "@/lib/toast";
import { type BodyProps, SetupPageBase } from "../-setup-page-base";

export const Route = createFileRoute("/_setup/setup/projects/")({
	component: Page,
});

function Page() {
	return (
		<SetupPageBase
			heading={tc("setup:projects:heading")}
			Body={Body}
			pageId={"Projects"}
		/>
	);
}

const environmentGetSettings = queryOptions({
	queryKey: ["environmentGetSettings"],
	queryFn: commands.environmentGetSettings,
});

function Body({ environment }: BodyProps) {
	const queryClient = useQueryClient();

	const pickProjectDefaultPath = useMutation({
		mutationFn: async () => await commands.environmentPickProjectDefaultPath(),
		onError: (e) => {
			console.error(e);
			toastThrownError(e);
		},
		onSuccess: (result) => {
			switch (result.type) {
				case "NoFolderSelected":
					// no-op
					break;
				case "InvalidSelection":
					toastError(tc("general:toast:invalid directory"));
					break;
				case "Successful":
					toastSuccess(tc("settings:toast:default project path updated"));
					break;
				default:
					assertNever(result);
			}
		},
		onSettled: async () => {
			await queryClient.invalidateQueries({
				queryKey: environmentGetSettings.queryKey,
			});
		},
	});

	const setProjectListSyncMode = useMutation({
		mutationFn: async (mode: TauriSyncWithLitedbMode) =>
			await commands.environmentSetProjectListSyncMode(mode),
		onMutate: async (mode) => {
			await queryClient.cancelQueries(environmentGetSettings);
			const current = queryClient.getQueryData(environmentGetSettings.queryKey);
			if (current != null) {
				queryClient.setQueryData(environmentGetSettings.queryKey, {
					...current,
					project_list_sync_mode: mode,
				});
			}
			return current;
		},
		onError: (e, _, prev) => {
			console.error(e);
			toastThrownError(e);
			queryClient.setQueryData(environmentGetSettings.queryKey, prev);
		},
		onSettled: async () => {
			await queryClient.invalidateQueries(environmentGetSettings);
		},
	});

	const finishedPages = useQuery({
		queryKey: ["environmentGetFinishedSetupPages"],
		queryFn: async () => commands.environmentGetFinishedSetupPages(),
		initialData: [],
	}).data;
	const existingUser = finishedPages.includes("ProjectPath");

	return (
		<>
			<h3>{tc("setup:projects:projects-path")}</h3>
			<CardDescription className={"whitespace-normal"}>
				{tc("setup:projects:projects-path description")}
			</CardDescription>
			<FilePathRow
				path={environment.default_project_path}
				pick={pickProjectDefaultPath.mutate}
				withOpen={false}
			/>
			<ProjectPathWarnings projectPath={environment.default_project_path} />
			<div
				className={`flex flex-col grow gap-3 compact:gap-2 ${existingUser ? "outline-2 outline-offset-[6px] rounded-sm outline-warning relative" : ""}`}
			>
				{existingUser && (
					<div
						className={
							"absolute top-[30%] right-[calc(100%+2em)] transform-[translateY(-50%)] bg-background outline-warning outline-2 text-warning-foreground px-3 py-2 rounded-md " +
							"before:content-[''] before:absolute before:left-full before:top-1/2 before:transform-[translateY(-50%)] before:border-t-[6px] before:border-b-[6px] before:border-l-[--spacing(6)] before:border-transparent before:border-l-warning"
						}
					>
						{tc("setup:projects:new-setting-is-added")}
					</div>
				)}
				<h3>{tc("setup:projects:sync-mode")}</h3>
				<Select
					value={environment.project_list_sync_mode}
					onValueChange={(value) =>
						setProjectListSyncMode.mutate(value as never)
					}
				>
					<SelectTrigger className={"min-w-0 whitespace-normal"}>
						<SelectValue />
					</SelectTrigger>
					<SelectContent>
						<SelectItem value={"ProjectsUnion"}>
							{tc("setup:projects:sync-mode:ProjectsUnion")}
						</SelectItem>
						<SelectItem value={"TrustLitedb"}>
							{tc("setup:projects:sync-mode:TrustLitedb")}
						</SelectItem>
					</SelectContent>
				</Select>
				<CardDescription className={"whitespace-normal"}>
					{tc("setup:projects:sync-mode:ProjectsUnion:description")}
				</CardDescription>
				<CardDescription className={"whitespace-normal"}>
					{tc("setup:projects:sync-mode:TrustLitedb:description")}
				</CardDescription>
			</div>
		</>
	);
}
