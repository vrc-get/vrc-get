import { Progress as ProgressPrimitive } from "radix-ui";
import type * as React from "react";

import { cn } from "@/lib/utils";

// https://github.com/shadcn-ui/ui/pull/3471
function Progress({
	className,
	value,
	max,
	...props
}: React.ComponentProps<typeof ProgressPrimitive.Root>) {
	return (
		<ProgressPrimitive.Root
			data-slot="progress"
			value={value}
			max={max}
			className={cn(
				"relative h-4 w-full overflow-hidden rounded-full bg-secondary",
				className,
			)}
			{...props}
		>
			<ProgressPrimitive.Indicator
				data-slot="progress-indicator"
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
}

export { Progress };
