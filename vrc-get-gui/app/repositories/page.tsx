"use client"

import {Button} from "@/components/ui/button";
import {Checkbox} from "@/components/ui/checkbox";
import {DialogDescription, DialogFooter, DialogOpen, DialogTitle} from "@/components/ui/dialog";
import {Input} from "@/components/ui/input";
import {Tooltip, TooltipContent, TooltipTrigger} from "@/components/ui/tooltip";
import {useQuery} from "@tanstack/react-query";
import {
	deepLinkTakeAddRepository,
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
import React, {Suspense, useCallback, useEffect, useId, useMemo, useState} from "react";
import {XCircleIcon} from "@heroicons/react/24/outline";
import {toastError, toastSuccess, toastThrownError} from "@/lib/toast";
import {tc, tt} from "@/lib/i18n";
import {useTauriListen} from "@/lib/use-tauri-listen";
import {ReorderableList, useReorderableList} from "@/components/ReorderableList";
import {ScrollableCardTable} from "@/components/ScrollableCardTable";
import {assertNever} from "@/lib/assert-never";

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

	const addRepository = useCallback(async function addRepository(url: string, headers: { [key: string]: string }) {
		try {
			setState({type: 'loadingRepository'});
			const info = await environmentDownloadRepository(url, headers);
			switch (info.type) {
				case "BadUrl":
					toastError(tt("vpm repositories:toast:invalid url"));
					setState({type: 'normal'});
					return;
				case "DownloadError":
					toastError(tt("vpm repositories:toast:load failed", {message: info.message}));
					setState({type: 'normal'});
					return;
				case "Duplicated":
					setState({type: 'duplicated'});
					return;
				case "Success":
					break;
				default:
					assertNever(info, "info");
			}
			setState({type: 'confirming', repo: info.value, url, headers})
		} catch (e) {
			toastThrownError(e);
			setState({type: 'normal'});
		}
	}, []);

	async function removeRepository(id: string) {
		try {
			await environmentRemoveRepository(id);
			await result.refetch();
		} catch (e) {
			toastThrownError(e);
		}
	}

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

	let dialogBody;
	switch (state.type) {
		case "normal":
			dialogBody = null;
			break;
		case "enteringRepositoryInfo":
			dialogBody = <EnteringRepositoryInfo
				cancel={cancel}
				addRepository={(url, headers) => addRepository(url, headers)}
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
					toastSuccess(tt("vpm repositories:toast:repository added"));
					// noinspection ES6MissingAwait
					result.refetch();
				} catch (e) {
					toastThrownError(e);
					setState({type: 'normal'});
				}
			}
			dialogBody = <Confirming repo={state.repo} headers={state.headers} cancel={cancel} add={doAddRepository}/>;
			break;
		default:
			assertNever(state, "state");
	}
	const dialog = dialogBody ?
		<DialogOpen>
			<DialogTitle>{tc("vpm repositories:button:add repository")}</DialogTitle>{dialogBody}
		</DialogOpen> : null;

	return (
		<VStack className={"p-4 overflow-hidden"}>
			<HNavBar className={"flex-shrink-0"}>
				<p className="cursor-pointer py-1.5 font-bold flex-grow-0">
					{tc("vpm repositories:community repositories")}
				</p>
				<Button
					onClick={() => setState({type: 'enteringRepositoryInfo'})}>{tc("vpm repositories:button:add repository")}</Button>
			</HNavBar>
			<ScrollableCardTable>
				<RepositoryTableBody
					userRepos={result.data?.user_repositories || []}
					hiddenUserRepos={hiddenUserRepos}
					removeRepository={removeRepository}
					refetch={() => result.refetch()}
				/>
			</ScrollableCardTable>
			{dialog}
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
							<XCircleIcon className={"size-5 text-destructive"}/>
						</Button>
					</TooltipTrigger>
					<TooltipContent>{tc("vpm repositories:remove repository")}</TooltipContent>
				</Tooltip>
			</td>
			{dialog}
		</tr>
	)
}

