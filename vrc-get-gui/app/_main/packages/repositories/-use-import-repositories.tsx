import { queryOptions } from "@tanstack/react-query";
import type React from "react";
import { useEffect, useRef, useState } from "react";
import {
	Accordion,
	AccordionContent,
	AccordionItem,
	AccordionTrigger,
} from "@/components/ui/accordion";
import { Button } from "@/components/ui/button";
import { DialogFooter } from "@/components/ui/dialog";
import { Progress } from "@/components/ui/progress";
import { assertNever } from "@/lib/assert-never";
import type {
	TauriDownloadRepository,
	TauriRepositoryDescriptor,
} from "@/lib/bindings";
import { commands } from "@/lib/bindings";
import { callAsyncCommand } from "@/lib/call-async-command";
import { type DialogContext, showDialog } from "@/lib/dialog";
import { tc, tt } from "@/lib/i18n";
import { queryClient } from "@/lib/query-client";
import { toastSuccess } from "@/lib/toast";
import { useEffectEvent } from "@/lib/use-effect-event";

type ParsedRepositories = {
	repositories: TauriRepositoryDescriptor[];
	unparsable_lines: string[];
};

const environmentRepositoriesInfo = queryOptions({
	queryKey: ["environmentRepositoriesInfo"],
	queryFn: commands.environmentRepositoriesInfo,
});

export async function importRepositories() {
	using dialog = showDialog();

	const pickResult = await commands.environmentImportRepositoryPick();
	switch (pickResult.type) {
		case "NoFilePicked":
			// no-op
			return;
		case "ParsedRepositories":
			// continue
			break;
		default:
			assertNever(pickResult, "pickResult");
	}
	console.log("confirmingRepositories", pickResult);

	const repositories = await dialog.ask(ConfirmingRepositoryList, {
		pickResult,
	});
	if (repositories == null) return;

	const packages = await dialog.ask(LoadingRepositories, {
		repositories,
	});
	if (packages == null) return;

	const repositoriesToAdd = await dialog.ask(ConfirmingPackages, {
		packages,
	});
	if (repositoriesToAdd == null) return;

	dialog.replace(<AddingRepositories />);
	await commands.environmentImportAddRepositories(repositoriesToAdd);
	toastSuccess(tt("vpm repositories:toast:repository added"));
	dialog.close();

	await queryClient.invalidateQueries(environmentRepositoriesInfo);
}

function shortRepositoryDescription(
	repo: TauriRepositoryDescriptor,
): React.ReactNode {
	if (Object.keys(repo.headers).length > 0) {
		return tc("vpm repositories:dialog:repository with headers", {
			repoUrl: repo.url,
		});
	}
	return repo.url;
}

function ConfirmingRepositoryList({
	pickResult,
	dialog,
}: {
	pickResult: ParsedRepositories;
	dialog: DialogContext<TauriRepositoryDescriptor[] | null>;
}) {
	return (
		<>
			{/* TODO: use ScrollArea (I failed to use it inside dialog) */}
			<div className={"max-h-[50vh] overflow-y-auto font-normal"}>
				<p className={"font-normal whitespace-normal"}>
					{tc("vpm repositories:dialog:confirm repository list")}
				</p>

				<ul className={"list-disc pl-6"}>
					{pickResult.repositories.map((info) => (
						<li key={info.url}>{shortRepositoryDescription(info)}</li>
					))}
				</ul>

				{pickResult.unparsable_lines.length > 0 && (
					<>
						<p className={"font-normal whitespace-normal"}>
							{tc("vpm repositories:dialog:unparsable lines list")}
						</p>
						<ul className={"list-disc pl-6"}>
							{pickResult.unparsable_lines.map((line, idx) => (
								// biome-ignore lint/suspicious/noArrayIndexKey: unchanged
								<li key={idx} className={"whitespace-pre"}>
									{line}
								</li>
							))}
						</ul>
					</>
				)}
			</div>
			<DialogFooter className={"gap-2"}>
				<Button onClick={() => dialog.close(null)}>
					{tc("general:button:cancel")}
				</Button>
				<Button onClick={() => dialog.close(pickResult.repositories)}>
					{tc("vpm repositories:dialog:button:continue importing repositories")}
				</Button>
			</DialogFooter>
		</>
	);
}

