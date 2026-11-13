/*
 * Copyright (c) 2023 anatawa12 and other contributors
 * SPDX-License-Identifier: MPL-2.0
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org.
 */

import type React from "react";
import { Card } from "@/components/ui/card";
import { ScrollArea, ScrollBar } from "@/components/ui/scroll-area";
import { cn } from "@/lib/utils";

export const ScrollableCardTable = ({
	children,
	className,
	...props
}: React.ComponentProps<typeof ScrollArea> & {
	children: React.ReactNode;
	className?: string;
}) => {
	return (
		<Card className={cn("overflow-hidden", className)}>
			<ScrollArea
				type="auto"
				className="h-full w-full vrc-get-scrollable-card"
				scrollBarClassName="bg-background py-2.5 vrc-get-scrollable-card-vertical-bar"
				{...props}
			>
				<table className="relative table-auto text-left w-full">
					{children}
				</table>
				<div className={"pb-2.5"} />
				<ScrollBar
					orientation="horizontal"
					className="bg-background ps-2.5 vrc-get-scrollable-card-horizontal-bar"
				/>
			</ScrollArea>
		</Card>
	);
};

ScrollableCardTable.displayName = "ScrollableCardTable";
