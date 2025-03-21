import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
	DialogDescription,
	DialogFooter,
	DialogOpen,
	DialogTitle,
} from "@/components/ui/dialog";
import { Label } from "@/components/ui/label";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import { assertNever } from "@/lib/assert-never";
import { tc } from "@/lib/i18n";
import React, { useId, useState } from "react";

type UnityInstallation = [path: string, version: string, fromHub: boolean];

type StateUnitySelector =
	| {
			state: "normal";
	  }
	| {
			state: "selecting";
			unityVersions: UnityInstallation[];
			supportKeepUsing: boolean; // if true, show the option to keep using this unity in the future
			resolve: (unityInfo: SelectResult | null) => void;
	  };

type SelectResult = {
	unityPath: string;
	keepUsingThisVersion: boolean;
};

type ResultUnitySelector = {
	dialog: React.ReactNode;
	select(
		unityList: UnityInstallation[],
		supportKeepUsing?: true,
	): Promise<SelectResult | null>;
};

export function useUnitySelectorDialog(): ResultUnitySelector {
	const [installStatus, setInstallStatus] = React.useState<StateUnitySelector>({
		state: "normal",
	});

	function select(
		unityVersions: UnityInstallation[],
		supportKeepUsing?: boolean,
	): Promise<SelectResult | null> {
		return new Promise<SelectResult | null>((resolve) => {
			setInstallStatus({
				state: "selecting",
				unityVersions,
				resolve,
				supportKeepUsing: supportKeepUsing ?? false,
			});
		});
	}
	let dialog: React.ReactNode = null;

	switch (installStatus.state) {
		case "normal":
			break;
		case "selecting": {
			const cancel = () => {
				setInstallStatus({ state: "normal" });
				installStatus.resolve(null);
			};
			const resolveWrapper = (
				unityPath: string,
				keepUsingThisVersion: boolean,
			) => {
				setInstallStatus({ state: "normal" });
				installStatus.resolve({ unityPath, keepUsingThisVersion });
			};
			dialog = (
				<DialogOpen className={"whitespace-normal"}>
					<DialogTitle>{tc("projects:dialog:select unity header")}</DialogTitle>
					<SelectUnityVersionDialog
						unityVersions={installStatus.unityVersions}
						cancel={cancel}
						withKeepUsing={installStatus.supportKeepUsing}
						onSelect={resolveWrapper}
					/>
				</DialogOpen>
			);
			break;
		}
		default:
			assertNever(installStatus);
	}

	return { dialog, select };
}

function SelectUnityVersionDialog({
	unityVersions,
	cancel,
	withKeepUsing,
	onSelect,
}: {
	unityVersions: UnityInstallation[];
	cancel: () => void;
	withKeepUsing: boolean;
	onSelect: (unityPath: string, keepUsingThisVersion: boolean) => void;
}) {
	const id = useId();

	const [selectedUnityPath, setSelectedUnityPath] = useState<string | null>(
		null,
	);
	const [keepUsingThisVersion, setKeepUsingThisVersion] = useState(false);

	return (
		<>
			<DialogDescription>
				<p>{tc("projects:dialog:multiple unity found")}</p>
				{withKeepUsing && (
					<div className={"my-2"}>
						<label
							className={
								"flex cursor-pointer items-center gap-2 p-2 whitespace-normal"
							}
						>
							<Checkbox
								checked={keepUsingThisVersion}
								onCheckedChange={(e) => setKeepUsingThisVersion(e === true)}
								className="hover:before:content-none"
							/>
							{tc("projects:dialog:keep using this version")}
						</label>
					</div>
				)}
				<RadioGroup
					onValueChange={(path) => setSelectedUnityPath(path)}
					value={selectedUnityPath ?? undefined}
				>
					{unityVersions.map(([path, version, _]) => (
						<div key={path} className={"flex items-center gap-2"}>
							<RadioGroupItem value={path} id={`${id}:${path}`} />
							<Label htmlFor={`${id}:${path}`}>{`${version} (${path})`}</Label>
						</div>
					))}
				</RadioGroup>
			</DialogDescription>
			<DialogFooter>
				<Button onClick={cancel} className="mr-1">
					{tc("general:button:cancel")}
				</Button>
				<Button
					// biome-ignore lint/style/noNonNullAssertion: disabled button
					onClick={() => onSelect(selectedUnityPath!, keepUsingThisVersion)}
					disabled={selectedUnityPath == null}
				>
					{tc("general:button:continue")}
				</Button>
			</DialogFooter>
		</>
	);
}
