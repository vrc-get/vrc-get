import * as ProgressPrimitive from "@radix-ui/react-progress";
import type * as React from "react";

import { cn } from "@/lib/utils";

// https://github.com/shadcn-ui/ui/pull/3471
const Progress = ({
	className,
	value,
	max,
	...props
}: React.ComponentProps<typeof ProgressPrimitive.Root>) => (
	<ProgressPrimitive.Root
		value={value}
		max={max}
		className={cn(
			"relative h-4 w-full overflow-hidden rounded-full bg-secondary",
			className,
		)}
		{...props}
	>
		<ProgressPrimitive.Indicator
			// Removed `transition-all` as a workaround for flickers in WebKit.
			// ref: https://github.com/vrc-get/vrc-get/issues/2640
			// ref: https://bugs.webkit.org/show_bug.cgi?id=304741
			className="h-full w-full flex-1 bg-primary"
			style={{
				transform: `translateX(-${
					100 - Math.min(100, ((value || 0) / (max || 100)) * 100)
				}%)`,
			}}
		/>
	</ProgressPrimitive.Root>
);
Progress.displayName = ProgressPrimitive.Root.displayName;

export { Progress };
