"use client"

import React from "react";
import {Navbar, Card, Typography} from "@material-tailwind/react";

export function VStack({className, children}: { className?: string, children: React.ReactNode }) {
	return (
		<div className={`flex flex-col overflow-hidden w-full gap-3 ${className}`}>
			{children}
		</div>
	);
}

export function HNavBar({className, children}: { className?: string, children: React.ReactNode }) {
	return (
		<Navbar className={`${className} mx-auto px-4 py-2`}>
			<div className="container mx-auto flex flex-wrap items-center justify-between text-blue-gray-900 gap-2">
				{children}
			</div>
		</Navbar>
	)
}

export function HContent({className, children}: { className?: string, children?: React.ReactNode }) {
	return (
	<div className={"flex flex-col gap-2 flex-shrink overflow-y-auto flex-grow " + className}>
		{children}
	</div>
	)
}

export function HSection({className, children}: { className?: string, children?: React.ReactNode }) {
	return (
		<Card className={"p-4 flex flex-col gap-2 " + className}>
			{children}
		</Card>
	)
}

export function HSectionRow({className, children}: { className?: string, children?: React.ReactNode }) {
	return (
		<div className={"flex gap-1 items-center " + className}>
			{children}
		</div>
	)
}

export function HSectionTitle({className, children}: { className?: string, children?: React.ReactNode }) {
	return (
		<Typography variant="h4" className={"block " + className}>{children}</Typography>
	)
}

export function HSectionSubTitle({className, children}: { className?: string, children?: React.ReactNode }) {
	return (
		<Typography variant="h5" className={"block font-normal " + className}>{children}</Typography>
	)
}

export function HSectionText({className, children}: { className?: string, children?: React.ReactNode }) {
	return (
		<Typography variant="paragraph" className={"block text-wrap " + className}>{children}</Typography>
	)
}
