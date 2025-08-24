"use client";

import {
	queryOptions,
	useMutation,
	useQuery,
	useQueryClient,
} from "@tanstack/react-query";
import { createFileRoute } from "@tanstack/react-router";
import { ChevronDown, CircleX } from "lucide-react";
import { Suspense, useCallback, useEffect, useId, useMemo } from "react";
import { HNavBar, VStack } from "@/components/layout";
import { ScrollableCardTable } from "@/components/ScrollableCardTable";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
	DialogDescription,
	DialogFooter,
	DialogTitle,
} from "@/components/ui/dialog";
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
	Tooltip,
	TooltipContent,
	TooltipTrigger,
} from "@/components/ui/tooltip";
import type { TauriUserRepository } from "@/lib/bindings";
import { commands } from "@/lib/bindings";
import { type DialogContext, openSingleDialog } from "@/lib/dialog";
import { tc, tt } from "@/lib/i18n";
import { usePrevPathName } from "@/lib/prev-page";
import { toastThrownError } from "@/lib/toast";
import { useTauriListen } from "@/lib/use-tauri-listen";
import { cn } from "@/lib/utils";
import { HeadingPageName } from "../-tab-selector";
import { addRepository, openAddRepositoryDialog } from "./-use-add-repository";
import { importRepositories } from "./-use-import-repositories";

export const Route = createFileRoute("/_main/packages/repositories/")({
	component: Page,
});

function Page() {
	return (
		<Suspense>
			<PageBody />
		</Suspense>
	);
}

const environmentRepositoriesInfo = queryOptions({
	queryKey: ["environmentRepositoriesInfo"],
	queryFn: commands.environmentRepositoriesInfo,
});

function PageBody() {
	const result = useQuery(environmentRepositoriesInfo);

	const exportRepositories = useMutation({
		mutationFn: async () => await commands.environmentExportRepositories(),
		onError: (e) => {
			console.error(e);
			toastThrownError(e);
		},
	});

	const importRepositoriesMutation = useMutation({
		mutationFn: async () => await importRepositories(),
		onError: (e) => {
			console.error(e);
			toastThrownError(e);
		},
	});

	const processDeepLink = useCallback(async function processDeepLink() {
		const data = await commands.deepLinkTakeAddRepository();
		if (data == null) return;
		await addRepository(data.url, data.headers);
	}, []);

	const hiddenUserRepos = useMemo(
		() => new Set(result.data?.hidden_user_repositories),
		[result.data?.hidden_user_repositories],
	);

	useTauriListen<null>("deep-link-add-repository", (_) => {
		void processDeepLink();
	});

	// biome-ignore lint/correctness/useExhaustiveDependencies: we want to do on mount
	useEffect(() => {
		void processDeepLink();
		// Only for initial load
		// eslint-disable-next-line react-hooks/exhaustive-deps
	}, []);

	const bodyAnimation = usePrevPathName().startsWith("/packages")
		? "slide-right"
		: "";

	return (
		<VStack>
			<HNavBar
				className="shrink-0"
				leading={<HeadingPageName pageType={"/packages/repositories"} />}
				trailing={
					<DropdownMenu>
						<div className={"flex divide-x"}>
							<Button
								className={"rounded-r-none compact:h-10"}
								onClick={() => openAddRepositoryDialog()}
							>
								{tc("vpm repositories:button:add repository")}
							</Button>
							<DropdownMenuTrigger
								asChild
								className={"rounded-l-none pl-2 pr-2 compact:h-10"}
							>
								<Button>
									<ChevronDown className={"w-4 h-4"} />
								</Button>
							</DropdownMenuTrigger>
						</div>
						<DropdownMenuContent>
							<DropdownMenuItem
								onClick={() => importRepositoriesMutation.mutate()}
							>
								{tc("vpm repositories:button:import repositories")}
							</DropdownMenuItem>
							<DropdownMenuItem onClick={() => exportRepositories.mutate()}>
								{tc("vpm repositories:button:export repositories")}
							</DropdownMenuItem>
						</DropdownMenuContent>
					</DropdownMenu>
				}
			/>
			<main
				className={`shrink overflow-hidden flex w-full h-full ${bodyAnimation}`}
			>
				<ScrollableCardTable className={"h-full w-full"}>
					<RepositoryTableBody
						userRepos={result.data?.user_repositories || []}
						hiddenUserRepos={hiddenUserRepos}
					/>
				</ScrollableCardTable>
			</main>
		</VStack>
	);
}

function RepositoryTableBody({
	userRepos,
	hiddenUserRepos,
}: {
	userRepos: TauriUserRepository[];
	hiddenUserRepos: Set<string>;
}) {
	const TABLE_HEAD = [
		"", // checkbox
		"general:name",
		"vpm repositories:url",
		"", // actions
	];

	return (
		<>
			<thead>
				<tr>
					{TABLE_HEAD.map((head, index) => (
						<th
							// biome-ignore lint/suspicious/noArrayIndexKey: static array
							key={index}
							className={
								"sticky top-0 z-10 border-b border-primary bg-secondary text-secondary-foreground px-2.5 py-1.5"
							}
						>
							<small className="font-normal leading-none">{tc(head)}</small>
						</th>
					))}
				</tr>
			</thead>
			<tbody>
				<RepositoryRow
					repoId={"com.vrchat.repos.official"}
					url={"https://packages.vrchat.com/official?download"}
					displayName={tt("vpm repositories:source:official")}
					hiddenUserRepos={hiddenUserRepos}
					canRemove={false}
				/>
				<RepositoryRow
					repoId={"com.vrchat.repos.curated"}
					url={"https://packages.vrchat.com/curated?download"}
					displayName={tt("vpm repositories:source:curated")}
					hiddenUserRepos={hiddenUserRepos}
					className={"border-b border-primary/10"}
					canRemove={false}
				/>
				{userRepos.map((repo) => (
					<RepositoryRow
						key={repo.id}
						repoId={repo.id}
						displayName={repo.display_name}
						url={repo.url}
						hiddenUserRepos={hiddenUserRepos}
					/>
				))}
			</tbody>
		</>
	);
}