function LoadingRepositories({
	repositories,
	dialog,
}: {
	repositories: TauriRepositoryDescriptor[];
	dialog: DialogContext<
		[TauriRepositoryDescriptor, TauriDownloadRepository][] | null
	>;
}) {
	const cancelRef = useRef<() => void>(() => {});
	const totalCount = repositories.length;
	const [downloaded, setDownloaded] = useState(0);

	const event = useEffectEvent(() => {
		const [cancel, resultPromise] = callAsyncCommand(
			commands.environmentImportDownloadRepositories,
			[repositories],
			(downloaded) => setDownloaded(downloaded),
		);
		cancelRef.current = cancel;
		resultPromise.then((x) => dialog.close(x === "cancelled" ? null : x));
	});

	useEffect(() => event(), []);

	return (
		<>
			<div>
				<p>{tc("vpm repositories:dialog:downloading repositories...")}</p>
				<Progress value={downloaded} max={totalCount} />
				<div className={"text-center"}>
					{tc("vpm repositories:dialog:downloaded n/m", {
						downloaded,
						totalCount,
					})}
				</div>
			</div>
			<DialogFooter>
				<Button onClick={() => cancelRef.current?.()}>
					{tc("general:button:cancel")}
				</Button>
			</DialogFooter>
		</>
	);
}

function ConfirmingPackages({
	packages,
	dialog,
}: {
	packages: [TauriRepositoryDescriptor, TauriDownloadRepository][];
	dialog: DialogContext<TauriRepositoryDescriptor[] | null>;
}) {
	async function add() {
		dialog.close(
			packages
				.filter(([_, download]) => download.type === "Success")
				.map(([repo, _]) => repo),
		);
	}

	return (
		<>
			{/* TODO: use ScrollArea (I failed to use it inside dialog) */}
			<div className={"font-normal"}>
				<p className={"whitespace-normal"}>
					{tc("vpm repositories:dialog:confirm packages list")}
				</p>
				<Accordion
					type="single"
					collapsible
					className="max-h-[50vh] overflow-y-auto w-full"
				>
					{packages.map(([repo, download]) => {
						let error: boolean;
						let content: React.ReactNode;
						switch (download.type) {
							case "BadUrl":
								throw new Error("BadUrl should not be here");
							case "Duplicated":
								error = true;
								content = tc(
									"vpm repositories:dialog:download error:duplicated",
								);
								break;
							case "DownloadError":
								error = true;
								content = tc(
									"vpm repositories:dialog:download error:download error",
								);
								break;
							case "Success":
								error = false;
								content = (
									<ul className={"list-disc pl-6"}>
										{download.value.packages.map((info) => (
											<li key={info.name}>{info.display_name ?? info.name}</li>
										))}
									</ul>
								);
								break;
							default:
								assertNever(download, "download");
						}
						const destrucive = error ? "text-destructive" : "";
						return (
							<AccordionItem value={repo.url} key={repo.url}>
								<AccordionTrigger className={`${destrucive} py-2 text-base`}>
									{shortRepositoryDescription(repo)}
								</AccordionTrigger>
								<AccordionContent className={destrucive}>
									{content}
								</AccordionContent>
							</AccordionItem>
						);
					})}
				</Accordion>
			</div>
			<DialogFooter>
				<Button onClick={() => dialog.close(null)}>
					{tc("general:button:cancel")}
				</Button>
				<Button onClick={add} className={"ml-2"}>
					{tc("vpm repositories:button:add repositories")}
				</Button>
			</DialogFooter>
		</>
	);
}

function AddingRepositories() {
	return (
		<div>
			<p>{tc("vpm repositories:dialog:adding repositories...")}</p>
		</div>
	);
}
