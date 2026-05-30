"use client";

import {
	type CollisionDetection,
	closestCenter,
	DndContext,
	type DragEndEvent,
	type DragOverEvent,
	DragOverlay,
	type DragStartEvent,
	defaultDropAnimation,
	type Modifier,
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
	useRef,
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

type UserRepoWithListId = TauriUserRepository & { listId: string };

function Page() {
	return (
		<Suspense>
			<PageBody />
		</Suspense>
	);
}

const restrictToVerticalAxis: Modifier = ({ transform }) => ({
	...transform,
	x: 0,
});

const DRAG_OVERLAY_MODIFIERS = [restrictToVerticalAxis];

const TABLE_HEAD = [
	"", // checkbox
	"general:name",
	"vpm repositories:url",
	"", // actions
	"", // grip handle
] as const;

const environmentRepositoriesInfo = queryOptions({
	queryKey: ["environmentRepositoriesInfo"],
	queryFn: commands.environmentRepositoriesInfo,
});

// Scrolls the given viewport element when the pointer is near the top or bottom
// edge during drag. dnd-kit's built-in autoscroll is disabled because it causes
// jitter with Radix UI ScrollArea (wrong container detection + double-smoothing).
function useDragAutoScroll(
	viewportRef: React.RefObject<HTMLElement | null>,
	isActive: boolean,
): void {
	useEffect(() => {
		if (!isActive) return;

		const THRESHOLD = 80; // px from edge to begin scrolling
		const MAX_SPEED = 15; // px/frame at the very edge

		let pointerY = 0;
		const onPointerMove = (e: PointerEvent) => {
			pointerY = e.clientY;
		};
		window.addEventListener("pointermove", onPointerMove, { passive: true });

		let rafId: number;
		const tick = () => {
			const viewport = viewportRef.current;
			if (viewport) {
				const { top, bottom } = viewport.getBoundingClientRect();
				const distFromTop = pointerY - top;
				const distFromBottom = bottom - pointerY;

				let delta = 0;
				if (distFromTop >= 0 && distFromTop < THRESHOLD) {
					delta = -MAX_SPEED * (1 - distFromTop / THRESHOLD);
				} else if (distFromBottom >= 0 && distFromBottom < THRESHOLD) {
					delta = MAX_SPEED * (1 - distFromBottom / THRESHOLD);
				}

				if (delta !== 0) {
					viewport.scrollTo({
						top: viewport.scrollTop + delta,
						behavior: "instant",
					});
				}
			}
			rafId = requestAnimationFrame(tick);
		};
		rafId = requestAnimationFrame(tick);

		return () => {
			window.removeEventListener("pointermove", onPointerMove);
			cancelAnimationFrame(rafId);
		};
	}, [isActive, viewportRef]);
}

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

	const guiAnimation = useQuery({
		queryKey: ["environmentGuiAnimation"],
		queryFn: commands.environmentGuiAnimation,
		initialData: true,
	}).data;

	const userRepos = result.data?.user_repositories;

	const augmentedUserRepos = useMemo<UserRepoWithListId[]>(
		() => (userRepos ?? []).map((r) => ({ ...r, listId: String(r.index) })),
		[userRepos],
	);

	const [orderedListIds, setOrderedListIds] = useState<string[]>(() =>
		augmentedUserRepos.map((r) => r.listId),
	);

	useEffect(() => {
		setOrderedListIds(augmentedUserRepos.map((r) => r.listId));
	}, [augmentedUserRepos]);

	const userRepoByListId = useMemo(
		() => new Map(augmentedUserRepos.map((r) => [r.listId, r])),
		[augmentedUserRepos],
	);

	const [activeId, setActiveId] = useState<string | null>(null);
	const [overId, setOverId] = useState<string | null>(null);
	const [droppingListId, setDroppingListId] = useState<string | null>(null);
	const droppingTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
	const [columnWidths, setColumnWidths] = useState<number[]>([]);
	const theadRowRef = useRef<HTMLTableRowElement>(null);
	const scrollViewportRef = useRef<HTMLDivElement>(null);

	useEffect(() => {
		return () => {
			if (droppingTimerRef.current !== null)
				clearTimeout(droppingTimerRef.current);
		};
	}, []);

	const sensors = useSensors(useSensor(PointerSensor));

	const orderedListIdsSet = useMemo(
		() => new Set(orderedListIds),
		[orderedListIds],
	);

	const collisionDetection = useCallback<CollisionDetection>(
		(args) =>
			closestCenter({
				...args,
				droppableContainers: args.droppableContainers.filter((c) =>
					orderedListIdsSet.has(c.id as string),
				),
			}),
		[orderedListIdsSet],
	);

	const queryClient = useQueryClient();
	const reorderMutation = useMutation({
		mutationFn: (listIds: string[]) => {
			const indices = listIds
				.map((lid) => userRepoByListId.get(lid)?.index)
				.filter((i): i is number => i !== undefined);
			return commands.environmentReorderRepositories(indices);
		},
		onSettled: () => queryClient.invalidateQueries(environmentRepositoriesInfo),
		onError: (e) => {
			toastThrownError(e);
		},
	});

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

	const activeVisualIndex = useMemo(() => {
		if (!activeId) return 0;
		const effectiveId = overId ?? activeId;
		return orderedListIds.indexOf(effectiveId) + 2; // +2 for the 2 fixed rows
	}, [activeId, overId, orderedListIds]);

	function handleDragStart(event: DragStartEvent) {
		setActiveId(event.active.id as string);
		if (theadRowRef.current) {
			const widths = Array.from(
				theadRowRef.current.querySelectorAll("th"),
				(th) => th.getBoundingClientRect().width,
			);
			setColumnWidths(widths);
		}
	}

	function handleDragOver(event: DragOverEvent) {
		setOverId((event.over?.id as string | null) ?? null);
	}

	function handleDragEnd(event: DragEndEvent) {
		const { active, over } = event;
		setActiveId(null);
		setOverId(null);
		if (over && active.id !== over.id) {
			const droppedListId = active.id as string;
			const oldIndex = orderedListIds.indexOf(droppedListId);
			const newIndex = orderedListIds.indexOf(over.id as string);
			const newListIds = arrayMove(orderedListIds, oldIndex, newIndex);
			setOrderedListIds(newListIds);
			reorderMutation.mutate(newListIds);
			if (guiAnimation) {
				setDroppingListId(droppedListId);
				if (droppingTimerRef.current !== null)
					clearTimeout(droppingTimerRef.current);
				droppingTimerRef.current = setTimeout(() => {
					setDroppingListId(null);
					droppingTimerRef.current = null;
				}, defaultDropAnimation.duration ?? 250);
			}
		}
	}

	function handleDragCancel() {
		setActiveId(null);
		setOverId(null);
	}

	useDragAutoScroll(scrollViewportRef, activeId !== null);

	const bodyAnimation = usePrevPathName().startsWith("/packages")
		? "slide-right"
		: "";

	return (
		<DndContext
			sensors={sensors}
			collisionDetection={collisionDetection}
			autoScroll={false}
			onDragStart={handleDragStart}
			onDragOver={handleDragOver}
			onDragEnd={handleDragEnd}
			onDragCancel={handleDragCancel}
		>
			<VStack>
				<div style={activeId !== null ? { pointerEvents: "none" } : undefined}>
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
				</div>
				<main
					className={`shrink overflow-hidden flex w-full h-full ${bodyAnimation}`}
				>
					<ScrollableCardTable
						className={"h-full w-full"}
						viewportRef={scrollViewportRef}
					>
						<RepositoryTableBody
							orderedListIds={orderedListIds}
							userRepoByListId={userRepoByListId}
							hiddenUserRepos={hiddenUserRepos}
							theadRowRef={theadRowRef}
							guiAnimation={guiAnimation}
							onToggleVisibility={(id, shown) =>
								setHideRepository.mutate({ id, shown })
							}
							isDragActive={activeId !== null}
							droppingListId={droppingListId}
						/>
					</ScrollableCardTable>
				</main>
			</VStack>
			<DragOverlay
				modifiers={DRAG_OVERLAY_MODIFIERS}
				dropAnimation={guiAnimation ? defaultDropAnimation : null}
			>
				{activeId ? (
					<RepositoryDragOverlay
						repo={userRepoByListId.get(activeId)}
						selected={
							!hiddenUserRepos.has(userRepoByListId.get(activeId)?.id ?? "")
						}
						columnWidths={columnWidths}
						visualIndex={activeVisualIndex}
						guiAnimation={guiAnimation}
					/>
				) : null}
			</DragOverlay>
		</DndContext>
	);
}

