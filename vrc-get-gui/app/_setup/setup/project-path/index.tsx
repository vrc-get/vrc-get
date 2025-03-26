"use client";

import {
	FilePathRow,
	ProjectPathWarnings,
} from "@/components/common-setting-parts";
import { CardDescription } from "@/components/ui/card";
import { assertNever } from "@/lib/assert-never";
import { commands } from "@/lib/bindings";
import { tc } from "@/lib/i18n";
import { toastError, toastSuccess, toastThrownError } from "@/lib/toast";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { createFileRoute } from "@tanstack/react-router";
import { type BodyProps, SetupPageBase } from "../-setup-page-base";

export const Route = createFileRoute("/_setup/setup/project-path/")({
	component: Page,
});

function Page() {
	return (
		<SetupPageBase
			heading={tc("setup:project-path:heading")}
			Body={Body}
			nextPage={"/setup/backups"}
			prevPage={"/setup/unity-hub"}
			pageId={"ProjectPath"}
		/>
	);
}

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
				queryKey: ["environmentGetSettings"],
			});
		},
	});

	return (
		<>
			<CardDescription className={"whitespace-normal"}>
				{tc("setup:project-path:description")}
			</CardDescription>
			<FilePathRow
				path={environment.default_project_path}
				pick={pickProjectDefaultPath.mutate}
				withOpen={false}
			/>
			<ProjectPathWarnings projectPath={environment.default_project_path} />
		</>
	);
}
