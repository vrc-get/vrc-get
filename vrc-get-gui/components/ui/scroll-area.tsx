import * as ScrollAreaPrimitive from "@radix-ui/react-scroll-area";
import type * as React from "react";

import { cn } from "@/lib/utils";

export type ViewportRef = React.ComponentRef<
	typeof ScrollAreaPrimitive.Viewport
>;

const ScrollArea = ({
	className,
	children,
	scrollBarClassName,
	viewportClassName,
	viewportRef,
	...props
}: React.ComponentProps<typeof ScrollAreaPrimitive.Root> & {
	scrollBarClassName?: string;
	viewportClassName?: string;
	viewportRef?: React.Ref<ViewportRef>;
}) => (
	<ScrollAreaPrimitive.Root
		className={cn("relative overflow-hidden", className)}
		{...props}
	>
		<ScrollAreaPrimitive.Viewport
			className={`h-full w-full rounded-[inherit] scroll-smooth ${viewportClassName}`}
			ref={viewportRef}
		>
			{children}
		</ScrollAreaPrimitive.Viewport>
		<ScrollBar className={scrollBarClassName} />
		<ScrollAreaPrimitive.Corner />
	</ScrollAreaPrimitive.Root>
);
ScrollArea.displayName = ScrollAreaPrimitive.Root.displayName;

const ScrollBar = ({
	className,
	orientation = "vertical",
	...props
}: React.ComponentProps<typeof ScrollAreaPrimitive.ScrollAreaScrollbar>) => (
	<ScrollAreaPrimitive.ScrollAreaScrollbar
		orientation={orientation}
		className={cn(
			"flex touch-none select-none transition-colors z-20",
			orientation === "vertical" &&
				"h-full w-2.5 border-l border-l-transparent p-[1px]",
			orientation === "horizontal" &&
				"h-2.5 flex-col border-t border-t-transparent p-[1px]",
			className,
		)}
		{...props}
	>
		<ScrollAreaPrimitive.ScrollAreaThumb className="relative flex-1 rounded-full bg-border" />
	</ScrollAreaPrimitive.ScrollAreaScrollbar>
);
ScrollBar.displayName = ScrollAreaPrimitive.ScrollAreaScrollbar.displayName;

export { ScrollArea, ScrollBar };
