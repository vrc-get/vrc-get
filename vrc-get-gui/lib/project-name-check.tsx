import { assertNever } from "@/lib/assert-never";
import { type TauriProjectDirCheckResult, commands } from "@/lib/bindings";
import { tc } from "@/lib/i18n";
import { useQuery } from "@tanstack/react-query";
import { useDebounce } from "@uidotdev/usehooks";
import { RefreshCw } from "lucide-react";

export function useProjectNameCheck(
	projectLocation: string,
	projectName: string,
): "checking" | TauriProjectDirCheckResult {
	const projectNameDebounced = useDebounce(projectName, 500);

	const projectNameCheckStateTest = useQuery({
		queryKey: [
			"environmentCheckProjectName",
			projectLocation,
			projectNameDebounced,
		],
		queryFn: () =>
			commands.environmentCheckProjectName(
				projectLocation,
				projectNameDebounced,
			),
	});

	return projectNameDebounced !== projectName ||
		projectNameCheckStateTest.isFetching
		? "checking"
		: (projectNameCheckStateTest.data ?? "checking");
}

export function ProjectNameCheckResult({
	projectNameCheckState,
}: {
	projectNameCheckState: "checking" | TauriProjectDirCheckResult;
}) {
	switch (projectNameCheckState) {
		case "Ok":
			return (
				<small className={"whitespace-normal text-success"}>
					{tc("projects:hint:create project ready")}
				</small>
			);
		case "InvalidNameForFolderName":
			return (
				<small className={"whitespace-normal text-destructive"}>
					{tc("projects:hint:invalid project name")}
				</small>
			);
		case "MayCompatibilityProblem":
			return (
				<small className={"whitespace-normal text-warning"}>
					{tc("projects:hint:warn symbol in project name")}
				</small>
			);
		case "WideChar":
			return (
				<small className={"whitespace-normal text-warning"}>
					{tc("projects:hint:warn multibyte char in project name")}
				</small>
			);
		case "AlreadyExists":
			return (
				<small className={"whitespace-normal text-destructive"}>
					{tc("projects:hint:project already exists")}
				</small>
			);
		case "checking":
			return (
				<small className={"whitespace-normal"}>
					<RefreshCw className={"w-5 h-5 animate-spin"} />
				</small>
			);
		default:
			assertNever(projectNameCheckState);
	}
}
