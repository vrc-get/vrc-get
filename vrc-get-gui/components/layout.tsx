"use client";

import type React from "react";
import { Card } from "@/components/ui/card";
import { cn } from "@/lib/utils";

export function VStack({
	className,
	children,
}: {
	className?: string;
	children: React.ReactNode;
}) {
	return (
		<div className={`flex flex-col w-full gap-3 compact:gap-2 ${className}`}>
			{children}
		</div>
	);
}

export function HNavBar({
	className,
	leading,
	trailing,
	commonClassName,
	leadingClassName,
	trailingClassName,
}: {
	className?: string;
	leading: React.ReactNode;
	trailing?: React.ReactNode;
	commonClassName?: string;
	leadingClassName?: string;
	trailingClassName?: string;
}) {
	return (
		<Card
			className={`${className} shrink-0 mx-auto px-2 py-2 w-full compact:p-1`}
		>
			<div className="mx-auto flex flex-wrap items-center justify-between text-primary gap-2 w-full">
				<div
					className={cn(
						"flex items-center gap-2 me-auto grow shrink h-full",
						commonClassName,
						leadingClassName,
					)}
				>
					{leading}
				</div>
				<div
					className={cn(
						"flex items-center gap-2 ms-auto flex-wrap justify-end h-full",
						commonClassName,
						trailingClassName,
					)}
				>
					{trailing}
				</div>
			</div>
		</Card>
	);
}

export function HNavBarText({ children }: { children?: React.ReactNode }) {
	return (
		<div className="-me-2 h-10 px-3 flex items-center grow-0">
			<p className="cursor-pointer font-bold">{children}</p>
		</div>
	);
}
