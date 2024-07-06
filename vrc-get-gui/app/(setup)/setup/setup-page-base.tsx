import {useRouter} from "next/navigation";
import {useQuery} from "@tanstack/react-query";
import {environmentGetSettings, TauriEnvironmentSettings} from "@/lib/bindings";
import {Card, CardFooter, CardHeader} from "@/components/ui/card";
import {Button} from "@/components/ui/button";
import React from "react";
import {Circle, CircleCheck, CircleChevronRight} from "lucide-react";
import {loadOSApi} from "@/lib/os";

export type BodyProps = Readonly<{ environment: TauriEnvironmentSettings, refetch: () => void }>;

export type PageId = "Appearance" | "UnityHub" | "ProjectPath" | "Backups" | "SystemSetting";

export function SetupPageBase(
	{
		heading,
		Body,
		nextPage,
		backContent = "Back",
		nextContent = "Next",
		pageId,
		withoutSteps = false,
	}: {
		heading: React.ReactNode;
		Body: React.ComponentType<BodyProps>;
		nextPage: string;
		backContent?: React.ReactNode;
		nextContent?: React.ReactNode;
		pageId: PageId | null;
		withoutSteps?: boolean;
	}
) {
	const router = useRouter();

	const result = useQuery({
		queryKey: ["environmentGetSettings"],
		queryFn: environmentGetSettings,
	})

	const onBack = () => {
		router.back()
	};

	const onNext = () => {
		// TODO: fetch next page from backend
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
							? <p>Loading...</p>
							: <Body environment={result.data} refetch={() => result.refetch()}/>
					}
					<div className={"flex-grow"}/>
					<CardFooter className="p-0 pt-3 items-end flex-row gap-2 justify-end">
						{backContent && <Button onClick={onBack}>{backContent}</Button>}
						{nextContent && <Button onClick={onNext}>{nextContent}</Button>}
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
		current: PageId | null;
	}
) {
	// TODO: get progress from backend
	const finisheds: PageId[] = ["Appearance", "UnityHub"];

	const osType = useQuery({
		queryKey: ["osType"],
		queryFn: async () => loadOSApi().then(os => os.type()),
		initialData: "Windows_NT" as const
	}).data;

	const isMac = osType === "Darwin";

	return <Card className={"w-48 p-4"}>
		<ol className={"flex flex-col gap-2"}>
			<StepElement current={current} finisheds={finisheds} pageId={"Appearance"}>Appearance</StepElement>
			<StepElement current={current} finisheds={finisheds} pageId={"UnityHub"}>Unity Hub</StepElement>
			<StepElement current={current} finisheds={finisheds} pageId={"ProjectPath"}>Save Location</StepElement>
			<StepElement current={current} finisheds={finisheds} pageId={"Backups"}>Backup</StepElement>
			{!isMac && <StepElement current={current} finisheds={finisheds} pageId={"SystemSetting"}>System</StepElement>}
		</ol>
	</Card>
}

function StepElement(
	{
		current,
		finisheds,
		pageId,
		children,
	}: {
		current: PageId | null;
		finisheds: PageId[];
		pageId: PageId;
		children: React.ReactNode;
	}
) {
	const finished = finisheds.includes(pageId);
	const active = current === pageId;
	return <li className={`${active ? "text-foreground" : finished ? "text-success" : "text-foreground/50"} flex gap-1`}>
		{finished ? <CircleCheck/> : active ? <CircleChevronRight/> : <Circle/>}
		{children}
	</li>
}
