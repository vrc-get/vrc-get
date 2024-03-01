"use client"

import {Typography} from "@material-tailwind/react";
import {utilGetVersion} from "@/lib/bindings";
import {useQuery} from "@tanstack/react-query";

export default function Page() {
	const currentVersionResult = useQuery({
		queryKey: ["utilGetVersion"],
		queryFn: utilGetVersion,
		refetchOnMount: false,
		refetchOnReconnect: false,
		refetchOnWindowFocus: false,
		refetchInterval: false,
	});

	const currentVersion = currentVersionResult.status == "success" ? currentVersionResult.data : "Loading...";

	return (
		<div className={"p-4 whitespace-normal"}>
			<Typography>Editing Settings is not supported yet. Please use <code>vrc-get</code> cli or official VCC instead for
				now.</Typography>
			<Typography>Version {currentVersion}</Typography>
		</div>
	);
}
