import { commands } from "@/lib/bindings";
import { cn } from "@/lib/utils";
import { ExternalLink as LucideExternalLink } from "lucide-react";
import type React from "react";

export function ExternalLink({
	children,
	className,
	href,
}: {
	children: React.ReactNode;
	className?: string;
	href?: string;
}): React.JSX.Element {
	const body = (
		<>
			{children}
			<LucideExternalLink
				className={"inline ml-1 size-[1.1em] align-text-top"}
			/>
		</>
	);
	if (href) {
		return (
			<a
				className={cn(className, "underline inline")}
				type={"button"}
				// biome-ignore lint/a11y/useValidAnchor: This is navigation with external browser, not a action
				onClick={() => commands.utilOpenUrl(href)}
			>
				{body}
			</a>
		);
	} else {
		return <span className={className}>{body}</span>;
	}
}