function RepositoryTableBody({
	orderedListIds,
	userRepoByListId,
	hiddenUserRepos,
	theadRowRef,
	guiAnimation,
	onToggleVisibility,
	isDragActive,
	droppingListId,
}: {
	orderedListIds: string[];
	userRepoByListId: Map<string, UserRepoWithListId>;
	hiddenUserRepos: Set<string>;
	theadRowRef: React.RefObject<HTMLTableRowElement | null>;
	guiAnimation: boolean;
	onToggleVisibility: (id: string, shown: boolean) => void;
	isDragActive: boolean;
	droppingListId: string | null;
}) {
	return (
		<>
			<thead>
				<tr ref={theadRowRef}>
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
					rowIndex={0}
					guiAnimation={guiAnimation}
					onToggleVisibility={onToggleVisibility}
					isDragActive={isDragActive}
					droppingListId={droppingListId}
				/>
				<RepositoryRow
					repoId={"com.vrchat.repos.curated"}
					url={"https://packages.vrchat.com/curated?download"}
					displayName={tt("vpm repositories:source:curated")}
					hiddenUserRepos={hiddenUserRepos}
					className={"border-b border-primary/10"}
					canRemove={false}
					rowIndex={1}
					guiAnimation={guiAnimation}
					onToggleVisibility={onToggleVisibility}
					isDragActive={isDragActive}
					droppingListId={droppingListId}
				/>
				<SortableContext
					items={orderedListIds}
					strategy={verticalListSortingStrategy}
				>
					{orderedListIds.map((listId, index) => {
						const repo = userRepoByListId.get(listId);
						if (!repo) return null;
						return (
							<RepositoryRow
								key={listId}
								listId={listId}
								repoId={repo.id}
								repoIndex={repo.index}
								displayName={repo.display_name}
								url={repo.url}
								hiddenUserRepos={hiddenUserRepos}
								rowIndex={2 + index}
								guiAnimation={guiAnimation}
								onToggleVisibility={onToggleVisibility}
								isDragActive={isDragActive}
								droppingListId={droppingListId}
							/>
						);
					})}
				</SortableContext>
			</tbody>
		</>
	);
}

