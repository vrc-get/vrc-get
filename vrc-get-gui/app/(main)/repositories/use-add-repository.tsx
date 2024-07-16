import {
	ReorderableList,
	useReorderableList,
} from "@/components/ReorderableList";
import { Button } from "@/components/ui/button";
import {
	DialogDescription,
	DialogFooter,
	DialogOpen,
	DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { assertNever } from "@/lib/assert-never";
import {
	type TauriRemoteRepositoryInfo,
	environmentAddRepository,
	environmentDownloadRepository,
} from "@/lib/bindings";
import { tc, tt } from "@/lib/i18n";
import { toastError, toastSuccess, toastThrownError } from "@/lib/toast";
import type React from "react";
import { useCallback, useState } from "react";

type State =
	| {
			type: "normal";
	  }
	| {
			type: "enteringRepositoryInfo";
	  }
	| {
			type: "loadingRepository";
	  }
	| {
			type: "duplicated";
	  }
	| {
			type: "confirming";
			repo: TauriRemoteRepositoryInfo;
			url: string;
			headers: { [key: string]: string };
	  };

interface AddRepository {
	dialog: React.ReactNode;
	openAddDialog: () => void;
	addRepository: (
		url: string,
		headers: { [p: string]: string },
	) => Promise<void>;
}

export function useAddRepository({
	refetch,
}: {
	refetch: () => void;
}): AddRepository {
	const [state, setState] = useState<State>({ type: "normal" });

	function cancel() {
		setState({ type: "normal" });
	}

	const openAddDialog = useCallback(() => {
		setState({ type: "enteringRepositoryInfo" });
	}, []);

	const addRepository = useCallback(async function addRepository(
		url: string,
		headers: { [key: string]: string },
	) {
		try {
			setState({ type: "loadingRepository" });
			const info = await environmentDownloadRepository(url, headers);
			switch (info.type) {
				case "BadUrl":
					toastError(tt("vpm repositories:toast:invalid url"));
					setState({ type: "normal" });
					return;
				case "DownloadError":
					toastError(
						tt("vpm repositories:toast:load failed", { message: info.message }),
					);
					setState({ type: "normal" });
					return;
				case "Duplicated":
					setState({ type: "duplicated" });
					return;
				case "Success":
					break;
				default:
					assertNever(info, "info");
			}
			setState({ type: "confirming", repo: info.value, url, headers });
		} catch (e) {
			toastThrownError(e);
			setState({ type: "normal" });
		}
	}, []);

	let dialogBody: React.ReactNode;
	switch (state.type) {
		case "normal":
			dialogBody = null;
			break;
		case "enteringRepositoryInfo":
			dialogBody = (
				<EnteringRepositoryInfo
					cancel={() => setState({ type: "normal" })}
					addRepository={(url, headers) => addRepository(url, headers)}
				/>
			);
			break;
		case "loadingRepository":
			dialogBody = <LoadingRepository cancel={cancel} />;
			break;
		case "duplicated":
			dialogBody = <Duplicated cancel={cancel} />;
			break;
		case "confirming": {
			const doAddRepository = async () => {
				try {
					await environmentAddRepository(state.url, state.headers);
					setState({ type: "normal" });
					toastSuccess(tt("vpm repositories:toast:repository added"));
					// noinspection ES6MissingAwait
					refetch();
				} catch (e) {
					toastThrownError(e);
					setState({ type: "normal" });
				}
			};
			dialogBody = (
				<Confirming
					repo={state.repo}
					headers={state.headers}
					cancel={cancel}
					add={doAddRepository}
				/>
			);
			break;
		}
		default:
			assertNever(state, "state");
	}

	const dialog = dialogBody ? (
		<DialogOpen>
			<DialogTitle>{tc("vpm repositories:button:add repository")}</DialogTitle>
			{dialogBody}
		</DialogOpen>
	) : null;

	return {
		dialog,
		addRepository,
		openAddDialog,
	};
}

function EnteringRepositoryInfo({
	cancel,
	addRepository,
}: {
	cancel: () => void;
	addRepository: (url: string, headers: { [name: string]: string }) => void;
}) {
	const [url, setUrl] = useState("");

	const reordableListContext = useReorderableList({
		defaultValue: { name: "", value: "" },
		allowEmpty: false,
		reorderable: false,
	});

	let foundHeaderNameError = false;
	let foundHeaderValueError = false;
	let foundDuplicateHeader = false;

	const headerNameSet = new Set<string>();

	for (const { value, name } of reordableListContext.value) {
		const trimedName = name.trim();
		const trimedValue = value.trim();
		if (trimedName !== "" || trimedValue !== "") {
			// header (field) name is token (RFC 9110 section 5.1)
			//   https://www.rfc-editor.org/rfc/rfc9110.html#name-field-names
			// token is defined in 5.6.2
			//   https://www.rfc-editor.org/rfc/rfc9110.html#name-tokens
			if (
				trimedName === "" ||
				!trimedName.match(/[!#$%&'*+\-.^_`|~0-9a-zA-Z]/)
			) {
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

	const hasError =
		foundHeaderNameError || foundHeaderValueError || foundDuplicateHeader;

	const onAddRepository = () => {
		const headers: { [name: string]: string } = {};
		for (const header of reordableListContext.value) {
			if (header.name.trim() === "") continue;
			headers[header.name.trim()] = header.value.trim();
		}
		addRepository(url, headers);
	};

	return (
		<>
			<DialogDescription>
				<p className={"font-normal"}>
					{tc("vpm repositories:dialog:enter repository info")}
				</p>
				<Input
					className={"w-full"}
					type={"vpm repositories:url"}
					value={url}
					onChange={(e) => setUrl(e.target.value)}
					placeholder={"https://vpm.anatawa12.com/vpm.json"}
				/>
				<details>
					<summary className={"font-bold"}>
						{tc("vpm repositories:dialog:headers")}
					</summary>
					{/* TODO: use ScrollArea (I failed to use it inside dialog) */}
					<div className={"w-full max-h-[50vh] overflow-y-auto"}>
						<table className={"w-full"}>
							<thead>
								<tr>
									<th className={"sticky top-0 z-10 bg-background"}>
										{tc("vpm repositories:dialog:header name")}
									</th>
									<th className={"sticky top-0 z-10 bg-background"}>
										{tc("vpm repositories:dialog:header value")}
									</th>
									<th className={"sticky top-0 z-10 bg-background"} />
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
													onChange={(e) =>
														reordableListContext.update(id, (old) => ({
															...old,
															name: e.target.value,
														}))
													}
												/>
											</td>
											<td>
												<Input
													type={"text"}
													value={value.value}
													className={"w-full"}
													onChange={(e) =>
														reordableListContext.update(id, (old) => ({
															...old,
															value: e.target.value,
														}))
													}
												/>
											</td>
										</>
									)}
								/>
							</tbody>
						</table>
					</div>
				</details>
				{foundHeaderNameError && (
					<p className={"text-destructive"}>
						{tc("vpm repositories:hint:invalid header names")}
					</p>
				)}
				{foundHeaderValueError && (
					<p className={"text-destructive"}>
						{tc("vpm repositories:hint:invalid header values")}
					</p>
				)}
				{foundDuplicateHeader && (
					<p className={"text-destructive"}>
						{tc("vpm repositories:hint:duplicate headers")}
					</p>
				)}
			</DialogDescription>
			<DialogFooter>
				<Button onClick={cancel}>{tc("general:button:cancel")}</Button>
				<Button
					onClick={onAddRepository}
					className={"ml-2"}
					disabled={hasError}
				>
					{tc("vpm repositories:button:add repository")}
				</Button>
			</DialogFooter>
		</>
	);
}

function LoadingRepository({
	cancel,
}: {
	cancel: () => void;
}) {
	return (
		<>
			<DialogDescription>
				<p>{tc("vpm repositories:dialog:downloading...")}</p>
			</DialogDescription>
			<DialogFooter>
				<Button onClick={cancel}>{tc("general:button:cancel")}</Button>
			</DialogFooter>
		</>
	);
}

function Duplicated({
	cancel,
}: {
	cancel: () => void;
}) {
	return (
		<>
			<DialogDescription>
				<p>{tc("vpm repositories:dialog:already added")}</p>
			</DialogDescription>
			<DialogFooter>
				<Button onClick={cancel}>{tc("general:button:ok")}</Button>
			</DialogFooter>
		</>
	);
}

function Confirming({
	repo,
	cancel,
	add,
	headers,
}: {
	repo: TauriRemoteRepositoryInfo;
	headers: { [key: string]: string };
	cancel: () => void;
	add: () => void;
}) {
	return (
		<>
			{/* TODO: use ScrollArea (I failed to use it inside dialog) */}
			<DialogDescription className={"max-h-[50vh] overflow-y-auto font-normal"}>
				<p className={"font-normal"}>
					{tc("vpm repositories:dialog:name", { name: repo.display_name })}
				</p>
				<p className={"font-normal"}>
					{tc("vpm repositories:dialog:url", { url: repo.url })}
				</p>
				{Object.keys(headers).length > 0 && (
					<>
						<p className={"font-normal"}>
							{tc("vpm repositories:dialog:headers")}
						</p>
						<ul className={"list-disc pl-6"}>
							{Object.entries(headers).map(([key, value], idx) => (
								<li key={key}>
									{key}: {value}
								</li>
							))}
						</ul>
					</>
				)}
				<p className={"font-normal"}>
					{tc("vpm repositories:dialog:packages")}
				</p>
				<ul className={"list-disc pl-6"}>
					{repo.packages.map((info, idx) => (
						<li key={info.name}>{info.display_name ?? info.name}</li>
					))}
				</ul>
			</DialogDescription>
			<DialogFooter>
				<Button onClick={cancel}>{tc("general:button:cancel")}</Button>
				<Button onClick={add} className={"ml-2"}>
					{tc("vpm repositories:button:add repository")}
				</Button>
			</DialogFooter>
		</>
	);
}
