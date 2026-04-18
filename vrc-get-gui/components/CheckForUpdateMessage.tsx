import React, { useState } from "react";
import { ExternalLink } from "@/components/ExternalLink";
import { Button } from "@/components/ui/button";
import { DialogFooter, DialogTitle } from "@/components/ui/dialog";
import { Progress } from "@/components/ui/progress";
import { assertNever } from "@/lib/assert-never";
import type { CheckForUpdateResponse } from "@/lib/bindings";
import { commands } from "@/lib/bindings";
import { callAsyncCommand } from "@/lib/call-async-command";
import type { DialogContext } from "@/lib/dialog";
import globalInfo from "@/lib/global-info";
import { localizeExternalComponent, tc } from "@/lib/i18n";

type ConfirmStatus =
	| {
			state: "confirming";
	  }
	| {
			state: "downloading";
			total: number;
			downloaded: number;
	  }
	| {
			state: "waitingForRelaunch";
	  };

export function CheckForUpdateMessage({
	response,
	dialog,
}: {
	response: CheckForUpdateResponse;
	dialog: DialogContext<boolean>;
}) {
	const [confirmStatus, setConfirmStatus] = useState<ConfirmStatus>({
		state: "confirming",
	});

	const startDownload = async () => {
		setConfirmStatus({ state: "downloading", downloaded: 0, total: 100 });
		try {
			const [, promise] = callAsyncCommand(
				commands.utilInstallAndUpgrade,
				[response.version],
				(progress) => {
					switch (progress.type) {
						case "DownloadProgress":
							if (progress.total != null) {
								// likely: there is known total size by Content-Length header
								const { total, received } = progress;
								setConfirmStatus((s) => {
									if (s.state !== "downloading") return s;
									return {
										...s,
										downloaded: s.downloaded + received,
										total: total,
									};
								});
							} else {
								// unlikely: there is no Content-Length header

								// this data is based on previous releases.
								const estimatedTotalSize: number = {
									WindowsNT: 6 * 1000 * 1000,
									Linux: 90 * 1000 * 1000,
									Darwin: 20 * 1000 * 1000,
								}[globalInfo.osType];

								const { received } = progress;

								setConfirmStatus((s) => {
									if (s.state !== "downloading") return s;
									const downloadNew = s.downloaded + received;
									return {
										...s,
										downloaded:
											downloadNew > estimatedTotalSize
												? estimatedTotalSize
												: downloadNew,
										total: estimatedTotalSize,
									};
								});
							}
							break;
						case "DownloadComplete":
							setConfirmStatus({ state: "waitingForRelaunch" });
							break;
						default:
							assertNever(progress);
					}
				},
			);
			await promise;
		} catch (e) {
			dialog.error(e);
		}
	};

	const openAlcomWebsite = async () => {
		await commands.utilOpenUrl("https://an12.net/alcom/");
	};

	switch (confirmStatus.state) {
		case "confirming": {
			let message: React.ReactNode;

			switch (response.updater_status) {
				case "Updatable":
					message = <p>{tc("check update:dialog:new version description")}</p>;
					break;
				case "NoPlatform":
					message = (
						<p>
							{tc("check update:dialog:new version no platform description")}
						</p>
					);
					break;
				case "NotUpdatable":
					message = (
						<p>
							{tc("check update:dialog:new version not updatable description")}
						</p>
					);
					break;
				case "UpdaterDisabled":
					message = (
						<p>
							{tc(
								"check update:dialog:new version updater disabled base description",
							)}
							<br />
							{localizeExternalComponent(response.updater_disabled_messages, {
								localized:
									"check update:dialog:new version updater how to upgrade fallback",
							})}
						</p>
					);
					break;
				default:
					assertNever(response.updater_status);
			}

			const withDownloadButton = response.updater_status === "Updatable";
			const withDownloadLink =
				!withDownloadButton && response.updater_status !== "UpdaterDisabled";

			return (
				<>
					<DialogTitle>{tc("check update:dialog:title")}</DialogTitle>
					<div>
						{message}
						<p>
							{tc("check update:dialog:current version")}{" "}
							{response.current_version}
						</p>
						<p>
							{tc("check update:dialog:latest version")}{" "}
							{response.latest_version}
						</p>
						<h3>{tc("check update:dialog:changelog")}</h3>
						<p className={"whitespace-pre-wrap"}>
							<LinkedText
								text={
									response.update_description ?? "no description is provided"
								}
							/>
						</p>
					</div>
					<DialogFooter className={"gap-2"}>
						<Button onClick={() => dialog.close(false)}>
							{tc("check update:dialog:dismiss")}
						</Button>
						{withDownloadButton && (
							<Button onClick={startDownload}>
								{tc("check update:dialog:update")}
							</Button>
						)}
						{withDownloadLink && (
							<Button onClick={openAlcomWebsite}>
								{tc("check update:dialog:open download page")}
							</Button>
						)}
					</DialogFooter>
				</>
			);
		}
		case "downloading":
			return (
				<>
					<DialogTitle>{tc("check update:dialog:title")}</DialogTitle>
					<div>
						<p>{tc("check update:dialog:downloading...")}</p>
						<Progress
							value={confirmStatus.downloaded}
							max={confirmStatus.total}
						/>
					</div>
				</>
			);
		case "waitingForRelaunch":
			return (
				<>
					<DialogTitle>{tc("check update:dialog:title")}</DialogTitle>
					<div>
						<p>{tc("check update:dialog:relaunching...")}</p>
					</div>
				</>
			);
	}
}

const LinkedText = React.memo(({ text }: { text: string }) => {
	const urlRegex =
		/https:\/\/[a-zA-Z0-9]+(?:\.[a-zA-Z0-9]+)+\/[a-zA-Z0-9$\-_.+!*'()%/?#]*/g;
	const components: React.ReactNode[] = [];
	let lastMatchEnd = 0;
	for (const match of text.matchAll(urlRegex)) {
		const leading = text.substring(lastMatchEnd, match.index);
		components.push(leading);
		components.push(<ExternalLink href={match[0]}>{match[0]}</ExternalLink>);
		lastMatchEnd = match.index + match[0].length;
	}
	components.push(text.substring(lastMatchEnd));

	return React.createElement(React.Fragment, {}, components);
});
