"use client"

import {Button} from "@/components/ui/button";
import {Checkbox} from "@/components/ui/checkbox";
import {DialogDescription, DialogFooter, DialogOpen, DialogTitle} from "@/components/ui/dialog";
import {Tooltip, TooltipContent, TooltipTrigger} from "@/components/ui/tooltip";
import {useQuery} from "@tanstack/react-query";
import {
	deepLinkTakeAddRepository,
	environmentHideRepository,
	environmentRemoveRepository,
	environmentRepositoriesInfo,
	environmentShowRepository,
	TauriUserRepository
} from "@/lib/bindings";
import {HNavBar, VStack} from "@/components/layout";
import React, {Suspense, useCallback, useEffect, useId, useMemo, useState} from "react";
import {CircleX} from "lucide-react"; 
import {toastThrownError} from "@/lib/toast";
import {tc} from "@/lib/i18n";
import {useTauriListen} from "@/lib/use-tauri-listen";
import {ScrollableCardTable} from "@/components/ScrollableCardTable";
import {useAddRepository} from "@/app/repositories/use-add-repository";

export default function Page(props: {}) {
	return <Suspense><PageBody {...props}/></Suspense>
}

function PageBody() {
	const result = useQuery({
		queryKey: ["environmentRepositoriesInfo"],
		queryFn: environmentRepositoriesInfo,
	})

	const addRepositoryInfo = useAddRepository({
		refetch: () => result.refetch(),
	});

	const hiddenUserRepos = useMemo(() => new Set(result.data?.hidden_user_repositories), [result]);

	async function removeRepository(id: string) {
		try {
			await environmentRemoveRepository(id);
			await result.refetch();
		} catch (e) {
			toastThrownError(e);
		}
	}

	const addRepository = addRepositoryInfo.addRepository;
	const processDeepLink = useCallback(async function processDeepLink() {
		const data = await deepLinkTakeAddRepository();
		if (data == null) return;
		await addRepository(data.url, data.headers);
	}, [addRepository]);

	useTauriListen<null>("deep-link-add-repository", useCallback((_) => {
		// noinspection JSIgnoredPromiseFromCall
		processDeepLink()
	}, [processDeepLink]));

	useEffect(() => {
		// noinspection JSIgnoredPromiseFromCall
		processDeepLink()
		// Only for initial load
		// eslint-disable-next-line react-hooks/exhaustive-deps
	}, []);

	return (
		<VStack className={"p-4 overflow-hidden"}>
			<HNavBar className={"flex-shrink-0"}>
				<p className="cursor-pointer py-1.5 font-bold flex-grow-0">
					{tc("vpm repositories:community repositories")}
				</p>
				<Button onClick={addRepositoryInfo.openAddDialog}>{tc("vpm repositories:button:add repository")}</Button>
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
		</VStack>
	);
}

function RepositoryTableBody(
	{
		userRepos,
		hiddenUserRepos,
		removeRepository,
		refetch,
	}: {
		userRepos: TauriUserRepository[],
		hiddenUserRepos: Set<string>,
		removeRepository: (id: string) => void,
		refetch: () => void,
	}
) {
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
					<th key={index}
							className={`sticky top-0 z-10 border-b border-primary bg-secondary text-secondary-foreground p-2.5`}>
						<small className="font-normal leading-none">{tc(head)}</small>
					</th>
				))}
			</tr>
			</thead>
			<tbody>
			{
				userRepos.map((repo) =>
					<RepositoryRow
						key={repo.id}
						repo={repo}
						hiddenUserRepos={hiddenUserRepos}
						remove={() => removeRepository(repo.id)}
						refetch={refetch}
					/>)
			}
			</tbody>
		</>
	);
}

function RepositoryRow(
	{
		repo,
		hiddenUserRepos,
		remove,
		refetch,
	}: {
		repo: TauriUserRepository,
		hiddenUserRepos: Set<string>,
		remove: () => void,
		refetch: () => void,
	}
) {
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
	}

	let dialog;
	if (removeDialogOpen) {
		dialog = <DialogOpen>
			<DialogTitle>{tc("vpm repositories:remove repository")}</DialogTitle>
			<DialogDescription>
				<p className={"whitespace-normal font-normal"}>
					{tc("vpm repositories:dialog:confirm remove description", {name: repo.display_name})}
				</p>
			</DialogDescription>
			<DialogFooter>
				<Button onClick={() => setRemoveDialogOpen(false)}>{tc("general:button:cancel")}</Button>
				<Button onClick={() => {
					remove();
					setRemoveDialogOpen(false);
				}} className={"ml-2"}>{tc("vpm repositories:remove repository")}</Button>
			</DialogFooter>
		</DialogOpen>;
	}

	return (
		<tr className="even:bg-secondary/30">
			<td className={cellClass}>
				<Checkbox id={id}
									checked={selected} onCheckedChange={onChange}/>
			</td>
			<td className={cellClass}>
				<label htmlFor={id}>
					<p className="font-normal">
						{repo.display_name}
					</p>
				</label>
			</td>
			<td className={cellClass}>
				<p className="font-normal">
					{repo.url}
				</p>
			</td>
			<td className={`${cellClass} w-0`}>
				<Tooltip>
					<TooltipTrigger asChild>
						<Button onClick={() => setRemoveDialogOpen(true)} variant={"ghost"} size={"icon"}>
							<CircleX className={"size-5 text-destructive"}/>
						</Button>
					</TooltipTrigger>
					<TooltipContent>{tc("vpm repositories:remove repository")}</TooltipContent>
				</Tooltip>
			</td>
			{dialog}
		</tr>
	)
}
