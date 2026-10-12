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
	ProjectSyncModeSelect,
} from "@/components/common-setting-parts";
import { CardDescription } from "@/components/ui/card";
import { assertNever } from "@/lib/assert-never";
import { commands } from "@/lib/bindings";
import { tc } from "@/lib/i18n";
import { toastError, toastSuccess, toastThrownError } from "@/lib/toast";
import {
	type BodyProps,
	NesSettings,
	SetupPageBase,
} from "../-setup-page-base";

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
			<NesSettings isNewSettings={existingUser}>
				<h3>{tc("setup:projects:sync-mode")}</h3>
				<ProjectSyncModeSelect
					projectListSyncMode={environment.project_list_sync_mode}
				/>
				<CardDescription className={"whitespace-normal"}>
					{tc("setup:projects:sync-mode:ProjectsUnion:description")}
				</CardDescription>
				<CardDescription className={"whitespace-normal"}>
					{tc("setup:projects:sync-mode:TrustLitedb:description")}
				</CardDescription>
			</NesSettings>
		</>
	);
}
