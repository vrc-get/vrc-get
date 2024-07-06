"use client";

import React from "react";
import {environmentSetUseAlcomForVccProtocol, utilIsBadHostname} from "@/lib/bindings";
import {tc} from "@/lib/i18n";
import {useQuery} from "@tanstack/react-query";
import {loadOSApi} from "@/lib/os";
import type {OsType} from "@tauri-apps/api/os";
import {Checkbox} from "@/components/ui/checkbox";
import {BodyProps, SetupPageBase} from "../setup-page-base";

export default function Page() {
	return <SetupPageBase
		heading={tc("setup:system-setting:heading")}
		Body={Body}
		nextPage={"/setup/finish"}
		pageId={"SystemSetting"}
	/>
}

function Body({environment, refetch}: BodyProps) {
	const useAlcomForVccProtocol = environment.use_alcom_for_vcc_protocol;

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
					{tc("setup:system-setting:vcc scheme description")}
				</p>
			</div> : <div>
				<p className={"text-sm whitespace-normal text-muted-foreground"}>
					{tc("setup:system-setting:macos bug message")}
				</p>
			</div>}
			{isBadHostName.data && <div className={"mt-3"}>
				<p className={"text-sm whitespace-normal text-warning"}>
					{tc("setup:system-setting:hostname-with-non-ascii")}
				</p>
			</div>}
		</>
	)
}
