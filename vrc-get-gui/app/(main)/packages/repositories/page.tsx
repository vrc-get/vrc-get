"use client";

import { ScrollableCardTable } from "@/components/ScrollableCardTable";
import { HNavBar, VStack } from "@/components/layout";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
	DialogDescription,
	DialogFooter,
	DialogOpen,
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
import {
	type TauriUserRepository,
	deepLinkTakeAddRepository,
	environmentExportRepositories,
	environmentHideRepository,
	environmentRemoveRepository,
	environmentRepositoriesInfo,
	environmentShowRepository,
} from "@/lib/bindings";
import { tc } from "@/lib/i18n";
import { toastThrownError } from "@/lib/toast";
import { useFilePickerFunction } from "@/lib/use-file-picker-dialog";
import { useTauriListen } from "@/lib/use-tauri-listen";
import { useQuery } from "@tanstack/react-query";
import { ChevronDown, CircleX } from "lucide-react";
import type React from "react";
import {
	Suspense,
	useCallback,
	useEffect,
	useId,
	useMemo,
	useState,
} from "react";
import { HeadingPageName } from "../tab-selector";
import { useAddRepository } from "./use-add-repository";
import { useImportRepositories } from "./use-import-repositories";

export default function Page() {
	return (
		<Suspense>
			<PageBody />
		</Suspense>
	);
}

function PageBody() {
	const result = useQuery({
		queryKey: ["environmentRepositoriesInfo"],
		queryFn: environmentRepositoriesInfo,
	});

	const addRepositoryInfo = useAddRepository({
		refetch: () => result.refetch(),
	});

	const importRepositoryInfo = useImportRepositories({
		refetch: () => result.refetch(),
	});

	const [exportRepositoriesRaw, exportDialog] = useFilePickerFunction(
		environmentExportRepositories,
	);

	const exportRepositories = useCallback(async () => {
		try {
			await exportRepositoriesRaw();
		} catch (e) {
			toastThrownError(e);
		}
	}, [exportRepositoriesRaw]);

	const hiddenUserRepos = useMemo(
		() => new Set(result.data?.hidden_user_repositories),
		[result],
	);

	async function removeRepository(id: string) {
		try {
			await environmentRemoveRepository(id);
			await result.refetch();
		} catch (e) {
			toastThrownError(e);
		}
	}

	const addRepository = addRepositoryInfo.addRepository;
	const processDeepLink = useCallback(
		async function processDeepLink() {
			const data = await deepLinkTakeAddRepository();
			if (data == null) return;
			await addRepository(data.url, data.headers);
		},
		[addRepository],
	);

	useTauriListen<null>(
		"deep-link-add-repository",
		useCallback(
			(_) => {
				// noinspection JSIgnoredPromiseFromCall
				processDeepLink();
			},
			[processDeepLink],
		),
	);

	// biome-ignore lint/correctness/useExhaustiveDependencies: we want to do on mount
	useEffect(() => {
		// noinspection JSIgnoredPromiseFromCall
		processDeepLink();
		// Only for initial load
		// eslint-disable-next-line react-hooks/exhaustive-deps
	}, []);

	return (
		<VStack>
			<HNavBar className={"flex-shrink-0"}>
				<HeadingPageName pageType={"/packages/repositories"} />
				<div className={"flex-grow"} />
				<DropdownMenu>
					<div className={"flex divide-x"}>
						<Button
							className={"rounded-r-none"}
							onClick={addRepositoryInfo.openAddDialog}
						>
							{tc("vpm repositories:button:add repository")}
						</Button>
						<DropdownMenuTrigger asChild className={"rounded-l-none pl-2 pr-2"}>
							<Button>
								<ChevronDown className={"w-4 h-4"} />
							</Button>
						</DropdownMenuTrigger>
					</div>
					<DropdownMenuContent>
						<DropdownMenuItem
							onClick={importRepositoryInfo.startImportingRepositories}
						>
							{tc("vpm repositories:button:import repositories")}
						</DropdownMenuItem>
						<DropdownMenuItem onClick={exportRepositories}>
							{tc("vpm repositories:button:export repositories")}
						</DropdownMenuItem>
					</DropdownMenuContent>
				</DropdownMenu>
			</HNavBar>
			<ScrollableCardTable>
				<RepositoryTableBody
					userRepos={result.data?.user_repositories || []}
					hiddenUserRepos={hiddenUserRepos}
					removeRepository={removeRepository}
					refetch={() => result.refetch()}
				/>
			</ScrollableCardTable>
			{addRepositoryInfo.dialog}
			{importRepositoryInfo.dialog}
			{exportDialog}
		</VStack>
	);
}

