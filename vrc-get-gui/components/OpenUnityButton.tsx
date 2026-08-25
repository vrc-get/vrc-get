import { queryOptions, useQuery, useQueryClient } from "@tanstack/react-query";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { LoaderCircle } from "lucide-react";
import type React from "react";
import { useRef, useState } from "react";
import { Button } from "@/components/ui/button";
import { commands } from "@/lib/bindings";
import { tc } from "@/lib/i18n";
import { openUnity } from "@/lib/open-unity";
import { toastError, toastNormal, toastThrownError } from "@/lib/toast";

const UNITY_STATUS_IDLE_POLL_INTERVAL_MS = 5000;
const UNITY_STATUS_ACTIVE_POLL_INTERVAL_MS = 1000;

function unityStatusQueryOptions(projectPath: string) {
	return queryOptions({
		queryKey: ["projectUnityStatus", projectPath],
		queryFn: () => commands.projectUnityStatus(projectPath),
		refetchInterval: (query) =>
			query.state.data?.status === "Opening" ||
			query.state.data?.status === "Open"
				? UNITY_STATUS_ACTIVE_POLL_INTERVAL_MS
				: UNITY_STATUS_IDLE_POLL_INTERVAL_MS,
		refetchIntervalInBackground: false,
	});
}

function PreventDoubleClick({
	delayMs,
	// we merge disabled
	disabled,
	onClick,
	...props
}: {
	delayMs: number;
} & React.ComponentProps<typeof Button>) {
	// We use both ref and state because
	// - We need state for rendering the button as disabled
	// - We need ref to prevent double-clicking extremely quickly
	const clickedRef = useRef(false);
	const [disabledSelf, setDisabledSelf] = useState(false);

	const clickWrapper = (e: React.MouseEvent<HTMLButtonElement>) => {
		// Prevent quick double clicking
		if (clickedRef.current) return;

		clickedRef.current = true;
		setDisabledSelf(true);
		setTimeout(() => {
			clickedRef.current = false;
			setDisabledSelf(false);
		}, delayMs);

		onClick?.(e);
	};

	return (
		<Button
			onClick={clickWrapper}
			disabled={disabledSelf || disabled}
			{...props}
		/>
	);
}

export function OpenUnityButton({
	projectPath,
	unityVersion,
	unityRevision,
	// avoid overriding following props
	children: _1,
	onClick: _2,
	disabled,
	...props
}: {
	projectPath: string;
	unityVersion: string | null;
	unityRevision: string | null;
} & React.ComponentProps<typeof Button>) {
	const environmentProjects = queryOptions({
		queryKey: ["environmentProjects"],
		queryFn: commands.environmentProjects,
	});
	const unityStatusOptions = unityStatusQueryOptions(projectPath);
	const { data: unityStatus } = useQuery(unityStatusOptions);

	const queryClient = useQueryClient();

	const openUnityWithUpdateList = async () => {
		await openUnity(projectPath, unityVersion, unityRevision);
		await queryClient.invalidateQueries(unityStatusOptions);
		setTimeout(() => {
			queryClient.invalidateQueries(environmentProjects);
		}, 3000);
	};

	const bringUnityToFront = async () => {
		try {
			const result = await commands.projectBringUnityToFront(projectPath);
			switch (result) {
				case "BroughtToFront":
					break;
				case "FailedToBringToFront":
					try {
						await getCurrentWindow().setFocus();
					} catch (error) {
						console.error(error);
					}
					toastNormal(tc("projects:toast:bring unity to front failed"));
					break;
				case "WindowNotFound":
					toastError(tc("projects:toast:unity window not found"));
					break;
				case "Unsupported":
					toastError(tc("projects:toast:bring unity to front unsupported"));
					break;
			}
		} catch (error) {
			toastThrownError(error);
		} finally {
			await queryClient.invalidateQueries(unityStatusOptions);
		}
	};

	switch (unityStatus?.status) {
		case "Opening":
			return (
				<PreventDoubleClick delayMs={1000} {...props} disabled aria-busy>
					<span className="inline-flex items-center gap-2">
						<LoaderCircle className="size-4 animate-spin" aria-hidden />
						{tc("projects:button:opening unity")}
					</span>
				</PreventDoubleClick>
			);
		case "Open":
			if (!unityStatus.can_bring_to_front) {
				return (
					<PreventDoubleClick delayMs={1000} {...props} disabled>
						{tc("projects:button:unity is open")}
					</PreventDoubleClick>
				);
			} else {
				return (
					<PreventDoubleClick
						delayMs={1000}
						onClick={bringUnityToFront}
						{...props}
						disabled={disabled}
					>
						{tc("projects:button:bring unity to front")}
					</PreventDoubleClick>
				);
			}
		default:
			return (
				<PreventDoubleClick
					delayMs={1000}
					onClick={openUnityWithUpdateList}
					{...props}
					disabled={disabled}
				>
					{tc("projects:button:open unity")}
				</PreventDoubleClick>
			);
	}
}
