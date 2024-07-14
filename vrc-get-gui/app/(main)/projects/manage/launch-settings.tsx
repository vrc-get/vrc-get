import {
	ReorderableList,
	useReorderableList,
} from "@/components/ReorderableList";
import { Button } from "@/components/ui/button";
import {
	DialogDescription,
	DialogFooter,
	DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import {
	projectGetCustomUnityArgs,
	projectSetCustomUnityArgs,
} from "@/lib/bindings";
import { tc } from "@/lib/i18n";
import type React from "react";
import { useEffect, useState } from "react";

// Note: remember to change similar in rust side
const defaultArgs = ["-debugCodeOptimization"];

export function LaunchSettings({
	projectPath,
	close,
}: {
	projectPath: string;
	close: () => void;
}) {
	const [customizeCommandline, setCustomizeCommandline] = useState(false);

	const reorderableListContext = useReorderableList<string>({
		defaultValue: "",
		defaultArray: defaultArgs,
	});

	// only for render; should not be modified
	const defaultValueContext = useReorderableList<string>({
		defaultValue: "",
		defaultArray: defaultArgs,
	});

	// biome-ignore lint/correctness/useExhaustiveDependencies: we want to change on projectPath
	useEffect(() => {
		void (async () => {
			const args = await projectGetCustomUnityArgs(projectPath);
			if (args != null) {
				setCustomizeCommandline(true);
				reorderableListContext.setList(args);
			}
		})();
	}, [projectPath]);

	const save = async () => {
		await projectSetCustomUnityArgs(
			projectPath,
			customizeCommandline ? reorderableListContext.value : null,
		);
		close();
	};

	let errorMessage: React.ReactNode;

	if (
		customizeCommandline &&
		reorderableListContext.value.some((x) => x.length === 0)
	) {
		errorMessage = tc("projects:hint:some arguments are empty");
	}

	return (
		<>
			<DialogTitle>{tc("projects:dialog:launch options")}</DialogTitle>
			{/* TODO: use ScrollArea (I failed to use it inside dialog) */}
			<DialogDescription className={"max-h-[50dvh] overflow-y-auto"}>
				<h3 className={"text-lg"}>
					{tc("projects:dialog:command-line arguments")}
				</h3>
				{customizeCommandline ? (
					<>
						<Button onClick={() => setCustomizeCommandline(false)}>
							{tc("projects:dialog:use default command line arguments")}
						</Button>
						{reorderableListContext.value.length > 0 ? (
							<table className={"w-full my-2"}>
								<ReorderableList
									context={reorderableListContext}
									renderItem={(arg, id) => (
										<Input
											value={arg}
											onChange={(e) =>
												reorderableListContext.update(id, e.target.value)
											}
											className={"w-full"}
										/>
									)}
								/>
							</table>
						) : (
							<div>
								{tc("projects:dialog:empty command line arguments")}
								<Button
									className={"m-2"}
									onClick={() => reorderableListContext.add("")}
								>
									{tc("general:button:add")}
								</Button>
							</div>
						)}
						<div className={"flex gap-1 m-1 items-center"}>
							<Button
								onClick={() => reorderableListContext.setList(defaultArgs)}
							>
								{tc("general:button:reset")}
							</Button>
							<div className={"text-destructive whitespace-normal"}>
								{errorMessage}
							</div>
						</div>
					</>
				) : (
					<>
						<Button onClick={() => setCustomizeCommandline(true)}>
							{tc("projects:dialog:customize command line arguments")}
						</Button>
						<table className={"w-full my-2"}>
							<ReorderableList
								context={defaultValueContext}
								disabled
								renderItem={(arg) => (
									<Input disabled value={arg} className={"w-full"} />
								)}
							/>
						</table>
					</>
				)}
			</DialogDescription>
			<DialogFooter>
				<Button onClick={close} variant={"destructive"}>
					{tc("general:button:cancel")}
				</Button>
				<Button onClick={save} disabled={errorMessage != null}>
					{tc("general:button:save")}
				</Button>
			</DialogFooter>
		</>
	);
}
