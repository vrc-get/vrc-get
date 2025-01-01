import {
	Accordion,
	AccordionContent,
	AccordionItem,
	AccordionTrigger,
} from "@/components/ui/accordion";
import { Button } from "@/components/ui/button";
import {
	DialogDescription,
	DialogFooter,
	DialogOpen,
	DialogTitle,
} from "@/components/ui/dialog";
import { Progress } from "@/components/ui/progress";
import { assertNever } from "@/lib/assert-never";
import type {
	TauriDownloadRepository,
	TauriRepositoryDescriptor,
} from "@/lib/bindings";
import { commands } from "@/lib/bindings";
import { callAsyncCommand } from "@/lib/call-async-command";
import { tc, tt } from "@/lib/i18n";
import { toastSuccess, toastThrownError } from "@/lib/toast";
import { useFilePickerFunction } from "@/lib/use-file-picker-dialog";
import type React from "react";
import { useCallback, useState } from "react";

type ParsedRepositories = {
	repositories: TauriRepositoryDescriptor[];
	unparsable_lines: string[];
};

type State =
	| {
			type: "normal";
	  }
	| {
			type: "confirmingRepositories";
			pickResult: ParsedRepositories;
	  }
	| {
			type: "loadingRepositories";
			totalCount: number;
			downloaded: number;
			cancel: () => void;
	  }
	| {
			type: "confirmingPackages";
			repositories: [TauriRepositoryDescriptor, TauriDownloadRepository][];
	  }
	| {
			type: "addingRepositories";
	  };

interface AddRepository {
	dialog: React.ReactNode;
	startImportingRepositories: () => void;
}

export function useImportRepositories({
	refetch,
}: {
	refetch: () => void;
}): AddRepository {
	const [state, setState] = useState<State>({ type: "normal" });
	const [importRepositoryPick, pickDialog] = useFilePickerFunction(
		commands.environmentImportRepositoryPick,
	);

	function cancel() {
		if ("cancel" in state) state.cancel();
		setState({ type: "normal" });
	}

	const startImportingRepositories = useCallback(
		async function startImportingRepositories() {
			try {
				const pickResult = await importRepositoryPick();
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
				setState({ type: "confirmingRepositories", pickResult });
			} catch (e) {
				toastThrownError(e);
				setState({ type: "normal" });
			}
		},
		[importRepositoryPick],
	);

	const downloadRepositories = useCallback(async function downloadRepositories(
		repositories: TauriRepositoryDescriptor[],
	) {
		try {
			const totalCount = repositories.length;
			const [cancel, resultPromise] = callAsyncCommand(
				commands.environmentImportDownloadRepositories,
				[repositories],
				(downloaded) => {
					setState({
						type: "loadingRepositories",
						totalCount,
						downloaded,
						cancel,
					});
				},
			);
			setState({
				type: "loadingRepositories",
				totalCount,
				downloaded: 0,
				cancel,
			});
			const result = await resultPromise;
			if (result === "cancelled") {
				return;
			}
			setState({ type: "confirmingPackages", repositories: result });
		} catch (e) {
			toastThrownError(e);
			setState({ type: "normal" });
		}
	}, []);

	const addRepositories = useCallback(
		async function addRepositories(repositories: TauriRepositoryDescriptor[]) {
			try {
				setState({ type: "addingRepositories" });
				await commands.environmentImportAddRepositories(repositories);
				toastSuccess(tt("vpm repositories:toast:repository added"));
				refetch();
				setState({ type: "normal" });
			} catch (e) {
				toastThrownError(e);
				setState({ type: "normal" });
			}
		},
		[refetch],
	);

	let dialogBody: React.ReactNode;
	switch (state.type) {
		case "normal":
			dialogBody = null;
			break;
		case "confirmingRepositories":
			dialogBody = (
				<ConfirmingRepositoryList
					pickResult={state.pickResult}
					cancel={cancel}
					importRepositories={downloadRepositories}
				/>
			);
			break;
		case "loadingRepositories":
			dialogBody = (
				<LoadingRepositories
					cancel={cancel}
					downloaded={state.downloaded}
					totalCount={state.totalCount}
				/>
			);
			break;
		case "confirmingPackages":
			dialogBody = (
				<ConfirmingPackages
					repositories={state.repositories}
					cancel={cancel}
					addRepositories={addRepositories}
				/>
			);
			break;
		case "addingRepositories":
			dialogBody = <AddingRepositories />;
			break;
		default:
			assertNever(state, "state");
	}

	const confirmDialog = dialogBody ? (
		<DialogOpen>
			<DialogTitle>
				{tc("vpm repositories:dialog:import repositories")}
			</DialogTitle>
			{dialogBody}
		</DialogOpen>
	) : null;

	return {
		dialog: (
			<>
				{pickDialog}
				{confirmDialog}
			</>
		),
		startImportingRepositories,
	};
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
	cancel,
	importRepositories,
}: {
	pickResult: ParsedRepositories;
	cancel: () => void;
	importRepositories: (repositories: TauriRepositoryDescriptor[]) => void;
}) {
	const onContinue = useCallback(
		async function onContinue() {
			importRepositories(pickResult.repositories);
		},
		[importRepositories, pickResult.repositories],
	);

	return (
		<>
			{/* TODO: use ScrollArea (I failed to use it inside dialog) */}
			<DialogDescription className={"max-h-[50vh] overflow-y-auto font-normal"}>
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
			</DialogDescription>
			<DialogFooter className={"gap-2"}>
				<Button onClick={cancel}>{tc("general:button:cancel")}</Button>
				<Button onClick={onContinue}>
					{tc("vpm repositories:dialog:button:continue importing repositories")}
				</Button>
			</DialogFooter>
		</>
	);
}

