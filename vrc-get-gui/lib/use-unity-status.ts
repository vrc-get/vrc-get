import { useQuery } from "@tanstack/react-query";
import { commands } from "@/lib/bindings";

export function useUnityStatus(
	projectPath: string | null | undefined,
): boolean {
	const { data: isUnityRunning = false } = useQuery({
		queryKey: ["projectIsUnityLaunching", projectPath],
		queryFn: () => {
			if (!projectPath) return Promise.resolve(false);
			return commands.projectIsUnityLaunching(projectPath);
		},
		enabled: !!projectPath,
		refetchInterval: 2000,
	});

	return isUnityRunning;
}
