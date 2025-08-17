import { Slot } from "@radix-ui/react-slot";
import { cva, type VariantProps } from "class-variance-authority";
import type * as React from "react";

import { cn } from "@/lib/utils";

const buttonVariants = cva(
	"inline-flex items-center justify-center whitespace-nowrap rounded-md text-sm font-medium ring-offset-background transition-colors focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50",
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
				default: "h-10 px-4 py-2 compact:h-8",
				sm: "h-9 rounded-md px-3 compact:h-7",
				lg: "h-11 rounded-md px-8 compact:h-9",
				icon: "h-10 w-10",
			},
		},
		defaultVariants: {
			variant: "default",
			size: "default",
		},
	},
);

export interface ButtonProps
	extends React.ComponentProps<"button">,
		VariantProps<typeof buttonVariants> {
	asChild?: boolean;
}

const Button = ({
	className,
	variant,
	size,
	asChild = false,
	...props
}: ButtonProps) => {
	const Comp = asChild ? Slot : "button";
	return (
		<Comp
			className={cn(buttonVariants({ variant, size, className }))}
			{...props}
		/>
	);
};
Button.displayName = "Button";

export { Button, buttonVariants };
