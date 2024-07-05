"use client";

import {Card, CardDescription, CardFooter, CardHeader} from "@/components/ui/card";
import React from "react";
import {Button} from "@/components/ui/button";
import {FilePathRow} from "@/components/common-setting-parts";
import {useRouter} from "next/navigation";
import {
	environmentGetSettings,
	environmentPickProjectDefaultPath,
	environmentSetUseAlcomForVccProtocol, utilIsBadHostname
} from "@/lib/bindings";
import {tc} from "@/lib/i18n";
import {useQuery} from "@tanstack/react-query";
import {isWindows, loadOSApi} from "@/lib/os";
import type {OsType} from "@tauri-apps/api/os";
import {Checkbox} from "@/components/ui/checkbox";

export default function Page() {
	const router = useRouter();

	const result = useQuery({
		queryKey: ["environmentGetSettings"],
		queryFn: environmentGetSettings
	})

	const onBack = () => {
		router.back()
	};

	const onNext = () => {
		// TODO: fetch next page from backend
	};

	return <div className={"w-full flex items-center justify-center"}>
		<Card className={"w-[500px] min-w-[50vw] p-4"}>
			<CardHeader>
				<h1 className={"text-center"}>System Configuration</h1>
			</CardHeader>
			<div className={"pb-4"}/>
			{
				!result.data
					? <p>Loading...</p>
					: <WithLoadedData
						useAlcomForVccProtocol={result.data.use_alcom_for_vcc_protocol}
						refetch={() => result.refetch()}
					/>
			}
			<CardFooter className="p-0 pt-3 items-end flex-row gap-2 justify-end">
				<Button onClick={onBack}>Back</Button>
				<Button onClick={onNext}>Next</Button>
			</CardFooter>
		</Card>
	</div>
}

function WithLoadedData(
	{
		useAlcomForVccProtocol,
		refetch,
	}: {
		useAlcomForVccProtocol: boolean;
		refetch: () => void;
	}
) {
	const isBadHostName = useQuery({
		queryKey: ["util_is_bad_hostname"],
		queryFn: utilIsBadHostname,
		initialData: false
	})

	const [osType, setOsType] = React.useState<OsType>("Windows_NT");

	React.useEffect(() => {
		(async () => {
			const os = await loadOSApi();
			setOsType(await os.type());
		})();
	}, [])

	const changeUseAlcomForVcc = async (value: "indeterminate" | boolean) => {
		await environmentSetUseAlcomForVccProtocol(value === true);
		refetch();
	};

	const isMac = osType == "Darwin";

	return (
		<>
			{!isMac ? <div>
				<label className={"flex items-center gap-2"}>
					<Checkbox checked={useAlcomForVccProtocol} onCheckedChange={(e) => changeUseAlcomForVcc(e)}/>
					{tc("settings:use alcom for vcc scheme")}
				</label>
				<p className={"text-sm whitespace-normal text-muted-foreground"}>
					You can use ALCOM for vcc:// scheme instead of VCC to install repository to your PC.
				</p>
			</div> : <div>
				<p className={"text-sm whitespace-normal text-muted-foreground"}>
					There is nothing to configure on macOS. (This page should not be shown, showing this page is a bug)
				</p>
			</div>}
			{isBadHostName.data && <div className={"mt-3"}>
				<p className={"text-sm whitespace-normal text-warning"}>
					Your hostname (PC Name) contains non-ASCII characters. This may cause problems with Unity 2022.<br/>
					It's highly recommended to change your PC Name to ASCII characters.
				</p>
			</div>}
		</>
	)
}
