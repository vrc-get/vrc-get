import { Button } from "@/components/ui/button";
import {
	DialogDescription,
	DialogFooter,
	DialogOpen,
	DialogTitle,
} from "@/components/ui/dialog";
import { Progress } from "@/components/ui/progress";
import type { CheckForUpdateResponse } from "@/lib/bindings";
import { commands } from "@/lib/bindings";
import { tc } from "@/lib/i18n";
import { toastThrownError } from "@/lib/toast";
import { useState } from "react";

type ConfirmStatus = "confirming" | "downloading" | "waitingForRelaunch";

interface DownloadProgressEvent {
	chunkLength: number;
	contentLength: number;
}

export function CheckForUpdateMessage({
	response,
	close,
}: {
	response: CheckForUpdateResponse;
	close: () => void;
}) {
	const [confirmStatus, setConfirmStatus] =
		useState<ConfirmStatus>("confirming");
	const [downloadedBytes, setDownloadedBytes] = useState(0);
	const [totalBytes, setTotalBytes] = useState(100);

	// TODO: make utilInstallAndUpgrade async command
	console.log("downloadedBytes / totalBytes", downloadedBytes, totalBytes);

	const startDownload = async () => {
		setConfirmStatus("downloading");
		try {
			await commands.utilInstallAndUpgrade(response.version);
		} catch (e) {
			toastThrownError(e);
			console.error(e);
			close();
		}
	};

	switch (confirmStatus) {
		case "confirming":
			return (
				<DialogOpen>
					<DialogTitle>{tc("check update:dialog:title")}</DialogTitle>
					<DialogDescription>
						<p>{tc("check update:dialog:new version description")}</p>
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
							{response.update_description}
						</p>
					</DialogDescription>
					<DialogFooter className={"gap-2"}>
						<Button onClick={close}>{tc("check update:dialog:dismiss")}</Button>
						<Button onClick={startDownload}>
							{tc("check update:dialog:update")}
						</Button>
					</DialogFooter>
				</DialogOpen>
			);
		case "downloading":
			return (
				<DialogOpen>
					<DialogTitle>{tc("check update:dialog:title")}</DialogTitle>
					<DialogDescription>
						<p>{tc("check update:dialog:downloading...")}</p>
						<Progress value={downloadedBytes} max={totalBytes} />
					</DialogDescription>
				</DialogOpen>
			);
		case "waitingForRelaunch":
			return (
				<DialogOpen>
					<DialogTitle>{tc("check update:dialog:title")}</DialogTitle>
					<DialogDescription>
						<p>{tc("check update:dialog:relaunching...")}</p>
					</DialogDescription>
				</DialogOpen>
			);
	}
}
