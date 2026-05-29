"use client";

import {
	closestCenter,
	DndContext,
	type DragEndEvent,
	PointerSensor,
	useSensor,
	useSensors,
} from "@dnd-kit/core";
import {
	arrayMove,
	SortableContext,
	useSortable,
	verticalListSortingStrategy,
} from "@dnd-kit/sortable";
import {
	queryOptions,
	useMutation,
	useQuery,
	useQueryClient,
} from "@tanstack/react-query";
import { createFileRoute } from "@tanstack/react-router";
import { ChevronDown, CircleX, GripVertical } from "lucide-react";
import {
	Suspense,
	useCallback,
	useEffect,
	useId,
	useMemo,
	useState,
} from "react";
import { HNavBar, VStack } from "@/components/layout";
import { ScrollableCardTable } from "@/components/ScrollableCardTable";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { DialogFooter, DialogTitle } from "@/components/ui/dialog";
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

	const userRepos = result.data?.user_repositories;

	const [orderedIds, setOrderedIds] = useState<string[]>(
		() => userRepos?.map((r) => r.id) ?? [],
	);

	useEffect(() => {
		setOrderedIds(userRepos?.map((r) => r.id) ?? []);
	}, [userRepos]);

	const userRepoMap = useMemo(
		() => new Map((userRepos ?? []).map((r) => [r.id, r])),
		[userRepos],
	);

	const sensors = useSensors(useSensor(PointerSensor));

	const queryClient = useQueryClient();
	const reorderMutation = useMutation({
		mutationFn: (ids: string[]) => commands.environmentReorderRepositories(ids),
		onSettled: () => queryClient.invalidateQueries(environmentRepositoriesInfo),
		onError: (e) => {
			toastThrownError(e);
		},
	});

	function handleDragEnd(event: DragEndEvent) {
		const { active, over } = event;
		if (!over || active.id === over.id) return;
		const oldIndex = orderedIds.indexOf(active.id as string);
		const newIndex = orderedIds.indexOf(over.id as string);
		const newIds = arrayMove(orderedIds, oldIndex, newIndex);
		setOrderedIds(newIds);
		reorderMutation.mutate(newIds);
	}

	const bodyAnimation = usePrevPathName().startsWith("/packages")
		? "slide-right"
		: "";

	return (
		<DndContext
			sensors={sensors}
			collisionDetection={closestCenter}
			onDragEnd={handleDragEnd}
		>
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
							orderedIds={orderedIds}
							userRepoMap={userRepoMap}
							hiddenUserRepos={hiddenUserRepos}
						/>
					</ScrollableCardTable>
				</main>
			</VStack>
		</DndContext>
	);
}

function RepositoryTableBody({
	orderedIds,
	userRepoMap,
	hiddenUserRepos,
}: {
	orderedIds: string[];
	userRepoMap: Map<string, TauriUserRepository>;
	hiddenUserRepos: Set<string>;
}) {
	const TABLE_HEAD = [
		"", // checkbox
		"general:name",
		"vpm repositories:url",
		"", // actions
		"", // grip handle
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
				<SortableContext
					items={orderedIds}
					strategy={verticalListSortingStrategy}
				>
					{orderedIds.map((id) => {
						const repo = userRepoMap.get(id);
						if (!repo) return null;
						return (
							<RepositoryRow
								key={repo.id}
								repoId={repo.id}
								displayName={repo.display_name}
								url={repo.url}
								hiddenUserRepos={hiddenUserRepos}
							/>
						);
					})}
				</SortableContext>
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

	const {
		attributes,
		listeners,
		setNodeRef,
		transform,
		transition,
		isDragging,
	} = useSortable({ id: repoId, disabled: !canRemove });

	const dragStyle: React.CSSProperties = {
		transform: transform ? `translateY(${transform.y}px)` : undefined,
		transition,
		opacity: isDragging ? 0.5 : 1,
		position: "relative",
	};

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
		<tr
			ref={setNodeRef}
			style={dragStyle}
			className={cn("even:bg-secondary/30", className)}
		>
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
			<td
				className={`${cellClass} w-0 ${canRemove ? "cursor-move" : "cursor-not-allowed"}`}
				{...(canRemove ? { ...listeners, ...attributes } : {})}
			>
				<GripVertical
					className={`size-5 text-muted-foreground${canRemove ? "" : " opacity-50"}`}
				/>
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
			<div>
				<p className={"whitespace-normal font-normal"}>
					{tc("vpm repositories:dialog:confirm remove description", {
						name: displayName,
					})}
				</p>
			</div>
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
