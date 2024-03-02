"use client"

import React from "react";
import {Navbar} from "@material-tailwind/react";

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
