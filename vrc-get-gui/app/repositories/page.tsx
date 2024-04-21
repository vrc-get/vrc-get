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
import {HContent, HNavBar, HSection, VStack} from "@/components/layout";
import React, {Suspense, useMemo, useState} from "react";
import {MinusCircleIcon, PlusCircleIcon, XCircleIcon} from "@heroicons/react/24/outline";
import {nop} from "@/lib/nop";
import {toastError, toastSuccess, toastThrownError} from "@/lib/toast";
import {tc, tt} from "@/lib/i18n";
import {InputNoLabel} from "@/components/InputNoLabel";
import Table from "@/components/Table";

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
			const _exhaustiveCheck: never = state;
	}
	const dialog = dialogBody ?
		<Dialog handler={nop} open><DialogHeader>{tc("vpm repositories:button:add repository")}</DialogHeader>{dialogBody}</Dialog> : null;

	const [removeDialogOpen, setRemoveDialogOpen] = useState("");


	return (
		<VStack className={"p-4 overflow-y-auto"}>
			<HNavBar className={"flex-shrink-0"}>
				<Typography variant="h4" className="cursor-pointer py-1.5 font-bold flex-grow-0">
					{tc("vpm repositories:community repositories")}
				</Typography>
				<Button onClick={() => setState({type: 'enteringRepositoryInfo'})}>{tc("vpm repositories:button:add repository")}</Button>
			</HNavBar>
			<HContent>
				<HSection>
					<Table
						header={["", tc("vpm repositories:community repositories"), tc("vpm repositories:url"), ""]}
						layout={["auto", "1fr", "1fr", "auto"]}
						rows={(result.data?.user_repositories ?? []).map((repo, repoIndex) => {
							const id = `repository-${repo.id}`;
											
							const selected = !hiddenUserRepos.has(repo.id);
							const onChange = () => {
								if (selected) {
									environmentHideRepository(repo.id).then(() => result.refetch());
								} else {
									environmentShowRepository(repo.id).then(() => result.refetch());
								}
							}

							let localDialog
							if (removeDialogOpen === repo.id) {
								localDialog = <Dialog handler={nop} open>
									<DialogHeader>{tc("remove repository")}</DialogHeader>
									<DialogBody>
										<Typography
											className={"whitespace-normal font-normal"}>{tc("do you want to remove the repository <b>{{name}}</b>?", {name: repo.display_name})}</Typography>
									</DialogBody>
									<DialogFooter>
										<Button onClick={() => setRemoveDialogOpen("")}>{tc("cancel")}</Button>
										<Button onClick={() => {
											removeRepository(repo.id)
											setRemoveDialogOpen("");
										}} className={"ml-2"}>{tc("remove repository")}</Button>
									</DialogFooter>
								</Dialog>;
							}
						
						
							return [
								(<Checkbox
									key={`checkbox-${repoIndex}`}
									ripple={false}
									containerProps={{className: "p-0 rounded-none"}}
									id={id}
									checked={selected}
									onChange={onChange}
								/>),
								(<Typography
									key={`display-${repoIndex}`}
								>
									{repo.display_name}
								</Typography>),
								(<Typography key={`url-${repoIndex}`} className="font-normal">
									{repo.url}
								</Typography>),
								(<>
								<Tooltip key={`remove-${repoIndex}`} content={tc("remove repository")}>
									<IconButton variant={"text"} onClick={() =>setRemoveDialogOpen(repo.id)}>
										<XCircleIcon className={"size-5 text-red-700"}/>
									</IconButton>
								</Tooltip>
								{localDialog}
								</>
								)
							]
						})}
					/>
					{dialog}
				</HSection>
			</HContent>
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
		"general:name",
		"vpm repositories:url",
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
		dialog = <Dialog handler={nop} open>
			<DialogHeader>{tc("vpm repositories:remove repository")}</DialogHeader>
			<DialogBody>
				<Typography
					className={"whitespace-normal font-normal"}>{tc("vpm repositories:dialog:confirm remove description", {name: repo.display_name})}</Typography>
			</DialogBody>
			<DialogFooter>
				<Button onClick={() => setRemoveDialogOpen(false)}>{tc("general:button:cancel")}</Button>
				<Button onClick={() => {
					remove();
					setRemoveDialogOpen(false);
				}} className={"ml-2"}>{tc("vpm repositories:remove repository")}</Button>
			</DialogFooter>
		</Dialog>;
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
				<Tooltip content={tc("vpm repositories:remove repository")}>
					<IconButton onClick={() => setRemoveDialogOpen(true)} variant={"text"}>
						<XCircleIcon className={"size-5 text-red-700"}/>
					</IconButton>
				</Tooltip>
			</td>
			{dialog}
		</tr>
	)
}

