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
			<button
				className={cn(className, "underline")}
				type={"button"}
				onClick={() => commands.utilOpenUrl(href)}
			>
				{body}
			</button>
		);
	} else {
		return <span className={className}>{body}</span>;
	}
}
