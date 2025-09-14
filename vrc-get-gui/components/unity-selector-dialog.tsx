import { useId, useState } from "react";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
	DialogFooter,
	DialogTitle,
} from "@/components/ui/dialog";
import { Label } from "@/components/ui/label";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import type { DialogContext } from "@/lib/dialog";
import { tc } from "@/lib/i18n";

type UnityInstallation = [path: string, version: string, fromHub: boolean];

type SelectResult = {
	unityPath: string;
	keepUsingThisVersion: boolean;
};

export function UnitySelectorDialog({
	unityVersions,
	supportKeepUsing = false,
	dialog,
}: {
	unityVersions: UnityInstallation[];
	supportKeepUsing?: boolean;
	dialog: DialogContext<SelectResult | null>;
}) {
	const id = useId();

	const [selectedUnityPath, setSelectedUnityPath] = useState<string | null>(
		null,
	);
	const [keepUsingThisVersion, setKeepUsingThisVersion] = useState(false);

	return (
		<div className={"contents whitespace-normal"}>
			<DialogTitle>{tc("projects:dialog:select unity header")}</DialogTitle>
			<div>
				<p>{tc("projects:dialog:multiple unity found")}</p>
				{supportKeepUsing && (
					<span className={"block my-2"}>
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
					</span>
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
			</div>
			<DialogFooter>
				<Button onClick={() => dialog.close(null)} className="mr-2">
					{tc("general:button:cancel")}
				</Button>
				<Button
					onClick={() =>
						dialog.close({
							// biome-ignore lint/style/noNonNullAssertion: disabled button
							unityPath: selectedUnityPath!,
							keepUsingThisVersion,
						})
					}
					disabled={selectedUnityPath == null}
				>
					{tc("general:button:continue")}
				</Button>
			</DialogFooter>
		</div>
	);
}