function RepositoryRow({
	repoId,
	displayName,
	url,
	hiddenUserRepos,
	className,
	canRemove = true,
}: {
	repoId: TauriUserRepository["id"];
	displayName: TauriUserRepository["display_name"];
	url: TauriUserRepository["url"];
	hiddenUserRepos: Set<string>;
	className?: string;
	canRemove?: boolean;
}) {
	const cellClass = "p-2.5 compact:py-1";
	const id = useId();

	const queryClient = useQueryClient();
	const setHideRepository = useMutation({
		mutationFn: async ({ id, shown }: { id: string; shown: boolean }) => {
			if (shown) {
				await commands.environmentShowRepository(id);
			} else {
				await commands.environmentHideRepository(id);
			}
		},
		onMutate: async ({ id, shown }: { id: string; shown: boolean }) => {
			await queryClient.cancelQueries(environmentRepositoriesInfo);
			const data = queryClient.getQueryData(
				environmentRepositoriesInfo.queryKey,
			);
			if (data !== undefined) {
				let hidden_user_repositories: string[];
				if (shown) {
					if (data.hidden_user_repositories.includes(id)) {
						hidden_user_repositories = data.hidden_user_repositories;
					} else {
						hidden_user_repositories = [...data.hidden_user_repositories, id];
					}
				} else {
					hidden_user_repositories = data.hidden_user_repositories.filter(
						(x) => x !== id,
					);
				}

				queryClient.setQueryData(environmentRepositoriesInfo.queryKey, {
					...data,
					hidden_user_repositories,
				});
			}
			return data;
		},
		onError: (e, _, ctx) => {
			reportError(e);
			console.error(e);
			queryClient.setQueryData(environmentRepositoriesInfo.queryKey, ctx);
		},
		onSettled: async () => {
			await queryClient.invalidateQueries(environmentRepositoriesInfo);
		},
	});

	const selected = !hiddenUserRepos.has(repoId);

	return (
		<tr className={cn("even:bg-secondary/30", className)}>
			<td className={cellClass}>
				<Checkbox
					id={id}
					checked={selected}
					onCheckedChange={(x) =>
						setHideRepository.mutate({ id: repoId, shown: x === true })
					}
				/>
			</td>
			<td className={cellClass}>
				<label htmlFor={id}>
					<p className="font-normal">{displayName}</p>
				</label>
			</td>
			<td className={cellClass}>
				<p className="font-normal">{url}</p>
			</td>
			<td className={`${cellClass} w-0`}>
				<Tooltip>
					<TooltipTrigger asChild={canRemove}>
						<Button
							disabled={!canRemove}
							onClick={() => {
								void openSingleDialog(RemoveRepositoryDialog, {
									displayName,
									id: repoId,
								});
							}}
							variant={"ghost"}
							size={"icon"}
						>
							<CircleX className={"size-5 text-destructive"} />
						</Button>
					</TooltipTrigger>
					<TooltipContent>
						{canRemove
							? tc("vpm repositories:remove repository")
							: tc(
									"vpm repositories:tooltip:remove curated or official repository",
								)}
					</TooltipContent>
				</Tooltip>
			</td>
		</tr>
	);
}

function RemoveRepositoryDialog({
	dialog,
	displayName,
	id,
}: {
	dialog: DialogContext<void>;
	displayName: string;
	id: string;
}) {
	const queryClient = useQueryClient();

	const removeRepository = useMutation({
		mutationFn: async (id: string) =>
			await commands.environmentRemoveRepository(id),
		onMutate: async (id) => {
			await queryClient.cancelQueries(environmentRepositoriesInfo);
			const data = queryClient.getQueryData(
				environmentRepositoriesInfo.queryKey,
			);
			if (data !== undefined) {
				queryClient.setQueryData(environmentRepositoriesInfo.queryKey, {
					...data,
					user_repositories: data.user_repositories.filter((x) => x.id !== id),
				});
			}
		},
	});

	return (
		<>
			<DialogTitle>{tc("vpm repositories:remove repository")}</DialogTitle>
			<DialogDescription>
				<p className={"whitespace-normal font-normal"}>
					{tc("vpm repositories:dialog:confirm remove description", {
						name: displayName,
					})}
				</p>
			</DialogDescription>
			<DialogFooter>
				<Button onClick={() => dialog.close()}>
					{tc("general:button:cancel")}
				</Button>
				<Button
					onClick={() => {
						dialog.close();
						removeRepository.mutate(id);
					}}
					className={"ml-2"}
				>
					{tc("vpm repositories:remove repository")}
				</Button>
			</DialogFooter>
		</>
	);
}
