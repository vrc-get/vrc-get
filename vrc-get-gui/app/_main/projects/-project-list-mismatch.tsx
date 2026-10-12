import { queryOptions, useMutation } from "@tanstack/react-query";
import type React from "react";
import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogTitle,
} from "@/components/ui/dialog";
import { commands, type TauriPartialProject } from "@/lib/bindings";
import { tc } from "@/lib/i18n";
import { cn } from "@/lib/utils";

const environmentProjects = queryOptions({
	queryKey: ["environmentProjects"],
	queryFn: commands.environmentProjects,
});

export function MismatchDialogContent({
	data,
	setDialogOpen,
}: {
	data: Awaited<ReturnType<typeof commands.environmentProjects>>;
	setDialogOpen: (state: boolean | null) => void;
}) {
	const [keepProjects, setKeepProjects] = useState(() =>
		data.only_in_vrc_get.map((p) => p.path),
	);

	const containsVcc = containsIntermediate(
		keepProjects,
		data.only_in_vcc.map((p) => p.path),
	);
	const containsVrcGet = containsIntermediate(
		keepProjects,
		data.only_in_vrc_get.map((p) => p.path),
	);

	const setKeep = (projects: TauriPartialProject[], keepProject: boolean) => {
		setKeepProjects((paths) => {
			const projectsPaths = projects.map((p) => p.path);
			if (keepProject) {
				return [...paths, ...projectsPaths];
			} else {
				return paths.filter((path) => !projectsPaths.includes(path));
			}
		});
	};

	const applyChanges = useMutation({
		mutationFn: async () => {
			const keeps = new Set(keepProjects);
			const litedb = data.only_in_vcc.map((x) => x.path);
			const sqlite = data.only_in_vrc_get.map((x) => x.path);
			await commands.environmentFixProjectListMismatch(
				litedb.filter((x) => keeps.has(x)),
				sqlite.filter((x) => keeps.has(x)),
				litedb.filter((x) => !keeps.has(x)),
				sqlite.filter((x) => !keeps.has(x)),
			);
		},
		onSettled: async (_1, _2, _3, _4, c) => {
			await c.client.invalidateQueries(environmentProjects);
			setDialogOpen(null);
		},
	});

	return (
		<DialogContent className={"max-h-[90vh]"}>
			<DialogTitle>{tc("projects:dialog:mismatch-found")}</DialogTitle>
			<DialogDescription>
				{tc("projects:dialog:mismatch-found:description", {
					only_in_vcc: data.only_in_vcc.length,
					only_in_vrc_get: data.only_in_vrc_get.length,
				})}
			</DialogDescription>

			<div className={"overflow-y-auto"}>
				<div
					className={
						"grid grid-cols-[1em_1fr_min-content_min-content_min-content] [&_.project-row:nth-child(even_of_.project-row)]:bg-secondary"
					}
				>
					<ProjectRow className={"row-start-1 row-end-2 *:p-1"}>
						<div className={"col-span-2 flex justify-center"}>
							{tc("projects:dialog:mismatch-project-table:path")}
						</div>
						<div>{tc("projects:dialog:mismatch-project-table:source")}</div>
						<div>{tc("projects:dialog:mismatch-project-table:keep")}</div>
						<div>{tc("projects:dialog:mismatch-project-table:remove")}</div>
					</ProjectRow>
					<ProjectRow className={"row-start-1 row-end-2 invisible"}>
						<div></div>
						<div>VCC</div>
					</ProjectRow>
					<ProjectRow className={"row-start-1 row-end-2 invisible"}>
						<div></div>
						<div>ALCOM</div>
					</ProjectRow>
					{data.only_in_vcc.length !== 0 && (
						<>
							<div className={"col-span-full border-t-2"} />
							<ProjectToggleRow
								indent={false}
								title={
									<span className="font-bold">
										{tc("projects:dialog:mismatch-project-table:only in vcc")}
									</span>
								}
								source={""}
								keep={containsVcc}
								setKeepProject={(keep) => setKeep(data.only_in_vcc, keep)}
							/>
						</>
					)}
					{data.only_in_vcc.map((proj, i) => (
						<ProjectToggleRow
							key={proj.path}
							title={proj.path}
							source={"VCC"}
							keep={keepProjects.includes(proj.path)}
							setKeepProject={(keep) => setKeep([proj], keep)}
						/>
					))}
					{data.only_in_vrc_get.length !== 0 && (
						<>
							<div className={"col-span-full border-t-2"} />
							<ProjectToggleRow
								indent={false}
								title={
									<span className="font-bold">
										{tc("projects:dialog:mismatch-project-table:only in alcom")}
									</span>
								}
								source={""}
								keep={containsVrcGet}
								setKeepProject={(keep) => setKeep(data.only_in_vrc_get, keep)}
							/>
						</>
					)}
					{data.only_in_vrc_get.map((proj) => (
						<ProjectToggleRow
							key={proj.path}
							title={proj.path}
							source={"ALCOM"}
							keep={keepProjects.includes(proj.path)}
							setKeepProject={(keep) => setKeep([proj], keep)}
						/>
					))}
				</div>
			</div>
			<DialogFooter>
				<Button size={"sm"} onClick={() => applyChanges.mutate()}>
					{tc("projects:dialog:apply changes")}
				</Button>
				<Button size={"sm"} onClick={() => setDialogOpen(false)}>
					{tc("projects:dialog:check later")}
				</Button>
			</DialogFooter>
		</DialogContent>
	);
}

function ProjectRow({
	children,
	className,
}: {
	children: React.ReactNode;
	className?: string;
}) {
	return (
		<div
			className={`${cn(
				"col-span-full grid grid-cols-subgrid",
				className,
			)} project-row`}
		>
			{children}
		</div>
	);
}

function ProjectToggleRow({
	title,
	keep,
	source,
	setKeepProject,
	indent = true,
}: {
	title: React.ReactNode;
	keep: boolean | "indeterminate";
	setKeepProject: (keep: boolean) => void;
	source: string;
	indent?: boolean;
}) {
	return (
		<ProjectRow className="*:p-1">
			<div className={indent ? "col-start-2" : "col-span-2"}>{title}</div>
			<div className={"flex justify-center"}>{source}</div>
			<div className={"flex justify-center"}>
				<Checkbox
					checked={keep}
					onCheckedChange={(checked) => {
						if (checked === true) setKeepProject(true);
					}}
				/>
			</div>
			<div className={"flex justify-center"}>
				<Checkbox
					checked={invertIndeterminate(keep)}
					onCheckedChange={(checked) => {
						if (checked === true) setKeepProject(false);
					}}
				/>
			</div>
		</ProjectRow>
	);
}

function containsIntermediate(
	array: string[],
	values: string[],
): boolean | "indeterminate" {
	let hasContains = false;
	let hasMissing = false;
	for (let i = values.length - 1; i >= 0; i--) {
		if (array.includes(values[i])) {
			hasContains = true;
		} else {
			hasMissing = true;
		}
	}
	if (!hasContains) return false;
	if (!hasMissing) return true;
	return "indeterminate";
}

function invertIndeterminate(value: boolean | "indeterminate") {
	if (value === "indeterminate") return "indeterminate";
	return !value;
}
