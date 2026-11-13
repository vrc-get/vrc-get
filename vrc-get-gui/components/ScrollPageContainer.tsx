/*
 * Copyright (c) 2023 anatawa12 and other contributors
 * SPDX-License-Identifier: MPL-2.0
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org.
 */

import { ScrollArea } from "@/components/ui/scroll-area";

export function ScrollPageContainer({
	children,
	className,
	viewportClassName,
}: {
	children: React.ReactNode;
	className?: string;
	viewportClassName?: string;
}) {
	return (
		<ScrollArea
			className={`-mr-3 pr-3 compact:mr-0 ${className}`}
			scrollBarClassName={"bg-background rounded-full border-l-0 p-[1.5px]"}
			viewportClassName={`${viewportClassName}`}
		>
			{children}
		</ScrollArea>
	);
}