function LoadingRepositories({
	cancel,
	downloaded,
	totalCount,
}: {
	cancel: () => void;
	downloaded: number;
	totalCount: number;
}) {
	return (
		<>
			<DialogDescription>
				<p>{tc("vpm repositories:dialog:downloading repositories...")}</p>
				<Progress value={downloaded} max={totalCount} />
				<div className={"text-center"}>
					{tc("vpm repositories:dialog:downloaded n/m", {
						downloaded,
						totalCount,
					})}
				</div>
			</DialogDescription>
			<DialogFooter>
				<Button onClick={cancel}>{tc("general:button:cancel")}</Button>
			</DialogFooter>
		</>
	);
}

function ConfirmingPackages({
	repositories,
	cancel,
	addRepositories,
}: {
	repositories: [TauriRepositoryDescriptor, TauriDownloadRepository][];
	cancel: () => void;
	addRepositories: (repositories: TauriRepositoryDescriptor[]) => void;
}) {
	const add = useCallback(
		async function add() {
			addRepositories(
				repositories
					.filter(([_, download]) => download.type === "Success")
					.map(([repo, _]) => repo),
			);
		},
		[addRepositories, repositories],
	);

	return (
		<>
			{/* TODO: use ScrollArea (I failed to use it inside dialog) */}
			<DialogDescription className={"font-normal"}>
				<p className={"whitespace-normal"}>
					{tc("vpm repositories:dialog:confirm packages list")}
				</p>
				<Accordion
					type="single"
					collapsible
					className="max-h-[50vh] overflow-y-auto w-full"
				>
					{repositories.map(([repo, download]) => {
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
										{download.value.packages.map((info, idx) => (
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
			</DialogDescription>
			<DialogFooter>
				<Button onClick={cancel}>{tc("general:button:cancel")}</Button>
				<Button onClick={add} className={"ml-2"}>
					{tc("vpm repositories:button:add repositories")}
				</Button>
			</DialogFooter>
		</>
	);
}

function AddingRepositories() {
	return (
		<>
			<DialogDescription>
				<p>{tc("vpm repositories:dialog:adding repositories...")}</p>
			</DialogDescription>
		</>
	);
}
