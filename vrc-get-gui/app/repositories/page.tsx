"use client"

import {
	Button,
	Card,
	Checkbox,
	Dialog,
	DialogBody,
	DialogFooter,
	DialogHeader,
	IconButton,
	Input,
	List,
	ListItem,
	Tooltip,
	Typography
} from "@material-tailwind/react";
import {useQuery} from "@tanstack/react-query";
import {
	environmentAddRepository,
	environmentDownloadRepository,
	environmentHideRepository,
	environmentRemoveRepository,
	environmentRepositoriesInfo,
	environmentShowRepository,
	TauriRemoteRepositoryInfo,
	TauriUserRepository
} from "@/lib/bindings";
import {HNavBar, VStack} from "@/components/layout";
import React, {Suspense, useMemo, useState} from "react";
import {XCircleIcon} from "@heroicons/react/24/outline";
import {nop} from "@/lib/nop";
import {toastError, toastSuccess, toastThrownError} from "@/lib/toast";
import {tc, tt} from "@/lib/i18n";

export default function Page(props: {}) {
	return <Suspense><PageBody {...props}/></Suspense>
}

type State = {
	type: 'normal',
} | {
	type: 'enteringRepositoryInfo',
} | {
	type: 'loadingRepository',
} | {
	type: 'duplicated',
} | {
	type: 'confirming',
	repo: TauriRemoteRepositoryInfo,
	url: string,
	headers: { [key: string]: string },
}

function PageBody() {
	const [state, setState] = useState<State>({type: 'normal'});

	const result = useQuery({
		queryKey: ["environmentRepositoriesInfo"],
		queryFn: environmentRepositoriesInfo,
	})

	const hiddenUserRepos = useMemo(() => new Set(result.data?.hidden_user_repositories), [result]);

	function cancel() {
		setState({type: 'normal'});
	}

	async function addRepository(url: string, headers: { [key: string]: string }) {
		try {
			setState({type: 'loadingRepository'});
			const info = await environmentDownloadRepository(url, headers);
			switch (info.type) {
				case "BadUrl":
					toastError(tt("invalid url"));
					setState({type: 'normal'});
					return;
				case "DownloadError":
					toastError(tt("failed to download the repository: {{message}}", {message: info.message}));
					setState({type: 'normal'});
					return;
				case "Duplicated":
					setState({type: 'duplicated'});
					return;
				case "Success":
					break;
				default:
					const _exhaustiveCheck: never = info;
			}
			setState({type: 'confirming', repo: info.value, url, headers})
		} catch (e) {
			toastThrownError(e);
			setState({type: 'normal'});
		}
	}

	async function removeRepository(id: string) {
		try {
			await environmentRemoveRepository(id);
			await result.refetch();
		} catch (e) {
			toastThrownError(e);
		}
	}

	let dialogBody;
	switch (state.type) {
		case "normal":
			dialogBody = null;
			break;
		case "enteringRepositoryInfo":
			dialogBody = <EnteringRepositoryInfo
				cancel={cancel}
				addRepository={url => addRepository(url, {})}
			/>;
			break;
		case "loadingRepository":
			dialogBody = <LoadingRepository cancel={cancel}/>;
			break;
		case "duplicated":
			dialogBody = <Duplicated cancel={cancel}/>;
			break
		case "confirming":
			const doAddRepository = async () => {
				try {
					await environmentAddRepository(state.url, state.headers);
					setState({type: 'normal'});
					toastSuccess(tt("added the repository successfully!"));
					// noinspection ES6MissingAwait
					result.refetch();
				} catch (e) {
					toastThrownError(e);
					setState({type: 'normal'});
				}
			}
			dialogBody = <Confirming repo={state.repo} cancel={cancel} add={doAddRepository}/>;
			break;
		default:
			const _exhaustiveCheck: never = state;
	}
	const dialog = dialogBody ?
		<Dialog handler={nop} open><DialogHeader>{tc("add repository")}</DialogHeader>{dialogBody}</Dialog> : null;

	return (
		<VStack className={"p-4 overflow-y-auto"}>
			<HNavBar className={"flex-shrink-0"}>
				<Typography className="cursor-pointer py-1.5 font-bold flex-grow-0">
					{tc("community repositories")}
				</Typography>
				<Button onClick={() => setState({type: 'enteringRepositoryInfo'})}>{tc("add repository")}</Button>
			</HNavBar>
			<main className="flex-shrink flex-grow overflow-hidden flex">
				<Card className="w-full overflow-x-auto overflow-y-scroll shadow-none">
					<RepositoryTable
						userRepos={result.data?.user_repositories || []}
						hiddenUserRepos={hiddenUserRepos}
						removeRepository={removeRepository}
						refetch={() => result.refetch()}
					/>
					{dialog}
				</Card>
			</main>
		</VStack>
	);
}

