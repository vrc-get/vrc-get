import type React from "react";
import { Button } from "@/components/ui/button";
import {
	Tooltip,
	TooltipContent,
	TooltipTrigger,
} from "@/components/ui/tooltip";

export function TooltipButton({
	tooltip,
	side = "top",
	className,
	children,
	...props
}: {
	tooltip: React.ReactNode;
	side?: React.ComponentProps<typeof TooltipContent>["side"];
	className?: string;
} & React.ComponentProps<typeof Button>) {
	return (
		<Tooltip>
			<TooltipTrigger asChild>
				<Button className={className} {...props}>{children}</Button>
			</TooltipTrigger>
			<TooltipContent side={side}>{tooltip}</TooltipContent>
		</Tooltip>
	);
}
