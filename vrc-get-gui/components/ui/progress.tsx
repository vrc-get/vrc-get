import * as ProgressPrimitive from "@radix-ui/react-progress";
import type * as React from "react";

import { cn } from "@/lib/utils";

function Progress({
	className,
	value,
	max,
	...props
}: React.ComponentProps<typeof ProgressPrimitive.Root>) {
	return (
		<ProgressPrimitive.Root
			data-slot="progress"
			className={cn(
				"bg-primary/20 relative h-2 w-full overflow-hidden rounded-full",
				className,
			)}
			max={max}
			{...props}
		>
			<ProgressPrimitive.Indicator
				data-slot="progress-indicator"
				// Removed `transition-all` as a workaround for flickers in WebKit.
				// ref: https://github.com/vrc-get/vrc-get/issues/2640
				// ref: https://bugs.webkit.org/show_bug.cgi?id=304741
				className="bg-primary h-full w-full flex-1"
				style={{
					transform: `translateX(-${100 - Math.min(100, ((value || 0) / (max || 100)) * 100)}%)`,
				}}
			/>
		</ProgressPrimitive.Root>
	);
}

export { Progress };
