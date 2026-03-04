import * as AccordionPrimitive from "@radix-ui/react-accordion";
import { ChevronDown } from "lucide-react";
import type * as React from "react";

import { cn } from "@/lib/utils";

const Accordion = ({
	...props
}: React.ComponentProps<typeof AccordionPrimitive.Root>) => {
	return <AccordionPrimitive.Root data-slot="accordion" {...props} />;
};

const AccordionItem = ({
	className,
	...props
}: React.ComponentProps<typeof AccordionPrimitive.Item>) => {
	return (
		<AccordionPrimitive.Item
			data-slot="accordion-item"
			className={cn("border-b", className)}
			{...props}
		/>
	);
};

const AccordionTrigger = ({
	className,
	children,
	...props
}: React.ComponentProps<typeof AccordionPrimitive.Trigger>) => {
	return (
		<AccordionPrimitive.Header className="flex">
			<AccordionPrimitive.Trigger
				data-slot="accordion-trigger"
				className={cn(
					"focus-visible:border-ring focus-visible:ring-ring/50 flex flex-1 items-center justify-between py-4 font-medium outline-none hover:underline focus-visible:ring-[3px] disabled:pointer-events-none disabled:opacity-50 [&[data-state=open]>svg]:rotate-180",
					className,
				)}
				{...props}
			>
				{children}
				<ChevronDown className="size-4 shrink-0 transition-transform duration-200" />
			</AccordionPrimitive.Trigger>
		</AccordionPrimitive.Header>
	);
};

const AccordionContent = ({
	className,
	children,
	...props
}: React.ComponentProps<typeof AccordionPrimitive.Content>) => {
	return (
		<AccordionPrimitive.Content
			data-slot="accordion-content"
			className="overflow-hidden text-sm data-[state=closed]:animate-accordion-up data-[state=open]:animate-accordion-down"
			{...props}
		>
			<div className={cn("pt-0 pb-4", className)}>{children}</div>
		</AccordionPrimitive.Content>
	);
};

export { Accordion, AccordionItem, AccordionTrigger, AccordionContent };
