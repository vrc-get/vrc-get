"use client";

import { Card } from "@/components/ui/card";
import type React from "react";

export function VStack({
	className,
	children,
}: { className?: string; children: React.ReactNode }) {
	return (
		<div className={`flex flex-col w-full gap-3 ${className}`}>{children}</div>
	);
}

export function HNavBar({
	className,
	leading,
	trailing,
	commonClassName,
}: {
	className?: string;
	leading: React.ReactNode;
	trailing?: React.ReactNode;
	commonClassName?: string;
}) {
	return (
		<Card className={`${className} mx-auto px-4 py-2 w-full`}>
			<div className="mx-auto flex flex-wrap items-center justify-between text-primary gap-2">
				<div
					className={`flex items-center gap-2 ${commonClassName} me-auto flex-grow flex-shrink`}
				>
					{leading}
				</div>
				<div
					className={`flex items-center gap-2 ${commonClassName} ms-auto flex-wrap justify-end`}
				>
					{trailing}
				</div>
			</div>
		</Card>
	);
}
