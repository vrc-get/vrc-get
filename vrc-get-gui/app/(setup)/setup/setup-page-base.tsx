import {useRouter} from "next/navigation";
import {useQuery} from "@tanstack/react-query";
import {environmentGetSettings, TauriEnvironmentSettings} from "@/lib/bindings";
import {Card, CardFooter, CardHeader} from "@/components/ui/card";
import {Button} from "@/components/ui/button";
import React from "react";

export type BodyProps = Readonly<{ environment: TauriEnvironmentSettings, refetch: () => void }>;

export function SetupPageBase(
	{
		heading,
		Body,
		nextPage,
		backContent = "Back",
		nextContent = "Next",
	}: {
		heading: React.ReactNode;
		Body: React.ComponentType<BodyProps>;
		nextPage: string;
		backContent?: React.ReactNode;
		nextContent?: React.ReactNode;
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
		<Card className={"w-[500px] min-w-[50vw] min-h-[50dvh] p-4 flex flex-col"}>
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
		</Card>
	</div>
}
