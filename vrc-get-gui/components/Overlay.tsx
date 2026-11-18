/*
 * Copyright (c) 2023 anatawa12 and other contributors
 * SPDX-License-Identifier: MPL-2.0
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org.
 */

import React from "react";
import { cn } from "@/lib/utils";

/**
 * Overlays multiple elements to one place with grid layout
 *
 * This allows centering text and putting button at left / right
 *
 * @param children The contents to be overlay
 * @param className the className of the container div
 * @constructor
 */
export function Overlay({
	children,
	className,
}: {
	className?: string;
	children?: React.ReactNode;
}) {
	return (
		<div className={cn("grid", className)}>
			{React.Children.map(children, (child) => {
				if (React.isValidElement(child)) {
					const childElement = child as React.ReactHTMLElement<HTMLElement>;
					return React.cloneElement(childElement, {
						style: {
							...childElement.props.style,
							gridArea: "1/1/2/2",
						},
					});
				} else {
					return <div style={{ gridArea: "1/1/2/2" }}>{child}</div>;
				}
			})}
		</div>
	);
}