function RepositoryTableBody({
	userRepos,
	hiddenUserRepos,
	removeRepository,
	refetch,
}: {
	userRepos: TauriUserRepository[];
	hiddenUserRepos: Set<string>;
	removeRepository: (id: string) => void;
	refetch: () => void;
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
								"sticky top-0 z-10 border-b border-primary bg-secondary text-secondary-foreground p-2.5"
							}
						>
							<small className="font-normal leading-none">{tc(head)}</small>
						</th>
					))}
				</tr>
			</thead>
			<tbody>
				{userRepos.map((repo) => (
					<RepositoryRow
						key={repo.id}
						repo={repo}
						hiddenUserRepos={hiddenUserRepos}
						remove={() => removeRepository(repo.id)}
						refetch={refetch}
					/>
				))}
			</tbody>
		</>
	);
}

function RepositoryRow({
	repo,
	hiddenUserRepos,
	remove,
	refetch,
}: {
	repo: TauriUserRepository;
	hiddenUserRepos: Set<string>;
	remove: () => void;
	refetch: () => void;
}) {
	const cellClass = "p-2.5";
	const id = useId();

	const [removeDialogOpen, setRemoveDialogOpen] = useState(false);

	const selected = !hiddenUserRepos.has(repo.id);
	const onChange = () => {
		if (selected) {
			environmentHideRepository(repo.id).then(refetch);
		} else {
			environmentShowRepository(repo.id).then(refetch);
		}
	};

	let dialog: React.ReactNode;
	if (removeDialogOpen) {
		dialog = (
			<DialogOpen>
				<DialogTitle>{tc("vpm repositories:remove repository")}</DialogTitle>
				<DialogDescription>
					<p className={"whitespace-normal font-normal"}>
						{tc("vpm repositories:dialog:confirm remove description", {
							name: repo.display_name,
						})}
					</p>
				</DialogDescription>
				<DialogFooter>
					<Button onClick={() => setRemoveDialogOpen(false)}>
						{tc("general:button:cancel")}
					</Button>
					<Button
						onClick={() => {
							remove();
							setRemoveDialogOpen(false);
						}}
						className={"ml-2"}
					>
						{tc("vpm repositories:remove repository")}
					</Button>
				</DialogFooter>
			</DialogOpen>
		);
	}

	return (
		<tr className="even:bg-secondary/30">
			<td className={cellClass}>
				<Checkbox id={id} checked={selected} onCheckedChange={onChange} />
			</td>
			<td className={cellClass}>
				<label htmlFor={id}>
					<p className="font-normal">{repo.display_name}</p>
				</label>
			</td>
			<td className={cellClass}>
				<p className="font-normal">{repo.url}</p>
			</td>
			<td className={`${cellClass} w-0`}>
				<Tooltip>
					<TooltipTrigger asChild>
						<Button
							onClick={() => setRemoveDialogOpen(true)}
							variant={"ghost"}
							size={"icon"}
						>
							<CircleX className={"size-5 text-destructive"} />
						</Button>
					</TooltipTrigger>
					<TooltipContent>
						{tc("vpm repositories:remove repository")}
					</TooltipContent>
				</Tooltip>
			</td>
			{dialog}
		</tr>
	);
}
