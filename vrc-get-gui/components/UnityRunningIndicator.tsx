import { useUnityStatus } from "@/lib/use-unity-status";
import {
	Tooltip,
	TooltipContent,
	TooltipProvider,
	TooltipTrigger,
} from "@/components/ui/tooltip";
import { tc } from "@/lib/i18n";

export function UnityRunningIndicator({
	projectPath,
}: {
	projectPath: string;
}) {
	const isUnityRunning = useUnityStatus(projectPath);

	if (!isUnityRunning) {
		return null;
	}

	return (
		<TooltipProvider>
			<Tooltip>
				<TooltipTrigger asChild>
					<span className="relative flex h-2 w-2">
						<span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-success opacity-75" />
						<span className="relative inline-flex rounded-full h-2 w-2 bg-success" />
					</span>
				</TooltipTrigger>
				<TooltipContent>
					<p>{tc("projects:tooltip:unity is running")}</p>
				</TooltipContent>
			</Tooltip>
		</TooltipProvider>
	);
}
