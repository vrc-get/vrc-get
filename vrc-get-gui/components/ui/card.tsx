import type * as React from "react";

import { cn } from "@/lib/utils";

const Card = ({ className, ...props }: React.ComponentProps<"div">) => (
	<div
		className={cn(
			"rounded-xl border bg-card text-card-foreground shadow-xs",
			className,
		)}
		{...props}
	/>
);
Card.displayName = "Card";

const CardHeader = ({ className, ...props }: React.ComponentProps<"div">) => (
	<div className={cn("flex flex-col space-y-1.5", className)} {...props} />
);
CardHeader.displayName = "CardHeader";

const CardTitle = ({ className, ...props }: React.ComponentProps<"h3">) => (
	<h3
		className={cn(
			"text-2xl font-semibold leading-none tracking-tight",
			className,
		)}
		{...props}
	/>
);
CardTitle.displayName = "CardTitle";

const CardDescription = ({
	className,
	...props
}: React.ComponentProps<"p">) => (
	<p className={cn("text-sm text-muted-foreground", className)} {...props} />
);
CardDescription.displayName = "CardDescription";

const CardContent = ({ className, ...props }: React.ComponentProps<"div">) => (
	<div className={cn("p-6 pt-0", className)} {...props} />
);
CardContent.displayName = "CardContent";

const CardFooter = ({ className, ...props }: React.ComponentProps<"div">) => (
	<div className={cn("flex items-center p-6 pt-0", className)} {...props} />
);
CardFooter.displayName = "CardFooter";

export {
	Card,
	CardHeader,
	CardFooter,
	CardTitle,
	CardDescription,
	CardContent,
};
