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

export function ScrollableCard({
	children,
	className,
}: {
	children: React.ReactNode;
	className?: string;
}) {
	return (
		<Card className={cn("pl-2.5 pt-2.5 overflow-hidden flex", className)}>
			<ScrollArea
				type="always"
				className={"w-full shrink overflow-hidden vrc-get-scrollable-card"}
				scrollBarClassName={"bg-background pb-2.5"}
			>
				{children}
				<div className={"pb-2.5"} />
				<ScrollBar className={"bg-background"} orientation="horizontal" />
			</ScrollArea>
		</Card>
	);
}
