"use client";

import { FilePathRow } from "@/components/common-setting-parts";
import {
	Accordion,
	AccordionContent,
	AccordionItem,
	AccordionTrigger,
} from "@/components/ui/accordion";
import { Button } from "@/components/ui/button";
import { CardDescription } from "@/components/ui/card";
import { assertNever } from "@/lib/assert-never";
import { commands } from "@/lib/bindings";
import { tc, tt } from "@/lib/i18n";
import { toastError, toastSuccess, toastThrownError } from "@/lib/toast";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { createFileRoute } from "@tanstack/react-router";
import { type BodyProps, SetupPageBase } from "../-setup-page-base";

export const Route = createFileRoute("/_setup/setup/unity-hub/")({
	component: Page,
});

function Page() {
	return (
		<SetupPageBase
			heading={tc("setup:unity-hub:heading")}
			Body={Body}
			// user should set unity hub path so we re-update unity paths
			onFinish={() => commands.environmentUpdateUnityPathsFromUnityHub()}
			nextPage={"/setup/project-path"}
			prevPage={"/setup/appearance"}
			pageId={"UnityHub"}
		/>
	);
}

function Body({ environment }: BodyProps) {
	const hubInstalled = !!environment.unity_hub;

	const queryClient = useQueryClient();

	const pickUnityHub = useMutation({
		mutationFn: async () => await commands.environmentPickUnityHub(),
		onError: (e) => {
			console.error(e);
			toastThrownError(e);
		},
		onSuccess: (result) => {
			switch (result.type) {
				case "NoFolderSelected":
					// no-op
					break;
				case "InvalidSelection":
					toastError(tc("general:toast:invalid directory"));
					break;
				case "Successful":
					toastSuccess(tc("settings:toast:unity hub path updated"));
					break;
				default:
					assertNever(result);
			}
		},
		onSettled: async () => {
			await queryClient.invalidateQueries({
				queryKey: ["environmentGetSettings"],
			});
		},
	});

	return (
		<>
			<CardDescription className={"whitespace-normal"}>
				{tc("setup:unity-hub:description")}
			</CardDescription>
			{hubInstalled ? (
				<>
					<div className={"pb-4"} />
					<p className={"whitespace-normal text-muted-foreground"}>
						{tc("setup:unity-hub:using this unity hub")}:
					</p>
					<FilePathRow
						path={environment.unity_hub ?? ""}
						pick={pickUnityHub.mutate}
						notFoundMessage={"Unity Hub Not Found"}
						withOpen={false}
					/>
				</>
			) : (
				<>
					<div className={"p-2"} />
					<div className={"flex flex-row flex-wrap gap-2"}>
						<Button
							onClick={() =>
								commands.utilOpenUrl(tt("setup:unity-hub:unity hub link"))
							}
						>
							{tc("setup:unity-hub:download unity hub from unity.com")}
						</Button>
						<Button
							onClick={() =>
								queryClient.invalidateQueries({
									queryKey: ["environmentGetSettings"],
								})
							}
						>
							{tc("setup:unity-hub:recheck installation")}
						</Button>
					</div>
					<Accordion type="single" collapsible>
						<AccordionItem value={"you-have"} className={"border-none"}>
							<AccordionTrigger className={"text-sm"}>
								{tc("setup:unity-hub:detection failed collapse")}
							</AccordionTrigger>
							<AccordionContent>
								<p className={"whitespace-normal"}>
									{tc("setup:unity-hub:detection failed description")}
								</p>
								<FilePathRow
									path={environment.unity_hub ?? ""}
									pick={pickUnityHub.mutate}
									notFoundMessage={"Unity Hub Not Found"}
									withOpen={false}
								/>
							</AccordionContent>
						</AccordionItem>
					</Accordion>
					<div className={"flex w-full"}>
						<span className={"text-destructive"}>
							{tc("setup:unity-hub:not found")}
						</span>
					</div>
				</>
			)}
		</>
	);
}
