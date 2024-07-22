"use client";

import {
	FilePathRow,
	ProjectPathWarnings,
} from "@/components/common-setting-parts";
import { CardDescription } from "@/components/ui/card";
import { environmentPickProjectDefaultPath } from "@/lib/bindings";
import { tc } from "@/lib/i18n";
import { type BodyProps, SetupPageBase } from "../setup-page-base";

export default function Page() {
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

function Body({ environment, refetch }: BodyProps) {
	return (
		<>
			<CardDescription className={"whitespace-normal"}>
				{tc("setup:project-path:description")}
			</CardDescription>
			<FilePathRow
				withoutSelect
				path={environment.default_project_path}
				pick={environmentPickProjectDefaultPath}
				refetch={refetch}
				successMessage={tc("settings:toast:default project path updated")}
			/>
			<ProjectPathWarnings projectPath={environment.default_project_path} />
		</>
	);
}
