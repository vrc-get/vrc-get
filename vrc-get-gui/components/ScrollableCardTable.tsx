import { Card } from "@/components/ui/card";
import { ScrollArea, ScrollBar } from "@/components/ui/scroll-area";
import { cn } from "@/lib/utils";
import type React from "react";
import { forwardRef } from "react";

export const ScrollableCardTable = forwardRef<
	React.ElementRef<typeof ScrollArea>,
	React.ComponentPropsWithoutRef<typeof ScrollArea> & {
		children: React.ReactNode;
		className?: string;
	}
>(({ children, className }, ref) => {
	return (
		<Card className={cn("overflow-hidden", className)}>
			<ScrollArea
				type="auto"
				className="h-full w-full vrc-get-scrollable-card"
				scrollBarClassName="bg-background py-2.5 vrc-get-scrollable-card-vertical-bar"
				ref={ref}
			>
				<table className="relative table-auto text-left w-full">
					{children}
				</table>
				<div className={"pb-2.5"} />
				<ScrollBar
					orientation="horizontal"
					className="bg-background ps-2.5 vrc-get-scrollable-card-horizontal-bar"
				/>
			</ScrollArea>
		</Card>
	);
});