const CELL_CLASS = "p-2.5 compact:py-1 align-middle";

function RepositoryRowCells({
	labelId,
	displayName,
	url,
	canRemove,
	selected,
	onCheckedChange,
	onRemove,
	dragListeners,
	dragAttributes,
}: {
	labelId?: string;
	displayName: string;
	url: string | null | undefined;
	canRemove: boolean;
	selected: boolean;
	onCheckedChange?: (shown: boolean) => void;
	onRemove?: () => void;
	dragListeners?: ReturnType<typeof useSortable>["listeners"];
	dragAttributes?: ReturnType<typeof useSortable>["attributes"];
}) {
	const interactive = onCheckedChange !== undefined;
	return (
		<>
			<td className={CELL_CLASS}>
				{interactive ? (
					<div className="flex">
						<Checkbox
							id={labelId}
							checked={selected}
							onCheckedChange={(x) => onCheckedChange(x === true)}
						/>
					</div>
				) : (
					<div className="pointer-events-none flex">
						<Checkbox checked={selected} />
					</div>
				)}
			</td>
			<td className={CELL_CLASS}>
				{interactive ? (
					<label htmlFor={labelId}>
						<p className="font-normal">{displayName}</p>
					</label>
				) : (
					<p className="font-normal">{displayName}</p>
				)}
			</td>
			<td className={CELL_CLASS}>
				<p className="font-normal">{url}</p>
			</td>
			<td className={`${CELL_CLASS} w-0`}>
				{interactive ? (
					<Tooltip>
						<TooltipTrigger asChild={canRemove}>
							<Button
								disabled={!canRemove}
								onClick={onRemove}
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
				) : (
					<Button variant={"ghost"} size={"icon"} disabled>
						<CircleX className={"size-5 text-destructive"} />
					</Button>
				)}
			</td>
			<td
				className={cn(
					CELL_CLASS,
					"w-0",
					canRemove ? "cursor-move" : "cursor-not-allowed",
				)}
				{...(canRemove ? dragListeners : undefined)}
				{...(canRemove ? dragAttributes : undefined)}
			>
				<GripVertical
					className={cn(
						"size-5 text-muted-foreground",
						!canRemove && "opacity-50",
					)}
				/>
			</td>
		</>
	);
}

