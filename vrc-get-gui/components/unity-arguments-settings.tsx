import type React from "react";
import { useState } from "react";
import { tc } from "@/lib/i18n";
import { ReorderableList, useReorderableList } from "./ReorderableList";
import { Button } from "./ui/button";
import { Input } from "./ui/input";

const internal = Symbol("useLaunchSettingsDescriptionInternal");

interface LaunchSettingsDescriptionContext {
	currentValue: string[] | null;
	hasError: boolean;
	[internal]: {
		customizeCommandline: boolean;
		setCustomizeCommandline: (value: boolean) => void;
		reorderableListContext: ReturnType<typeof useReorderableList<string>>;
		errorMessage: React.ReactNode;
		defaultUnityArgs: string[];
	};
}

export function useUnityArgumentsSettings(
	initialValue: string[] | null,
	defaultUnityArgs: string[],
): LaunchSettingsDescriptionContext {
	const [customizeCommandline, setCustomizeCommandline] = useState(
		initialValue != null,
	);

	const reorderableListContext = useReorderableList<string>({
		defaultValue: "",
		defaultArray: initialValue ?? defaultUnityArgs,
	});

	let errorMessage: React.ReactNode;

	if (
		customizeCommandline &&
		reorderableListContext.value.some((x) => x.length === 0)
	) {
		errorMessage = tc("projects:hint:some arguments are empty");
	}

	return {
		get currentValue() {
			return customizeCommandline ? reorderableListContext.value : null;
		},
		hasError: errorMessage != null,
		[internal]: {
			customizeCommandline,
			setCustomizeCommandline,
			reorderableListContext,
			errorMessage,
			defaultUnityArgs,
		},
	};
}

export function UnityArgumentsSettings({
	context,
}: {
	context: LaunchSettingsDescriptionContext;
}) {
	const {
		customizeCommandline,
		setCustomizeCommandline,
		reorderableListContext,
		errorMessage,
		defaultUnityArgs,
	} = context[internal];

	// only for render; should not be modified
	const defaultValueContext = useReorderableList<string>({
		defaultValue: "",
		defaultArray: defaultUnityArgs,
	});

	if (customizeCommandline) {
		return (
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
						onClick={() => reorderableListContext.setList(defaultUnityArgs)}
					>
						{tc("general:button:reset")}
					</Button>
					<div className={"text-destructive whitespace-normal"}>
						{errorMessage}
					</div>
				</div>
			</>
		);
	} else {
		return (
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
		);
	}
}
