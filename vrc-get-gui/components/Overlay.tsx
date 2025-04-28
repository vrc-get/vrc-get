import { cn } from "@/lib/utils";
import React from "react";

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
}: { className?: string; children?: React.ReactNode }) {
	return (
		<div className={cn("grid", className)}>
			{React.Children.map(children, (child, i) => {
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
