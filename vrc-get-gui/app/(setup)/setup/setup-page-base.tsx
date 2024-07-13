import {useRouter} from "next/navigation";
import {useQuery} from "@tanstack/react-query";
import {
	environmentFinishedSetupPage,
	environmentGetFinishedSetupPages,
	environmentGetSettings,
	SetupPages,
	TauriEnvironmentSettings
} from "@/lib/bindings";
import {Card, CardFooter, CardHeader} from "@/components/ui/card";
import {Button} from "@/components/ui/button";
import React from "react";
import {Circle, CircleAlert, CircleCheck, CircleChevronRight} from "lucide-react";
import {tc} from "@/lib/i18n";
import {useGlobalInfo} from "@/lib/global-info";

export type BodyProps = Readonly<{ environment: TauriEnvironmentSettings, refetch: () => void }>;

export function SetupPageBase(
	{
		heading,
		Body,
		nextPage,
		prevPage,
		backContent = tc("setup:back"),
		nextContent = tc("setup:next"),
		pageId,
		withoutSteps = false,
	}: {
		heading: React.ReactNode;
		Body: React.ComponentType<BodyProps>;
		nextPage: string;
		prevPage: string | null;
		backContent?: React.ReactNode;
		nextContent?: React.ReactNode;
		pageId: SetupPages | null;
		withoutSteps?: boolean;
	}
) {
	const router = useRouter();

	const result = useQuery({
		queryKey: ["environmentGetSettings"],
		queryFn: environmentGetSettings,
	})

	const onNext = async () => {
		if (pageId)
			await environmentFinishedSetupPage(pageId);
		router.push(nextPage)
	};

	return <div className={"w-full flex items-center justify-center"}>
		<div className={"flex gap-4"}>
			{!withoutSteps && <StepCard current={pageId}/>}
			<Card className={`${withoutSteps ? "w-[30rem]" : "w-96"} min-w-[50vw] min-h-[max(50dvh,20rem)] p-4 flex gap-3`}>
				<div className={"flex flex-col flex-grow"}>
					<CardHeader>
						<h1 className={"text-center"}>{heading}</h1>
					</CardHeader>
					<div className={"pb-4"}/>
					{
						!result.data
							? <p>{tc("setup:loading")}</p>
							: <Body environment={result.data} refetch={() => result.refetch()}/>
					}
					<div className={"flex-grow"}/>
					<CardFooter className="p-0 pt-3 items-end flex-row gap-2 justify-end">
						{prevPage && <Button onClick={() => router.push(prevPage)}>{backContent}</Button>}
						<Button onClick={onNext}>{nextContent}</Button>
					</CardFooter>
				</div>
			</Card>
		</div>
	</div>
}

function StepCard(
	{
		current,
	}: {
		current: SetupPages | null;
	}
) {
	// TODO: get progress from backend
	const finisheds = useQuery({
		queryKey: ["environmentGetFinishedSetupPages"],
		queryFn: async () => environmentGetFinishedSetupPages(),
		initialData: []
	}).data;

	const isMac = useGlobalInfo().osType === "Darwin";

	return <Card className={"w-48 p-4"}>
		<ol className={"flex flex-col gap-2"}>
			<StepElement current={current} finisheds={finisheds} pageId={"Appearance"}/>
			<StepElement current={current} finisheds={finisheds} pageId={"UnityHub"}/>
			<StepElement current={current} finisheds={finisheds} pageId={"ProjectPath"}/>
			<StepElement current={current} finisheds={finisheds} pageId={"Backups"}/>
			{!isMac && <StepElement current={current} finisheds={finisheds} pageId={"SystemSetting"}/>}
		</ol>
	</Card>
}

function StepElement(
	{
		current,
		finisheds,
		pageId,
	}: {
		current: SetupPages | null;
		finisheds: SetupPages[];
		pageId: SetupPages;
	}
) {
	const finished = finisheds.includes(pageId);
	const active = current === pageId;
	return <li className={`${active ? "text-foreground" : finished ? "text-success" : "text-foreground/50"} flex gap-1`}>
		{finished ? <CircleCheck/> : active ? <CircleChevronRight/> : <Circle/>}
		{tc(`setup:steps card:${pageId}`)}
	</li>
}

export function WarningMessage(
	{
		children,
	}: {
		children: React.ReactNode;
	}
) {
	return <div className={"flex items-center gap-2"}>
		<div className="flex-grow-0 flex-shrink-0">
			<CircleAlert className="text-warning w-5 h-5"/>
		</div>
		<p className={"whitespace-normal text-sm"}>
			{children}
		</p>
	</div>
}
