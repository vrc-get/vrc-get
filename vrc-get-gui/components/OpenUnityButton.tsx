import { Button } from "@/components/ui/button";
import { tc } from "@/lib/i18n";
import { openUnity } from "@/lib/open-unity";
import type React from "react";
import { useState } from "react";
import { useRef } from "react";

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

    const queryClient = useQueryClient();

    const openUnityWithUpdateList = async () => {
        await openUnity(projectPath, unityVersion, unityRevision);
        setTimeout(() => {
            queryClient.invalidateQueries(environmentProjects);
        }, 3000);
    };

	return (
		<PreventDoubleClick
			delayMs={1000}
			onClick={openUnityWithUpdateList}
			{...props}
		>
			{tc("projects:button:open unity")}
		</PreventDoubleClick>
	);
}
