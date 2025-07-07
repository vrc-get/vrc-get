import { Star, StarOff } from "lucide-react";
import { cn } from "@/lib/utils";

export function FavoriteStarToggleButton({
	favorite,
	disabled,
	onToggle,
	className,
}: {
	favorite: boolean;
	disabled?: boolean;
	onToggle?: () => void;
	className?: string;
}) {
	if (disabled) {
		return (
			<StarOff
				strokeWidth={favorite ? 1.5 : 3}
				className={cn(
					"size-4 transition-colors cursor-pointer",
					"text-foreground/30",
					"opacity-0 group-hover:opacity-100",
					"hover:opacity-100",
					className,
				)}
				fill={favorite ? "currentColor" : "none"}
				onClick={() => {
					if (!disabled) {
						onToggle?.();
					}
				}}
			/>
		);
	} else {
		return (
			<Star
				strokeWidth={favorite ? 1.5 : 3}
				className={cn(
					"size-4 transition-colors cursor-pointer",
					favorite ? "text-foreground" : "text-foreground/30",
					!favorite && "opacity-0 group-hover:opacity-100",
					"hover:text-foreground hover:opacity-100",
					className,
				)}
				fill={favorite ? "currentColor" : "none"}
				onClick={() => {
					if (!disabled) {
						onToggle?.();
					}
				}}
			/>
		);
	}
}
