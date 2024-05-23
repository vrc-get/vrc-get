"use client"

import React from "react";
import {Card} from "@/components/ui/card";

export function VStack({className, children}: { className?: string, children: React.ReactNode }) {
	return (
		<div className={`flex flex-col overflow-hidden w-full gap-3 ${className}`}>
			{children}
		</div>
	);
}

export function HNavBar({className, children}: { className?: string, children: React.ReactNode }) {
	return (
		<Card className={`${className} mx-auto px-4 py-2 w-full max-w-screen-2xl`}>
			<div className="mx-auto flex flex-wrap items-center justify-between text-primary gap-2">
				{children}
			</div>
		</Card>
	)
}