function RepositoryTable(
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
		"name",
		"url",
		"", // actions
	];

	return (
		<table className="relative table-auto text-left">
			<thead>
			<tr>
				{TABLE_HEAD.map((head, index) => (
					<th key={index}
							className={`sticky top-0 z-10 border-b border-blue-gray-100 bg-blue-gray-50 p-2.5`}>
						<Typography variant="small" className="font-normal leading-none">{tc(head)}</Typography>
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
		</table>
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
	const id = `repository-${repo.id}`;

	const selected = !hiddenUserRepos.has(repo.id);
	const onChange = () => {
		if (selected) {
			environmentHideRepository(repo.id).then(refetch);
		} else {
			environmentShowRepository(repo.id).then(refetch);
		}
	}

	return (
		<tr className="even:bg-blue-gray-50/50">
			<td className={cellClass}>
				<Checkbox ripple={false} containerProps={{className: "p-0 rounded-none"}} id={id}
									checked={selected} onChange={onChange}/>
			</td>
			<td className={cellClass}>
				<label htmlFor={id}>
					<Typography className="font-normal">
						{repo.display_name}
					</Typography>
				</label>
			</td>
			<td className={cellClass}>
				<Typography className="font-normal">
					{repo.url}
				</Typography>
			</td>
			<td className={`${cellClass} w-0`}>
				<Tooltip content={tc("remove repository")}>
					<IconButton onClick={remove} variant={"text"}>
						<XCircleIcon className={"size-5 text-red-700"}/>
					</IconButton>
				</Tooltip>
			</td>
		</tr>
	)
}

function EnteringRepositoryInfo(
	{
		cancel,
		addRepository,
	}: {
		cancel: () => void,
		addRepository: (url: string) => void,
	}
) {
	const [url, setUrl] = useState("");

	return (
		<>
			<DialogBody>
				<Typography>
					{tc("enter information about the repository")}
				</Typography>
				<Input type={"url"} label={"URL"} value={url} onChange={e => setUrl(e.target.value)}
							 placeholder={"https://vpm.anatawa12.com/vpm.json"}></Input>
				{/* TODO: headers */}
			</DialogBody>
			<DialogFooter>
				<Button onClick={cancel}>{tc("cancel")}</Button>
				<Button onClick={() => addRepository(url)} className={"ml-2"}>{tc("add repository")}</Button>
			</DialogFooter>
		</>
	);
}

function LoadingRepository(
	{
		cancel,
	}: {
		cancel: () => void,
	}
) {
	return (
		<>
			<DialogBody>
				<Typography>
					{tc("downloading the repository")}
				</Typography>
			</DialogBody>
			<DialogFooter>
				<Button onClick={cancel}>{tc("cancel")}</Button>
			</DialogFooter>
		</>
	);
}

function Duplicated(
	{
		cancel,
	}: {
		cancel: () => void,
	}
) {
	return (
		<>
			<DialogBody>
				<Typography>
					{tc("the repository is already added.")}
				</Typography>
			</DialogBody>
			<DialogFooter>
				<Button onClick={cancel}>{tc("ok")}</Button>
			</DialogFooter>
		</>
	);
}

function Confirming(
	{
		repo,
		cancel,
		add,
	}: {
		repo: TauriRemoteRepositoryInfo,
		cancel: () => void,
		add: () => void,
	}
) {
	return (
		<>
			<DialogBody className={"max-h-[50vh] overflow-y-auto font-normal"}>
				<Typography className={"font-normal"}>{tc("name: <b>{{name}}</b>", {name: repo.display_name})}</Typography>
				<Typography className={"font-normal"}>{tc("url: {{url}}", {url: repo.url})}</Typography>
				<Typography className={"font-normal"}>{tc("packages:")}</Typography>
				<ul className={"list-disc pl-6"}>
					{
						repo.packages.map((info, idx) => (
							<li key={idx}>{info.display_name ?? info.name}</li>
						))
					}
				</ul>
			</DialogBody>
			<DialogFooter>
				<Button onClick={cancel}>{tc("cancel")}</Button>
				<Button onClick={add} className={"ml-2"}>{tc("add repository")}</Button>
			</DialogFooter>
		</>
	);
}
