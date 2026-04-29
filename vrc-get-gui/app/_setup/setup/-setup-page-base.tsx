import { useQuery } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { Circle, CircleCheck, CircleChevronRight } from "lucide-react";
import type React from "react";
import { Button } from "@/components/ui/button";
import { Card, CardFooter, CardHeader } from "@/components/ui/card";
import type { SetupPages, TauriEnvironmentSettings } from "@/lib/bindings";
import { commands } from "@/lib/bindings";
import { useGlobalInfo } from "@/lib/global-info";
import { tc } from "@/lib/i18n";

export type BodyProps = Readonly<{
	environment: TauriEnvironmentSettings;
}>;

export function SetupPageBase({
	heading,
	Body,
	nextPage,
	prevPage,
	onFinish,
	backContent = tc("setup:back"),
	nextContent = tc("setup:next"),
	pageId,
	withoutSteps = false,
}: {
	heading: React.ReactNode;
	Body: React.ComponentType<BodyProps>;
	nextPage: string;
	prevPage: string | null;
	onFinish?: () => void;
	backContent?: React.ReactNode;
	nextContent?: React.ReactNode;
	pageId: SetupPages | null;
	withoutSteps?: boolean;
}) {
	const navigate = useNavigate();

	const result = useQuery({
		queryKey: ["environmentGetSettings"],
		queryFn: commands.environmentGetSettings,
	});

	const onNext = async () => {
		if (pageId) await commands.environmentFinishedSetupPage(pageId);
		navigate({ to: nextPage });
		onFinish?.();
	};

	return (
		<div className={"w-full flex items-center justify-center"}>
			<div className={"flex gap-4"}>
				{!withoutSteps && <StepCard current={pageId} />}
				<Card
					className={`${withoutSteps ? "w-[30rem]" : "w-96"} min-w-[50vw] min-h-[max(50dvh,20rem)] p-4 flex gap-3 compact:min-h-[max(40dvh,20rem)]`}
				>
					<div className={"flex flex-col grow gap-3 compact:gap-2"}>
						<CardHeader>
							<h1 className={"text-center"}>{heading}</h1>
						</CardHeader>
						<div className={"pb-4"} />
						{!result.data ? (
							<p>{tc("setup:loading")}</p>
						) : (
							<Body environment={result.data} />
						)}
						<div className={"grow"} />
						<CardFooter className="p-0 pt-3 items-end flex-row gap-2 justify-end compact:-m-2">
							{prevPage && (
								<Button onClick={() => navigate({ to: prevPage })}>
									{backContent}
								</Button>
							)}
							<Button onClick={onNext}>{nextContent}</Button>
						</CardFooter>
					</div>
				</Card>
			</div>
		</div>
	);
}

function StepCard({ current }: { current: SetupPages | null }) {
	// TODO: get progress from backend
	const finisheds = useQuery({
		queryKey: ["environmentGetFinishedSetupPages"],
		queryFn: async () => commands.environmentGetFinishedSetupPages(),
		initialData: [],
	}).data;

	const shouldInstallDeepLink = useGlobalInfo().shouldInstallDeepLink;

	return (
		<Card className={"w-48 p-4"}>
			<ol className={"flex flex-col gap-2"}>
				<StepElement
					current={current}
					finisheds={finisheds}
					pageId={"Appearance"}
				/>
				<StepElement
					current={current}
					finisheds={finisheds}
					pageId={"UnityHub"}
				/>
				<StepElement
					current={current}
					finisheds={finisheds}
					pageId={"ProjectPath"}
				/>
				<StepElement
					current={current}
					finisheds={finisheds}
					pageId={"Backups"}
				/>
				{shouldInstallDeepLink && (
					<StepElement
						current={current}
						finisheds={finisheds}
						pageId={"SystemSetting"}
					/>
				)}
			</ol>
		</Card>
	);
}

function StepElement({
	current,
	finisheds,
	pageId,
}: {
	current: SetupPages | null;
	finisheds: SetupPages[];
	pageId: SetupPages;
}) {
	const finished = finisheds.includes(pageId);
	const active = current === pageId;
	return (
		<li
			className={`${active ? "text-foreground" : finished ? "text-success" : "text-foreground/50"} flex gap-1`}
		>
			{finished ? (
				<CircleCheck />
			) : active ? (
				<CircleChevronRight />
			) : (
				<Circle />
			)}
			{tc(`setup:steps card:${pageId}`)}
		</li>
	);
}
