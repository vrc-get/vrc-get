import { cva, type VariantProps } from "class-variance-authority";
import { Slot } from "radix-ui";
import type * as React from "react";

import { cn } from "@/lib/utils";

const buttonVariants = cva(
	"inline-flex shrink-0 items-center justify-center gap-2 rounded-md text-sm font-medium whitespace-nowrap transition-all outline-none focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50 disabled:pointer-events-none disabled:opacity-50 aria-invalid:border-destructive aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4",
	{
		variants: {
			variant: {
				default:
					"bg-primary text-primary-foreground hover:bg-primary/90 shadow-primary/50 hover:shadow-primary/50 shadow-sm hover:shadow-md transition-shadow uppercase",
				destructive:
					"bg-destructive text-destructive-foreground hover:bg-destructive/90 shadow-destructive/50 hover:shadow-destructive/50 shadow-sm hover:shadow-md transition-shadow uppercase focus-visible:ring-destructive/20 dark:focus-visible:ring-destructive/40",
				warning:
					"bg-warning text-warning-foreground hover:bg-warning/90 shadow-warning/50 hover:shadow-warning/50 shadow-sm hover:shadow-md transition-shadow uppercase",
				"outline-success":
					"border border-input hover:text-accent-foreground border-success hover:border-success/70 text-success hover:text-success/70",
				secondary:
					"bg-secondary text-secondary-foreground hover:bg-secondary/80 shadow-secondary/50 hover:shadow-secondary/50 shadow-sm hover:shadow-md transition-shadow uppercase",
				ghost:
					"hover:bg-accent text-accent-foreground hover:text-accent-foreground dark:hover:bg-accent/50",
				"ghost-destructive":
					"hover:bg-destructive/10 text-destructive hover:text-destructive",
				link: "text-primary underline-offset-4 hover:underline",
				info: "bg-info text-info-foreground hover:bg-info/90 shadow-info/50 hover:shadow-info/50 shadow-sm hover:shadow-md transition-shadow uppercase",
				success:
					"bg-success text-success-foreground hover:bg-success/90 shadow-success/50 hover:shadow-success/50 shadow-sm hover:shadow-md transition-shadow uppercase",
			},
			size: {
				default: "h-9 px-4 py-2 has-[>svg]:px-3 compact:h-8",
				xs: "h-6 gap-1 rounded-md px-2 text-xs has-[>svg]:px-1.5 [&_svg:not([class*='size-'])]:size-3 compact:h-5",
				sm: "h-8 gap-1.5 rounded-md px-3 has-[>svg]:px-2.5 compact:h-7",
				lg: "h-10 rounded-md px-6 has-[>svg]:px-4 compact:h-9",
				icon: "size-9 compact:size-8",
				"icon-xs":
					"size-6 rounded-md [&_svg:not([class*='size-'])]:size-3 compact:size-5",
				"icon-sm": "size-8 compact:size-7",
				"icon-lg": "size-10 compact:size-8",
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
	variant = "default",
	size = "default",
	asChild = false,
	...props
}: React.ComponentProps<"button"> &
	VariantProps<typeof buttonVariants> & {
		asChild?: boolean;
	}) {
	const Comp = asChild ? Slot.Root : "button";

	return (
		<Comp
			data-slot="button"
			data-variant={variant}
			data-size={size}
			className={cn(buttonVariants({ variant, size, className }))}
			{...props}
		/>
	);
}

export { Button, buttonVariants };