function EnteringRepositoryInfo(
	{
		cancel,
		addRepository,
	}: {
		cancel: () => void,
		addRepository: (url: string, headers: { [name: string]: string }) => void,
	}
) {
	const [url, setUrl] = useState("");

	const reordableListContext = useReorderableList({
		defaultValue: {name: "", value: ""},
		allowEmpty: false,
		reorderable: false,
	})

	let foundHeaderNameError = false;
	let foundHeaderValueError = false;
	let foundDuplicateHeader = false;

	let headerNameSet = new Set<string>();

	for (let {value, name} of reordableListContext.value) {
		let trimedName = name.trim();
		let trimedValue = value.trim();
		if (trimedName != "" || trimedValue != "") {
			// header (field) name is token (RFC 9110 section 5.1)
			//   https://www.rfc-editor.org/rfc/rfc9110.html#name-field-names
			// token is defined in 5.6.2
			//   https://www.rfc-editor.org/rfc/rfc9110.html#name-tokens
			if (trimedName == '' || !trimedName.match(/[!#$%&'*+\-.^_`|~0-9a-zA-Z]/)) {
				foundHeaderNameError = true;
			}

			if (headerNameSet.has(trimedName)) {
				foundDuplicateHeader = true;
			}
			headerNameSet.add(trimedName);

			// header (field) value is field-value (RFC 9110 section 5.5)
			//  note: empty value is allowed
			// field-value    = *field-content
			// field-content  = field-vchar
			//     [ 1*( SP / HTAB / field-vchar ) field-vchar ]
			// field-vchar    = VCHAR / obs-text
			// obs-text       = %x80-FF
			//   ; field-vchar   = [\x21-\x7E\x80-\xFF]
			//   ; field-content = [\x21-\x7E\x80-\xFF]([\t\x20-\x7E\x80-\xFF]+[\x21-\x7E\x80-\xFF])?
			//   ; field-value   = ([\x21-\x7E\x80-\xFF]([\t\x20-\x7E\x80-\xFF]+[\x21-\x7E\x80-\xFF])?)*
			//   ;               = [\t\x20-\x7E\x80-\xFF]* in trimmed value

			// in vrc-get, non-ascii characters are encoded as utf-8 so any non-ascii characters are fit in [\x80-\xFF]
			if (!trimedValue.match(/^[\t\x20-\x7E\u0080-\uFFFF]*$/)) {
				foundHeaderValueError = true;
			}
		}
	}

	const hasError = foundHeaderNameError || foundHeaderValueError || foundDuplicateHeader;

	const onAddRepository = () => {
		const headers: { [name: string]: string } = {};
		for (const header of reordableListContext.value) {
			if (header.name.trim() === "") continue;
			headers[header.name.trim()] = header.value.trim();
		}
		addRepository(url, headers);
	}

	return (
		<>
			<DialogDescription>
				<p className={'font-normal'}>
					{tc("vpm repositories:dialog:enter repository info")}
				</p>
				<Input className={"w-full"} type={"vpm repositories:url"} value={url} onChange={e => setUrl(e.target.value)}
							 placeholder={"https://vpm.anatawa12.com/vpm.json"}></Input>
				<details>
					<summary className={"font-bold"}>{tc("vpm repositories:dialog:headers")}</summary>
					{/* TODO: use ScrollArea (I failed to use it inside dialog) */}
					<div className={"w-full max-h-[50vh] overflow-y-auto"}>
						<table className={"w-full"}>
							<thead>
							<tr>
								<th className={"sticky top-0 z-10 bg-background"}>{tc("vpm repositories:dialog:header name")}</th>
								<th className={"sticky top-0 z-10 bg-background"}>{tc("vpm repositories:dialog:header value")}</th>
								<th className={"sticky top-0 z-10 bg-background"}></th>
							</tr>
							</thead>
							<tbody>
							<ReorderableList
								context={reordableListContext}
								renderItem={(value, id) => (
									<>
										<td>
											<Input
												type={"text"}
												value={value.name}
												className={"w-full"}
												onChange={e => reordableListContext.update(id, old => ({...old, name: e.target.value}))}
											/>
										</td>
										<td>
											<Input
												type={"text"}
												value={value.value}
												className={"w-full"}
												onChange={e => reordableListContext.update(id, old => ({...old, value: e.target.value}))}
											/>
										</td>
									</>
								)}/>
							</tbody>
						</table>
					</div>
				</details>
				{foundHeaderNameError &&
					<p className={"text-destructive"}>{tc("vpm repositories:hint:invalid header names")}</p>}
				{foundHeaderValueError &&
					<p className={"text-destructive"}>{tc("vpm repositories:hint:invalid header values")}</p>}
				{foundDuplicateHeader &&
					<p className={"text-destructive"}>{tc("vpm repositories:hint:duplicate headers")}</p>}
			</DialogDescription>
			<DialogFooter>
				<Button onClick={cancel}>{tc("general:button:cancel")}</Button>
				<Button onClick={onAddRepository} className={"ml-2"}
								disabled={hasError}>{tc("vpm repositories:button:add repository")}</Button>
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
			<DialogDescription>
				<p>
					{tc("vpm repositories:dialog:downloading...")}
				</p>
			</DialogDescription>
			<DialogFooter>
				<Button onClick={cancel}>{tc("general:button:cancel")}</Button>
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
			<DialogDescription>
				<p>
					{tc("vpm repositories:dialog:already added")}
				</p>
			</DialogDescription>
			<DialogFooter>
				<Button onClick={cancel}>{tc("general:button:ok")}</Button>
			</DialogFooter>
		</>
	);
}

function Confirming(
	{
		repo,
		cancel,
		add,
		headers,
	}: {
		repo: TauriRemoteRepositoryInfo,
		headers: { [key: string]: string },
		cancel: () => void,
		add: () => void,
	}
) {
	return (
		<>
			{/* TODO: use ScrollArea (I failed to use it inside dialog) */}
			<DialogDescription className={"max-h-[50vh] overflow-y-auto font-normal"}>
				<p
					className={"font-normal"}>{tc("vpm repositories:dialog:name", {name: repo.display_name})}</p>
				<p className={"font-normal"}>{tc("vpm repositories:dialog:url", {url: repo.url})}</p>
				{Object.keys(headers).length > 0 && (
					<>
						<p className={"font-normal"}>{tc("vpm repositories:dialog:headers")}</p>
						<ul className={"list-disc pl-6"}>
							{
								Object.entries(headers).map(([key, value], idx) => (
									<li key={idx}>{key}: {value}</li>
								))
							}
						</ul>
					</>
				)}
				<p className={"font-normal"}>{tc("vpm repositories:dialog:packages")}</p>
				<ul className={"list-disc pl-6"}>
					{
						repo.packages.map((info, idx) => (
							<li key={idx}>{info.display_name ?? info.name}</li>
						))
					}
				</ul>
			</DialogDescription>
			<DialogFooter>
				<Button onClick={cancel}>{tc("general:button:cancel")}</Button>
				<Button onClick={add} className={"ml-2"}>{tc("vpm repositories:button:add repository")}</Button>
			</DialogFooter>
		</>
	);
}
