"use client";

import { Checkbox } from "@/components/ui/checkbox";
import { commands } from "@/lib/bindings";
import { useGlobalInfo } from "@/lib/global-info";
import { tc } from "@/lib/i18n";
import { toastThrownError } from "@/lib/toast";
import {
	queryOptions,
	useMutation,
	useQuery,
	useQueryClient,
} from "@tanstack/react-query";
import { createFileRoute } from "@tanstack/react-router";
import { type BodyProps, SetupPageBase } from "../-setup-page-base";

export const Route = createFileRoute("/_setup/setup/system-setting/")({
	component: Page,
});
function Page() {
	return (
		<SetupPageBase
			heading={tc("setup:system-setting:heading")}
			Body={Body}
			nextPage={"/setup/finish"}
			prevPage={"/setup/backups"}
			pageId={"SystemSetting"}
		/>
	);
}

const environmentGetSettings = queryOptions({
	queryKey: ["environmentGetSettings"],
	queryFn: commands.environmentGetSettings,
});

function Body({ environment }: BodyProps) {
	const useAlcomForVccProtocol = environment.use_alcom_for_vcc_protocol;

	const isBadHostName = useQuery({
		queryKey: ["util_is_bad_hostname"],
		queryFn: commands.utilIsBadHostname,
		initialData: false,
	});

	const queryClient = useQueryClient();

	const setUseAlcomForVccProtocol = useMutation({
		mutationFn: async (use: boolean) =>
			await commands.environmentSetUseAlcomForVccProtocol(use),
		onMutate: async (use) => {
			await queryClient.cancelQueries(environmentGetSettings);
			const current = queryClient.getQueryData(environmentGetSettings.queryKey);
			if (current != null) {
				queryClient.setQueryData(environmentGetSettings.queryKey, {
					...current,
					use_alcom_for_vcc_protocol: use,
				});
			}
			return current;
		},
		onError: (e, _, prev) => {
			console.error(e);
			toastThrownError(e);
			queryClient.setQueryData(environmentGetSettings.queryKey, prev);
		},
		onSettled: async () => {
			await queryClient.invalidateQueries(environmentGetSettings);
		},
	});

	const isMac = useGlobalInfo().osType === "Darwin";

	return (
		<>
			{!isMac ? (
				<div>
					<label className={"flex items-center gap-2"}>
						<Checkbox
							checked={useAlcomForVccProtocol}
							onCheckedChange={(e) =>
								setUseAlcomForVccProtocol.mutate(e === true)
							}
						/>
						{tc("settings:use alcom for vcc scheme")}
					</label>
					<p className={"text-sm whitespace-normal text-muted-foreground"}>
						{tc("setup:system-setting:vcc scheme description")}
					</p>
				</div>
			) : (
				<div>
					<p className={"text-sm whitespace-normal text-muted-foreground"}>
						{tc("setup:system-setting:macos bug message")}
					</p>
				</div>
			)}
			{isBadHostName.data && (
				<div className={"mt-3"}>
					<p className={"text-sm whitespace-normal text-warning"}>
						{tc("setup:system-setting:hostname-with-non-ascii")}
					</p>
				</div>
			)}
		</>
	);
}