function RepositoryRow({
	listId,
	repoId,
	repoIndex,
	displayName,
	url,
	hiddenUserRepos,
	className,
	canRemove = true,
	rowIndex,
	guiAnimation,
	onToggleVisibility,
	isDragActive,
	droppingListId,
}: {
	listId?: string;
	repoId: TauriUserRepository["id"];
	repoIndex?: number;
	displayName: TauriUserRepository["display_name"];
	url: TauriUserRepository["url"];
	hiddenUserRepos: Set<string>;
	className?: string;
	canRemove?: boolean;
	rowIndex: number;
	guiAnimation: boolean;
	onToggleVisibility: (id: string, shown: boolean) => void;
	isDragActive: boolean;
	droppingListId: string | null;
}) {
	const labelId = useId();

	const {
		attributes,
		listeners,
		setNodeRef,
		transform,
		transition,
		isDragging,
	} = useSortable({ id: listId ?? repoId, disabled: !canRemove });

	const dragStyle = useMemo<React.CSSProperties>(
		() => ({
			transform: transform ? `translateY(${transform.y}px)` : undefined,
			transition: guiAnimation
				? [transition, isDragActive ? undefined : "background-color 200ms ease"]
						.filter(Boolean)
						.join(", ") || undefined
				: undefined,
			opacity: isDragging || listId === droppingListId ? 0 : 1,
			position: "relative",
		}),
		[
			transform,
			transition,
			isDragging,
			listId,
			droppingListId,
			guiAnimation,
			isDragActive,
		],
	);

	const selected = !hiddenUserRepos.has(repoId);

	return (
		<tr
			ref={setNodeRef}
			style={dragStyle}
			className={cn(rowIndex % 2 === 1 ? "bg-secondary/30" : "", className)}
		>
			<RepositoryRowCells
				labelId={labelId}
				displayName={displayName}
				url={url}
				canRemove={canRemove}
				selected={selected}
				onCheckedChange={(shown) => onToggleVisibility(repoId, shown)}
				onRemove={() =>
					void openSingleDialog(RemoveRepositoryDialog, {
						displayName,
						index: repoIndex ?? 0,
					})
				}
				dragListeners={listeners}
				dragAttributes={attributes}
			/>
		</tr>
	);
}

function RepositoryDragOverlay({
	repo,
	selected,
	columnWidths,
	visualIndex,
	guiAnimation,
}: {
	repo: TauriUserRepository | undefined;
	selected: boolean;
	columnWidths: number[];
	visualIndex: number;
	guiAnimation: boolean;
}) {
	const style = useMemo<React.CSSProperties>(
		() => ({
			transition: guiAnimation ? "background-color 200ms ease" : undefined,
		}),
		[guiAnimation],
	);

	if (!repo) return null;
	return (
		<table
			className={cn(
				"w-full table-fixed text-left",
				visualIndex % 2 === 1 ? "bg-secondary/30" : "",
			)}
			style={style}
		>
			{columnWidths.length > 0 && (
				<colgroup>
					{columnWidths.map((w, i) => (
						// biome-ignore lint/suspicious/noArrayIndexKey: fixed column order
						<col key={i} style={{ width: w }} />
					))}
				</colgroup>
			)}
			<tbody>
				<tr>
					<RepositoryRowCells
						displayName={repo.display_name}
						url={repo.url}
						canRemove={true}
						selected={selected}
					/>
				</tr>
			</tbody>
		</table>
	);
}

function RemoveRepositoryDialog({
	dialog,
	displayName,
	index,
}: {
	dialog: DialogContext<void>;
	displayName: string;
	index: number;
}) {
	const queryClient = useQueryClient();

	const removeRepository = useMutation({
		mutationFn: async (index: number) =>
			await commands.environmentRemoveRepository(index),
		onMutate: async (index) => {
			await queryClient.cancelQueries(environmentRepositoriesInfo);
			const data = queryClient.getQueryData(
				environmentRepositoriesInfo.queryKey,
			);
			if (data !== undefined) {
				queryClient.setQueryData(environmentRepositoriesInfo.queryKey, {
					...data,
					user_repositories: data.user_repositories.filter(
						(x) => x.index !== index,
					),
				});
			}
		},
		onSettled: () => queryClient.invalidateQueries(environmentRepositoriesInfo),
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
						removeRepository.mutate(index);
					}}
					className={"ml-2"}
				>
					{tc("vpm repositories:remove repository")}
				</Button>
			</DialogFooter>
		</>
	);
}
