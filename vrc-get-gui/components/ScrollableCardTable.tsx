import {Card} from "@/components/ui/card";
import {ScrollArea, ScrollBar} from "@/components/ui/scroll-area";
import {cn} from "@/lib/utils";
import React from "react";

export function ScrollableCardTable(
	{
		children,
		className,
	}: {
		children: React.ReactNode
		className?: string
	}
) {
	return <Card className={cn("overflow-hidden", className)}>
		<ScrollArea type="auto" className="h-full w-full" scrollBarClassName="bg-background py-2.5">
			<table className="relative table-auto text-left w-full">
				{children}
			</table>
			<div className={"pb-2.5"}/>
			<ScrollBar orientation="horizontal" className="bg-background ps-2.5"/>
		</ScrollArea>
	</Card>
}