let globalHeaderId = 0;

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
	type Header = { name: string, value: string, id: number };
	const [headerArray, setHeaderArray] = useState<Header[]>(() => [{
		name: "",
		value: "",
		id: globalHeaderId++,
	}]);

	let foundHeaderNameError = false;
	let foundHeaderValueError = false;
	let foundDuplicateHeader = false;

	let headerNameSet = new Set<string>();

	for (let {name, value} of headerArray) {
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

	const addHeader = () => {
		setHeaderArray(old => [...old, {
			name: "",
			value: "",
			id: globalHeaderId++,
		}]);
	}

	const removeHeader = (idx: number) => {
		setHeaderArray(old => {
			const newArray = [...old];
			newArray.splice(idx, 1);
			if (newArray.length === 0) {
				newArray.push({
					name: "",
					value: "",
					id: globalHeaderId++,
				});
			}
			return newArray;
		});
	}

	const onAddRepository = () => {
		const headers: { [name: string]: string } = {};
		for (const header of headerArray) {
			if (header.name.trim() === "") continue;
			headers[header.name.trim()] = header.value.trim();
		}
		addRepository(url, headers);
	}

	return (
		<>
			<DialogBody>
				<Typography className={'font-normal'}>
					{tc("vpm repositories:dialog:enter repository info")}
				</Typography>
				<Input type={"vpm repositories:url"} label={"URL"} value={url} onChange={e => setUrl(e.target.value)}
							 placeholder={"https://vpm.anatawa12.com/vpm.json"}></Input>
				<details>
					<summary className={"font-bold"}>{tc("vpm repositories:dialog:headers")}</summary>
					<div className={"w-full max-h-[50vh] overflow-y-auto"}>
						<table className={"w-full"}>
							<thead>
							<tr>
								<th className={"sticky top-0 z-10 bg-white"}>{tc("vpm repositories:dialog:header name")}</th>
								<th className={"sticky top-0 z-10 bg-white"}>{tc("vpm repositories:dialog:header value")}</th>
								<th className={"sticky top-0 z-10 bg-white"}></th>
							</tr>
							</thead>
							<tbody>
							{
								headerArray.map(({name, value, id}, idx) => (
									<tr key={id}>
										<td>
											<InputNoLabel
												type={"text"}
												value={name}
												className={"w-96"}
												onChange={e => {
													setHeaderArray(old => {
														const newArray = [...old];
														newArray[idx] = {...newArray[idx]};
														newArray[idx].name = e.target.value;
														return newArray;
													})
												}}
											/>
										</td>
										<td>
											<InputNoLabel
												type={"text"}
												value={value}
												onChange={e => {
													setHeaderArray(old => {
														const newArray = [...old];
														newArray[idx] = {...newArray[idx]};
														newArray[idx].value = e.target.value;
														return newArray;
													})
												}}
											/>
										</td>
										<td className={"w-20"}>
											<Tooltip content={tc("vpm repositories:tooltip:add header")} className={"z-[19999]"}>
												<IconButton variant={"text"} onClick={addHeader}>
													<PlusCircleIcon color={"green"} className={"size-5"}/>
												</IconButton>
											</Tooltip>
											<Tooltip content={tc("vpm repositories:tooltip:remove header")} className={"z-[19999]"}>
												<IconButton variant={"text"} onClick={() => removeHeader(idx)}>
													<MinusCircleIcon color={"red"} className={"size-5"}/>
												</IconButton>
											</Tooltip>
										</td>
									</tr>
								))
							}
							</tbody>
						</table>
					</div>
				</details>
				{foundHeaderNameError && <Typography className={"text-red-700"}>{tc("vpm repositories:hint:invalid header names")}</Typography>}
				{foundHeaderValueError && <Typography className={"text-red-700"}>{tc("vpm repositories:hint:invalid header values")}</Typography>}
				{foundDuplicateHeader && <Typography className={"text-red-700"}>{tc("vpm repositories:hint:duplicate headers")}</Typography>}
			</DialogBody>
			<DialogFooter>
				<Button onClick={cancel}>{tc("general:button:cancel")}</Button>
				<Button onClick={onAddRepository} className={"ml-2"} disabled={hasError}>{tc("vpm repositories:button:add repository")}</Button>
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
					{tc("vpm repositories:dialog:downloading...")}
				</Typography>
			</DialogBody>
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
			<DialogBody>
				<Typography>
					{tc("vpm repositories:dialog:already added")}
				</Typography>
			</DialogBody>
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
			<DialogBody className={"max-h-[50vh] overflow-y-auto font-normal"}>
				<Typography className={"font-normal"}>{tc("vpm repositories:dialog:name", {name: repo.display_name})}</Typography>
				<Typography className={"font-normal"}>{tc("vpm repositories:dialog:url", {url: repo.url})}</Typography>
				{Object.keys(headers).length > 0 && (
					<>
						<Typography className={"font-normal"}>{tc("vpm repositories:dialog:headers")}</Typography>
						<ul className={"list-disc pl-6"}>
							{
								Object.entries(headers).map(([key, value], idx) => (
									<li key={idx}>{key}: {value}</li>
								))
							}
						</ul>
					</>
				)}
				<Typography className={"font-normal"}>{tc("vpm repositories:dialog:packages")}</Typography>
				<ul className={"list-disc pl-6"}>
					{
						repo.packages.map((info, idx) => (
							<li key={idx}>{info.display_name ?? info.name}</li>
						))
					}
				</ul>
			</DialogBody>
			<DialogFooter>
				<Button onClick={cancel}>{tc("general:button:cancel")}</Button>
				<Button onClick={add} className={"ml-2"}>{tc("vpm repositories:button:add repository")}</Button>
			</DialogFooter>
		</>
	);
}
