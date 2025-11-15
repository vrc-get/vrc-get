import * as React from "react";
import { Slot } from "@radix-ui/react-slot";
import { cva, type VariantProps } from "class-variance-authority";

import { cn } from "@/lib/utils";

const buttonVariants = cva(
	"inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md text-sm font-medium transition-all disabled:pointer-events-none disabled:opacity-50 [&_svg]:pointer-events-none [&_svg:not([class*='size-'])]:size-4 shrink-0 [&_svg]:shrink-0 outline-none focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-[3px] aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 aria-invalid:border-destructive",
	{
		variants: {
			variant: {
				default:
					"bg-primary text-primary-foreground hover:bg-primary/90 shadow-primary/50 hover:shadow-primary/50 shadow-sm hover:shadow-md transition-shadow uppercase",
				destructive:
					"bg-destructive text-destructive-foreground hover:bg-destructive/90 shadow-destructive/50 hover:shadow-destructive/50 shadow-sm hover:shadow-md transition-shadow uppercase",
				warning:
					"bg-warning text-warning-foreground hover:bg-warning/90 shadow-warning/50 hover:shadow-warning/50 shadow-sm hover:shadow-md transition-shadow uppercase",
				"outline-success":
					"border border-input hover:text-accent-foreground border-success hover:border-success/70 text-success hover:text-success/70",
				secondary:
					"bg-secondary text-secondary-foreground hover:bg-secondary/80 shadow-secondary/50 hover:shadow-secondary/50 shadow-sm hover:shadow-md transition-shadow uppercase",
				ghost:
					"hover:bg-accent text-accent-foreground hover:text-accent-foreground",
				"ghost-destructive":
					"hover:bg-destructive/10 text-destructive hover:text-destructive",
				link: "text-primary underline-offset-4 hover:underline",
				info: "bg-info text-info-foreground hover:bg-info/90 shadow-info/50 hover:shadow-info/50 shadow-sm hover:shadow-md transition-shadow uppercase",
				success:
					"bg-success text-success-foreground hover:bg-success/90 shadow-success/50 hover:shadow-success/50 shadow-sm hover:shadow-md transition-shadow uppercase",
			},
			size: {
				default: "h-9 px-4 py-2 has-[>svg]:px-3",
				sm: "h-8 rounded-md gap-1.5 px-3 has-[>svg]:px-2.5",
				lg: "h-10 rounded-md px-6 has-[>svg]:px-4",
				icon: "size-9",
				"icon-sm": "size-8",
				"icon-lg": "size-10",
			},
		},
		defaultVariants: {
			variant: "default",
			size: "default",
		},
	},
);

function Button({
	className,
	variant,
	size,
	asChild = false,
	...props
}: React.ComponentProps<"button"> &
	VariantProps<typeof buttonVariants> & {
		asChild?: boolean;
	}) {
	const Comp = asChild ? Slot : "button";

	return (
		<Comp
			data-slot="button"
			className={cn(buttonVariants({ variant, size, className }))}
			{...props}
		/>
	);
}

export { Button, buttonVariants };
