"use client";

import {
	queryOptions,
	useMutation,
	useQuery,
	useQueryClient,
} from "@tanstack/react-query";
import { createFileRoute } from "@tanstack/react-router";
import { CircleX } from "lucide-react";
import { Suspense, useId } from "react";
import { HNavBar, VStack } from "@/components/layout";
import { ScrollableCardTable } from "@/components/ScrollableCardTable";
import { Button } from "@/components/ui/button";
import {
	Dialog,
	DialogClose,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogTitle,
	DialogTrigger,
} from "@/components/ui/dialog";
import {
	Tooltip,
	TooltipContent,
	TooltipTrigger,
} from "@/components/ui/tooltip";
import type { TauriUserPackage } from "@/lib/bindings";
import { commands } from "@/lib/bindings";
import { tc } from "@/lib/i18n";
import { usePrevPathName } from "@/lib/prev-page";
import { toastError, toastSuccess, toastThrownError } from "@/lib/toast";
import { toVersionString } from "@/lib/version";
import { HeadingPageName } from "../-tab-selector";

export const Route = createFileRoute("/_main/packages/user-packages/")({
	component: Page,
});

function Page() {
	return (
		<Suspense>
			<PageBody />
		</Suspense>
	);
}

const environmentGetUserPackages = queryOptions({
	queryKey: ["environmentGetUserPackages"],
	queryFn: commands.environmentGetUserPackages,
});

function PageBody() {
	const result = useQuery(environmentGetUserPackages);

	const queryClient = useQueryClient();
	const addUserPackageWithPicker = useMutation({
		mutationFn: async () =>
			await commands.environmentAddUserPackageWithPicker(),
		onSuccess: async (result) => {
			switch (result) {
				case "NoFolderSelected":
					break;
				case "InvalidSelection":
					toastError(tc("user packages:toast:invalid selection"));
					break;
				case "AlreadyAdded":
					toastSuccess(tc("user packages:toast:package already added"));
					break;
				case "Successful":
					toastSuccess(tc("user packages:toast:package added"));
					await queryClient.invalidateQueries(environmentGetUserPackages);
					break;
			}
		},
		onError: (error) => {
			console.error(error);
			toastThrownError(error);
		},
	});

	const bodyAnimation = usePrevPathName().startsWith("/packages")
		? "slide-left"
		: "";

	return (
		<VStack>
			<HNavBar
				className={"shrink-0"}
				trailingClassName="-mr-2"
				leading={<HeadingPageName pageType={"/packages/user-packages"} />}
				trailing={
					<Button onClick={() => addUserPackageWithPicker.mutate()}>
						{tc("user packages:button:add package")}
					</Button>
				}
			/>
			<main
				className={`shrink overflow-hidden flex w-full h-full ${bodyAnimation}`}
			>
				<ScrollableCardTable className={"h-full w-full"}>
					<RepositoryTableBody userPackages={result.data || []} />
				</ScrollableCardTable>
			</main>
		</VStack>
	);
}

function RepositoryTableBody({
	userPackages,
}: {
	userPackages: TauriUserPackage[];
}) {
	const queryClient = useQueryClient();
	const removeUserPackages = useMutation({
		mutationFn: async (path: string) =>
			await commands.environmentRemoveUserPackages(path),
		onMutate: async (path) => {
			await queryClient.invalidateQueries(environmentGetUserPackages);
			const data = queryClient.getQueryData(
				environmentGetUserPackages.queryKey,
			);
			if (data !== undefined) {
				queryClient.setQueryData(
					environmentGetUserPackages.queryKey,
					data.filter((x) => x.path === path),
				);
			}
			return data;
		},
		onError: (error, _, ctx) => {
			console.error(error);
			toastThrownError(error);
			queryClient.setQueryData(environmentGetUserPackages.queryKey, ctx);
		},
		onSettled: async () => {
			await queryClient.invalidateQueries(environmentGetUserPackages);
		},
	});

	const TABLE_HEAD = [
		"general:name",
		"user packages:path",
		"user packages:version",
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
				{userPackages.map((pkg) => (
					<PackageRow
						key={pkg.path}
						pkg={pkg}
						remove={() => removeUserPackages.mutate(pkg.path)}
					/>
				))}
			</tbody>
		</>
	);
}

function PackageRow({
	pkg,
	remove,
}: {
	pkg: TauriUserPackage;
	remove: () => void;
}) {
	const cellClass = "p-2.5 compact:py-1";
	const id = useId();

	const pkgDisplayNames = pkg.package.display_name ?? pkg.package.name;

	return (
		<tr className="even:bg-secondary/30">
			<td className={cellClass}>
				<label htmlFor={id}>
					<p className="font-normal">{pkgDisplayNames}</p>
				</label>
			</td>
			<td className={cellClass}>
				<p className="font-normal">{pkg.path}</p>
			</td>
			<td className={cellClass}>
				<p className="font-normal">{toVersionString(pkg.package.version)}</p>
			</td>
			<td className={`${cellClass} w-0`}>
				<Dialog>
					<Tooltip>
						<TooltipTrigger asChild>
							<DialogTrigger asChild>
								<Button variant={"ghost"} size={"icon"}>
									<CircleX className={"size-5 text-destructive"} />
								</Button>
							</DialogTrigger>
						</TooltipTrigger>
						<TooltipContent>
							{tc("user packages:tooltip:remove package")}
						</TooltipContent>
						<DialogContent>
							<DialogTitle>
								{tc("user packages:dialog:remove package")}
							</DialogTitle>
							<DialogDescription>
								<p className={"whitespace-normal font-normal"}>
									{tc("user packages:dialog:confirm remove description", {
										name: pkgDisplayNames,
										path: pkg.path,
									})}
								</p>
							</DialogDescription>
							<DialogFooter>
								<DialogClose asChild>
									<Button>{tc("general:button:cancel")}</Button>
								</DialogClose>
								<DialogClose asChild>
									<Button onClick={remove} className={"ml-2"}>
										{tc("user packages:dialog:button:remove package")}
									</Button>
								</DialogClose>
							</DialogFooter>
						</DialogContent>
					</Tooltip>
				</Dialog>
			</td>
		</tr>
	);
}
