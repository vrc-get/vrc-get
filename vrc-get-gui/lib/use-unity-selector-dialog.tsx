import React, {useState} from "react";
import {Button} from "@/components/ui/button";
import {DialogDescription, DialogFooter, DialogOpen, DialogTitle} from "@/components/ui/dialog";
import {Label} from "@/components/ui/label";
import {RadioGroup, RadioGroupItem} from "@/components/ui/radio-group";
import {tc} from "@/lib/i18n";

type UnityInstallation = [path: string, version: string, fromHub: boolean];

type StateUnitySelector = {
	state: "normal";
} | {
	state: "selecting";
	unityVersions: UnityInstallation[];
	resolve: (unityPath: string | null) => void;
}

type ResultUnitySelector = {
	dialog: React.ReactNode;
	select: (unityList: [path: string, version: string, fromHub: boolean][]) => Promise<string | null>;
}

export function useUnitySelectorDialog(): ResultUnitySelector {
	const [installStatus, setInstallStatus] = React.useState<StateUnitySelector>({state: "normal"});

	const select = (unityVersions: UnityInstallation[]) => {
		return new Promise<string | null>((resolve) => {
			setInstallStatus({state: "selecting", unityVersions, resolve});
		});
	}
	let dialog: React.ReactNode = null;

	switch (installStatus.state) {
		case "normal":
			break;
		case "selecting":
			const resolveWrapper = (unityPath: string | null) => {
				setInstallStatus({state: "normal"});
				installStatus.resolve(unityPath);
			};
			dialog = <DialogOpen className={"whitespace-normal"}>
				<DialogTitle>{tc("projects:manage:dialog:select unity header")}</DialogTitle>
				<SelectUnityVersionDialog
					unityVersions={installStatus.unityVersions}
					cancel={() => resolveWrapper(null)}
					onSelect={(unityPath) => resolveWrapper(unityPath)}
				/>
			</DialogOpen>;
			break;
		default:
			const _: never = installStatus;
	}

	return {dialog, select};
}

function SelectUnityVersionDialog(
	{
		unityVersions,
		cancel,
		onSelect,
	}: {
		unityVersions: UnityInstallation[],
		cancel: () => void,
		onSelect: (unityPath: string) => void,
	}) {
	const name = useState(() => `select-unity-version-${Math.random().toString(36).slice(2)}-radio`)[0];

	const [selectedUnityPath, setSelectedUnityPath] = useState<string | null>(null);

	return (
		<>
			<DialogDescription>
				<p>
					{tc("projects:manage:dialog:multiple unity found")}
				</p>
				{unityVersions.map(([path, version, _]) =>
					<RadioGroup
						key={path}
						name={name}
						className={"flex items-center space-x-2"}
						onValueChange={(path) => setSelectedUnityPath(path)}
					>
						<RadioGroupItem value={path} id={path} />
						<Label htmlFor={path}>{`${version} (${path})`}</Label>
					</RadioGroup>
				)}
			</DialogDescription>
			<DialogFooter>
				<Button onClick={cancel} className="mr-1">{tc("general:button:cancel")}</Button>
				<Button
					onClick={() => onSelect(selectedUnityPath!)}
					disabled={selectedUnityPath == null}
				>{tc("projects:manage:button:continue")}</Button>
			</DialogFooter>
		</>
	);
}
